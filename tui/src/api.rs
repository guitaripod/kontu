//! HTTP client for the kontu Cloudflare Worker (see `SPEC.md` §9). All `/api/*`
//! calls send the configured bearer token.

use anyhow::{bail, Context, Result};
use reqwest::Client;
use serde::de::DeserializeOwned;
use serde_json::json;

use crate::cost::CostDefaults;
use crate::models::{FilterState, ListingDetail, ListingsPage, SortColumn};

#[derive(Clone)]
pub struct KontuClient {
    http: Client,
    base: String,
    token: String,
}

impl KontuClient {
    pub fn new(base: impl Into<String>, token: impl Into<String>) -> Result<Self> {
        let http = Client::builder()
            .gzip(true)
            .user_agent(concat!("kontu/", env!("CARGO_PKG_VERSION")))
            .build()
            .context("building HTTP client")?;
        Ok(Self {
            http,
            base: base.into().trim_end_matches('/').to_string(),
            token: token.into(),
        })
    }

    fn url(&self, path: &str) -> String {
        format!("{}{}", self.base, path)
    }

    async fn get_json<T: DeserializeOwned>(&self, path: &str, query: &[(String, String)]) -> Result<T> {
        let resp = self
            .http
            .get(self.url(path))
            .bearer_auth(&self.token)
            .query(query)
            .send()
            .await
            .with_context(|| format!("GET {path}"))?;
        Self::parse(resp, path).await
    }

    async fn parse<T: DeserializeOwned>(resp: reqwest::Response, path: &str) -> Result<T> {
        let status = resp.status();
        let body = resp.text().await.unwrap_or_default();
        if !status.is_success() {
            bail!("{path} -> HTTP {status}: {}", body.chars().take(300).collect::<String>());
        }
        serde_json::from_str(&body).with_context(|| format!("decoding {path} response"))
    }

    /// Unauthenticated health probe.
    pub async fn health(&self) -> Result<bool> {
        let resp = self.http.get(self.url("/health")).send().await?;
        Ok(resp.status().is_success())
    }

    pub async fn list_listings(
        &self,
        filter: &FilterState,
        sort: SortColumn,
        descending: bool,
        limit: u32,
        offset: u32,
    ) -> Result<ListingsPage> {
        let mut q = filter.to_query_pairs();
        q.push(("sort".into(), sort.as_param().into()));
        q.push(("order".into(), if descending { "desc" } else { "asc" }.into()));
        q.push(("limit".into(), limit.to_string()));
        q.push(("offset".into(), offset.to_string()));
        self.get_json("/api/listings", &q).await
    }

    pub async fn get_listing(&self, id: i64) -> Result<ListingDetail> {
        self.get_json(&format!("/api/listings/{id}"), &[]).await
    }

    pub async fn cost_defaults(&self) -> Result<CostDefaults> {
        self.get_json("/api/cost-defaults", &[]).await
    }

    pub async fn market(&self, municipality: &str) -> Result<serde_json::Value> {
        self.get_json(&format!("/api/market/{municipality}"), &[]).await
    }

    /// POST raw Oikotie cards to the Worker import endpoint, chunked to stay under
    /// D1 per-invocation query limits. Returns aggregated counts.
    pub async fn import_oikotie(&self, cards: &[serde_json::Value]) -> Result<serde_json::Value> {
        let mut totals = [0u64; 4];
        for chunk in cards.chunks(40) {
            let resp = self
                .http
                .post(self.url("/api/import"))
                .bearer_auth(&self.token)
                .json(&serde_json::json!({ "source": "oikotie", "cards": chunk }))
                .send()
                .await
                .context("POST /api/import")?;
            let v: serde_json::Value = Self::parse(resp, "/api/import").await?;
            for (i, key) in ["received", "inserted", "updated", "skipped"].iter().enumerate() {
                totals[i] += v.get(*key).and_then(serde_json::Value::as_u64).unwrap_or(0);
            }
        }
        Ok(serde_json::json!({
            "received": totals[0], "inserted": totals[1], "updated": totals[2], "skipped": totals[3]
        }))
    }

    /// POST raw Etuovi announcements to the Worker import endpoint, chunked.
    pub async fn import_etuovi(&self, announcements: &[serde_json::Value]) -> Result<serde_json::Value> {
        let mut totals = [0u64; 4];
        for chunk in announcements.chunks(40) {
            let resp = self
                .http
                .post(self.url("/api/import"))
                .bearer_auth(&self.token)
                .json(&serde_json::json!({ "source": "etuovi", "announcements": chunk }))
                .send()
                .await
                .context("POST /api/import")?;
            let v: serde_json::Value = Self::parse(resp, "/api/import").await?;
            for (i, key) in ["received", "inserted", "updated", "skipped"].iter().enumerate() {
                totals[i] += v.get(*key).and_then(serde_json::Value::as_u64).unwrap_or(0);
            }
        }
        Ok(serde_json::json!({
            "received": totals[0], "inserted": totals[1], "updated": totals[2], "skipped": totals[3]
        }))
    }

    /// Fetch raw photo bytes from the Worker's R2-backed photo route.
    pub async fn photo_bytes(&self, key: &str) -> Result<Vec<u8>> {
        let resp = self
            .http
            .get(self.url(&format!("/api/photos/{key}")))
            .bearer_auth(&self.token)
            .send()
            .await
            .with_context(|| format!("GET photo {key}"))?;
        if !resp.status().is_success() {
            bail!("photo {key} -> HTTP {}", resp.status());
        }
        Ok(resp.bytes().await?.to_vec())
    }

    pub async fn trigger_sync(&self) -> Result<serde_json::Value> {
        let resp = self
            .http
            .post(self.url("/api/sync"))
            .bearer_auth(&self.token)
            .send()
            .await
            .context("POST /api/sync")?;
        Self::parse(resp, "/api/sync").await
    }

    pub async fn set_note(&self, id: i64, note: &str) -> Result<()> {
        let resp = self
            .http
            .put(self.url(&format!("/api/listings/{id}/notes")))
            .bearer_auth(&self.token)
            .json(&json!({ "note": note }))
            .send()
            .await?;
        let _: serde_json::Value = Self::parse(resp, "set_note").await?;
        Ok(())
    }

    pub async fn set_score(&self, id: i64, score: i32, deal_breaker: bool) -> Result<()> {
        let resp = self
            .http
            .put(self.url(&format!("/api/listings/{id}/score")))
            .bearer_auth(&self.token)
            .json(&json!({ "score": score, "deal_breaker": deal_breaker }))
            .send()
            .await?;
        let _: serde_json::Value = Self::parse(resp, "set_score").await?;
        Ok(())
    }

    /// Publish (or refresh) a listing's public web page from a computed snapshot.
    pub async fn publish_page(&self, id: i64, tier: &str, payload: serde_json::Value) -> Result<()> {
        let resp = self
            .http
            .post(self.url("/api/publish"))
            .bearer_auth(&self.token)
            .json(&json!({ "id": id, "tier": tier, "payload": payload }))
            .send()
            .await?;
        let _: serde_json::Value = Self::parse(resp, "publish_page").await?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn builds_normalized_urls() {
        let c = KontuClient::new("http://localhost:8787/", "tok").unwrap();
        assert_eq!(c.url("/api/listings"), "http://localhost:8787/api/listings");
        assert_eq!(c.url("/health"), "http://localhost:8787/health");
    }
}
