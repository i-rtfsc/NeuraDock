use anyhow::{bail, Context, Result};
use serde_json::{json, Value};

use super::helpers::get_codex_dir;

const DEEPSEEK_CATALOG: &str = include_str!("assets/deepseek-models.json");

/// Keep selection order, reject empty names, and require the default in the catalog.
pub(super) fn validate_models(models: &[String], default_model: &str) -> Result<Vec<String>> {
    let mut names = Vec::new();
    for name in models {
        let name = name.trim();
        if name.is_empty() || name.chars().any(char::is_control) {
            bail!("Model names must be nonempty and cannot contain control characters");
        }
        super::helpers::validate_codex_model(name)?;
        if !names.iter().any(|existing| existing == name) {
            names.push(name.to_string());
        }
    }
    if !names.iter().any(|name| name == default_model) {
        bail!("The default model must be included in the selected models");
    }
    Ok(names)
}

/// Unknown models use only the fields required by Codex. Known model metadata
/// is reused when available; DeepSeek always uses the user's supplied template.
pub(super) fn build_catalog(models: &[String], default_model: &str) -> Result<String> {
    let names = validate_models(models, default_model)?;
    let deepseek: Value = serde_json::from_str(DEEPSEEK_CATALOG)?;
    let cached = get_codex_dir()
        .ok()
        .and_then(|dir| std::fs::read_to_string(dir.join("models_cache.json")).ok())
        .and_then(|content| serde_json::from_str::<Value>(&content).ok());
    build_catalog_from(&names, &deepseek, cached.as_ref())
}

fn build_catalog_from(
    names: &[String],
    deepseek: &Value,
    cached: Option<&Value>,
) -> Result<String> {
    let mut entries = Vec::new();
    for (index, name) in names.iter().enumerate() {
        let find = |catalog: &Value| {
            catalog
                .get("models")?
                .as_array()?
                .iter()
                .find(|entry| entry.get("slug").and_then(Value::as_str) == Some(name.as_str()))
                .cloned()
        };
        let mut entry = find(deepseek).or_else(|| cached.and_then(find)).unwrap_or_else(|| json!({
            "slug": name,
            "display_name": name,
            "base_instructions": "You are Codex, a coding agent. Help the user complete their task using the available tools.",
            "supported_reasoning_levels": [],
            "shell_type": "shell_command",
            "visibility": "list",
            "supported_in_api": true,
            "priority": index,
            "support_verbosity": false,
            "truncation_policy": { "mode": "tokens", "limit": 10000 },
            "experimental_supported_tools": [],
            "input_modalities": ["text"]
        }));
        let object = entry
            .as_object_mut()
            .context("Invalid model catalog entry")?;
        object.insert("visibility".into(), json!("list"));
        object.insert("supported_in_api".into(), json!(true));
        object.insert("priority".into(), json!(index));
        // A gateway's selected catalog should not offer upgrades outside it.
        object.insert("upgrade".into(), Value::Null);
        entries.push(entry);
    }
    Ok(serde_json::to_string_pretty(&json!({ "models": entries }))? + "\n")
}

pub(super) fn catalog_model_names(path: &std::path::Path) -> Vec<String> {
    std::fs::read_to_string(path)
        .ok()
        .and_then(|text| serde_json::from_str::<Value>(&text).ok())
        .and_then(|catalog| catalog.get("models")?.as_array().cloned())
        .unwrap_or_default()
        .iter()
        .filter_map(|entry| entry.get("slug")?.as_str().map(str::to_string))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn catalog_preserves_deepseek_and_keeps_other_models_minimal() {
        let reference: Value = serde_json::from_str(DEEPSEEK_CATALOG).unwrap();
        let names = vec!["deepseek-v4-flash".into(), "custom-model".into()];
        let catalog: Value =
            serde_json::from_str(&build_catalog_from(&names, &reference, None).unwrap()).unwrap();
        assert_eq!(
            catalog["models"][0]["base_instructions"],
            reference["models"][0]["base_instructions"]
        );
        assert_eq!(catalog["models"][0]["context_window"], 1_048_576);
        assert_eq!(
            catalog["models"][0]["supported_reasoning_levels"],
            reference["models"][0]["supported_reasoning_levels"]
        );
        assert_eq!(catalog["models"][1]["slug"], "custom-model");
        assert!(catalog["models"][1].get("context_window").is_none());
        assert_eq!(catalog["models"][1]["input_modalities"], json!(["text"]));
    }

    #[test]
    fn catalog_selection_requires_a_default_and_deduplicates() {
        assert!(validate_models(&[], "model-a").is_err());
        assert!(validate_models(&["model-a".into()], "model-b").is_err());
        assert!(validate_models(
            &[
                "deepseek-v4-flash".into(),
                "anthropic/claude-sonnet-test".into()
            ],
            "deepseek-v4-flash"
        )
        .is_err());
        assert!(validate_models(&["sonnet".into()], "sonnet").is_err());
        assert_eq!(
            validate_models(&["model-a".into(), "model-a".into()], "model-a").unwrap(),
            vec!["model-a"]
        );
    }
}
