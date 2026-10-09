use super::gateway_helpers::{canonical_url, display_endpoint, model_without_context_marker};
use sha2::{Digest, Sha256};
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
        "model_catalog_json",
        "model_reasoning_effort",
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

pub(super) fn catalog_path(config: &str) -> Option<PathBuf> {
    let doc = config.parse::<toml_edit::DocumentMut>().ok()?;
    let raw = doc.get("model_catalog_json")?.as_str()?;
    Some(if let Some(relative) = raw.strip_prefix("~/") {
        dirs::home_dir()?.join(relative)
    } else {
        PathBuf::from(raw)
    })
}

pub(super) fn attach_catalog_fingerprint(
    fingerprint: &mut AgentConfigFingerprint,
    path: PathBuf,
    content: Option<&str>,
) {
    if !fingerprint.paths.contains(&path) {
        fingerprint.paths.push(path);
    }
    let catalog = content.and_then(|text| serde_json::from_str::<serde_json::Value>(text).ok());
    let valid = catalog
        .as_ref()
        .and_then(|catalog| catalog.get("models"))
        .and_then(serde_json::Value::as_array)
        .is_some_and(|models| {
            !models.is_empty()
                && models.iter().all(|model| {
                    model
                        .get("slug")
                        .and_then(serde_json::Value::as_str)
                        .is_some()
                })
                && fingerprint.model.as_ref().is_none_or(|default| {
                    models
                        .iter()
                        .any(|model| model["slug"].as_str() == Some(default))
                })
        });
    if !valid {
        fingerprint.parse_ok = false;
        fingerprint
            .managed_values
            .insert("catalog.status".into(), "missing_or_invalid".into());
    } else if let Some(catalog) = catalog {
        let normalized = serde_json::to_string(&catalog).unwrap_or_default();
        fingerprint.managed_values.insert(
            "catalog.sha256".into(),
            format!("{:x}", Sha256::digest(normalized.as_bytes())),
        );
    }
}

#[cfg(test)]
mod catalog_tests {
    use super::*;

    #[test]
    fn model_catalog_fingerprint_tracks_metadata_and_missing_files() {
        let mut valid = AgentConfigFingerprint {
            parse_ok: true,
            model: Some("model-a".into()),
            ..Default::default()
        };
        attach_catalog_fingerprint(
            &mut valid,
            "models.json".into(),
            Some(r#"{"models":[{"slug":"model-a"}]}"#),
        );
        let mut changed = valid.clone();
        attach_catalog_fingerprint(
            &mut changed,
            "models.json".into(),
            Some(r#"{"models":[{"slug":"model-a","context_window":1000}]}"#),
        );
        assert_ne!(valid.managed_values, changed.managed_values);
        let mut missing = AgentConfigFingerprint {
            parse_ok: true,
            ..Default::default()
        };
        attach_catalog_fingerprint(&mut missing, "models.json".into(), None);
        assert!(!missing.parse_ok);
    }
}
