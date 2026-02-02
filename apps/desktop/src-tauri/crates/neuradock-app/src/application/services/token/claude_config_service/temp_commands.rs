use anyhow::Result;

use super::helpers::ensure_sk_prefix;
use neuradock_domain::token::ApiToken;

pub(super) fn generate_temp_commands_impl(
    token: &ApiToken,
    base_url: &str,
    model: Option<&str>,
) -> Result<String> {
    let api_key = ensure_sk_prefix(token.key());
    generate_temp_commands_with_key_impl(&api_key, base_url, model)
}

pub(super) fn generate_temp_commands_with_key_impl(
    api_key: &str,
    base_url: &str,
    model: Option<&str>,
) -> Result<String> {
    let api_key = ensure_sk_prefix(api_key);
    let mut commands = vec![
        format!("export ANTHROPIC_AUTH_TOKEN=\"{}\"", api_key),
        format!("export ANTHROPIC_BASE_URL=\"{}\"", base_url),
        "export CLAUDE_CODE_DISABLE_NONESSENTIAL_TRAFFIC=\"1\"".to_string(),
        "export DISABLE_TELEMETRY=\"1\"".to_string(),
        "export API_TIMEOUT_MS=\"3000000\"".to_string(),
        "export CLAUDE_CODE_ATTRIBUTION_HEADER=\"0\"".to_string(),
    ];

    if let Some(m) = model {
        commands.push(format!("export ANTHROPIC_DEFAULT_HAIKU_MODEL=\"{}\"", m));
        commands.push(format!("export ANTHROPIC_DEFAULT_SONNET_MODEL=\"{}\"", m));
        commands.push(format!("export ANTHROPIC_DEFAULT_OPUS_MODEL=\"{}\"", m));
    }

    Ok(commands.join("\n"))
}
