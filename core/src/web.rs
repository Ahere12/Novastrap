//! Shared HTTP layer: response cache, per-host rate limiting, retry with backoff,
//! timeouts and typed (validated) JSON decoding. Every Roblox/third-party call goes here.

use crate::error::{AppError, AppResult};
use serde::de::DeserializeOwned;
use std::collections::HashMap;
use std::sync::Mutex;
use std::time::{Duration, Instant};

const MAX_ATTEMPTS: u32 = 3;
/// Minimum spacing between requests to the same host.
const MIN_INTERVAL: Duration = Duration::from_millis(120);
const MAX_CACHE_ENTRIES: usize = 512;

pub struct Api {
    client: reqwest::Client,
    cache: Mutex<HashMap<String, (Instant, String)>>,
    last_request: tokio::sync::Mutex<HashMap<String, Instant>>,
}

pub fn host_of(url: &str) -> &str {
    let rest = url.split_once("://").map_or(url, |(_, r)| r);
    rest.split(['/', '?', '#']).next().unwrap_or("")
}

/// Backoff for attempt `n` (0-based), honouring a server `Retry-After` when given.
pub fn backoff(attempt: u32, retry_after_secs: Option<u64>) -> Duration {
    match retry_after_secs {
        Some(s) => Duration::from_secs(s.min(30)),
        None => Duration::from_millis(400 * 2u64.pow(attempt)),
    }
}

impl Api {
    pub fn new(client: reqwest::Client) -> Self {
        Api { client, cache: Mutex::new(HashMap::new()), last_request: tokio::sync::Mutex::new(HashMap::new()) }
    }

    pub fn client(&self) -> &reqwest::Client {
        &self.client
    }

    fn cached(&self, key: &str, ttl: Duration) -> Option<String> {
        let cache = self.cache.lock().unwrap();
        cache.get(key).filter(|(at, _)| at.elapsed() < ttl).map(|(_, body)| body.clone())
    }

    fn store(&self, key: String, body: String) {
        let mut cache = self.cache.lock().unwrap();
        if cache.len() >= MAX_CACHE_ENTRIES {
            // Drop the oldest entry.
            if let Some(oldest) = cache.iter().min_by_key(|(_, (at, _))| *at).map(|(k, _)| k.clone()) {
                cache.remove(&oldest);
            }
        }
        cache.insert(key, (Instant::now(), body));
    }

    pub fn clear_cache(&self) -> usize {
        let mut c = self.cache.lock().unwrap();
        let n = c.len();
        c.clear();
        n
    }

    async fn throttle(&self, host: &str) {
        let mut map = self.last_request.lock().await;
        if let Some(last) = map.get(host) {
            let since = last.elapsed();
            if since < MIN_INTERVAL {
                tokio::time::sleep(MIN_INTERVAL - since).await;
            }
        }
        map.insert(host.to_owned(), Instant::now());
    }

    /// Sends with retries. Retries network errors, 429 and 5xx only.
    async fn send_text(&self, build: impl Fn() -> reqwest::RequestBuilder, host: &str) -> AppResult<String> {
        let mut attempt = 0;
        loop {
            self.throttle(host).await;
            let result = build().send().await;
            let err = match result {
                Ok(res) if res.status().is_success() => return Ok(res.text().await?),
                Ok(res) => {
                    let code = res.status().as_u16();
                    let retry_after = res
                        .headers()
                        .get("retry-after")
                        .and_then(|v| v.to_str().ok())
                        .and_then(|v| v.parse().ok());
                    (AppError::HttpStatus(code), retry_after)
                }
                Err(e) => (AppError::from(e), None),
            };
            attempt += 1;
            if !err.0.retryable() || attempt >= MAX_ATTEMPTS {
                return Err(err.0);
            }
            log::warn!("{host}: {} (attempt {attempt}), retrying", err.0);
            tokio::time::sleep(backoff(attempt - 1, err.1)).await;
        }
    }

    fn decode<T: DeserializeOwned>(body: &str, url: &str) -> AppResult<T> {
        serde_json::from_str(body)
            .map_err(|e| AppError::UpstreamChanged(format!("{}: {e}", host_of(url))))
    }

    /// GET JSON, served from cache when younger than `ttl` (use `Duration::ZERO` to bypass).
    pub async fn get_json<T: DeserializeOwned>(&self, url: &str, ttl: Duration) -> AppResult<T> {
        if let Some(body) = self.cached(url, ttl) {
            return Self::decode(&body, url);
        }
        let body = self.send_text(|| self.client.get(url), host_of(url)).await?;
        let value = Self::decode(&body, url)?;
        if !ttl.is_zero() {
            self.store(url.to_owned(), body);
        }
        Ok(value)
    }

    /// POST JSON (idempotent lookups only, e.g. username → id). Cached by URL + body.
    pub async fn post_json<T: DeserializeOwned>(
        &self,
        url: &str,
        body: &serde_json::Value,
        ttl: Duration,
    ) -> AppResult<T> {
        let key = format!("POST {url} {body}");
        if let Some(cached) = self.cached(&key, ttl) {
            return Self::decode(&cached, url);
        }
        let text = self.send_text(|| self.client.post(url).json(body), host_of(url)).await?;
        let value = Self::decode(&text, url)?;
        if !ttl.is_zero() {
            self.store(key, text);
        }
        Ok(value)
    }
}

/// Common TTLs.
pub mod ttl {
    use std::time::Duration;
    pub const SHORT: Duration = Duration::from_secs(30);
    pub const MEDIUM: Duration = Duration::from_secs(5 * 60);
    pub const LONG: Duration = Duration::from_secs(60 * 60);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hosts() {
        assert_eq!(host_of("https://games.roblox.com/v1/games?x=1"), "games.roblox.com");
        assert_eq!(host_of("https://a.b"), "a.b");
    }

    #[test]
    fn backoff_rules() {
        assert_eq!(backoff(0, None), Duration::from_millis(400));
        assert_eq!(backoff(2, None), Duration::from_millis(1600));
        assert_eq!(backoff(0, Some(5)), Duration::from_secs(5));
        assert_eq!(backoff(0, Some(999)), Duration::from_secs(30));
    }

    #[test]
    fn cache_respects_ttl_and_bound() {
        let api = Api::new(reqwest::Client::new());
        api.store("k".into(), "v".into());
        assert_eq!(api.cached("k", Duration::from_secs(60)).as_deref(), Some("v"));
        assert_eq!(api.cached("k", Duration::ZERO), None);
        for i in 0..MAX_CACHE_ENTRIES + 10 {
            api.store(format!("k{i}"), "v".into());
        }
        assert!(api.cache.lock().unwrap().len() <= MAX_CACHE_ENTRIES);
        assert!(api.clear_cache() > 0);
    }

    #[test]
    fn decode_reports_upstream_change() {
        #[derive(serde::Deserialize, Debug)]
        #[allow(dead_code)]
        struct T {
            id: u64,
        }
        let e = Api::decode::<T>(r#"{"nope":1}"#, "https://games.roblox.com/x").unwrap_err();
        assert_eq!(e.kind(), "upstream_changed");
    }
}
