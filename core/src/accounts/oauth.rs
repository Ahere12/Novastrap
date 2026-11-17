//! Roblox OAuth 2.0 (authorization code + PKCE, public client, loopback redirect).
//! Docs: https://create.roblox.com/docs/cloud/auth/oauth2-overview
//! Novastrap never sees the user's password: sign-in happens on roblox.com in the browser.

use crate::error::{AppError, AppResult};
use base64::Engine;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::io::{BufRead, BufReader, Write};
use std::net::TcpListener;
use std::time::{Duration, Instant};

pub const AUTHORIZE_URL: &str = "https://apis.roblox.com/oauth/v1/authorize";
pub const TOKEN_URL: &str = "https://apis.roblox.com/oauth/v1/token";
pub const REVOKE_URL: &str = "https://apis.roblox.com/oauth/v1/token/revoke";
pub const USERINFO_URL: &str = "https://apis.roblox.com/oauth/v1/userinfo";
pub const REDIRECT_PORT: u16 = 47821;
pub const REDIRECT_URI: &str = "http://localhost:47821/oauth/callback";
pub const SCOPES: &str = "openid profile";

const UNRESERVED: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789-._~";

pub fn random_token(len: usize) -> String {
    let mut bytes = vec![0u8; len];
    getrandom::fill(&mut bytes).expect("OS RNG unavailable");
    bytes.iter().map(|b| UNRESERVED[*b as usize % UNRESERVED.len()] as char).collect()
}

pub fn pkce_challenge(verifier: &str) -> String {
    base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(Sha256::digest(verifier.as_bytes()))
}

fn encode(s: &str) -> String {
    s.bytes()
        .map(|b| if UNRESERVED.contains(&b) { (b as char).to_string() } else { format!("%{b:02X}") })
        .collect()
}

fn decode(s: &str) -> String {
    let bytes = s.as_bytes();
    let mut out = Vec::with_capacity(bytes.len());
    let mut i = 0;
    while i < bytes.len() {
        match bytes[i] {
            b'%' if i + 2 < bytes.len() => {
                match std::str::from_utf8(&bytes[i + 1..i + 3]).ok().and_then(|h| u8::from_str_radix(h, 16).ok()).ok_or(()) {
                    Ok(b) => {
                        out.push(b);
                        i += 3;
                        continue;
                    }
                    Err(_) => out.push(b'%'),
                }
            }
            b'+' => out.push(b' '),
            b => out.push(b),
        }
        i += 1;
    }
    String::from_utf8_lossy(&out).into_owned()
}

pub struct PendingLogin {
    pub url: String,
    pub state: String,
    pub verifier: String,
}

pub fn begin(client_id: &str) -> AppResult<PendingLogin> {
    if client_id.trim().is_empty() {
        return Err(AppError::InvalidInput(
            "Add your Roblox OAuth client ID in Settings → Accounts first.".into(),
        ));
    }
    let verifier = random_token(64);
    let state = random_token(32);
    let url = format!(
        "{AUTHORIZE_URL}?client_id={}&redirect_uri={}&scope={}&response_type=code&state={}&code_challenge={}&code_challenge_method=S256",
        encode(client_id.trim()),
        encode(REDIRECT_URI),
        encode(SCOPES),
        state,
        pkce_challenge(&verifier)
    );
    Ok(PendingLogin { url, state, verifier })
}

/// Parses the request line `GET /oauth/callback?code=..&state=.. HTTP/1.1`.
pub fn parse_callback(request_line: &str, expected_state: &str) -> AppResult<String> {
    let target = request_line.split_whitespace().nth(1).unwrap_or("");
    let (path, query) = target.split_once('?').unwrap_or((target, ""));
    if path != "/oauth/callback" {
        return Err(AppError::InvalidInput("unexpected callback path".into()));
    }
    let param = |k: &str| {
        query.split('&').find_map(|kv| kv.split_once('=').filter(|(a, _)| *a == k).map(|(_, v)| decode(v)))
    };
    if let Some(err) = param("error") {
        let desc = param("error_description").unwrap_or_default();
        return Err(AppError::InvalidInput(format!("Roblox sign-in was cancelled or failed: {err} {desc}")));
    }
    if param("state").as_deref() != Some(expected_state) {
        return Err(AppError::Verification("sign-in state mismatch; please try again".into()));
    }
    param("code").filter(|c| !c.is_empty()).ok_or_else(|| AppError::InvalidInput("no authorization code".into()))
}

/// Blocks until the browser hits the loopback redirect (or `timeout`). Run on a worker thread.
pub fn wait_for_code(listener: TcpListener, expected_state: &str, timeout: Duration) -> AppResult<String> {
    listener.set_nonblocking(true)?;
    let deadline = Instant::now() + timeout;
    loop {
        match listener.accept() {
            Ok((mut stream, _)) => {
                stream.set_nonblocking(false)?;
                stream.set_read_timeout(Some(Duration::from_secs(5)))?;
                let mut line = String::new();
                BufReader::new(&stream).read_line(&mut line)?;
                if !line.contains("/oauth/callback") {
                    let _ = stream.write_all(b"HTTP/1.1 404 Not Found\r\nContent-Length: 0\r\n\r\n");
                    continue;
                }
                let result = parse_callback(&line, expected_state);
                let body = match &result {
                    Ok(_) => "<h2>Signed in to Novastrap.</h2><p>You can close this tab.</p>",
                    Err(_) => "<h2>Sign-in didn't complete.</h2><p>Return to Novastrap for details.</p>",
                };
                let page = format!(
                    "<!doctype html><meta charset=utf-8><title>Novastrap</title><body style=\"font-family:system-ui;background:#0b0c11;color:#e8e9f1;padding:48px\">{body}</body>"
                );
                let _ = write!(
                    stream,
                    "HTTP/1.1 200 OK\r\nContent-Type: text/html; charset=utf-8\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{page}",
                    page.len()
                );
                return result;
            }
            Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                if Instant::now() > deadline {
                    return Err(AppError::Network("sign-in timed out".into()));
                }
                std::thread::sleep(Duration::from_millis(150));
            }
            Err(e) => return Err(e.into()),
        }
    }
}

