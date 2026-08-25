use super::gateway_helpers::{canonical_url, display_endpoint, model_without_context_marker};
use std::{collections::BTreeMap, path::PathBuf};

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub(super) struct AgentConfigFingerprint {
    pub(super) managed_values: BTreeMap<String, String>,
    pub(super) endpoint: Option<String>,
    pub(super) model: Option<String>,
    pub(super) context1m: bool,
    pub(super) credential_present: bool,
    pub(super) configured: bool,
    pub(super) parse_ok: bool,
    pub(super) exists: bool,
    pub(super) paths: Vec<PathBuf>,
}

pub(super) fn claude_fingerprint(
    content: &str,
    path: PathBuf,
) -> anyhow::Result<AgentConfigFingerprint> {
    let mut out = AgentConfigFingerprint {
        parse_ok: true,
        exists: path.exists(),
        paths: vec![path],
        ..Default::default()
    };
    if content.trim().is_empty() {
        return Ok(out);
    }
    let root: serde_json::Value = serde_json::from_str(content)?;
    let Some(env) = root.get("env").and_then(serde_json::Value::as_object) else {
        return Ok(out);
    };
    for key in [
        "ANTHROPIC_AUTH_TOKEN",
        "ANTHROPIC_API_KEY",
        "ANTHROPIC_BASE_URL",
        "ANTHROPIC_DEFAULT_HAIKU_MODEL",
        "ANTHROPIC_DEFAULT_SONNET_MODEL",
        "ANTHROPIC_DEFAULT_OPUS_MODEL",
    ] {
        if let Some(value) = env
            .get(key)
            .and_then(serde_json::Value::as_str)
            .filter(|v| !v.trim().is_empty())
        {
            out.managed_values.insert(
                key.into(),
                if key == "ANTHROPIC_BASE_URL" {
                    canonical_url(value)
                } else {
                    value.trim().into()
                },
            );
        }
    }
    out.credential_present = out.managed_values.contains_key("ANTHROPIC_AUTH_TOKEN")
        || out.managed_values.contains_key("ANTHROPIC_API_KEY");
    out.endpoint = out
        .managed_values
        .get("ANTHROPIC_BASE_URL")
        .map(|v| display_endpoint(v));
    if let Some(model) = [
        "ANTHROPIC_DEFAULT_OPUS_MODEL",
        "ANTHROPIC_DEFAULT_SONNET_MODEL",
        "ANTHROPIC_DEFAULT_HAIKU_MODEL",
    ]
    .into_iter()
    .find_map(|k| out.managed_values.get(k))
    {
        let (model, one_m) = model_without_context_marker(model);
        out.model = Some(model);
        out.context1m = one_m;
    }
    out.configured = out.credential_present || out.endpoint.is_some() || out.model.is_some();
    Ok(out)
}

pub(super) fn codex_fingerprint(
    config: &str,
    auth: &str,
    paths: Vec<PathBuf>,
) -> anyhow::Result<AgentConfigFingerprint> {
    let mut out = AgentConfigFingerprint {
        parse_ok: true,
        exists: paths.iter().any(|p| p.exists()),
        paths,
        ..Default::default()
    };
    let doc = if config.trim().is_empty() {
        toml_edit::DocumentMut::new()
    } else {
        config.parse()?
    };
    for key in [
        "model",
        "model_provider",
        "preferred_auth_method",
        "model_context_window",
    ] {
        if let Some(item) = doc.get(key) {
            if let Some(v) = item.as_str() {
                out.managed_values.insert(format!("config.{key}"), v.into());
            } else if let Some(v) = item.as_integer() {
                out.managed_values
                    .insert(format!("config.{key}"), v.to_string());
            }
        }
    }
    if let Some(slug) = doc.get("model_provider").and_then(toml_edit::Item::as_str) {
        if let Some(provider) = doc
            .get("model_providers")
            .and_then(toml_edit::Item::as_table)
            .and_then(|t| t.get(slug))
            .and_then(toml_edit::Item::as_table)
        {
            for key in ["name", "base_url", "wire_api", "experimental_bearer_token"] {
                if let Some(v) = provider.get(key).and_then(toml_edit::Item::as_str) {
                    out.managed_values.insert(
                        format!("provider.{key}"),
                        if key == "base_url" {
                            canonical_url(v)
                        } else {
                            v.into()
                        },
                    );
                    if key == "experimental_bearer_token" && !v.trim().is_empty() {
                        out.credential_present = true;
                    }
                }
            }
        }
    }
    let auth: serde_json::Value = if auth.trim().is_empty() {
        serde_json::json!({})
    } else {
        serde_json::from_str(auth)?
    };
    if auth
        .get("OPENAI_API_KEY")
        .and_then(serde_json::Value::as_str)
        .is_some_and(|v| !v.trim().is_empty())
    {
        out.credential_present = true;
    }
    if auth.get("tokens").is_some_and(|v| !v.is_null()) {
        out.credential_present = true;
    }
    out.endpoint = out
        .managed_values
        .get("provider.base_url")
        .map(|v| display_endpoint(v));
    out.model = out.managed_values.get("config.model").cloned();
    out.context1m = out
        .managed_values
        .get("config.model_context_window")
        .and_then(|v| v.parse::<u64>().ok())
        .is_some_and(|v| v >= 1_000_000);
    out.configured = out.credential_present
        || out.endpoint.is_some()
        || out.model.is_some()
        || out.managed_values.contains_key("config.model_provider");
    Ok(out)
}
