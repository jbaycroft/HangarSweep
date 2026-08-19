use anyhow::{anyhow, Result};
use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine as _};
use rand::Rng;
use sha2::{Digest, Sha256};
use tauri::Emitter;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpListener;

use crate::db::{self, Character};

/// EVE SSO client ID (public — safe to embed in source).
/// Override at build time with the `EVE_CLIENT_ID` environment variable.
pub const CLIENT_ID: &str = if let Some(s) = option_env!("EVE_CLIENT_ID") {
    s
} else {
    "99de062eccd44b149d8b5075b7b821f4"
};

/// EVE SSO client secret.
/// **Never commit a live secret here.** Set `EVE_CLIENT_SECRET` at build time:
///   - CI/CD: stored as a GitHub Actions repository secret
///   - Local dev: `$env:EVE_CLIENT_SECRET="…"; npm run build:exe`
///   - Or register your own app at https://developers.eveonline.com/
pub const CLIENT_SECRET: &str = if let Some(s) = option_env!("EVE_CLIENT_SECRET") {
    s
} else {
    "" // auth will fail gracefully without this; set EVE_CLIENT_SECRET at build time
};

pub const CALLBACK_PORT: u16 = 57423;
pub const REDIRECT_URI: &str  = "http://localhost:57423/callback";
pub const SCOPES: &str = "publicData esi-universe.read_structures.v1 esi-assets.read_assets.v1 esi-ui.open_window.v1 esi-ui.write_waypoint.v1";


// ─── PKCE helpers ─────────────────────────────────────────────────────────────

pub struct PkceParams {
    pub verifier: String,
    pub challenge: String,
    pub state: String,
}

pub fn generate_pkce() -> PkceParams {
    let mut rng = rand::thread_rng();

    // 32 random bytes → base64url → code_verifier
    let verifier_bytes: Vec<u8> = (0..32).map(|_| rng.gen()).collect();
    let verifier = URL_SAFE_NO_PAD.encode(&verifier_bytes);

    // SHA-256(verifier) → base64url → code_challenge
    let mut hasher = Sha256::new();
    hasher.update(verifier.as_bytes());
    let challenge = URL_SAFE_NO_PAD.encode(hasher.finalize());

    // 16 random bytes → base64url → state
    let state_bytes: Vec<u8> = (0..16).map(|_| rng.gen()).collect();
    let state = URL_SAFE_NO_PAD.encode(&state_bytes);

    PkceParams { verifier, challenge, state }
}

pub fn build_auth_url(pkce: &PkceParams) -> String {
    format!(
        "https://login.eveonline.com/v2/oauth/authorize\
         ?response_type=code\
         &redirect_uri={}\
         &client_id={}\
         &scope={}\
         &code_challenge={}\
         &code_challenge_method=S256\
         &state={}",
        urlencoding_encode(REDIRECT_URI),
        CLIENT_ID,
        urlencoding_encode(SCOPES),
        pkce.challenge,
        pkce.state,
    )
}

fn urlencoding_encode(s: &str) -> String {
    s.bytes()
        .map(|b| match b {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                (b as char).to_string()
            }
            _ => format!("%{:02X}", b),
        })
        .collect()
}

// ─── Token exchange ───────────────────────────────────────────────────────────

#[derive(serde::Deserialize)]
struct TokenResponse {
    access_token: String,
    refresh_token: String,
    expires_in: i64,
}

pub async fn exchange_code(code: &str, verifier: &str) -> Result<(String, String, i64)> {
    let client = reqwest::Client::new();

    let credentials = URL_SAFE_NO_PAD.encode(format!("{}:{}", CLIENT_ID, CLIENT_SECRET).as_bytes());

    let resp = client
        .post("https://login.eveonline.com/v2/oauth/token")
        .header("Authorization", format!("Basic {}", credentials))
        .header("Content-Type", "application/x-www-form-urlencoded")
        .body(format!(
            "grant_type=authorization_code&code={}&redirect_uri={}&code_verifier={}",
            code,
            urlencoding_encode(REDIRECT_URI),
            verifier,
        ))
        .send()
        .await?
        .error_for_status()?;

    let token: TokenResponse = resp.json().await?;
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_secs() as i64;
    let expiry = now + token.expires_in;

    Ok((token.access_token, token.refresh_token, expiry))
}

