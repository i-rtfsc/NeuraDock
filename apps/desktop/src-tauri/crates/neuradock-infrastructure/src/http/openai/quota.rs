use chrono::{DateTime, TimeZone, Utc};
use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CodexUsageWindow {
    pub used_percent: f64,
    pub window_minutes: Option<i64>,
    pub resets_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CodexUsageQuota {
    pub plan_type: Option<String>,
    pub has_credits: Option<bool>,
    pub is_unlimited: Option<bool>,
    pub credit_balance: Option<String>,
    pub primary_window: Option<CodexUsageWindow>,
    pub secondary_window: Option<CodexUsageWindow>,
}

#[derive(Debug, Deserialize)]
struct UsagePayload {
    #[serde(default)]
    plan_type: Option<String>,
    #[serde(default)]
    credits: Option<CreditsPayload>,
    #[serde(default)]
    rate_limit: Option<RateLimitPayload>,
}

#[derive(Debug, Deserialize)]
struct CreditsPayload {
    has_credits: bool,
    unlimited: bool,
    #[serde(default, deserialize_with = "deserialize_optional_stringish")]
    balance: Option<String>,
}

#[derive(Debug, Deserialize)]
struct RateLimitPayload {
    #[serde(default, alias = "primary")]
    primary_window: Option<WindowPayload>,
    #[serde(default, alias = "secondary")]
    secondary_window: Option<WindowPayload>,
}

#[derive(Debug, Deserialize)]
struct WindowPayload {
    #[serde(deserialize_with = "deserialize_f64ish")]
    used_percent: f64,
    #[serde(default)]
    limit_window_seconds: Option<i64>,
    #[serde(default, deserialize_with = "deserialize_optional_i64ish")]
    window_minutes: Option<i64>,
    #[serde(default, alias = "resets_at", deserialize_with = "deserialize_optional_i64ish")]
    reset_at: Option<i64>,
}

pub async fn fetch_codex_usage(access_token: &str, account_id: Option<&str>) -> anyhow::Result<CodexUsageQuota> {
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(20))
        .user_agent("codex-cli")
        .build()?;

    let mut failures = Vec::new();

    for url in [
        "https://chatgpt.com/backend-api/wham/usage",
        "https://api.openai.com/api/codex/usage",
    ] {
        let mut req = client
            .get(url)
            .header("Authorization", format!("Bearer {}", access_token))
            .header("Accept", "application/json");

        if let Some(aid) = account_id {
            req = req.header("ChatGPT-Account-Id", aid);
        }

        match req.send().await {
            Ok(resp) => {
                let status = resp.status();
                let body = resp.text().await?;

                if !status.is_success() {
                    failures.push(format!("{url} -> {status}"));
                    continue;
                }

                let payload: UsagePayload = serde_json::from_str(&body).map_err(|e| {
                    anyhow::anyhow!("Decode error for quota payload from {}: {}; body={}", url, e, body)
                })?;

                let credits = payload.credits;
                let rate_limit = payload.rate_limit;

                return Ok(CodexUsageQuota {
                    plan_type: payload.plan_type,
                    has_credits: credits.as_ref().map(|details| details.has_credits),
                    is_unlimited: credits.as_ref().map(|details| details.unlimited),
                    credit_balance: credits.and_then(|details| details.balance),
                    primary_window: rate_limit
                        .as_ref()
                        .and_then(|details| details.primary_window.as_ref())
                        .map(CodexUsageWindow::from_payload),
                    secondary_window: rate_limit
                        .as_ref()
                        .and_then(|details| details.secondary_window.as_ref())
                        .map(CodexUsageWindow::from_payload),
                });
            }
            Err(error) => failures.push(format!("{url} -> {}", error)),
        }
    }

    anyhow::bail!("All Codex quota endpoints failed: {}", failures.join(" | "))
}

impl CodexUsageWindow {
    fn from_payload(payload: &WindowPayload) -> Self {
        let window_minutes = payload
            .window_minutes
            .or_else(|| payload.limit_window_seconds.map(|seconds| seconds / 60));

        Self {
            used_percent: payload.used_percent,
            window_minutes,
            resets_at: payload.reset_at.and_then(timestamp_to_datetime),
        }
    }
}

