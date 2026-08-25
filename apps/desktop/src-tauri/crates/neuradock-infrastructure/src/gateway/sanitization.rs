use serde_json::Value;

pub(crate) fn strip_one_m_model_marker(model: &str) -> &str {
    let trimmed = model.trim_end();
    let marker = "[1m]";
    if trimmed.len() >= marker.len()
        && trimmed[trimmed.len() - marker.len()..].eq_ignore_ascii_case(marker)
    {
        trimmed[..trimmed.len() - marker.len()].trim_end()
    } else {
        model
    }
}

pub(crate) fn build_upstream_url(base_url: &str, endpoint: &str) -> String {
    let base = base_url.trim_end_matches('/');
    let ep = endpoint.trim_start_matches('/');
    let origin_only = url::Url::parse(base)
        .map(|u| u.path() == "/" || u.path().is_empty())
        .unwrap_or(false);
    let mut combined = if base.ends_with("/v1") {
        format!("{base}/{ep}")
    } else if origin_only {
        format!("{base}/v1/{ep}")
    } else {
        format!("{base}/{ep}")
    };
    while combined.contains("/v1/v1") {
        combined = combined.replace("/v1/v1", "/v1");
    }
    combined
}

pub(crate) fn sanitized_url_for_log(value: &str) -> String {
    let Ok(mut parsed) = url::Url::parse(value) else {
        return "<invalid-upstream-url>".to_string();
    };
    let _ = parsed.set_username("");
    let _ = parsed.set_password(None);
    parsed.set_query(None);
    parsed.set_fragment(None);
    parsed.to_string()
}

pub(crate) fn value_error_message(value: &Value) -> Option<String> {
    value
        .pointer("/error/message")
        .or_else(|| value.get("message"))
        .and_then(Value::as_str)
        .map(sanitize_error_message)
}

pub(crate) fn sanitize_error_message(message: &str) -> String {
    let compact = message.split_whitespace().collect::<Vec<_>>().join(" ");
    compact.chars().take(500).collect()
}

pub(crate) fn redact_sensitive_message(message: &str, secret: &str) -> String {
    let sanitized = sanitize_error_message(message);
    if secret.is_empty() {
        sanitized
    } else {
        sanitized.replace(secret, "[redacted]")
    }
}
