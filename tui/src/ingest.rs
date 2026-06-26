//! Residential-IP ingestion. The Cloudflare Worker's datacenter IP is bot-blocked
//! by the portals, but the user's machine isn't — so `kontu pull` does the Oikotie
//! token handshake + cards fetch here, then pushes the raw cards to the Worker's
//! `/api/import` endpoint to normalize and store (reusing the Worker's logic).

use std::time::Duration;

use anyhow::{anyhow, Context, Result};
use reqwest::{Client, RequestBuilder};
use serde_json::{json, Value};

use crate::api::KontuClient;

const UA: &str =
    "Mozilla/5.0 (X11; Linux x86_64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/126.0.0.0 Safari/537.36";
const CARDS_PER_PAGE: usize = 24;
const DETAIL_CONCURRENCY: usize = 5;

/// Strip HTML tags and decode the few entities Oikotie uses, collapsing whitespace.
fn strip_html(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    let mut in_tag = false;
    for c in s.chars() {
        match c {
            '<' => in_tag = true,
            '>' => in_tag = false,
            _ if !in_tag => out.push(c),
            _ => {}
        }
    }
    out.replace("&nbsp;", " ")
        .replace("&amp;", "&")
        .replace("&euro;", "€")
        .replace("&auml;", "ä")
        .replace("&ouml;", "ö")
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
}

/// Parse an Oikotie listing detail page into `{details:{label:value}, fullDescription}`.
/// The structured info-table (Kunto, Rannan omistus, Lämmitys, …) is the reliable
/// signal; the free-text paragraphs add fibre/neighbour hints the card lacks. Each
/// value is scoped to before the next title so a multi-line field (e.g. the
/// renovations list, whose `dd` carries a modifier class) can't swallow the next.
fn parse_detail(html: &str) -> Value {
    let mut details = serde_json::Map::new();
    const TITLE: &str = "info-table__title\">";
    let mut pos = 0;
    while let Some(t) = html[pos..].find(TITLE) {
        let kstart = pos + t + TITLE.len();
        let kend = match html[kstart..].find("</dt>") {
            Some(e) => kstart + e,
            None => break,
        };
        let key = strip_html(&html[kstart..kend]);
        let next_title = html[kend..].find(TITLE).map(|x| kend + x).unwrap_or(html.len());
        let scope = &html[kend..next_title];
        let val = scope.find("info-table__value").and_then(|vc| {
            let after_class = vc + "info-table__value".len();
            let gt = scope[after_class..].find('>')? + after_class + 1;
            let end = scope[gt..].find("</dd>")? + gt;
            Some(strip_html(&scope[gt..end]))
        });
        if let Some(val) = val
            && !key.is_empty()
            && !val.is_empty()
        {
            details.entry(key).or_insert(Value::String(val));
        }
        pos = kend + "</dt>".len();
    }
    let mut desc = String::new();
    let mut p = 0;
    while let Some(t) = html[p..].find("<p class=\"paragraph") {
        let tag = p + t;
        if let Some(gt) = html[tag..].find('>') {
            let cstart = tag + gt + 1;
            if let Some(end) = html[cstart..].find("</p>") {
                let para = strip_html(&html[cstart..cstart + end]);
                if !para.is_empty() {
                    desc.push_str(&para);
                    desc.push(' ');
                }
                p = cstart + end;
                continue;
            }
        }
        p = tag + 1;
    }
    json!({ "details": details, "fullDescription": desc.trim() })
}

async fn fetch_detail(http: &Client, url: &str) -> Option<Value> {
    let html = http.get(url).send().await.ok()?.text().await.ok()?;
    Some(parse_detail(&html))
}

