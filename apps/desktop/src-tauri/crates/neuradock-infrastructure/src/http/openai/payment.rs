use anyhow::{anyhow, bail, Result};
use rquest_util::Emulation;
use serde::{Deserialize, Serialize};
use serde_json::json;

use neuradock_domain::codex::CodexAccount;

const PAYMENT_CHECKOUT_URL: &str = "https://chatgpt.com/backend-api/payments/checkout";
const CHECKOUT_BASE_URL: &str = "https://chatgpt.com/checkout/openai_llc/";

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PaymentPlan {
    Plus,
    Team,
}

impl PaymentPlan {
    fn promo_campaign_id(&self) -> &'static str {
        match self {
            Self::Plus => "plus-1-month-free",
            Self::Team => "team-1-month-free",
        }
    }

    fn plan_name(&self) -> &'static str {
        match self {
            Self::Plus => "chatgptplusplan",
            Self::Team => "chatgptteamplan",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PaymentInterval {
    Month,
    Year,
}

impl PaymentInterval {
    fn as_str(&self) -> &'static str {
        match self {
            Self::Month => "month",
            Self::Year => "year",
        }
    }
}

#[derive(Debug, Clone)]
pub struct PaymentLinkRequest {
    pub plan: PaymentPlan,
    pub country: String,
    pub currency: String,
    pub workspace_name: Option<String>,
    pub seat_quantity: Option<u32>,
    pub price_interval: Option<PaymentInterval>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PaymentLink {
    pub url: String,
    pub session_id: String,
    pub country: String,
    pub currency: String,
}

#[derive(Debug, Deserialize)]
struct CheckoutResponse {
    checkout_session_id: Option<String>,
    detail: Option<String>,
}

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

fn build_client(proxy_url: Option<&str>) -> Result<rquest::Client> {
    let effective_proxy = proxy_url
        .map(|s| s.to_string())
        .or_else(detect_env_proxy);

    let mut builder = rquest::Client::builder()
        .connect_timeout(std::time::Duration::from_secs(15))
        .timeout(std::time::Duration::from_secs(45))
        .emulation(Emulation::Chrome124);

    if let Some(proxy) = effective_proxy.as_deref() {
        builder = builder.proxy(rquest::Proxy::all(proxy)?);
    }

    builder.build().map_err(Into::into)
}

fn expected_currency(country: &str) -> Option<&'static str> {
    match country {
        "SG" => Some("SGD"),
        "US" => Some("USD"),
        "TR" => Some("TRY"),
        "JP" => Some("JPY"),
        "HK" => Some("HKD"),
        "GB" => Some("GBP"),
        "EU" => Some("EUR"),
        "AU" => Some("AUD"),
        "CA" => Some("CAD"),
        "IN" => Some("INR"),
        "BR" => Some("BRL"),
        "MX" => Some("MXN"),
        _ => None,
    }
}

fn extract_oai_did(cookie_header: &str) -> Option<String> {
    cookie_header
        .split(';')
        .map(|part| part.trim())
        .find_map(|part| part.strip_prefix("oai-did=").map(|value| value.trim().to_string()))
}

fn normalize_request(request: &PaymentLinkRequest) -> Result<(String, String)> {
    let country = request.country.trim().to_uppercase();
    if country.is_empty() {
        bail!("计费国家不能为空");
    }

    let expected = expected_currency(&country)
        .ok_or_else(|| anyhow!("暂不支持该计费国家: {}", request.country.trim()))?;
    let currency = request.currency.trim().to_uppercase();

    if currency.is_empty() {
        return Ok((country, expected.to_string()));
    }

    if currency != expected {
        bail!(
            "货币与计费国家不匹配：{} 对应 {}，当前为 {}",
            country,
            expected,
            currency
        );
    }

    Ok((country, currency))
}

fn build_payload(request: &PaymentLinkRequest, country: &str, currency: &str) -> Result<serde_json::Value> {
    let promo_campaign = json!({
        "promo_campaign_id": request.plan.promo_campaign_id(),
        "is_coupon_from_query_param": matches!(request.plan, PaymentPlan::Team),
    });

    Ok(match request.plan {
        PaymentPlan::Plus => json!({
            "plan_name": request.plan.plan_name(),
            "billing_details": {
                "country": country,
                "currency": currency,
            },
            "promo_campaign": promo_campaign,
            "checkout_ui_mode": "custom",
        }),
        PaymentPlan::Team => {
            let workspace_name = request
                .workspace_name
                .as_deref()
                .map(str::trim)
                .filter(|value| !value.is_empty())
                .unwrap_or("MyTeam");
            let seat_quantity = request.seat_quantity.unwrap_or(5).max(1);
            let interval = request
                .price_interval
                .clone()
                .unwrap_or(PaymentInterval::Month);

            json!({
                "plan_name": request.plan.plan_name(),
                "team_plan_data": {
                    "workspace_name": workspace_name,
                    "price_interval": interval.as_str(),
                    "seat_quantity": seat_quantity,
                },
                "billing_details": {
                    "country": country,
                    "currency": currency,
                },
                "promo_campaign": promo_campaign,
                "cancel_url": "https://chatgpt.com/#pricing",
                "checkout_ui_mode": "custom",
            })
        }
    })
}

