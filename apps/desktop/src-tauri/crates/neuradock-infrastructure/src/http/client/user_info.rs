use anyhow::{Context, Result};
use reqwest::{header, Client};
use std::collections::HashMap;

use super::types::{extract_domain, extract_set_cookies, SetCookieResult, UserInfo};

impl super::HttpClient {
    /// Get user info (quota and used quota) with retry
    pub async fn get_user_info(
        &self,
        url: &str,
        cookies: &HashMap<String, String>,
        api_user_key: &str,
        api_user_value: &str,
    ) -> Result<(UserInfo, SetCookieResult)> {
        let url = url.to_string();
        let cookies = cookies.clone();
        let api_user_key = api_user_key.to_string();
        let api_user_value = api_user_value.to_string();

        self.execute_with_retry("Get user info", move || {
            let url = url.clone();
            let cookies = cookies.clone();
            let api_user_key = api_user_key.clone();
            let api_user_value = api_user_value.clone();
            let client = self.client.clone();

            async move {
                Self::get_user_info_once(&client, &url, &cookies, &api_user_key, &api_user_value)
                    .await
            }
        })
        .await
    }

    /// Get user info (quota and used quota) - single attempt
    async fn get_user_info_once(
        client: &Client,
        url: &str,
        cookies: &HashMap<String, String>,
        api_user_key: &str,
        api_user_value: &str,
    ) -> Result<(UserInfo, SetCookieResult)> {
        // Build headers
        let mut headers = header::HeaderMap::new();
        headers.insert(
            header::ACCEPT,
            header::HeaderValue::from_static("application/json, text/plain, */*"),
        );
        headers.insert(
            header::ACCEPT_LANGUAGE,
            header::HeaderValue::from_static("zh-CN,zh;q=0.9,en;q=0.8"),
        );
        headers.insert(
            header::REFERER,
            header::HeaderValue::from_str(&extract_domain(url)?)?,
        );

        // Add API user header
        if !api_user_value.is_empty() {
            headers.insert(
                header::HeaderName::from_bytes(api_user_key.as_bytes())?,
                header::HeaderValue::from_str(api_user_value)?,
            );
        }

        // Build request with cookies
        let mut request = client.get(url).headers(headers);

        // Add cookies as header string
        let cookie_string = cookies
            .iter()
            .map(|(k, v)| format!("{}={}", k, v))
            .collect::<Vec<_>>()
            .join("; ");

        if !cookie_string.is_empty() {
            request = request.header(header::COOKIE, cookie_string);
        }

        // Send request
        let response = request
            .send()
            .await
            .context("Failed to send user info request")?;

        let status = response.status();
        log::info!("User info response status: {}", status);

        // Extract Set-Cookie headers before consuming the response body
        let set_cookie_result = extract_set_cookies(&response);

        if !status.is_success() {
            let error_text = response
                .text()
                .await
                .unwrap_or_else(|_| "Unable to read response".to_string());
            log::error!(
                "User info request failed with status {}: {}",
                status,
                crate::logging::log_utils::truncate_for_log(&error_text, 500)
            );

            // Try to extract message from JSON response
            let error_message =
                if let Ok(json) = serde_json::from_str::<serde_json::Value>(&error_text) {
                    json["message"].as_str().unwrap_or(&error_text).to_string()
                } else {
                    error_text.clone()
                };

            // Check if this is a WAF challenge
            if error_text.contains("acw_sc__v2") || error_text.contains("<script>var arg1=") {
                anyhow::bail!(
                    "WAF_CHALLENGE: {}",
                    crate::logging::log_utils::truncate_for_log(&error_text, 500)
                );
            }
            anyhow::bail!("{}", error_message);
        }

        // Get response text first to check for WAF challenge
        let response_text = response
            .text()
            .await
            .context("Failed to read user info response")?;

        log::debug!("User info response length: {} bytes", response_text.len());

        // Check if response is HTML (WAF challenge page)
        if response_text.trim().starts_with('<')
            || response_text.contains("acw_sc__v2")
            || response_text.contains("<script>var arg1=")
        {
            log::warn!(
                "Received WAF challenge page instead of JSON: {}",
                crate::logging::log_utils::truncate_for_log(&response_text, 200)
            );
            anyhow::bail!(
                "WAF_CHALLENGE: Received HTML instead of JSON - {}",
                crate::logging::log_utils::truncate_for_log(&response_text, 500)
            );
        }

        let data: serde_json::Value = serde_json::from_str(&response_text).context(format!(
            "Failed to parse user info response: {}",
            crate::logging::log_utils::truncate_for_log(&response_text, 200)
        ))?;

        // Body is not logged: `/api/user/self` returns the account's email,
        // username and access_token alongside the quota fields. This ran at
        // info! level, so it landed in the log file on every refresh.
        log::debug!(
            "User info API response received ({} top-level keys)",
            data.as_object().map(|o| o.len()).unwrap_or(0)
        );

        // Check if response has expected structure
        if data["data"].is_null() {
            anyhow::bail!("API response missing 'data' field: {}", data);
        }

        // Parse quota and used_quota (convert raw quota units to dollars, 500000 units = $1)
        let quota_bytes = data["data"]["quota"]
            .as_f64()
            .ok_or_else(|| anyhow::anyhow!("Missing or invalid 'quota' field in API response"))?;
        let used_quota_bytes = data["data"]["used_quota"].as_f64().ok_or_else(|| {
            anyhow::anyhow!("Missing or invalid 'used_quota' field in API response")
        })?;

        let (current_balance, total_consumed, total_quota) =
            normalize_quota(quota_bytes, used_quota_bytes);
        // NOTE: Upstream's HTTP payload still calls `quota`, `used_quota`, and `total_income`.
        // We normalize semantics right here so the rest of the app only deals with
        // `current_balance`, `total_consumed`, and `total_quota` to avoid confusion.
        Ok((
            UserInfo {
                current_balance,
                total_consumed,
                total_quota,
            },
            set_cookie_result,
        ))
    }
}

