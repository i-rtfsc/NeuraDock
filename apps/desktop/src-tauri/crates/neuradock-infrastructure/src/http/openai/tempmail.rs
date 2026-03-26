/// Tempmail.lol API v2 client
///
/// Endpoints used:
///   POST https://api.tempmail.lol/v2/inbox/create  → { address, token }
///   GET  https://api.tempmail.lol/v2/inbox?token=T → { emails: [...] }

use serde::{Deserialize, Serialize};

const BASE_URL: &str = "https://api.tempmail.lol/v2";
const OTP_PATTERN: &str = r"\b(\d{6})\b";
const POLL_INTERVAL_SECS: u64 = 3;
const DEFAULT_TIMEOUT_SECS: u64 = 120;
const OTP_SENT_AT_TOLERANCE_SECS: f64 = 2.0;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TempmailInbox {
    pub address: String,
    pub token: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct TempmailEmail {
    pub id: Option<String>,
    pub from: Option<String>,
    pub subject: Option<String>,
    pub body: Option<String>,
    pub html: Option<String>,
    pub received_at: Option<serde_json::Value>,
    pub date: Option<serde_json::Value>,
    #[serde(alias = "createdAt")]
    pub created_at: Option<serde_json::Value>,
    pub timestamp: Option<serde_json::Value>,
}

#[derive(Debug, Deserialize)]
struct InboxResponse {
    emails: Option<Vec<TempmailEmail>>,
}

pub struct TempmailClient {
    client: reqwest::Client,
}

impl TempmailClient {
    pub fn new(proxy_url: Option<&str>) -> anyhow::Result<Self> {
        let mut builder = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(30))
            .user_agent("Mozilla/5.0");

        if let Some(proxy) = proxy_url {
            builder = builder.proxy(reqwest::Proxy::all(proxy)?);
        }

        Ok(Self {
            client: builder.build()?,
        })
    }

    /// Create a new temporary inbox. Returns (address, token).
    pub async fn create_inbox(&self) -> anyhow::Result<TempmailInbox> {
        let resp = self
            .client
            .post(format!("{}/inbox/create", BASE_URL))
            .header("Accept", "application/json")
            .header("Content-Type", "application/json")
            .body("{}")
            .send()
            .await?;

        if !resp.status().is_success() {
            anyhow::bail!("Tempmail create failed: HTTP {}", resp.status());
        }

        let inbox: TempmailInbox = resp.json().await?;
        if inbox.address.is_empty() || inbox.token.is_empty() {
            anyhow::bail!("Tempmail returned empty address or token");
        }
        Ok(inbox)
    }

    /// Poll inbox until a 6-digit OTP from OpenAI is found, or timeout.
    /// `otp_sent_at_secs`: unix timestamp before which emails are ignored.
    pub async fn poll_for_otp(
        &self,
        token: &str,
        otp_sent_at_secs: Option<f64>,
        timeout_secs: Option<u64>,
        should_cancel: impl Fn() -> bool,
        mut progress_cb: impl FnMut(String),
    ) -> anyhow::Result<Option<String>> {
        let timeout = timeout_secs.unwrap_or(DEFAULT_TIMEOUT_SECS);
        let re = regex::Regex::new(OTP_PATTERN).unwrap();
        let started = std::time::Instant::now();
        let mut seen_ids = std::collections::HashSet::new();
        let mut first_poll = true;

        loop {
            if should_cancel() {
                progress_cb("[任务] ⛔ 已取消 OTP 等待".to_string());
                return Err(anyhow::anyhow!("注册已取消"));
            }

            if started.elapsed().as_secs() >= timeout {
                return Ok(None);
            }

            if first_poll {
                first_poll = false;
            } else {
                tokio::time::sleep(std::time::Duration::from_secs(POLL_INTERVAL_SECS)).await;
            }

            if should_cancel() {
                progress_cb("[任务] ⛔ 已取消 OTP 等待".to_string());
                return Err(anyhow::anyhow!("注册已取消"));
            }

            let resp = match self
                .client
                .get(format!("{}/inbox", BASE_URL))
                .query(&[("token", token)])
                .header("Accept", "application/json")
                .send()
                .await
            {
                Ok(r) => r,
                Err(e) => {
                    progress_cb(format!("[警告] 检查邮件出错: {}", e));
                    continue;
                }
            };

            if resp.status() == 404 {
                return Err(anyhow::anyhow!("Tempmail inbox expired"));
            }
            if !resp.status().is_success() {
                continue;
            }

            let body: serde_json::Value = match resp.json().await {
                Ok(v) => v,
                Err(_) => continue,
            };
            if body.as_object().map(|o| o.is_empty()).unwrap_or(false) {
                return Err(anyhow::anyhow!("Tempmail inbox expired"));
            }
            let inbox: InboxResponse = match serde_json::from_value(body) {
                Ok(v) => v,
                Err(_) => continue,
            };

            let mut emails = inbox.emails.unwrap_or_default();
            emails.sort_by(|a, b| {
                let a_ts = extract_timestamp(&a.received_at)
                    .or_else(|| extract_timestamp(&a.date))
                    .or_else(|| extract_timestamp(&a.created_at))
                    .or_else(|| extract_timestamp(&a.timestamp))
                    .unwrap_or_default();
                let b_ts = extract_timestamp(&b.received_at)
                    .or_else(|| extract_timestamp(&b.date))
                    .or_else(|| extract_timestamp(&b.created_at))
                    .or_else(|| extract_timestamp(&b.timestamp))
                    .unwrap_or_default();
                b_ts.partial_cmp(&a_ts).unwrap_or(std::cmp::Ordering::Equal)
            });

            for email in &emails {
                let msg_timestamp = extract_timestamp(&email.received_at)
                    .or_else(|| extract_timestamp(&email.date))
                    .or_else(|| extract_timestamp(&email.created_at))
                    .or_else(|| extract_timestamp(&email.timestamp));

                if let Some(anchor) = otp_sent_at_secs {
                    let min_allowed_timestamp = anchor - OTP_SENT_AT_TOLERANCE_SECS;
                    if msg_timestamp.is_none() || msg_timestamp.unwrap_or_default() <= min_allowed_timestamp {
                        continue;
                    }
                }

                // Deduplicate by id or date
                let msg_id = email
                    .id
                    .clone()
                    .or_else(|| email.date.as_ref().map(|v| v.to_string()))
                    .or_else(|| email.created_at.as_ref().map(|v| v.to_string()))
                    .or_else(|| email.timestamp.as_ref().map(|v| v.to_string()))
                    .unwrap_or_else(|| {
                        format!(
                            "{}:{}:{}",
                            email.from.as_deref().unwrap_or(""),
                            email.subject.as_deref().unwrap_or(""),
                            msg_timestamp.unwrap_or_default()
                        )
                    });

                if seen_ids.contains(&msg_id) {
                    continue;
                }
                seen_ids.insert(msg_id);

                let sender = email.from.as_deref().unwrap_or("").to_lowercase();
                let subject = email.subject.as_deref().unwrap_or("");
                let body = email.body.as_deref().unwrap_or("");
                let html = email.html.as_deref().unwrap_or("");
                let content = format!("{}\n{}\n{}\n{}", sender, subject, body, html);
                let content_lower = content.to_lowercase();

                if !sender.contains("openai") && !content_lower.contains("openai") {
                    continue;
                }

                if let Some(cap) = re.captures(&content) {
                    let code = cap[1].to_string();
                    progress_cb(format!("[任务] 📧 OTP 已收到: {}", code));
                    return Ok(Some(code));
                }
            }

            let elapsed = started.elapsed().as_secs();
            progress_cb(format!(
                "[任务] 已检查 {} 封邮件，等待 OTP... ({}/{}s)",
                seen_ids.len(),
                elapsed,
                timeout
            ));
        }
    }
}

fn extract_timestamp(v: &Option<serde_json::Value>) -> Option<f64> {
    let val = v.as_ref()?;
    if let Some(n) = val.as_f64() {
        let mut ts = n;
        while ts > 1e11 {
            ts /= 1000.0;
        }
        return Some(ts);
    }
    if let Some(s) = val.as_str() {
        if let Ok(dt) = chrono::DateTime::parse_from_rfc3339(s) {
            return Some(dt.timestamp() as f64);
        }
        if let Ok(n) = s.parse::<f64>() {
            let mut ts = n;
            while ts > 1e11 {
                ts /= 1000.0;
            }
            return Some(ts);
        }
    }
    None
}

#[cfg(test)]
mod tests {
    #[test]
    fn otp_regex_matches_six_digit_code() {
        let re = regex::Regex::new(super::OTP_PATTERN).expect("valid OTP regex");
        let caps = re.captures("Your OpenAI verification code is 123456.");
        assert_eq!(caps.and_then(|m| m.get(1)).map(|m| m.as_str()), Some("123456"));
    }
}
