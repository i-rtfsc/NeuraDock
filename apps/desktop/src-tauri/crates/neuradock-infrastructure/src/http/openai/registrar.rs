/// OpenAI account registrar — Chrome TLS fingerprinting via rquest.
///
/// Mirrors the flow from codex-manager (Python / curl_cffi / Chrome impersonate):
///   Phase 1 — Registration:  signup → password → OTP₁ → create_account
///   Phase 2 — Login/consent: new OAuth session → login → OTP₂ → consent →
///              workspace select → redirect chain → code → token exchange
///
/// Uses rquest with Chrome124 emulation to obtain the `oai-client-auth-session`
/// cookie that encodes the workspace_id — this requires Chrome TLS fingerprinting.

use std::collections::HashSet;
use std::sync::Arc;

use anyhow::{anyhow, bail, Result};
use base64::Engine;
use rand::Rng;
use rquest::cookie::{CookieStore, Jar};
use rquest_util::Emulation;
use serde_json::Value;

// ─── Constants ────────────────────────────────────────────────────────────────

const OAUTH_CLIENT_ID: &str = "app_EMoamEEZ73f0CkXaXp7hrann";
const OAUTH_AUTH_URL: &str = "https://auth.openai.com/oauth/authorize";
const OAUTH_TOKEN_URL: &str = "https://auth.openai.com/oauth/token";
const CODEX_REDIRECT_URI: &str = "http://localhost:1455/auth/callback";
const CODEX_SCOPE: &str =
    "openid profile email offline_access api.connectors.read api.connectors.invoke";
const CODEX_ORIGINATOR: &str = "codex_cli_rs";
const AUTH_OPENAI_BASE: &str = "https://auth.openai.com/";

const SENTINEL_URL: &str = "https://sentinel.openai.com/backend-api/sentinel/req";
const SIGNUP_URL: &str = "https://auth.openai.com/api/accounts/authorize/continue";
const REGISTER_URL: &str = "https://auth.openai.com/api/accounts/user/register";
const SEND_OTP_URL: &str = "https://auth.openai.com/api/accounts/email-otp/send";
const VALIDATE_OTP_URL: &str = "https://auth.openai.com/api/accounts/email-otp/validate";
const CREATE_ACCOUNT_URL: &str = "https://auth.openai.com/api/accounts/create_account";
const PASSWORD_VERIFY_URL: &str = "https://auth.openai.com/api/accounts/password/verify";
const SELECT_WORKSPACE_URL: &str = "https://auth.openai.com/api/accounts/workspace/select";

// ─── Result types ─────────────────────────────────────────────────────────────

#[derive(Debug, Clone)]
pub struct RegistrationResult {
    pub email: String,
    pub password: String,
    pub access_token: String,
    pub refresh_token: String,
    pub id_token: String,
    pub account_id: String,
    pub expires_at: String,
    pub web_session_cookie: Option<String>,
    pub web_session_device_id: Option<String>,
}

// ─── Crypto / random helpers ──────────────────────────────────────────────────

fn generate_pkce() -> (String, String) {
    use rand::RngCore;
    let mut raw = vec![0u8; 96];
    rand::thread_rng().fill_bytes(&mut raw);
    let verifier = base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(&raw);
    let challenge = {
        use sha2::{Digest, Sha256};
        let digest = Sha256::digest(verifier.as_bytes());
        base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(digest)
    };
    (verifier, challenge)
}

fn random_state() -> String {
    use rand::distributions::Alphanumeric;
    rand::thread_rng()
        .sample_iter(&Alphanumeric)
        .take(24)
        .map(char::from)
        .collect()
}

fn generate_password() -> String {
    use rand::distributions::Alphanumeric;
    let base: String = rand::thread_rng()
        .sample_iter(&Alphanumeric)
        .take(12)
        .map(char::from)
        .collect();
    format!("{}!A7z", base)
}

fn random_user_info() -> (String, String) {
    const NAMES: &[&str] = &[
        "Alex", "Jordan", "Morgan", "Taylor", "Casey", "Riley", "Quinn", "Avery", "Peyton",
        "Blake", "Drew", "Reese", "Logan", "Hayden", "Cameron",
    ];
    let name = NAMES[rand::thread_rng().gen_range(0..NAMES.len())];
    let year = rand::thread_rng().gen_range(1979u32..2002);
    let month = rand::thread_rng().gen_range(1u32..=12);
    let day = rand::thread_rng().gen_range(1u32..=28);
    (name.to_string(), format!("{}-{:02}-{:02}", year, month, day))
}

// ─── JWT helpers ──────────────────────────────────────────────────────────────

fn parse_jwt_claims(id_token: &str) -> Value {
    let segment = id_token.splitn(3, '.').nth(1).unwrap_or("");
    base64::engine::general_purpose::URL_SAFE_NO_PAD
        .decode(segment)
        .ok()
        .and_then(|b| serde_json::from_slice(&b).ok())
        .unwrap_or(Value::Null)
}

fn extract_account_id(claims: &Value) -> String {
    claims
        .pointer("/https:~1~1api.openai.com~1auth/chatgpt_account_id")
        .or_else(|| {
            claims
                .get("https://api.openai.com/auth")
                .and_then(|v| v.get("chatgpt_account_id"))
        })
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_string()
}

// ─── HTTP client setup ────────────────────────────────────────────────────────

