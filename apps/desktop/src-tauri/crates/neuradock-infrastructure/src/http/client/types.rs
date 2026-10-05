use anyhow::Result;
use chrono::{DateTime, Duration, Utc};
use reqwest::Response;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

pub const USER_AGENT: &str = "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/138.0.0.0 Safari/537.36";

/// HTTP retry configuration
#[derive(Debug, Clone)]
pub struct RetryConfig {
    /// Maximum number of retry attempts (default: 3)
    pub max_retries: u32,
    /// Initial backoff duration in milliseconds (default: 1000ms)
    pub initial_backoff_ms: u64,
    /// Maximum backoff duration in milliseconds (default: 10000ms)
    pub max_backoff_ms: u64,
    /// Backoff multiplier (default: 2.0 for exponential backoff)
    pub backoff_multiplier: f64,
}

impl Default for RetryConfig {
    fn default() -> Self {
        Self {
            max_retries: 3,
            initial_backoff_ms: 1000,
            max_backoff_ms: 10000,
            backoff_multiplier: 2.0,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserInfo {
    /// Current balance reported by the provider API (maps from `quota` field)
    pub current_balance: f64,
    /// Historical consumption reported by the provider API (maps from `used_quota` field)
    pub total_consumed: f64,
    /// Lifetime total quota, derived from the raw `quota + used_quota` unit sum
    /// (NOT from adding the two rounded dollar values above, which would compound
    /// their rounding errors). Upstream labels this as `total_income`.
    /// May differ from `current_balance + total_consumed` by up to 0.01.
    pub total_quota: f64,
}

/// Stable error codes attached to a check-in result. Kept in one place because
/// the application layer maps them onto attempt outcomes, and because a code is
/// never a formatted error: it must not carry an upstream body.
pub mod check_in_codes {
    /// The provider has no endpoint that can confirm a check-in.
    pub const NOT_SUPPORTED: &str = "not_supported";
    /// The provider answered that today's check-in already happened.
    pub const ALREADY_CHECKED_IN: &str = "already_checked_in";
    /// The response was not the shape we can confirm a reward from.
    pub const INVALID_RESPONSE: &str = "invalid_response";
    /// The provider served a challenge page. Kept as the literal prefix the
    /// WAF retry path already keys on.
    pub const WAF_CHALLENGE: &str = "WAF_CHALLENGE";
    /// 4xx: the request itself is the problem, retrying today will not help.
    pub const HTTP_CLIENT: &str = "http_status_client";
    /// 5xx: the provider is having a bad time, retry later.
    pub const HTTP_SERVER: &str = "http_status_server";
    pub const NETWORK: &str = "network";
    pub const TIMEOUT: &str = "timeout";
    pub const CANCELLED: &str = "cancelled";
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CheckInResult {
    pub success: bool,
    pub message: String,
    /// Reward explicitly reported by the check-in response, in dollars.
    #[serde(default)]
    pub reward: Option<f64>,
    /// Machine-readable classification. `None` means "no special case": the
    /// message is a provider sentence and success says whether it worked.
    #[serde(default)]
    pub error_code: Option<String>,
}

impl CheckInResult {
    /// A result the provider confirmed, with the reward when it was reported.
    pub fn confirmed(message: impl Into<String>, reward: Option<f64>) -> Self {
        Self {
            success: true,
            message: message.into(),
            reward,
            error_code: None,
        }
    }

    /// The provider already considered this quota day done.
    pub fn already_checked_in() -> Self {
        Self {
            success: true,
            message: "The provider reports this check-in was already done".to_string(),
            reward: None,
            error_code: Some(check_in_codes::ALREADY_CHECKED_IN.to_string()),
        }
    }

    /// A result that did not confirm a check-in. `code` must be a stable
    /// token, never a formatted error or an upstream body.
    pub fn failure(code: &str, message: impl Into<String>) -> Self {
        Self {
            success: false,
            message: message.into(),
            reward: None,
            error_code: Some(code.to_string()),
        }
    }

    pub fn error_code(&self) -> Option<&str> {
        self.error_code.as_deref()
    }
}

/// Extract domain from URL (including port if present)
pub(super) fn extract_domain(url: &str) -> Result<String> {
    let parsed = url::Url::parse(url)?;
    let host = parsed.host_str().unwrap_or("");

    if let Some(port) = parsed.port() {
        Ok(format!("{}://{}:{}", parsed.scheme(), host, port))
    } else {
        Ok(format!("{}://{}", parsed.scheme(), host))
    }
}

/// Result of extracting Set-Cookie headers from an HTTP response
#[derive(Debug, Clone, Default)]
pub struct SetCookieResult {
    /// Updated cookie key-value pairs from Set-Cookie headers
    pub cookies: HashMap<String, String>,
    /// Expiration time of the `session` cookie, if present
    pub session_expires_at: Option<DateTime<Utc>>,
}

impl SetCookieResult {
    pub fn is_empty(&self) -> bool {
        self.cookies.is_empty()
    }
}

/// Extract Set-Cookie headers from an HTTP response.
/// Must be called before consuming the response body.
pub(super) fn extract_set_cookies(response: &Response) -> SetCookieResult {
    let url = response.url().to_string();
    let mut result = SetCookieResult::default();

    for value in response
        .headers()
        .get_all(reqwest::header::SET_COOKIE)
        .iter()
    {
        let cookie_str = match value.to_str() {
            Ok(s) => s,
            Err(_) => continue,
        };

        // Parse "name=value; attr1; attr2=val2"
        let parts: Vec<&str> = cookie_str.splitn(2, ';').collect();
        let name_value = parts[0].trim();

        if let Some((name, value)) = name_value.split_once('=') {
            let name = name.trim().to_string();
            let value = value.trim().to_string();

            // For the "session" cookie, also parse expiration
            if name == "session" {
                if let Some(attrs) = parts.get(1) {
                    result.session_expires_at = parse_cookie_expiration(attrs);
                }
            }

            result.cookies.insert(name, value);
        }
    }

    if !result.cookies.is_empty() {
        log::info!(
            "[Set-Cookie] {} → {} cookie(s): [{}], session_expires_at: {:?}",
            url,
            result.cookies.len(),
            result
                .cookies
                .keys()
                .cloned()
                .collect::<Vec<_>>()
                .join(", "),
            result.session_expires_at,
        );
    } else {
        log::info!("[Set-Cookie] {} → No Set-Cookie headers", url);
    }

    result
}

/// Parse Expires or Max-Age from cookie attribute string
fn parse_cookie_expiration(attrs: &str) -> Option<DateTime<Utc>> {
    for attr in attrs.split(';') {
        let attr = attr.trim();

        if let Some(val) = attr
            .strip_prefix("Expires=")
            .or_else(|| attr.strip_prefix("expires="))
        {
            // RFC 2822 / RFC 1123 format: "Thu, 01 Jan 2026 00:00:00 GMT"
            if let Ok(dt) = DateTime::parse_from_rfc2822(val.trim()) {
                return Some(dt.with_timezone(&Utc));
            }
            // Try alternate format: "Thu, 01-Jan-2026 00:00:00 GMT"
            if let Ok(dt) =
                chrono::NaiveDateTime::parse_from_str(val.trim(), "%a, %d-%b-%Y %H:%M:%S GMT")
            {
                return Some(dt.and_utc());
            }
        }

        if let Some(val) = attr
            .strip_prefix("Max-Age=")
            .or_else(|| attr.strip_prefix("max-age="))
        {
            if let Ok(seconds) = val.trim().parse::<i64>() {
                return Some(Utc::now() + Duration::seconds(seconds));
            }
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_domain() {
        assert_eq!(
            extract_domain("https://example.com/api/user").unwrap(),
            "https://example.com"
        );
        assert_eq!(
            extract_domain("http://test.org:8080/path").unwrap(),
            "http://test.org:8080"
        );
    }

    #[test]
    fn test_parse_cookie_expiration_expires() {
        let attrs = " Path=/; Expires=Thu, 01 Jan 2026 00:00:00 GMT; HttpOnly";
        let result = parse_cookie_expiration(attrs);
        assert!(result.is_some());
    }

    #[test]
    fn test_parse_cookie_expiration_max_age() {
        let attrs = " Path=/; Max-Age=86400; HttpOnly";
        let result = parse_cookie_expiration(attrs);
        assert!(result.is_some());
        let expires = result.unwrap();
        let diff = expires.signed_duration_since(Utc::now());
        // Should be roughly 86400 seconds (1 day) from now
        assert!(diff.num_seconds() > 86300 && diff.num_seconds() <= 86400);
    }

    #[test]
    fn test_parse_cookie_expiration_none() {
        let attrs = " Path=/; HttpOnly; Secure";
        let result = parse_cookie_expiration(attrs);
        assert!(result.is_none());
    }
}