pub async fn refresh_token(refresh_token: &str) -> Result<(String, String, i64)> {
    let client = reqwest::Client::new();
    let credentials = URL_SAFE_NO_PAD.encode(format!("{}:{}", CLIENT_ID, CLIENT_SECRET).as_bytes());

    let resp = client
        .post("https://login.eveonline.com/v2/oauth/token")
        .header("Authorization", format!("Basic {}", credentials))
        .header("Content-Type", "application/x-www-form-urlencoded")
        .body(format!("grant_type=refresh_token&refresh_token={}", refresh_token))
        .send()
        .await?
        .error_for_status()?;

    let token: TokenResponse = resp.json().await?;
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_secs() as i64;
    let expiry = now + token.expires_in;

    Ok((token.access_token, token.refresh_token, expiry))
}

// ─── JWT sub-claim decoding ───────────────────────────────────────────────────

pub fn extract_character_id(access_token: &str) -> Result<i64> {
    let parts: Vec<&str> = access_token.split('.').collect();
    if parts.len() < 2 {
        return Err(anyhow!("Invalid JWT format"));
    }
    // Pad if needed for standard base64url decode
    let padded = {
        let s = parts[1];
        let rem = s.len() % 4;
        if rem == 0 { s.to_string() } else { format!("{}{}", s, "=".repeat(4 - rem)) }
    };
    let decoded = URL_SAFE_NO_PAD.decode(&parts[1]).or_else(|_| {
        base64::engine::general_purpose::URL_SAFE.decode(&padded)
    })?;
    let payload: serde_json::Value = serde_json::from_slice(&decoded)?;
    let sub = payload["sub"].as_str().ok_or_else(|| anyhow!("Missing sub claim"))?;
    // sub format: "CHARACTER:EVE:12345678"
    sub.split(':').last()
        .ok_or_else(|| anyhow!("Unexpected sub format: {}", sub))?
        .parse::<i64>()
        .map_err(|e| anyhow!("Failed to parse character ID: {}", e))
}

// ─── Character name lookup ────────────────────────────────────────────────────

pub async fn fetch_character_name(character_id: i64, access_token: &str) -> Result<String> {
    #[derive(serde::Deserialize)]
    struct Verify {
        #[serde(rename = "CharacterName")]
        character_name: String,
    }
    let client = reqwest::Client::new();
    let resp = client
        .get("https://esi.evetech.net/verify/")
        .bearer_auth(access_token)
        .send()
        .await?;

    if resp.status().is_success() {
        let v: Verify = resp.json().await?;
        return Ok(v.character_name);
    }

    // Fallback: direct ESI character endpoint
    #[derive(serde::Deserialize)]
    struct EsiChar { name: String }
    let resp2 = client
        .get(&format!("https://esi.evetech.net/latest/characters/{}/", character_id))
        .send()
        .await?;
    let c: EsiChar = resp2.json().await?;
    Ok(c.name)
}

// ─── Automatic token refresh guard ───────────────────────────────────────────

/// Returns a valid access token, refreshing it in the DB if it will expire in < 60 s.
pub async fn ensure_valid_token(
    character_id: i64,
    pool: &sqlx::SqlitePool,
) -> Result<String> {
    let char = db::get_character(pool, character_id).await?;
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_secs() as i64;

    if char.token_expiry - now < 60 {
        let (new_access, new_refresh, new_expiry) = refresh_token(&char.refresh_token).await?;
        db::update_tokens(pool, character_id, &new_access, &new_refresh, new_expiry).await?;
        Ok(new_access)
    } else {
        Ok(char.access_token)
    }
}

// ─── Callback TCP listener ────────────────────────────────────────────────────