fn timestamp_to_datetime(timestamp: i64) -> Option<DateTime<Utc>> {
    let seconds = if timestamp.abs() >= 1_000_000_000_000 {
        timestamp / 1000
    } else {
        timestamp
    };
    Utc.timestamp_opt(seconds, 0).single()
}

fn deserialize_optional_stringish<'de, D>(
    deserializer: D,
) -> std::result::Result<Option<String>, D::Error>
where
    D: serde::Deserializer<'de>,
{
    let value = Option::<Value>::deserialize(deserializer)?;
    Ok(match value {
        Some(Value::String(s)) if !s.trim().is_empty() => Some(s),
        Some(Value::Number(n)) => Some(n.to_string()),
        _ => None,
    })
}

fn deserialize_f64ish<'de, D>(deserializer: D) -> std::result::Result<f64, D::Error>
where
    D: serde::Deserializer<'de>,
{
    let value = Value::deserialize(deserializer)?;
    match value {
        Value::Number(n) => n
            .as_f64()
            .ok_or_else(|| serde::de::Error::custom("invalid numeric value")),
        Value::String(s) => s
            .parse::<f64>()
            .map_err(|_| serde::de::Error::custom("invalid numeric string")),
        _ => Err(serde::de::Error::custom("expected numeric value")),
    }
}

fn deserialize_optional_i64ish<'de, D>(
    deserializer: D,
) -> std::result::Result<Option<i64>, D::Error>
where
    D: serde::Deserializer<'de>,
{
    let value = Option::<Value>::deserialize(deserializer)?;
    Ok(match value {
        Some(Value::Number(n)) => n.as_i64(),
        Some(Value::String(s)) => s.parse::<i64>().ok(),
        _ => None,
    })
}

#[cfg(test)]
mod tests {
    use super::UsagePayload;

    #[test]
    fn parses_actual_codex_usage_shape() {
        let payload: UsagePayload = serde_json::from_value(serde_json::json!({
            "plan_type": "pro",
            "credits": {
                "has_credits": true,
                "unlimited": false,
                "balance": "$12.40"
            },
            "rate_limit": {
                "primary_window": {
                    "used_percent": 42,
                    "limit_window_seconds": 3600,
                    "reset_after_seconds": 120,
                    "reset_at": 1735689720
                },
                "secondary_window": {
                    "used_percent": 5,
                    "limit_window_seconds": 604800,
                    "reset_after_seconds": 43200,
                    "reset_at": 1736290800
                }
            }
        }))
        .expect("payload should deserialize");

        let primary = payload
            .rate_limit
            .as_ref()
            .and_then(|details| details.primary_window.as_ref())
            .expect("primary window");
        let secondary = payload
            .rate_limit
            .as_ref()
            .and_then(|details| details.secondary_window.as_ref())
            .expect("secondary window");

        assert_eq!(payload.plan_type.as_deref(), Some("pro"));
        assert_eq!(primary.limit_window_seconds, Some(3600));
        assert_eq!(secondary.reset_at, Some(1736290800));
    }

    #[test]
    fn parses_legacy_single_window_shape() {
        let payload: UsagePayload = serde_json::from_value(serde_json::json!({
            "plan_type": "plus",
            "credits": {
                "has_credits": true,
                "unlimited": false,
                "balance": 3.5
            },
            "rate_limit": {
                "primary": {
                    "used_percent": "12",
                    "window_minutes": "60",
                    "resets_at": "1735689720"
                }
            }
        }))
        .expect("legacy payload should deserialize");

        let primary = payload
            .rate_limit
            .as_ref()
            .and_then(|details| details.primary_window.as_ref())
            .expect("primary window");

        assert_eq!(payload.plan_type.as_deref(), Some("plus"));
        assert_eq!(payload.credits.and_then(|details| details.balance).as_deref(), Some("3.5"));
        assert_eq!(primary.window_minutes, Some(60));
        assert_eq!(primary.reset_at, Some(1735689720));
    }
}