/// Fetch a listing's full HD gallery image URLs from its Oikotie detail page —
/// the embedded `"images":[{url}]` JSON points at the watermarked thumbor CDN,
/// which is directly hotlinkable. Best-effort: empty on any error or non-Oikotie.
pub async fn fetch_gallery(url: &str) -> Vec<String> {
    let Ok(http) = Client::builder()
        .user_agent(UA)
        .timeout(std::time::Duration::from_secs(20))
        .build()
    else {
        return Vec::new();
    };
    let Ok(resp) = http
        .get(url)
        .header("Referer", "https://asunnot.oikotie.fi/")
        .send()
        .await
    else {
        return Vec::new();
    };
    let Ok(html) = resp.text().await else {
        return Vec::new();
    };
    let prefix = "\"url\":\"";
    let host = "https://cdn.asunnot.oikotie.fi/";
    let mut out: Vec<String> = Vec::new();
    let mut rest = html.as_str();
    while let Some(p) = rest.find(prefix) {
        let after = &rest[p + prefix.len()..];
        let Some(q) = after.find('"') else { break };
        let u = &after[..q];
        if u.starts_with(host) && u.contains("mediabank") && !out.iter().any(|x| x == u) {
            out.push(u.to_string());
        }
        rest = &after[q..];
        if out.len() >= 30 {
            break;
        }
    }
    out
}

/// The detail-page URL for an Oikotie card. `/api/cards` usually carries `url`, but
/// fall back to `links.self` and then the canonical id-derived path so enrichment
/// never silently no-ops if a card omits the field.
fn oikotie_card_url(c: &Value) -> Option<String> {
    if let Some(u) = c.get("url").and_then(Value::as_str) {
        return Some(u.to_string());
    }
    if let Some(u) = c.pointer("/links/self").and_then(Value::as_str) {
        return Some(u.to_string());
    }
    let id = c
        .get("id")
        .or_else(|| c.get("cardId"))
        .and_then(|v| v.as_i64().map(|n| n.to_string()).or_else(|| v.as_str().map(str::to_string)))?;
    Some(format!("https://asunnot.oikotie.fi/myytavat-asunnot/{id}"))
}

/// Fetch each card's detail page (bounded concurrency) and fold the structured
/// info-table + full description back into the card for the Worker to normalize.
async fn enrich_cards(http: &Client, cards: &mut [Value]) {
    use futures::stream::{self, StreamExt};
    let jobs: Vec<(usize, String)> = cards
        .iter()
        .enumerate()
        .filter_map(|(i, c)| oikotie_card_url(c).map(|u| (i, u)))
        .collect();
    let enriched: Vec<(usize, Option<Value>)> = stream::iter(jobs)
        .map(|(i, url)| {
            let http = http.clone();
            async move { (i, fetch_detail(&http, &url).await) }
        })
        .buffer_unordered(DETAIL_CONCURRENCY)
        .collect()
        .await;
    for (i, d) in enriched {
        if let (Some(d), Some(obj)) = (d, cards[i].as_object_mut()) {
            obj.insert("details".into(), d.get("details").cloned().unwrap_or(Value::Null));
            obj.insert("fullDescription".into(), d.get("fullDescription").cloned().unwrap_or(Value::Null));
        }
    }
}

struct Session {
    token: String,
    cuid: String,
    loaded: String,
}

/// Extract a `<meta name="NAME" content="VALUE">` value, scoped to the single
/// meta tag that carries `name="NAME"` (any attribute order). Returns `None`
/// rather than leaking a neighbouring tag's content when that tag has none.
fn meta(html: &str, name: &str) -> Option<String> {
    let anchor = format!("name=\"{name}\"");
    let pos = html.find(&anchor)?;
    let tag_start = html[..pos].rfind('<')?;
    let rel_end = html[tag_start..].find('>')?;
    let tag = &html[tag_start..tag_start + rel_end];
    let cidx = tag.find("content=\"")? + "content=\"".len();
    let end = tag[cidx..].find('"')?;
    Some(tag[cidx..cidx + end].to_string())
}

/// Oikotie buildingType[] bitmask codes (1=kerrostalo, 2=rivitalo, 4=omakotitalo
/// confirmed live; the rest are best-effort). Unmapped types contribute no code,
/// so an empty result means "all types".
fn building_type_codes(types: &[String]) -> Vec<i64> {
    types
        .iter()
        .filter_map(|t| {
            let t = t.to_lowercase();
            if t.contains("omakoti") {
                Some(4)
            } else if t.contains("rivi") {
                Some(2)
            } else if t.contains("pari") {
                Some(64)
            } else if t.contains("kerros") {
                Some(1)
            } else if t.contains("erillis") {
                Some(32)
            } else if t.contains("luhti") {
                Some(256)
            } else if t.contains("mökki") || t.contains("mokki") || t.contains("loma") || t.contains("vapaa") {
                Some(8)
            } else {
                None
            }
        })
        .collect()
}

