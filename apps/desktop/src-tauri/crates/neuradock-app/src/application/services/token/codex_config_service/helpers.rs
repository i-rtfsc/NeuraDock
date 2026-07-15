use anyhow::{Context, Result};
use std::path::PathBuf;

/// Top-level config.toml keys owned by the configure feature. Clearing the
/// configuration removes exactly these (plus the provider table we wrote);
/// everything else in the file belongs to the user.
pub(super) const MANAGED_TOP_LEVEL_KEYS: &[&str] = &[
    "model",
    "model_provider",
    "preferred_auth_method",
    "model_context_window",
];

/// Provider slug used when configuring from an independent key.
pub(super) const GENERIC_PROVIDER_SLUG: &str = "openai_compatible";
pub(super) const GENERIC_PROVIDER_NAME: &str = "OpenAI Compatible API";

/// Fallback model written only when neither the caller nor the existing
/// config supplies one, so Codex never points at a provider without a model.
pub(super) const DEFAULT_MODEL: &str = "gpt-5";

pub(super) fn get_codex_dir() -> Result<PathBuf> {
    let home = dirs::home_dir().context("Cannot find home directory")?;
    Ok(home.join(".codex"))
}

pub(super) fn get_codex_config_path() -> Result<PathBuf> {
    Ok(get_codex_dir()?.join("config.toml"))
}

pub(super) fn get_codex_auth_path() -> Result<PathBuf> {
    Ok(get_codex_dir()?.join("auth.json"))
}

/// Ensure API key has sk- prefix
pub(super) fn ensure_sk_prefix(key: &str) -> String {
    if key.starts_with("sk-") {
        key.to_string()
    } else {
        format!("sk-{}", key)
    }
}

pub(super) fn sanitize_provider_slug(provider_id: &str) -> String {
    provider_id
        .chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() {
                c.to_ascii_lowercase()
            } else {
                '_'
            }
        })
        .collect()
}

pub(super) fn ensure_v1_base_url(base_url: &str) -> String {
    let trimmed = base_url.trim_end_matches('/');
    if trimmed.ends_with("/v1") {
        trimmed.to_string()
    } else {
        format!("{}/v1", trimmed)
    }
}

/// Context window advertised to Codex when the 1M option is on.
pub(super) const CONTEXT_WINDOW_1M: i64 = 1_000_000;

/// Upsert the provider fields into an existing config.toml document,
/// leaving every unrelated key (mcp_servers, sandbox settings, other
/// model_providers entries, comments, key order) untouched.
pub(super) fn apply_provider_config(
    doc: &mut toml_edit::DocumentMut,
    provider_slug: &str,
    provider_name: &str,
    base_url: &str,
    model: Option<&str>,
    context_1m: bool,
) {
    use toml_edit::{value, Item, Table};

    match model {
        Some(model) => doc["model"] = value(model),
        // Keep the user's existing model; only fall back when none is set.
        None => {
            if doc.get("model").is_none() {
                doc["model"] = value(DEFAULT_MODEL);
            }
        }
    }
    doc["model_provider"] = value(provider_slug);
    doc["preferred_auth_method"] = value("apikey");

    // model_context_window is a managed key: set on, removed when off, so a
    // previous 1M configuration never lingers after the option is unchecked.
    if context_1m {
        doc["model_context_window"] = value(CONTEXT_WINDOW_1M);
    } else {
        doc.remove("model_context_window");
    }

    let providers = doc
        .entry("model_providers")
        .or_insert(Item::Table(Table::new()));
    if !providers.is_table() {
        *providers = Item::Table(Table::new());
    }
    let providers = providers.as_table_mut().expect("just ensured table");
    providers.set_implicit(true);

    let entry = providers
        .entry(provider_slug)
        .or_insert(Item::Table(Table::new()));
    if !entry.is_table() {
        *entry = Item::Table(Table::new());
    }
    let entry = entry.as_table_mut().expect("just ensured table");
    entry["name"] = value(provider_name);
    entry["base_url"] = value(ensure_v1_base_url(base_url));
    entry["wire_api"] = value("responses");
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn apply_preserves_unrelated_keys_and_comments() {
        let existing = r#"# my codex settings
approval_policy = "on-request"

[mcp_servers.docs]
command = "docs-server"

[model_providers.other]
name = "Other"
base_url = "https://other.example/v1"
wire_api = "responses"
"#;
        let mut doc: toml_edit::DocumentMut = existing.parse().unwrap();

        apply_provider_config(
            &mut doc,
            "relay",
            "Relay",
            "https://relay.example",
            None,
            false,
        );

        let out = doc.to_string();
        assert!(out.contains("# my codex settings"));
        assert!(out.contains("approval_policy = \"on-request\""));
        assert!(out.contains("[mcp_servers.docs]"));
        assert!(out.contains("[model_providers.other]"));
        assert!(out.contains("model_provider = \"relay\""));
        assert!(out.contains("base_url = \"https://relay.example/v1\""));
    }

    #[test]
    fn apply_keeps_existing_model_when_none_selected() {
        let mut doc: toml_edit::DocumentMut = "model = \"my-model\"\n".parse().unwrap();

        apply_provider_config(
            &mut doc,
            "relay",
            "Relay",
            "https://relay.example",
            None,
            false,
        );

        assert_eq!(doc["model"].as_str(), Some("my-model"));
    }

    #[test]
    fn apply_writes_default_model_on_empty_config() {
        let mut doc = toml_edit::DocumentMut::new();

        apply_provider_config(
            &mut doc,
            "relay",
            "Relay",
            "https://relay.example",
            None,
            false,
        );

        assert_eq!(doc["model"].as_str(), Some(DEFAULT_MODEL));
    }

    #[test]
    fn apply_overwrites_model_when_selected() {
        let mut doc: toml_edit::DocumentMut = "model = \"old\"\n".parse().unwrap();

        apply_provider_config(
            &mut doc,
            "relay",
            "Relay",
            "https://relay.example/v1",
            Some("new-model"),
            false,
        );

        assert_eq!(doc["model"].as_str(), Some("new-model"));
    }

    #[test]
    fn apply_sets_and_removes_context_window() {
        let mut doc = toml_edit::DocumentMut::new();

        apply_provider_config(&mut doc, "relay", "Relay", "https://r.example", None, true);
        assert_eq!(
            doc["model_context_window"].as_integer(),
            Some(CONTEXT_WINDOW_1M)
        );

        apply_provider_config(&mut doc, "relay", "Relay", "https://r.example", None, false);
        assert!(doc.get("model_context_window").is_none());
    }
}