/// Spawned as a background task when login begins.
/// Waits for EVE SSO to redirect the browser to our localhost callback,
/// exchanges the code for tokens, stores the character in SQLite, and
/// emits auth-complete / auth-error events to the frontend.
pub async fn run_callback_listener(
    verifier: String,
    expected_state: String,
    pool: sqlx::SqlitePool,
    app: tauri::AppHandle,
) {
    let listener = match TcpListener::bind(format!("127.0.0.1:{}", CALLBACK_PORT)).await {
        Ok(l) => l,
        Err(e) => {
            let _ = app.emit("auth-error", serde_json::json!({ "message": format!("Could not bind port {}: {}", CALLBACK_PORT, e) }));
            return;
        }
    };

    if let Ok((mut stream, _)) = listener.accept().await {
        let mut buf = vec![0u8; 8192];
        let n = stream.read(&mut buf).await.unwrap_or(0);
        let request = String::from_utf8_lossy(&buf[..n]);

        // Parse: GET /callback?code=...&state=... HTTP/1.1
        let code_and_state = request
            .lines()
            .next()
            .and_then(|line| line.split_whitespace().nth(1))
            .and_then(|path| path.split_once('?').map(|(_, qs)| qs.to_string()));

        let html_close = "HTTP/1.1 200 OK\r\nContent-Type: text/html\r\nConnection: close\r\n\r\n\
            <html><head><style>body{background:#0a0e1a;color:#c8d0e0;font-family:sans-serif;\
            display:flex;align-items:center;justify-content:center;height:100vh;margin:0}\
            h2{color:#f0c040}</style></head><body>\
            <div><h2>&#x2713; HangarSweep</h2><p>Authentication complete. Close this tab and return to the app.</p></div>\
            </body></html>";

        let html_err = "HTTP/1.1 400 Bad Request\r\nContent-Type: text/html\r\nConnection: close\r\n\r\n\
            <html><head><style>body{background:#0a0e1a;color:#c8d0e0;font-family:sans-serif;\
            display:flex;align-items:center;justify-content:center;height:100vh;margin:0}</style></head><body>\
            <p>Authentication failed. State mismatch or missing code.</p></body></html>";

        match code_and_state {
            None => {
                let _ = stream.write_all(html_err.as_bytes()).await;
                let _ = app.emit("auth-error", serde_json::json!({ "message": "No callback query string received" }));
            }
            Some(qs) => {
                let params: std::collections::HashMap<&str, &str> = qs
                    .split('&')
                    .filter_map(|pair| pair.split_once('='))
                    .collect();

                let code = params.get("code").copied().unwrap_or("");
                let state = params.get("state").copied().unwrap_or("");

                if state != expected_state || code.is_empty() {
                    let _ = stream.write_all(html_err.as_bytes()).await;
                    let _ = app.emit("auth-error", serde_json::json!({ "message": "State mismatch or empty code" }));
                    return;
                }

                let _ = stream.write_all(html_close.as_bytes()).await;

                // Exchange code → tokens
                match exchange_code(code, &verifier).await {
                    Err(e) => {
                        let _ = app.emit("auth-error", serde_json::json!({ "message": format!("Token exchange failed: {}", e) }));
                    }
                    Ok((access, refresh, expiry)) => {
                        match extract_character_id(&access) {
                            Err(e) => {
                                let _ = app.emit("auth-error", serde_json::json!({ "message": format!("JWT parse failed: {}", e) }));
                            }
                            Ok(char_id) => {
                                let name = fetch_character_name(char_id, &access)
                                    .await
                                    .unwrap_or_else(|_| char_id.to_string());

                                let character = Character {
                                    id: char_id,
                                    name: name.clone(),
                                    access_token: access,
                                    refresh_token: refresh,
                                    token_expiry: expiry,
                                };

                                if let Err(e) = db::upsert_character(&pool, &character).await {
                                    let _ = app.emit("auth-error", serde_json::json!({ "message": format!("DB write failed: {}", e) }));
                                } else {
                                    let _ = app.emit("auth-complete", serde_json::json!({ "character_id": char_id, "character_name": name }));
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}

// ─── Unit tests ───────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use base64::engine::general_purpose::URL_SAFE_NO_PAD;
    use sha2::{Digest, Sha256};

    // ── PKCE generation ───────────────────────────────────────────────────────

    #[test]
    fn test_pkce_verifier_is_valid_base64url() {
        let pkce = generate_pkce();
        // base64url chars are A-Z a-z 0-9 - _   (no padding)
        assert!(
            pkce.verifier.chars().all(|c| c.is_alphanumeric() || c == '-' || c == '_'),
            "verifier contains non-base64url characters: {}",
            pkce.verifier
        );
        assert!(!pkce.verifier.is_empty());
    }

    #[test]
    fn test_pkce_challenge_is_sha256_of_verifier() {
        let pkce = generate_pkce();
        // Re-derive: SHA-256(verifier_bytes) → base64url
        let mut hasher = Sha256::new();
        hasher.update(pkce.verifier.as_bytes());
        let expected = URL_SAFE_NO_PAD.encode(hasher.finalize());
        assert_eq!(pkce.challenge, expected);
    }

    #[test]
    fn test_pkce_state_is_valid_base64url() {
        let pkce = generate_pkce();
        assert!(
            pkce.state.chars().all(|c| c.is_alphanumeric() || c == '-' || c == '_'),
            "state contains non-base64url characters"
        );
        assert!(!pkce.state.is_empty());
    }

    #[test]
    fn test_pkce_generates_different_values_each_call() {
        let a = generate_pkce();
        let b = generate_pkce();
        // Astronomically unlikely to collide; if this ever fails, RNG is broken
        assert_ne!(a.verifier, b.verifier);
        assert_ne!(a.state, b.state);
    }

    #[test]
    fn test_pkce_verifier_minimum_length() {
        // RFC 7636: code_verifier MUST be 43-128 chars
        let pkce = generate_pkce();
        assert!(pkce.verifier.len() >= 43, "verifier too short: {}", pkce.verifier.len());
        assert!(pkce.verifier.len() <= 128, "verifier too long: {}", pkce.verifier.len());
    }

    // ── Auth URL construction ─────────────────────────────────────────────────

    #[test]
    fn test_build_auth_url_contains_response_type() {
        let pkce = generate_pkce();
        let url = build_auth_url(&pkce);
        assert!(url.contains("response_type=code"), "missing response_type: {url}");
    }

    #[test]
    fn test_build_auth_url_contains_challenge_method() {
        let pkce = generate_pkce();
        let url = build_auth_url(&pkce);
        assert!(url.contains("code_challenge_method=S256"), "missing S256 method: {url}");
    }

    #[test]
    fn test_build_auth_url_contains_state() {
        let pkce = generate_pkce();
        let url = build_auth_url(&pkce);
        assert!(url.contains(&pkce.state), "state not found in URL: {url}");
    }

    #[test]
    fn test_build_auth_url_contains_challenge() {
        let pkce = generate_pkce();
        let url = build_auth_url(&pkce);
        assert!(url.contains(&pkce.challenge), "challenge not found in URL: {url}");
    }

    #[test]
    fn test_build_auth_url_points_to_eve_login() {
        let pkce = generate_pkce();
        let url = build_auth_url(&pkce);
        assert!(
            url.starts_with("https://login.eveonline.com/v2/oauth/authorize"),
            "wrong base URL: {url}"
        );
    }

    // ── URL encoding ──────────────────────────────────────────────────────────

    #[test]
    fn test_urlencoding_passthrough_safe_chars() {
        let input = "abcABC0129-_.~";
        let encoded = urlencoding_encode(input);
        assert_eq!(encoded, input, "safe chars should pass through unchanged");
    }

    #[test]
    fn test_urlencoding_encodes_space() {
        assert_eq!(urlencoding_encode(" "), "%20");
    }

    #[test]
    fn test_urlencoding_encodes_colon_slash() {
        let encoded = urlencoding_encode("http://example.com/path?a=b");
        assert!(encoded.contains("%3A"), "colon should be encoded");
        assert!(encoded.contains("%2F"), "slash should be encoded");
        assert!(encoded.contains("%3F"), "? should be encoded");
        assert!(encoded.contains("%3D"), "= should be encoded");
    }

    #[test]
    fn test_urlencoding_empty_string() {
        assert_eq!(urlencoding_encode(""), "");
    }

    #[test]
    fn test_urlencoding_encodes_ampersand() {
        assert_eq!(urlencoding_encode("a&b"), "a%26b");
    }

    // ── JWT character ID extraction ───────────────────────────────────────────

    fn make_test_jwt(sub: &str) -> String {
        // Build a minimal JWT: header.payload.sig (sig can be anything for parsing tests)
        let header = URL_SAFE_NO_PAD.encode(r#"{"alg":"RS256","typ":"JWT"}"#);
        let payload_json = format!(r#"{{"sub":"{}","exp":9999999999}}"#, sub);
        let payload = URL_SAFE_NO_PAD.encode(&payload_json);
        format!("{header}.{payload}.fakesig")
    }

    #[test]
    fn test_extract_character_id_valid_sub() {
        let jwt = make_test_jwt("CHARACTER:EVE:90379793");
        let result = extract_character_id(&jwt);
        assert_eq!(result.unwrap(), 90_379_793i64);
    }

    #[test]
    fn test_extract_character_id_large_id() {
        let jwt = make_test_jwt("CHARACTER:EVE:2119604465");
        assert_eq!(extract_character_id(&jwt).unwrap(), 2_119_604_465i64);
    }

    #[test]
    fn test_extract_character_id_not_a_jwt() {
        assert!(extract_character_id("notajwt").is_err());
    }

    #[test]
    fn test_extract_character_id_only_one_part() {
        assert!(extract_character_id("onlyone").is_err());
    }

    #[test]
    fn test_extract_character_id_bad_payload_encoding() {
        // invalid base64 in payload position
        assert!(extract_character_id("header.!!!.sig").is_err());
    }

    #[test]
    fn test_extract_character_id_missing_sub_field() {
        let header = URL_SAFE_NO_PAD.encode(r#"{"alg":"RS256"}"#);
        let payload = URL_SAFE_NO_PAD.encode(r#"{"exp":9999}"#); // no sub
        let jwt = format!("{header}.{payload}.sig");
        assert!(extract_character_id(&jwt).is_err());
    }

    #[test]
    fn test_extract_character_id_non_numeric_id() {
        let jwt = make_test_jwt("CHARACTER:EVE:not-a-number");
        assert!(extract_character_id(&jwt).is_err());
    }
}