/// Detect proxy URL from environment variables (HTTP_PROXY / HTTPS_PROXY).
/// rquest with BoringSSL does not auto-read macOS system proxy settings the same way
/// reqwest with native-tls does, so we need to detect and pass it explicitly.
fn detect_env_proxy() -> Option<String> {
    for key in &["HTTPS_PROXY", "https_proxy", "HTTP_PROXY", "http_proxy", "ALL_PROXY", "all_proxy"] {
        if let Ok(v) = std::env::var(key) {
            let v = v.trim().to_string();
            if !v.is_empty() {
                return Some(v);
            }
        }
    }
    None
}

fn build_clients(proxy_url: Option<&str>) -> Result<(rquest::Client, rquest::Client, Arc<Jar>)> {
    let jar = Arc::new(Jar::default());

    // Resolve effective proxy: explicit > env var auto-detect
    let effective_proxy = proxy_url
        .map(|s| s.to_string())
        .or_else(detect_env_proxy);

    let mut follow_b = rquest::Client::builder()
        .connect_timeout(std::time::Duration::from_secs(15))
        .timeout(std::time::Duration::from_secs(60))
        .cookie_provider(Arc::clone(&jar))
        .emulation(Emulation::Chrome124)
        .redirect(rquest::redirect::Policy::limited(10));

    let mut nore_b = rquest::Client::builder()
        .connect_timeout(std::time::Duration::from_secs(15))
        .timeout(std::time::Duration::from_secs(60))
        .cookie_provider(Arc::clone(&jar))
        .emulation(Emulation::Chrome124)
        .redirect(rquest::redirect::Policy::none());

    if let Some(proxy) = effective_proxy.as_deref() {
        let p = rquest::Proxy::all(proxy)?;
        follow_b = follow_b.proxy(p.clone());
        nore_b = nore_b.proxy(p);
    }

    Ok((follow_b.build()?, nore_b.build()?, jar))
}

// ─── URL helpers ──────────────────────────────────────────────────────────────

fn build_auth_url(code_challenge: &str, state: &str) -> String {
    format!(
        "{url}?client_id={cid}&response_type=code\
         &redirect_uri={redir}&scope={scope}&state={state}\
         &code_challenge={cc}&code_challenge_method=S256\
         &prompt=login&id_token_add_organizations=true\
         &codex_cli_simplified_flow=true&originator={orig}",
        url = OAUTH_AUTH_URL,
        cid = OAUTH_CLIENT_ID,
        redir = urlencoding::encode(CODEX_REDIRECT_URI),
        scope = urlencoding::encode(CODEX_SCOPE),
        state = state,
        cc = code_challenge,
        orig = CODEX_ORIGINATOR,
    )
}

fn resolve_url(base: &str, location: &str) -> String {
    if location.starts_with("http://") || location.starts_with("https://") {
        return location.to_string();
    }
    if location.starts_with('/') {
        if let Ok(parsed) = url::Url::parse(base) {
            return format!(
                "{}://{}{}",
                parsed.scheme(),
                parsed.host_str().unwrap_or(""),
                location
            );
        }
    }
    format!("{}/{}", base.trim_end_matches('/'), location)
}

fn extract_code_from_url(callback_url: &str) -> Result<String> {
    let query = callback_url.split('?').nth(1).unwrap_or("");
    for part in query.split('&') {
        let mut kv = part.splitn(2, '=');
        if kv.next() == Some("code") {
            let val = kv.next().unwrap_or("");
            let decoded = urlencoding::decode(val)
                .map(|s| s.into_owned())
                .unwrap_or_else(|_| val.to_string());
            if !decoded.is_empty() {
                return Ok(decoded);
            }
        }
    }
    bail!(
        "callback URL 中未找到 code 参数: {}",
        &callback_url[..callback_url.len().min(200)]
    )
}

fn read_cookie(jar: &Jar, base_url: &str, name: &str) -> Option<String> {
    let url = url::Url::parse(base_url).ok()?;
    let hv = jar.cookies(&url)?;
    let header_str = hv.to_str().ok()?.to_string();
    for part in header_str.split(';') {
        let part = part.trim();
        if let Some(rest) = part.strip_prefix(name) {
            if rest.starts_with('=') {
                return Some(rest[1..].to_string());
            }
        }
    }
    None
}

// ─── Shared request helpers ───────────────────────────────────────────────────

/// GET auth_url, return (device_id, final_html, final_url).
async fn init_oauth_session(
    follow: &rquest::Client,
    jar: &Arc<Jar>,
    auth_url: &str,
) -> Result<(String, String, String)> {
    let resp = follow
        .get(auth_url)
        .header("Accept", "text/html,application/xhtml+xml,*/*")
        .header("Accept-Language", "en-US,en;q=0.9")
        .send()
        .await
        .map_err(|e| anyhow!("获取 auth 页失败: {}", e))?;
    let final_url = resp.url().as_str().to_string();
    let html = resp.text().await.unwrap_or_default();
    let device_id = read_cookie(jar, AUTH_OPENAI_BASE, "oai-did").unwrap_or_default();
    Ok((device_id, html, final_url))
}

