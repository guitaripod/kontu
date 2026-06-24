//! Residential-IP ingestion. The Cloudflare Worker's datacenter IP is bot-blocked
//! by the portals, but the user's machine isn't — so `kontu pull` does the Oikotie
//! token handshake + cards fetch here, then pushes the raw cards to the Worker's
//! `/api/import` endpoint to normalize and store (reusing the Worker's logic).

use std::time::Duration;

use anyhow::{anyhow, Context, Result};
use reqwest::{Client, RequestBuilder};
use serde_json::Value;

use crate::api::KontuClient;

const UA: &str =
    "Mozilla/5.0 (X11; Linux x86_64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/126.0.0.0 Safari/537.36";
const CARDS_PER_PAGE: usize = 24;

struct Session {
    token: String,
    cuid: String,
    loaded: String,
}

/// Extract a `<meta name="NAME" content="VALUE">` value from the page.
fn meta(html: &str, name: &str) -> Option<String> {
    let anchor = format!("name=\"{name}\"");
    let start = html.find(&anchor)? + anchor.len();
    let rest = &html[start..];
    let cidx = rest.find("content=\"")? + "content=\"".len();
    let end = rest[cidx..].find('"')?;
    Some(rest[cidx..cidx + end].to_string())
}

async fn handshake(http: &Client, municipality: &str) -> Result<Session> {
    let url = format!(
        "https://asunnot.oikotie.fi/myytavat-asunnot/{}",
        municipality.to_lowercase()
    );
    let html = http
        .get(&url)
        .send()
        .await
        .context("fetching oikotie search page")?
        .text()
        .await?;
    Ok(Session {
        token: meta(&html, "api-token")
            .ok_or_else(|| anyhow!("oikotie: api-token meta not found (page layout changed?)"))?,
        cuid: meta(&html, "cuid").ok_or_else(|| anyhow!("oikotie: cuid meta not found"))?,
        loaded: meta(&html, "loaded").ok_or_else(|| anyhow!("oikotie: loaded meta not found"))?,
    })
}

fn with_tokens(req: RequestBuilder, s: &Session) -> RequestBuilder {
    req.header("OTA-token", &s.token)
        .header("OTA-cuid", &s.cuid)
        .header("OTA-loaded", &s.loaded)
        .header("Accept", "application/json")
}

/// Resolve a municipality name to Oikotie's `(cardId, cardType, name)` location triple.
async fn resolve_location(http: &Client, s: &Session, query: &str) -> Result<(i64, i64, String)> {
    let resp: Value = with_tokens(
        http.get("https://asunnot.oikotie.fi/api/3.0/location")
            .query(&[("query", query)]),
        s,
    )
    .send()
    .await
    .context("oikotie location lookup")?
    .json()
    .await
    .context("decoding oikotie location response")?;

    let arr = resp
        .as_array()
        .ok_or_else(|| anyhow!("oikotie location: unexpected response"))?;
    let pick = arr
        .iter()
        .find(|it| {
            it.pointer("/card/cardType").and_then(Value::as_i64) == Some(6)
                && it
                    .pointer("/card/name")
                    .and_then(Value::as_str)
                    .map(|n| n.eq_ignore_ascii_case(query))
                    .unwrap_or(false)
        })
        .or_else(|| arr.first())
        .ok_or_else(|| anyhow!("location '{query}' not found on oikotie"))?;

    let id = pick
        .pointer("/card/cardId")
        .and_then(Value::as_i64)
        .ok_or_else(|| anyhow!("oikotie location missing cardId"))?;
    let ctype = pick.pointer("/card/cardType").and_then(Value::as_i64).unwrap_or(6);
    let name = pick
        .pointer("/card/name")
        .and_then(Value::as_str)
        .unwrap_or(query)
        .to_string();
    Ok((id, ctype, name))
}

async fn fetch_cards(
    http: &Client,
    s: &Session,
    loc: (i64, i64, String),
    price_max: Option<i64>,
    limit: usize,
) -> Result<Vec<Value>> {
    let locations = format!("[[{},{},\"{}\"]]", loc.0, loc.1, loc.2);
    let mut cards: Vec<Value> = Vec::new();
    let mut offset = 0usize;
    loop {
        let mut q: Vec<(String, String)> = vec![
            ("cardType".into(), "100".into()),
            ("locations".into(), locations.clone()),
            ("limit".into(), CARDS_PER_PAGE.to_string()),
            ("offset".into(), offset.to_string()),
            ("sortBy".into(), "published_desc".into()),
        ];
        if let Some(pm) = price_max {
            q.push(("price[max]".into(), pm.to_string()));
        }
        let resp: Value = with_tokens(
            http.get("https://asunnot.oikotie.fi/api/cards").query(&q),
            s,
        )
        .send()
        .await
        .context("oikotie cards request")?
        .json()
        .await
        .context("decoding oikotie cards response")?;

        let found = resp.get("found").and_then(Value::as_i64).unwrap_or(0);
        let batch = resp
            .get("cards")
            .and_then(Value::as_array)
            .cloned()
            .unwrap_or_default();
        let got = batch.len();
        cards.extend(batch);
        offset += CARDS_PER_PAGE;
        if got < CARDS_PER_PAGE || cards.len() as i64 >= found || cards.len() >= limit {
            break;
        }
        tokio::time::sleep(Duration::from_millis(600)).await;
    }
    cards.truncate(limit);
    Ok(cards)
}

/// Pull Oikotie for-sale listings for a municipality from the local (residential)
/// IP and import them into the Worker. Returns the import summary.
pub async fn pull_oikotie(
    client: &KontuClient,
    municipality: &str,
    price_max: Option<i64>,
    limit: usize,
) -> Result<Value> {
    let http = Client::builder().user_agent(UA).gzip(true).build()?;
    let session = handshake(&http, municipality).await?;
    let loc = resolve_location(&http, &session, municipality).await?;
    let cards = fetch_cards(&http, &session, loc, price_max, limit).await?;
    if cards.is_empty() {
        return Ok(serde_json::json!({ "received": 0, "inserted": 0, "updated": 0, "skipped": 0 }));
    }
    client.import_oikotie(&cards).await
}