pub async fn generate_payment_link(
    account: &CodexAccount,
    request: &PaymentLinkRequest,
    proxy_url: Option<&str>,
) -> Result<PaymentLink> {
    let access_token = account
        .access_token()
        .ok_or_else(|| anyhow!("账号缺少 access_token"))?;
    let (country, currency) = normalize_request(request)?;
    let payload = build_payload(request, &country, &currency)?;
    let client = build_client(proxy_url)?;

    let mut req = client
        .post(PAYMENT_CHECKOUT_URL)
        .header("authorization", format!("Bearer {}", access_token))
        .header("accept", "application/json")
        .header("content-type", "application/json")
        .header("oai-language", "zh-CN");

    if let Some(account_id) = account.account_id().filter(|value| !value.trim().is_empty()) {
        req = req.header("chatgpt-account-id", account_id);
    }

    if let Some(cookie_header) = account.web_session_cookie().filter(|value| !value.trim().is_empty()) {
        req = req.header("cookie", cookie_header);
        if let Some(device_id) = account
            .web_session_device_id()
            .map(|value| value.to_string())
            .or_else(|| extract_oai_did(cookie_header))
        {
            req = req.header("oai-device-id", device_id);
        }
    }

    let resp = req
        .json(&payload)
        .send()
        .await
        .map_err(|e| anyhow!("请求支付接口失败: {}", e))?;
    let status = resp.status();
    let body = resp.text().await.unwrap_or_default();

    if !status.is_success() {
        let preview = body.replace('\n', " ");
        bail!(
            "支付接口返回 {}：{}",
            status,
            &preview[..preview.len().min(240)]
        );
    }

    let data: CheckoutResponse = serde_json::from_str(&body)
        .map_err(|e| anyhow!("解析支付响应失败: {} (body={})", e, &body[..body.len().min(240)]))?;

    let session_id = data
        .checkout_session_id
        .filter(|value| !value.is_empty())
        .ok_or_else(|| anyhow!(data.detail.unwrap_or_else(|| "API 未返回 checkout_session_id".to_string())))?;

    Ok(PaymentLink {
        url: format!("{}{}", CHECKOUT_BASE_URL, session_id),
        session_id,
        country,
        currency,
    })
}

#[cfg(test)]
mod tests {
    use super::{build_payload, normalize_request, PaymentInterval, PaymentLinkRequest, PaymentPlan};

    #[test]
    fn normalizes_country_and_currency() {
        let request = PaymentLinkRequest {
            plan: PaymentPlan::Plus,
            country: "sg".to_string(),
            currency: "sgd".to_string(),
            workspace_name: None,
            seat_quantity: None,
            price_interval: None,
        };

        let (country, currency) = normalize_request(&request).expect("request should be valid");
        assert_eq!(country, "SG");
        assert_eq!(currency, "SGD");
    }

    #[test]
    fn builds_team_payload_from_request() {
        let request = PaymentLinkRequest {
            plan: PaymentPlan::Team,
            country: "JP".to_string(),
            currency: "JPY".to_string(),
            workspace_name: Some("NeuraDock Team".to_string()),
            seat_quantity: Some(7),
            price_interval: Some(PaymentInterval::Year),
        };

        let payload = build_payload(&request, "JP", "JPY").expect("payload should be built");
        assert_eq!(payload["plan_name"], "chatgptteamplan");
        assert_eq!(payload["billing_details"]["country"], "JP");
        assert_eq!(payload["billing_details"]["currency"], "JPY");
        assert_eq!(payload["team_plan_data"]["workspace_name"], "NeuraDock Team");
        assert_eq!(payload["team_plan_data"]["seat_quantity"], 7);
        assert_eq!(payload["team_plan_data"]["price_interval"], "year");
        assert_eq!(payload["promo_campaign"]["promo_campaign_id"], "team-1-month-free");
    }
}
