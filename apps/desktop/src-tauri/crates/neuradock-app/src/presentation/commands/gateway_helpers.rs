pub(super) fn canonical_url(value: &str) -> String {
    let trimmed = value.trim().trim_end_matches('/');
    let Ok(mut parsed) = url::Url::parse(trimmed) else {
        return trimmed.to_string();
    };
    parsed.set_query(None);
    parsed.set_fragment(None);
    let _ = parsed.set_username("");
    let _ = parsed.set_password(None);
    parsed.to_string().trim_end_matches('/').to_string()
}

pub(super) fn display_endpoint(value: &str) -> String {
    let Ok(parsed) = url::Url::parse(value) else {
        return value.to_string();
    };
    match (parsed.host_str(), parsed.port()) {
        (Some(host), Some(port)) => format!("{host}:{port}"),
        (Some(host), None) => host.to_string(),
        _ => value.to_string(),
    }
}

pub(super) fn model_without_context_marker(value: &str) -> (String, bool) {
    let trimmed = value.trim();
    if trimmed.len() >= 4 && trimmed[trimmed.len() - 4..].eq_ignore_ascii_case("[1m]") {
        (trimmed[..trimmed.len() - 4].trim_end().to_string(), true)
    } else {
        (trimmed.to_string(), false)
    }
}