/// Request a sentinel token (best-effort; returns empty string on failure).
async fn get_sentinel(follow: &rquest::Client, device_id: &str) -> String {
    if device_id.is_empty() {
        return String::new();
    }
    async {
        let r = follow
            .post(SENTINEL_URL)
            .header("origin", "https://sentinel.openai.com")
            .header(
                "referer",
                "https://sentinel.openai.com/backend-api/sentinel/frame.html?sv=20260219f9f6",
            )
            .header("content-type", "text/plain;charset=UTF-8")
            .body(format!(
                r#"{{"p":"","id":"{}","flow":"authorize_continue"}}"#,
                device_id
            ))
            .send()
            .await
            .ok()?;
        if !r.status().is_success() {
            return None;
        }
        let j: Value = r.json().await.ok()?;
        j.get("token")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string())
    }
    .await
    .unwrap_or_default()
}

fn merge_cookie_headers(jar: &Arc<Jar>) -> Option<String> {
    let bases = [AUTH_OPENAI_BASE, "https://openai.com/", "https://chatgpt.com/"];
    let mut seen = HashSet::new();
    let mut pairs = Vec::new();

    for base_url in bases {
        let Some(url) = url::Url::parse(base_url).ok() else {
            continue;
        };
        let Some(header_value) = jar.cookies(&url) else {
            continue;
        };
        let Ok(header_str) = header_value.to_str() else {
            continue;
        };

        for part in header_str.split(';') {
            let entry = part.trim();
            if entry.is_empty() {
                continue;
            }

            let Some((name, value)) = entry.split_once('=') else {
                continue;
            };

            let key = name.trim().to_string();
            if key.is_empty() || !seen.insert(key.clone()) {
                continue;
            }

            pairs.push(format!("{}={}", key, value.trim()));
        }
    }

    (!pairs.is_empty()).then(|| pairs.join("; "))
}

fn sentinel_header(sentinel: &str, device_id: &str, flow: &str) -> Option<String> {
    if sentinel.is_empty() || device_id.is_empty() {
        return None;
    }
    Some(format!(
        r#"{{"p": "", "t": "", "c": "{}", "id": "{}", "flow": "{}"}}"#,
        sentinel, device_id, flow
    ))
}