pub fn bind_listener() -> AppResult<TcpListener> {
    TcpListener::bind(("127.0.0.1", REDIRECT_PORT)).map_err(|e| {
        AppError::Busy(format!("port {REDIRECT_PORT} is in use, so sign-in can't receive Roblox's reply ({e})"))
    })
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Tokens {
    pub access_token: String,
    pub refresh_token: String,
    /// Unix seconds.
    pub expires_at: u64,
}

impl Tokens {
    pub fn expired(&self, now: u64) -> bool {
        now + 60 >= self.expires_at
    }
}

#[derive(Deserialize)]
struct TokenResponse {
    access_token: String,
    refresh_token: String,
    expires_in: u64,
}

pub fn now_secs() -> u64 {
    std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_secs()).unwrap_or(0)
}

async fn token_request(client: &reqwest::Client, form: &[(&str, &str)]) -> AppResult<Tokens> {
    let res = client.post(TOKEN_URL).form(form).send().await?;
    if !res.status().is_success() {
        return Err(AppError::HttpStatus(res.status().as_u16()));
    }
    let t: TokenResponse = res.json().await?;
    Ok(Tokens { access_token: t.access_token, refresh_token: t.refresh_token, expires_at: now_secs() + t.expires_in })
}

pub async fn exchange(client: &reqwest::Client, client_id: &str, code: &str, verifier: &str) -> AppResult<Tokens> {
    token_request(
        client,
        &[
            ("grant_type", "authorization_code"),
            ("client_id", client_id),
            ("code", code),
            ("code_verifier", verifier),
            ("redirect_uri", REDIRECT_URI),
        ],
    )
    .await
}

pub async fn refresh(client: &reqwest::Client, client_id: &str, refresh_token: &str) -> AppResult<Tokens> {
    token_request(
        client,
        &[("grant_type", "refresh_token"), ("client_id", client_id), ("refresh_token", refresh_token)],
    )
    .await
}

pub async fn revoke(client: &reqwest::Client, client_id: &str, token: &str) {
    let _ = client.post(REVOKE_URL).form(&[("client_id", client_id), ("token", token)]).send().await;
}

#[derive(Debug, Clone, Deserialize)]
pub struct UserInfo {
    pub sub: String,
    #[serde(default)]
    pub name: Option<String>,
    #[serde(default)]
    pub nickname: Option<String>,
    #[serde(default)]
    pub preferred_username: Option<String>,
}

pub async fn userinfo(client: &reqwest::Client, access_token: &str) -> AppResult<UserInfo> {
    let res = client.get(USERINFO_URL).bearer_auth(access_token).send().await?;
    if !res.status().is_success() {
        return Err(AppError::HttpStatus(res.status().as_u16()));
    }
    Ok(res.json().await?)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pkce_matches_rfc7636_example() {
        // RFC 7636 appendix B.
        assert_eq!(
            pkce_challenge("dBjftJeZ4CVP-mB92K27uhbUJU1p1r_wW1gFWFOEjXk"),
            "E9Melhoa2OwvFrEMTJguCHaoeK1t8URWbuGJSstw-cM"
        );
    }

    #[test]
    fn random_tokens_are_unreserved_and_unique() {
        let a = random_token(64);
        assert_eq!(a.len(), 64);
        assert!(a.bytes().all(|b| UNRESERVED.contains(&b)));
        assert_ne!(a, random_token(64));
    }

    #[test]
    fn begin_requires_client_id() {
        assert!(begin("  ").is_err());
        let p = begin("123456").unwrap();
        assert!(p.url.starts_with(AUTHORIZE_URL));
        assert!(p.url.contains("code_challenge_method=S256"));
        assert!(p.url.contains("redirect_uri=http%3A%2F%2Flocalhost%3A47821%2Foauth%2Fcallback"));
        assert!(p.url.contains("scope=openid%20profile"));
    }

    #[test]
    fn callback_parsing() {
        assert_eq!(parse_callback("GET /oauth/callback?code=abc%2B1&state=s HTTP/1.1", "s").unwrap(), "abc+1");
        assert_eq!(parse_callback("GET /oauth/callback?code=abc&state=bad HTTP/1.1", "s").unwrap_err().kind(), "verification");
        assert!(parse_callback("GET /oauth/callback?error=access_denied&state=s HTTP/1.1", "s").is_err());
        assert!(parse_callback("GET /other?code=x&state=s HTTP/1.1", "s").is_err());
    }

    #[test]
    fn expiry() {
        let t = Tokens { access_token: "a".into(), refresh_token: "r".into(), expires_at: 1000 };
        assert!(t.expired(950));
        assert!(!t.expired(100));
    }

    #[test]
    fn loopback_round_trip() {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let port = listener.local_addr().unwrap().port();
        let h = std::thread::spawn(move || wait_for_code(listener, "st", Duration::from_secs(5)));
        let mut s = std::net::TcpStream::connect(("127.0.0.1", port)).unwrap();
        s.write_all(b"GET /oauth/callback?code=thecode&state=st HTTP/1.1\r\nHost: x\r\n\r\n").unwrap();
        let mut resp = String::new();
        std::io::Read::read_to_string(&mut s, &mut resp).unwrap();
        assert!(resp.starts_with("HTTP/1.1 200"));
        assert_eq!(h.join().unwrap().unwrap(), "thecode");
    }
}
