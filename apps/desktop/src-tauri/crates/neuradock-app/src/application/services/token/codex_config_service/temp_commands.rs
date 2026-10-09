use anyhow::Result;
use serde_json::json;

use super::helpers::{
    apply_provider_config, ensure_sk_prefix, sanitize_provider_slug, GENERIC_PROVIDER_NAME,
    GENERIC_PROVIDER_SLUG,
};
use neuradock_domain::token::ApiToken;

pub(super) fn generate_temp_commands_impl(
    token: &ApiToken,
    provider_id: &str,
    provider_name: &str,
    base_url: &str,
    model: Option<&str>,
    context_1m: bool,
) -> Result<String> {
    let provider_slug = sanitize_provider_slug(provider_id);
    let provider_name = if provider_name.trim().is_empty() {
        provider_id
    } else {
        provider_name
    };
    build_commands(
        token.key(),
        &provider_slug,
        provider_name,
        base_url,
        model,
        context_1m,
    )
}

pub(super) fn generate_temp_commands_with_key_impl(
    api_key: &str,
    base_url: &str,
    model: Option<&str>,
    context_1m: bool,
) -> Result<String> {
    build_commands(
        api_key,
        GENERIC_PROVIDER_SLUG,
        GENERIC_PROVIDER_NAME,
        base_url,
        model,
        context_1m,
    )
}

fn build_commands(
    api_key: &str,
    provider_slug: &str,
    provider_name: &str,
    base_url: &str,
    model: Option<&str>,
    context_1m: bool,
) -> Result<String> {
    if let Some(model) = model {
        super::helpers::validate_codex_model(model)?;
    }
    let mut config = toml_edit::DocumentMut::new();
    apply_provider_config(
        &mut config,
        provider_slug,
        provider_name,
        base_url,
        model,
        context_1m,
    );
    super::helpers::apply_bearer_token(&mut config, api_key);
    let auth = serde_json::to_string_pretty(&json!({
        "OPENAI_API_KEY": ensure_sk_prefix(api_key),
    }))?;

    Ok([
        "export CODEX_HOME=\"$(mktemp -d \"${TMPDIR:-/tmp}/neuradock-codex.XXXXXX\")\"".to_string(),
        "mkdir -p \"$CODEX_HOME\"".to_string(),
        format!(
            "printf '%s\\n' {} > \"$CODEX_HOME/auth.json\"",
            shell_quote(&auth)
        ),
        format!(
            "printf '%s\\n' {} > \"$CODEX_HOME/config.toml\"",
            shell_quote(&config.to_string())
        ),
        "codex".to_string(),
    ]
    .join("\n"))
}

fn shell_quote(value: &str) -> String {
    format!("'{}'", value.replace('\'', "'\"'\"'"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn temporary_commands_create_isolated_codex_home() {
        let commands = generate_temp_commands_with_key_impl(
            "test-key",
            "https://relay.example",
            Some("gpt-test"),
            true,
        )
        .unwrap();

        assert!(commands.contains("export CODEX_HOME="));
        assert!(commands.contains("OPENAI_API_KEY"));
        assert!(commands.contains("sk-test-key"));
        assert!(commands.contains("model_context_window = 1000000"));
        assert!(commands.ends_with("codex"));
    }

    #[test]
    fn shell_quote_handles_single_quotes() {
        assert_eq!(shell_quote("a'b"), "'a'\"'\"'b'");
    }
}