async fn handshake(http: &Client) -> Result<Session> {
    let html = http
        .get("https://asunnot.oikotie.fi/myytavat-asunnot")
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
    loc: Option<(i64, i64, String)>,
    building_types: &[i64],
    card_type: i64,
    shore: bool,
    price_max: Option<i64>,
    limit: usize,
) -> Result<Vec<Value>> {
    let mut cards: Vec<Value> = Vec::new();
    let mut offset = 0usize;
    loop {
        let mut q: Vec<(String, String)> = vec![
            ("cardType".into(), card_type.to_string()),
            ("limit".into(), CARDS_PER_PAGE.to_string()),
            ("offset".into(), offset.to_string()),
            ("sortBy".into(), "published_desc".into()),
        ];
        if let Some((id, t, name)) = &loc {
            q.push(("locations".into(), format!("[[{},{},\"{}\"]]", id, t, name)));
        }
        for bt in building_types {
            q.push(("buildingType[]".into(), bt.to_string()));
        }
        if shore {
            q.push(("shoreOwnershipType[]".into(), "2".into()));
            q.push(("shoreOwnershipType[]".into(), "4".into()));
        }
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
    municipality: Option<&str>,
    property_types: &[String],
    shore: bool,
    price_max: Option<i64>,
    limit: usize,
    deep: bool,
) -> Result<Value> {
    let http = Client::builder()
        .user_agent(UA)
        .gzip(true)
        .timeout(Duration::from_secs(20))
        .connect_timeout(Duration::from_secs(8))
        .build()?;
    let session = handshake(&http).await?;
    let loc = match municipality {
        Some(m) => Some(resolve_location(&http, &session, m).await?),
        None => None,
    };
    // Tontit (plots) are a separate Oikotie card category (cardType 104) and carry
    // no residential buildingType, so skip the building-type filter for them.
    let is_tontti = property_types.iter().any(|t| t.to_lowercase().contains("tontti"));
    let card_type = if is_tontti { 104 } else { 100 };
    let codes = if is_tontti { Vec::new() } else { building_type_codes(property_types) };
    let mut cards = fetch_cards(&http, &session, loc, &codes, card_type, shore, price_max, limit).await?;
    if cards.is_empty() {
        return Ok(json!({ "received": 0, "inserted": 0, "updated": 0, "skipped": 0 }));
    }
    if deep {
        enrich_cards(&http, &mut cards).await;
    }
    client.import_oikotie(&cards).await
}

const ETUOVI_SEARCH: &str = "https://www.etuovi.com/api/v3/announcements/search/listpage";
const ETUOVI_SUGGEST: &str = "https://www.etuovi.com/api/v2/location/suggestions";

/// Split requested types into Etuovi residential subtypes + a leisure (cottage) flag.
fn etuovi_groups(types: &[String]) -> (Vec<&'static str>, bool) {
    let mut residential = Vec::new();
    let mut leisure = false;
    for t in types {
        let t = t.to_lowercase();
        if t.contains("omakoti") {
            residential.push("DETACHED_HOUSE");
        } else if t.contains("pari") {
            residential.push("SEMI_DETACHED_HOUSE");
        } else if t.contains("rivi") {
            residential.push("ROW_HOUSE");
        } else if t.contains("erillis") {
            residential.push("SEPARATE_HOUSE");
        } else if t.contains("kerros") {
            residential.push("APARTMENT_HOUSE");
        } else if t.contains("mökki") || t.contains("mokki") || t.contains("loma") || t.contains("vapaa") {
            leisure = true;
        }
    }
    if residential.is_empty() && !leisure {
        residential.push("DETACHED_HOUSE");
    }
    (residential, leisure)
}

async fn resolve_etuovi_location(http: &Client, name: &str) -> Result<Value> {
    let resp: Value = http
        .get(ETUOVI_SUGGEST)
        .query(&[("q", name)])
        .send()
        .await?
        .json()
        .await?;
    let arr = resp
        .as_array()
        .cloned()
        .or_else(|| resp.get("results").and_then(|v| v.as_array()).cloned())
        .unwrap_or_default();
    let pick = arr
        .iter()
        .find(|x| {
            x.get("type")
                .and_then(Value::as_str)
                .map(|t| t.contains("CITY"))
                .unwrap_or(false)
        })
        .or_else(|| arr.first());
    if let Some(p) = pick
        && let Some(code) = p.get("code").and_then(Value::as_str) {
            let typ = p.get("type").and_then(Value::as_str).unwrap_or("CITY");
            return Ok(json!([{ "code": code, "type": typ }]));
        }
    Ok(json!([]))
}

#[allow(clippy::too_many_arguments)]
async fn fetch_etuovi(
    http: &Client,
    group: &str,
    field: &str,
    subtypes: &[&str],
    loc_terms: &Value,
    shore: bool,
    price_max: Option<i64>,
    limit: usize,
) -> Result<Vec<Value>> {
    let mut out: Vec<Value> = Vec::new();
    let mut page = 1usize;
    loop {
        let mut body = json!({
            "propertyType": group,
            "locationSearchCriteria": { "unclassifiedLocationTerms": [], "classifiedLocationTerms": loc_terms },
            "sellerType": "ALL",
            "newBuildingSearchCriteria": "ALL_PROPERTIES",
            "pagination": {
                "firstResult": Value::Null,
                "maxResults": 50,
                "page": page,
                "sortingOrder": { "property": "SEARCH_PRICE", "direction": "ASC" }
            }
        });
        body[field] = json!(subtypes);
        if let Some(pm) = price_max {
            body["priceMax"] = json!(pm);
        }
        if shore {
            body["hasShore"] = json!(true);
        }
        let resp: Value = http
            .post(ETUOVI_SEARCH)
            .json(&body)
            .send()
            .await
            .context("etuovi search request")?
            .json()
            .await
            .context("decoding etuovi response")?;
        let total = resp
            .get("countOfAllResults")
            .or_else(|| resp.get("totalCount"))
            .or_else(|| resp.get("total"))
            .or_else(|| resp.get("totalResults"))
            .or_else(|| resp.get("count"))
            .and_then(Value::as_i64);
        let batch = resp
            .get("announcements")
            .and_then(Value::as_array)
            .cloned()
            .unwrap_or_default();
        let got = batch.len();
        out.extend(batch);
        page += 1;
        if got < 50 || total.is_some_and(|t| out.len() as i64 >= t) || out.len() >= limit {
            break;
        }
        tokio::time::sleep(Duration::from_millis(800)).await;
    }
    out.truncate(limit);
    Ok(out)
}

/// Pull Etuovi listings from the local IP and import them into the Worker.
pub async fn pull_etuovi(
    client: &KontuClient,
    municipality: Option<&str>,
    property_types: &[String],
    shore: bool,
    price_max: Option<i64>,
    limit: usize,
) -> Result<Value> {
    use reqwest::header::{HeaderMap, HeaderValue};
    let mut headers = HeaderMap::new();
    headers.insert("X-PORTAL-IDENTIFIER", HeaderValue::from_static("ETUOVI"));
    headers.insert(reqwest::header::ORIGIN, HeaderValue::from_static("https://www.etuovi.com"));
    headers.insert(reqwest::header::REFERER, HeaderValue::from_static("https://www.etuovi.com/"));
    let http = Client::builder()
        .user_agent(UA)
        .default_headers(headers)
        .gzip(true)
        .timeout(Duration::from_secs(20))
        .connect_timeout(Duration::from_secs(8))
        .build()?;

    let loc_terms = match municipality {
        Some(m) => resolve_etuovi_location(&http, m).await.unwrap_or_else(|_| json!([])),
        None => json!([]),
    };

    let (residential, leisure) = etuovi_groups(property_types);
    let mut announcements: Vec<Value> = Vec::new();
    if !residential.is_empty() {
        announcements.extend(
            fetch_etuovi(&http, "RESIDENTIAL", "residentialPropertyTypes", &residential, &loc_terms, shore, price_max, limit)
                .await
                .unwrap_or_default(),
        );
    }
    if leisure && announcements.len() < limit {
        let remaining = limit - announcements.len();
        announcements.extend(
            fetch_etuovi(&http, "LEISURE", "leisurePropertyTypes", &["COTTAGE"], &loc_terms, shore, price_max, remaining)
                .await
                .unwrap_or_default(),
        );
    }
    announcements.truncate(limit);
    if announcements.is_empty() {
        return Ok(json!({ "received": 0, "inserted": 0, "updated": 0, "skipped": 0 }));
    }
    client.import_etuovi(&announcements).await
}