/// Validate OTP and return the `continue_url` if present.
async fn validate_otp(follow: &rquest::Client, otp_code: &str) -> Result<Option<String>> {
    let body = format!(r#"{{"code":"{}"}}"#, otp_code);
    let resp = follow
        .post(VALIDATE_OTP_URL)
        .header("referer", "https://auth.openai.com/email-verification")
        .header("accept", "application/json")
        .header("content-type", "application/json")
        .body(body)
        .send()
        .await
        .map_err(|e| anyhow!("OTP 验证请求失败: {}", e))?;

    if !resp.status().is_success() {
        let st = resp.status();
        let body = resp.text().await.unwrap_or_default();
        bail!("OTP 验证失败 HTTP {} — {}", st, &body[..body.len().min(400)]);
    }

    let j: Value = resp.json().await.unwrap_or_default();
    let continue_url = j
        .get("continue_url")
        .and_then(|v| v.as_str())
        .filter(|s| !s.is_empty())
        .map(|s| s.to_string());
    Ok(continue_url)
}

/// Select workspace and return the `continue_url`.
async fn select_workspace(
    follow: &rquest::Client,
    workspace_id: &str,
    progress: &mut impl FnMut(&str),
) -> Result<String> {
    progress(&format!(
        "  📦 Workspace: {}",
        &workspace_id[..workspace_id.len().min(30)]
    ));
    let body = format!(r#"{{"workspace_id":"{}"}}"#, workspace_id);
    let resp = follow
        .post(SELECT_WORKSPACE_URL)
        .header(
            "referer",
            "https://auth.openai.com/sign-in-with-chatgpt/codex/consent",
        )
        .header("content-type", "application/json")
        .body(body)
        .send()
        .await
        .map_err(|e| anyhow!("Workspace 选择请求失败: {}", e))?;

    if !resp.status().is_success() {
        let st = resp.status();
        let body_txt = resp.text().await.unwrap_or_default();
        bail!(
            "Workspace 选择失败 HTTP {} — {}",
            st,
            &body_txt[..body_txt.len().min(400)]
        );
    }

    let j: Value = resp.json().await.unwrap_or_default();
    let url = j
        .get("continue_url")
        .and_then(|v| v.as_str())
        .filter(|s| !s.is_empty())
        .map(|s| s.to_string())
        .ok_or_else(|| anyhow!("Workspace 选择响应缺少 continue_url"))?;
    Ok(url)
}

/// Follow 3xx redirects manually (no-redirect client shares cookie jar with `follow`).
async fn follow_redirects_for_code(
    nore: &rquest::Client,
    start_url: &str,
    progress: &mut impl FnMut(&str),
) -> Result<String> {
    let mut current = start_url.to_string();

    for i in 0..12 {
        progress(&format!(
            "  ↪ 重定向 {}: {}",
            i + 1,
            &current[..current.len().min(80)]
        ));

        let resp = nore
            .get(&current)
            .header("Accept", "text/html,application/xhtml+xml,*/*")
            .send()
            .await
            .map_err(|e| anyhow!("重定向请求失败: {}", e))?;

        let status = resp.status().as_u16();
        let location = resp
            .headers()
            .get("location")
            .and_then(|v| v.to_str().ok())
            .map(|s| s.to_string());

        if !(300..400).contains(&(status as i32)) {
            if current.contains("code=") {
                return Ok(current);
            }
            bail!("重定向链结束，未找到 callback URL (HTTP {})", status);
        }

        let loc = location.ok_or_else(|| anyhow!("重定向响应缺少 Location 头"))?;
        let next = resolve_url(&current, &loc);

        if next.contains("code=") && next.contains("state=") {
            progress("  ✓ 找到 OAuth 回调 URL");
            return Ok(next);
        }
        current = next;
    }

    bail!("超过最大重定向次数，未找到 callback URL")
}

// ─── Workspace ID extraction ──────────────────────────────────────────────────

/// Extract workspace_id from HTML/text using many regex patterns (mirrors codex-manager).
fn extract_workspace_id_from_text(text: &str) -> Option<String> {
    use regex::Regex;
    let patterns = [
        r#"name="workspace_id"[^>]*value="([^"]+)""#,
        r#"name='workspace_id'[^>]*value='([^']+)'"#,
        r#"value="([^"]+)"[^>]*name="workspace_id""#,
        r#""workspace_id"\s*:\s*"([^"]+)""#,
        r#""workspaceId"\s*:\s*"([^"]+)""#,
        r#""default_workspace_id"\s*:\s*"([^"]+)""#,
        r#""defaultWorkspaceId"\s*:\s*"([^"]+)""#,
        r#""active_workspace_id"\s*:\s*"([^"]+)""#,
        r#""activeWorkspaceId"\s*:\s*"([^"]+)""#,
        r#""workspace"\s*:\s*\{[^{}]*"id"\s*:\s*"([^"]+)""#,
        r#""default_workspace"\s*:\s*\{[^{}]*"id"\s*:\s*"([^"]+)""#,
        r#""active_workspace"\s*:\s*\{[^{}]*"id"\s*:\s*"([^"]+)""#,
    ];
    for pat in &patterns {
        if let Ok(re) = Regex::new(pat) {
            if let Some(cap) = re.captures(text) {
                if let Some(m) = cap.get(1) {
                    let wid = m.as_str().trim().to_string();
                    if !wid.is_empty() {
                        return Some(wid);
                    }
                }
            }
        }
    }
    None
}

/// Extract workspace_id from URL query params.
fn extract_workspace_id_from_url(url: &str) -> Option<String> {
    let query = url.split('?').nth(1).unwrap_or("");
    for param in query.split('&') {
        let mut kv = param.splitn(2, '=');
        let key = kv.next().unwrap_or("");
        if matches!(key, "workspace_id" | "workspaceId" | "default_workspace_id" | "active_workspace_id") {
            if let Some(val) = kv.next() {
                let decoded = urlencoding::decode(val)
                    .map(|s| s.into_owned())
                    .unwrap_or_else(|_| val.to_string());
                let trimmed = decoded.trim().to_string();
                if !trimmed.is_empty() {
                    return Some(trimmed);
                }
            }
        }
    }
    None
}

/// Extract workspace_id from a base64url-encoded JSON cookie (e.g. oai-client-auth-session).
fn extract_workspace_id_from_cookie_value(cookie_val: &str) -> Option<String> {
    fn decode_base64_candidate(raw: &str) -> Option<Vec<u8>> {
        use base64::Engine;

        let padded = format!("{}{}", raw, "=".repeat((4 - raw.len() % 4) % 4));

        base64::engine::general_purpose::URL_SAFE_NO_PAD
            .decode(raw)
            .or_else(|_| base64::engine::general_purpose::URL_SAFE.decode(raw))
            .or_else(|_| base64::engine::general_purpose::STANDARD_NO_PAD.decode(raw))
            .or_else(|_| base64::engine::general_purpose::STANDARD.decode(raw))
            .or_else(|_| base64::engine::general_purpose::URL_SAFE_NO_PAD.decode(&padded))
            .or_else(|_| base64::engine::general_purpose::URL_SAFE.decode(&padded))
            .or_else(|_| base64::engine::general_purpose::STANDARD_NO_PAD.decode(&padded))
            .or_else(|_| base64::engine::general_purpose::STANDARD.decode(&padded))
            .ok()
    }

    let mut candidates = vec![cookie_val.trim().to_string()];
    let mut current = cookie_val.trim().to_string();
    for _ in 0..2 {
        let Ok(decoded) = urlencoding::decode(&current) else {
            break;
        };
        let decoded = decoded.trim().to_string();
        if decoded.is_empty() || decoded == current {
            break;
        }
        candidates.push(decoded.clone());
        current = decoded;
    }

    for candidate in candidates {
        if candidate.is_empty() {
            continue;
        }

        let mut payload_candidates = vec![candidate.as_str()];
        if candidate.contains('.') {
            payload_candidates.extend(candidate.split('.'));
        }

        for raw in payload_candidates {
            let raw = raw.trim().trim_matches('"').trim_matches('\'');
            if raw.is_empty() {
                continue;
            }

            let Some(decoded) = decode_base64_candidate(raw) else {
                continue;
            };

            let Ok(text) = std::str::from_utf8(&decoded) else {
                continue;
            };
            let Ok(value) = serde_json::from_str::<Value>(text) else {
                continue;
            };

            if let Some(wid) = extract_workspace_id_from_json(&value, 0) {
                return Some(wid);
            }
        }
    }

    None
}

fn extract_workspace_id_from_cookies(jar: &Arc<Jar>) -> Option<String> {
    let cookie_names = [
        "oai-client-auth-session",
        "oai_client_auth_session",
        "oai-client-auth-info",
        "oai_client_auth_info",
    ];
    let base_urls = [
        "https://auth.openai.com",
        "https://openai.com",
        "https://chatgpt.com",
    ];

    for base_str in &base_urls {
        for name in &cookie_names {
            if let Some(cookie_val) = read_cookie(jar, base_str, name) {
                if let Some(wid) = extract_workspace_id_from_cookie_value(&cookie_val) {
                    return Some(wid);
                }
            }
        }
    }

    None
}

fn debug_workspace_cookie_state(jar: &Arc<Jar>) -> String {
    let cookie_names = [
        "oai-client-auth-session",
        "oai_client_auth_session",
        "oai-client-auth-info",
        "oai_client_auth_info",
    ];
    let base_urls = [
        "https://auth.openai.com",
        "https://openai.com",
        "https://chatgpt.com",
    ];

    let mut parts = Vec::new();
    for base_str in &base_urls {
        for name in &cookie_names {
            if let Some(cookie_val) = read_cookie(jar, base_str, name) {
                parts.push(format!(
                    "{}@{} len={}",
                    name,
                    base_str,
                    cookie_val.len()
                ));
            }
        }
    }

    if parts.is_empty() {
        "(no workspace cookies)".to_string()
    } else {
        parts.join(", ")
    }
}

/// Recursively scan a JSON value for workspace_id (mirrors codex-manager's payload scanner).
fn extract_workspace_id_from_json(v: &Value, depth: u8) -> Option<String> {
    if depth > 5 {
        return None;
    }
    match v {
        Value::Object(map) => {
            // Check workspaces array first
            if let Some(Value::Array(arr)) = map.get("workspaces") {
                for item in arr {
                    if let Some(id) = item.get("id").and_then(|i| i.as_str()) {
                        let id = id.trim().to_string();
                        if !id.is_empty() {
                            return Some(id);
                        }
                    }
                }
            }
            // Direct keys
            for key in &[
                "workspace_id", "workspaceId",
                "default_workspace_id", "defaultWorkspaceId",
                "active_workspace_id", "activeWorkspaceId",
            ] {
                if let Some(id) = map.get(*key).and_then(|i| i.as_str()) {
                    let id = id.trim().to_string();
                    if !id.is_empty() {
                        return Some(id);
                    }
                }
            }
            // Nested workspace objects
            for key in &["workspace", "default_workspace", "active_workspace", "defaultWorkspace", "activeWorkspace"] {
                if let Some(obj) = map.get(*key) {
                    if let Some(id) = obj.get("id").and_then(|i| i.as_str()) {
                        let id = id.trim().to_string();
                        if !id.is_empty() {
                            return Some(id);
                        }
                    }
                }
            }
            // Recurse into all values
            for val in map.values() {
                if let Some(wid) = extract_workspace_id_from_json(val, depth + 1) {
                    return Some(wid);
                }
            }
            None
        }
        Value::Array(arr) => {
            for item in arr {
                if let Some(wid) = extract_workspace_id_from_json(item, depth + 1) {
                    return Some(wid);
                }
            }
            None
        }
        _ => None,
    }
}

/// Try all sources (JSON body, HTML text, URL, cookies) to find workspace_id.
/// Logs diagnostic info via the progress callback.
fn extract_workspace_id_from_all(
    html: &str,
    url: &str,
    json_body: Option<&Value>,
) -> Option<String> {
    // 1. Response JSON body (recursive)
    if let Some(v) = json_body {
        if let Some(wid) = extract_workspace_id_from_json(v, 0) {
            return Some(wid);
        }
    }

    // 2. HTML / text patterns
    if let Some(wid) = extract_workspace_id_from_text(html) {
        return Some(wid);
    }

    // 3. URL query params
    if let Some(wid) = extract_workspace_id_from_url(url) {
        return Some(wid);
    }

    None
}

async fn resolve_workspace_id_with_cookie_retry(
    html: &str,
    url: &str,
    json_body: Option<&Value>,
    jar: &Arc<Jar>,
    progress: &mut impl FnMut(&str),
) -> Result<String> {
    if let Some(wid) = extract_workspace_id_from_all(html, url, json_body) {
        return Ok(wid);
    }

    progress("  consent 页面缺少 workspace_id，回退到 Cookie 解析路径...");

    let backoff_seconds = [1u64, 2, 4];
    let max_attempts = backoff_seconds.len() + 1;

    for attempt in 1..=max_attempts {
        if let Some(wid) = extract_workspace_id_from_cookies(jar) {
            return Ok(wid);
        }

        progress(&format!(
            "  获取 Workspace ID 失败 (第 {}/{})：{}",
            attempt,
            max_attempts,
            debug_workspace_cookie_state(jar)
        ));

        if attempt < max_attempts {
            let wait = backoff_seconds[attempt - 1];
            progress(&format!("  等待 {} 秒后重试 Workspace ID...", wait));
            tokio::time::sleep(std::time::Duration::from_secs(wait)).await;
        }
    }

    bail!("未能从 consent 页面或 Cookie 提取 workspace_id")
}

/// Dump cookies from the jar for diagnostic purposes.
fn debug_cookies(jar: &Arc<Jar>) -> String {
    let base_urls = [
        "https://auth.openai.com",
        "https://openai.com",
        "https://chatgpt.com",
    ];
    let mut parts = Vec::new();
    for base_str in &base_urls {
        if let Ok(base) = url::Url::parse(base_str) {
            if let Some(hdr) = jar.cookies(&base) {
                if let Ok(s) = hdr.to_str() {
                    // Show only cookie names (not values) for safety
                    let names: Vec<&str> = s
                        .split(';')
                        .map(|p| p.trim().split('=').next().unwrap_or("?"))
                        .collect();
                    parts.push(format!("{}: [{}]", base_str, names.join(", ")));
                }
            }
        }
    }
    if parts.is_empty() {
        "(no cookies found)".to_string()
    } else {
        parts.join(" | ")
    }
}

// ─── Phase 2: Login / consent flow ───────────────────────────────────────────

/// After account registration, start a fresh OAuth session and log in to reach
/// the consent page, select a workspace, and obtain the authorization `code`.
///
/// Returns `(code, code_verifier)` for token exchange.
async fn advance_login_authorization<F, Fut>(
    follow: &rquest::Client,
    nore: &rquest::Client,
    jar: &Arc<Jar>,
    email: &str,
    password: &str,
    poll_otp: &F,
    progress: &mut impl FnMut(&str),
) -> Result<(String, String)>
where
    F: Fn(f64) -> Fut,
    Fut: std::future::Future<Output = Result<String>>,
{
    // New OAuth params for the login phase
    let (verifier2, challenge2) = generate_pkce();
    let state2 = random_state();
    let auth_url2 = build_auth_url(&challenge2, &state2);

    progress("🔄 初始化登录授权流程...");
    let (device_id2, html2, url2) = init_oauth_session(follow, jar, &auth_url2).await?;

    if !device_id2.is_empty() {
        progress(&format!(
            "  Device ID (login): {}",
            &device_id2[..device_id2.len().min(20)]
        ));
    }

    // Check if we're already at the consent page
    if url2.contains("sign-in-with-chatgpt/codex/consent")
        || html2.contains("sign-in-with-chatgpt/codex/consent")
    {
        progress("  已到达 consent 页面，提取 workspace_id...");
        let workspace_id =
            resolve_workspace_id_with_cookie_retry(&html2, &url2, None, jar, progress).await?;
        let continue_url = select_workspace(follow, &workspace_id, progress).await?;
        let callback_url = follow_redirects_for_code(nore, &continue_url, progress).await?;
        let code2 = extract_code_from_url(&callback_url)?;
        return Ok((code2, verifier2));
    }

    // Get sentinel for login
    let sentinel2 = get_sentinel(follow, &device_id2).await;

    // Submit email with screen_hint="login"
    progress("🔑 提交登录邮箱...");
    let login_body = format!(
        r#"{{"username":{{"value":"{}","kind":"email"}},"screen_hint":"login"}}"#,
        email
    );
    let mut login_req = follow
        .post(SIGNUP_URL)
        .header("referer", "https://auth.openai.com/log-in")
        .header("accept", "application/json")
        .header("content-type", "application/json");
    if let Some(hdr) = sentinel_header(&sentinel2, &device_id2, "authorize_continue") {
        login_req = login_req.header("openai-sentinel-token", hdr);
    }
    let _ = login_req.body(login_body).send().await;

    // Submit password via password/verify
    progress("🔒 提交登录密码...");
    let sentinel_pwd = get_sentinel(follow, &device_id2).await;
    let pwd_body = format!(r#"{{"password":"{}"}}"#, password);
    let mut pwd_req = follow
        .post(PASSWORD_VERIFY_URL)
        .header("referer", "https://auth.openai.com/log-in/password")
        .header("accept", "application/json")
        .header("content-type", "application/json");
    if let Some(hdr) = sentinel_header(&sentinel_pwd, &device_id2, "password_verify") {
        pwd_req = pwd_req.header("openai-sentinel-token", hdr);
    }
    let otp2_sent_at = chrono::Utc::now().timestamp_millis() as f64 / 1000.0;
    let pwd_resp = pwd_req
        .body(pwd_body)
        .send()
        .await
        .map_err(|e| anyhow!("密码验证请求失败: {}", e))?;

    // Follow password/verify continue_url if present
    if pwd_resp.status().is_success() {
        if let Ok(j) = pwd_resp.json::<Value>().await {
            if let Some(cu) = j.get("continue_url").and_then(|v| v.as_str()) {
                if !cu.is_empty() {
                    let _ = follow.get(cu).send().await;
                }
            }
        }
    }

    // Poll for the second OTP (triggered by password verify)
    progress("⏳ 等待第二次验证码邮件...");
    let otp2 = poll_otp(otp2_sent_at).await?;
    progress(&format!("  ✉ 第二次验证码: {}", otp2));

    // Validate OTP → get consent_url
    progress("✅ 验证第二次 OTP...");
    let consent_url = validate_otp(follow, &otp2).await?;

    // GET consent page
    let target = consent_url.as_deref().unwrap_or(auth_url2.as_str());
    progress(&format!(
        "  → 请求 consent 页面: {}",
        &target[..target.len().min(80)]
    ));
    let consent_resp = follow
        .get(target)
        .header("Accept", "text/html,application/xhtml+xml,*/*")
        .send()
        .await
        .map_err(|e| anyhow!("请求 consent 页失败: {}", e))?;
    let consent_final_url = consent_resp.url().as_str().to_string();
    let consent_html = consent_resp.text().await.unwrap_or_default();

    progress(&format!(
        "  consent 最终 URL: {}",
        &consent_final_url[..consent_final_url.len().min(100)]
    ));
    // Debug: print first 300 chars of consent HTML so we can diagnose issues
    if !consent_html.is_empty() {
        progress(&format!(
            "  consent HTML 预览: {}",
            &consent_html[..consent_html.len().min(300)]
        ));
    }
    // Debug: show which cookies are set in the jar
    progress(&format!("  🍪 Cookies: {}", debug_cookies(jar)));

    // Extract workspace_id — mirror codex-manager: HTML/URL first, then cookie retries.
    let workspace_id =
        resolve_workspace_id_with_cookie_retry(&consent_html, &consent_final_url, None, jar, progress)
            .await
            .map_err(|_| {
                anyhow!(
                    "未能从 consent 页面提取 workspace_id (URL={}, html_len={})",
                    &consent_final_url[..consent_final_url.len().min(120)],
                    consent_html.len()
                )
            })?;

    progress(&format!("  ✅ workspace_id: {}…", &workspace_id[..workspace_id.len().min(20)]));

    // Select workspace → continue_url → redirect chain → code
    let continue_url = select_workspace(follow, &workspace_id, progress).await?;
    let callback_url = follow_redirects_for_code(nore, &continue_url, progress).await?;
    let code2 = extract_code_from_url(&callback_url)?;

    Ok((code2, verifier2))
}

// ─── Main public function ─────────────────────────────────────────────────────

/// Run the full OpenAI account registration flow.
///
/// `poll_otp` is a repeatable async closure (called up to twice — once for
/// registration email verification, once for post-registration login OTP).
pub async fn run_full_registration<F, Fut>(
    email: &str,
    proxy_url: Option<&str>,
    mut progress: impl FnMut(&str),
    poll_otp: F,
) -> Result<RegistrationResult>
where
    F: Fn(f64) -> Fut,
    Fut: std::future::Future<Output = Result<String>>,
{
    // ─────────────────────────────────────────────────────────────────────────
    // Phase 1: Registration (email → signup → password → OTP₁ → create_account)
    // ─────────────────────────────────────────────────────────────────────────

    // 1. PKCE + auth URL for registration session
    progress("🔑 生成 PKCE 参数...");
    let (_, challenge1) = generate_pkce(); // verifier1 not used for token exchange
    let state1 = random_state();
    let auth_url1 = build_auth_url(&challenge1, &state1);

    // Build HTTP clients (Chrome124 TLS fingerprint via rquest/BoringSSL)
    progress("🌐 初始化 Chrome 指纹 HTTP 客户端...");
    let (follow, nore, jar) = build_clients(proxy_url)?;

    // 2. GET auth URL → populate oai-did in cookie jar
    progress("🍪 初始化 OAuth 会话...");
    let (device_id, _html1, _url1) = init_oauth_session(&follow, &jar, &auth_url1).await?;
    if !device_id.is_empty() {
        progress(&format!(
            "  Device ID: {}",
            &device_id[..device_id.len().min(20)]
        ));
    } else {
        progress("  ⚠ 未获取到 Device ID");
    }

    // 3. Sentinel token
    progress("🛡️ 请求 Sentinel 令牌...");
    let sentinel = get_sentinel(&follow, &device_id).await;
    if !sentinel.is_empty() {
        progress("  Sentinel token 获取成功");
    }

    // 4. Submit signup form
    progress(&format!("📝 提交注册表单 ({})...", email));
    let signup_body = format!(
        r#"{{"username":{{"value":"{}","kind":"email"}},"screen_hint":"signup"}}"#,
        email
    );
    let mut req = follow
        .post(SIGNUP_URL)
        .header("referer", "https://auth.openai.com/create-account")
        .header("accept", "application/json")
        .header("content-type", "application/json");
    if let Some(hdr) = sentinel_header(&sentinel, &device_id, "authorize_continue") {
        req = req.header("openai-sentinel-token", hdr);
    }
    let signup_resp = req
        .body(signup_body)
        .send()
        .await
        .map_err(|e| anyhow!("注册表单请求失败: {}", e))?;
    if !signup_resp.status().is_success() {
        let st = signup_resp.status();
        let body = signup_resp.text().await.unwrap_or_default();
        bail!("注册表单失败 HTTP {} — {}", st, &body[..body.len().min(400)]);
    }
    let _signup_json: Value = signup_resp.json().await.unwrap_or_default();

    // 5. Register password
    progress("🔒 注册密码...");
    let password = generate_password();
    let register_body = format!(r#"{{"password":"{}","username":"{}"}}"#, password, email);
    let reg_resp = follow
        .post(REGISTER_URL)
        .header("referer", "https://auth.openai.com/create-account/password")
        .header("accept", "application/json")
        .header("content-type", "application/json")
        .body(register_body)
        .send()
        .await
        .map_err(|e| anyhow!("密码注册请求失败: {}", e))?;
    if !reg_resp.status().is_success() {
        let st = reg_resp.status();
        let body = reg_resp.text().await.unwrap_or_default();
        bail!("密码注册失败 HTTP {} — {}", st, &body[..body.len().min(400)]);
    }

    // 6. Trigger OTP send
    progress("📨 触发验证码发送...");
    let otp1_sent_at = chrono::Utc::now().timestamp_millis() as f64 / 1000.0;
    let _ = follow
        .get(SEND_OTP_URL)
        .header("referer", "https://auth.openai.com/create-account/password")
        .header("accept", "application/json")
        .send()
        .await;

    // 7. Poll for OTP₁
    progress("⏳ 等待注册验证码邮件...");
    let otp1 = poll_otp(otp1_sent_at).await?;
    progress(&format!("  ✉ 验证码: {}", otp1));

    // 8. Validate OTP₁
    progress("✅ 验证 OTP...");
    let _ = validate_otp(&follow, &otp1).await?;

    // 9. Create account profile
    progress("👤 创建账户资料...");
    let (name, birthdate) = random_user_info();
    let create_body = format!(r#"{{"name":"{}","birthdate":"{}"}}"#, name, birthdate);
    let create_resp = follow
        .post(CREATE_ACCOUNT_URL)
        .header("referer", "https://auth.openai.com/about-you")
        .header("accept", "application/json")
        .header("content-type", "application/json")
        .body(create_body)
        .send()
        .await
        .map_err(|e| anyhow!("创建账户请求失败: {}", e))?;
    if !create_resp.status().is_success() {
        let st = create_resp.status();
        let txt = create_resp.text().await.unwrap_or_default();
        progress(&format!(
            "  ⚠ create_account HTTP {} (继续): {}",
            st,
            &txt[..txt.len().min(200)]
        ));
    } else {
        let _ = create_resp.text().await;
    }

    // ─────────────────────────────────────────────────────────────────────────
    // Phase 2: Login / consent → obtain authorization code + code_verifier
    // ─────────────────────────────────────────────────────────────────────────

    progress("🏢 推进 OAuth 授权（登录流程）...");
    let (auth_code, code_verifier) = advance_login_authorization(
        &follow,
        &nore,
        &jar,
        email,
        &password,
        &poll_otp,
        &mut progress,
    )
    .await?;

    progress(&format!(
        "  授权码: {}...",
        &auth_code[..auth_code.len().min(20)]
    ));

    // ─────────────────────────────────────────────────────────────────────────
    // Token exchange
    // ─────────────────────────────────────────────────────────────────────────

    progress("🎫 交换访问令牌...");
    let token_resp = rquest::Client::builder()
        .timeout(std::time::Duration::from_secs(30))
        .emulation(Emulation::Chrome124)
        .build()?
        .post(OAUTH_TOKEN_URL)
        .header("Content-Type", "application/x-www-form-urlencoded")
        .header("Accept", "application/json")
        .form(&[
            ("grant_type", "authorization_code"),
            ("client_id", OAUTH_CLIENT_ID),
            ("code", auth_code.as_str()),
            ("redirect_uri", CODEX_REDIRECT_URI),
            ("code_verifier", code_verifier.as_str()),
        ])
        .send()
        .await
        .map_err(|e| anyhow!("Token 交换请求失败: {}", e))?;

    if !token_resp.status().is_success() {
        let st = token_resp.status();
        let body = token_resp.text().await.unwrap_or_default();
        bail!("Token 交换失败 HTTP {} — {}", st, &body[..body.len().min(400)]);
    }

    let tok: Value = token_resp
        .json()
        .await
        .map_err(|e| anyhow!("Token 响应解析失败: {}", e))?;

    let access_token = tok["access_token"].as_str().unwrap_or("").to_string();
    let refresh_token = tok["refresh_token"].as_str().unwrap_or("").to_string();
    let id_token = tok["id_token"].as_str().unwrap_or("").to_string();
    let expires_in = tok["expires_in"].as_i64().unwrap_or(0);

    if access_token.is_empty() {
        bail!("Token 响应缺少 access_token: {:?}", tok);
    }

    let claims = parse_jwt_claims(&id_token);
    let account_id = extract_account_id(&claims);
    let expires_at = (chrono::Utc::now() + chrono::Duration::seconds(expires_in))
        .format("%Y-%m-%dT%H:%M:%SZ")
        .to_string();
    let web_session_cookie = merge_cookie_headers(&jar);
    let web_session_device_id = read_cookie(&jar, AUTH_OPENAI_BASE, "oai-did")
        .or_else(|| read_cookie(&jar, "https://openai.com/", "oai-did"));

    progress(&format!("🎉 注册完成! account_id={}", account_id));

    Ok(RegistrationResult {
        email: email.to_string(),
        password,
        access_token,
        refresh_token,
        id_token,
        account_id,
        expires_at,
        web_session_cookie,
        web_session_device_id,
    })
}

#[cfg(test)]
mod tests {
    use super::extract_workspace_id_from_cookie_value;
    use base64::Engine;
    use serde_json::json;

    #[test]
    fn extracts_workspace_id_from_jwt_like_cookie_payload() {
        let payload = json!({
            "workspaces": [
                { "id": "ws-cookie" }
            ]
        });
        let encoded =
            base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(payload.to_string());
        let cookie = format!("{}.signature.more", encoded);

        assert_eq!(
            extract_workspace_id_from_cookie_value(&cookie),
            Some("ws-cookie".to_string())
        );
    }

    #[test]
    fn extracts_workspace_id_from_url_encoded_cookie() {
        let payload = json!({
            "workspaces": [
                { "id": "ws-encoded" }
            ]
        });
        let encoded =
            base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(payload.to_string());
        let cookie = format!("{}%2Esignature%2Emore", encoded);

        assert_eq!(
            extract_workspace_id_from_cookie_value(&cookie),
            Some("ws-encoded".to_string())
        );
    }

    #[test]
    fn extracts_workspace_id_from_double_url_encoded_cookie() {
        let payload = json!({
            "workspaces": [
                { "id": "ws-double-encoded" }
            ]
        });
        let encoded =
            base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(payload.to_string());
        let cookie = format!("{}%252Esignature%252Emore", encoded);

        assert_eq!(
            extract_workspace_id_from_cookie_value(&cookie),
            Some("ws-double-encoded".to_string())
        );
    }
}