/// Raw quota units per dollar, as used by new-api / one-api style providers.
const QUOTA_UNITS_PER_DOLLAR: f64 = 500000.0;

fn round_cents(dollars: f64) -> f64 {
    (dollars * 100.0).round() / 100.0
}

/// Convert raw provider quota units into `(current_balance, total_consumed, total_quota)` dollars.
///
/// `total_quota` is derived from the RAW unit sum rather than by adding the two
/// already-rounded dollar values, so sub-cent remainders that would otherwise be
/// discarded twice are preserved. This bounds the derivation error at one cent.
///
/// Note this only addresses rounding performed *here*. If the provider's own
/// quota accounting loses value (as happened for one AnyRouter account on
/// 2026-07-11, where a check-in credited 24.98 instead of 25.00), no local
/// derivation can recover it — the shortfall arrives already baked into the
/// raw units. Such drift is persisted and inherited by every later snapshot,
/// so it must be corrected in the stored history.
fn normalize_quota(quota_units: f64, used_quota_units: f64) -> (f64, f64, f64) {
    let current_balance = round_cents(quota_units / QUOTA_UNITS_PER_DOLLAR);
    let total_consumed = round_cents(used_quota_units / QUOTA_UNITS_PER_DOLLAR);
    let total_quota = round_cents((quota_units + used_quota_units) / QUOTA_UNITS_PER_DOLLAR);
    (current_balance, total_consumed, total_quota)
}

#[cfg(test)]
mod quota_tests {
    use super::normalize_quota;

    const D: f64 = 500000.0;

    #[test]
    fn total_quota_differs_from_naive_sum_when_sub_cent_remainders_accumulate() {
        // Both operands carry a sub-cent remainder that individually rounds DOWN,
        // but together exceeds half a cent. The naive `current + consumed`
        // derivation loses that cent; deriving from the raw unit sum keeps it.
        let quota = 1000.004 * D;
        let used = 2000.004 * D;

        let (current, consumed, total) = normalize_quota(quota, used);

        assert!((current - 1000.0).abs() < 1e-9);
        assert!((consumed - 2000.0).abs() < 1e-9);
        assert!(
            (current + consumed - 3000.00).abs() < 1e-9,
            "naive sum: {}",
            current + consumed
        );
        assert!((total - 3000.01).abs() < 1e-9, "total_quota: {}", total);
    }

    #[test]
    fn a_two_cent_upstream_discrepancy_is_not_recoverable_by_rounding() {
        // Documents the limit of this fix, so it is not mistaken for a cure for
        // the AnyRouter 2026-07-11 incident. That day the persisted snapshot was
        // current=4522.10 / consumed=4347.88 and total_quota came out 0.02 below
        // the expected whole-dollar step (24.98 instead of 25.00).
        //
        // Two independent cent-roundings can only ever shift the sum by <=0.01,
        // so no derivation over these operands can reach 8870.00. The missing
        // 0.02 was lost in the provider's own unit accounting, not here.
        for (q, u) in [
            (4522.095, 4347.875),
            (4522.100, 4347.880),
            (4522.1049, 4347.8849),
        ] {
            let (current, consumed, total) = normalize_quota(q * D, u * D);
            assert!((current - 4522.10).abs() < 1e-9, "current: {}", current);
            assert!((consumed - 4347.88).abs() < 1e-9, "consumed: {}", consumed);
            assert!(
                total < 8870.0,
                "rounding must not be credited with recovering the lost 0.02: {}",
                total
            );
        }
    }

    #[test]
    fn exact_values_are_preserved() {
        let (current, consumed, total) = normalize_quota(4497.22 * D, 4347.78 * D);
        assert!((current - 4497.22).abs() < 1e-9);
        assert!((consumed - 4347.78).abs() < 1e-9);
        assert!((total - 8845.0).abs() < 1e-9);
    }

    #[test]
    fn zero_quota_yields_zeros() {
        assert_eq!(normalize_quota(0.0, 0.0), (0.0, 0.0, 0.0));
    }

    #[test]
    fn total_quota_never_deviates_from_naive_sum_by_more_than_a_cent() {
        // Sweep sub-cent offsets on both operands; the two derivations must
        // agree to within one cent, and total_quota must always be cent-exact.
        for i in 0..100 {
            for j in 0..100 {
                let quota = (1000.0 + i as f64 * 0.001) * D;
                let used = (2000.0 + j as f64 * 0.001) * D;
                let (current, consumed, total) = normalize_quota(quota, used);
                assert!(
                    (total - (current + consumed)).abs() <= 0.01 + 1e-9,
                    "i={} j={} total={} naive={}",
                    i,
                    j,
                    total,
                    current + consumed
                );
                assert!(
                    (total * 100.0 - (total * 100.0).round()).abs() < 1e-6,
                    "total_quota not cent-exact: {}",
                    total
                );
            }
        }
    }
}
