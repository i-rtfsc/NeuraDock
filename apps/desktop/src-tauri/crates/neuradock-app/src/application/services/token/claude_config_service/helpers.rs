use anyhow::{Context, Result};
use std::path::PathBuf;

// Keys that we manage in the env section
pub(super) const MANAGED_ENV_KEYS: &[&str] = &[
    "ANTHROPIC_AUTH_TOKEN",
    "ANTHROPIC_BASE_URL",
    "CLAUDE_CODE_DISABLE_NONESSENTIAL_TRAFFIC",
    "DISABLE_TELEMETRY",
    "API_TIMEOUT_MS",
    "CLAUDE_CODE_ATTRIBUTION_HEADER",
    "ANTHROPIC_DEFAULT_HAIKU_MODEL",
    "ANTHROPIC_DEFAULT_SONNET_MODEL",
    "ANTHROPIC_DEFAULT_OPUS_MODEL",
];

pub(super) fn get_claude_config_path() -> Result<PathBuf> {
    let home = dirs::home_dir().context("Cannot find home directory")?;
    Ok(home.join(".claude").join("settings.json"))
}

pub(super) fn get_claude_isolated_config_path(slug_hint: &str) -> Result<PathBuf> {
    let home = dirs::home_dir().context("Cannot find home directory")?;
    let slug = sanitize_config_slug(slug_hint);
    Ok(home.join(".claude").join("profiles.d").join(format!(
        "settings-{}.json",
        if slug.is_empty() { "profile" } else { &slug }
    )))
}

fn sanitize_config_slug(value: &str) -> String {
    value
        .chars()
        .map(|character| {
            if character.is_alphanumeric() || matches!(character, '-' | '_' | '.') {
                character
            } else {
                '-'
            }
        })
        .collect::<String>()
        .trim_matches('-')
        .to_string()
}

/// Ensure API key has sk- prefix
pub(super) fn ensure_sk_prefix(key: &str) -> String {
    if key.starts_with("sk-") {
        key.to_string()
    } else {
        format!("sk-{}", key)
    }
}

/// Claude Code selects the 1M-context variant via a `[1m]` model id suffix.
pub(super) fn with_1m_suffix(model: &str) -> String {
    if model.ends_with("[1m]") {
        model.to_string()
    } else {
        format!("{}[1m]", model)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn isolated_filename_preserves_readable_provider_names() {
        let path = get_claude_isolated_config_path("中转站 A/B").unwrap();
        assert_eq!(
            path.file_name().and_then(|name| name.to_str()),
            Some("settings-中转站-A-B.json")
        );
    }
}
