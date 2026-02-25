use anyhow::{Context, Result};
use reqwest::header;
use std::collections::HashMap;

use super::types::{extract_set_cookies, SetCookieResult};

impl super::HttpClient {
    /// Visit login page (for providers that trigger check-in on login page visit)
    pub async fn visit_login_page(
        &self,
        url: &str,
        cookies: &HashMap<String, String>,
    ) -> Result<SetCookieResult> {
        log::info!("Visiting login page: {}", url);

        // Build headers
        let mut headers = header::HeaderMap::new();
        headers.insert(
            header::ACCEPT,
            header::HeaderValue::from_static(
                "text/html,application/xhtml+xml,application/xml;q=0.9,*/*;q=0.8",
            ),
        );
        headers.insert(
            header::ACCEPT_LANGUAGE,
            header::HeaderValue::from_static("zh-CN,zh;q=0.9,en;q=0.8"),
        );

        // Build request with cookies
        let mut request = self.client.get(url).headers(headers);

        // Add cookies as header string
        let cookie_string = cookies
            .iter()
            .map(|(k, v)| format!("{}={}", k, v))
            .collect::<Vec<_>>()
            .join("; ");

        if !cookie_string.is_empty() {
            request = request.header(header::COOKIE, cookie_string);
        }

        // Send request (will auto-follow redirects)
        let response = request.send().await.context("Failed to visit login page")?;

        let status = response.status();
        let final_url = response.url().to_string();

        log::info!(
            "Login page visit status: {}, final URL after redirects: {}",
            status,
            final_url
        );

        // Extract Set-Cookie headers before consuming the response
        let set_cookie_result = extract_set_cookies(&response);

        if !status.is_success() {
            anyhow::bail!("Failed to visit login page, status: {}", status);
        }

        Ok(set_cookie_result)
    }
}
