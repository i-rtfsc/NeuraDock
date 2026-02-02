use std::collections::{HashMap, HashSet};
use std::sync::Arc;

use neuradock_domain::check_in::{Provider, ProviderConfig, ProviderRepository};
use neuradock_domain::custom_node::{CustomProviderNode, CustomProviderNodeRepository};
use neuradock_domain::shared::DomainError;
use neuradock_domain::shared::ProviderId;
use serde::Deserialize;
use tracing::info;

#[derive(Debug, Deserialize)]
struct BuiltinProviderNodeConfig {
    name: String,
    base_url: String,
}

#[derive(Debug, Deserialize)]
struct BuiltinProviderConfig {
    id: String,
    name: String,
    domain: String,
    default_nodes: Option<Vec<BuiltinProviderNodeConfig>>,
    login_path: String,
    sign_in_path: Option<String>,
    user_info_path: String,
    token_api_path: Option<String>,
    models_path: Option<String>,
    api_user_key: String,
    bypass_method: Option<String>,
    supports_check_in: Option<bool>,
    check_in_bugged: Option<bool>,
}

fn builtin_provider_configs() -> Result<Vec<BuiltinProviderConfig>, DomainError> {
    const RAW_CONFIG: &str = include_str!("../../../../config/providers/builtin_providers.json");
    serde_json::from_str(RAW_CONFIG).map_err(|e| {
        DomainError::Deserialization(format!("Failed to parse builtin providers: {e}"))
    })
}

/// Ensure built-in providers from embedded configuration exist in the database.
pub async fn seed_builtin_providers(
    provider_repo: Arc<dyn ProviderRepository>,
    custom_node_repo: Arc<dyn CustomProviderNodeRepository>,
) -> Result<(), DomainError> {
    let configs = builtin_provider_configs()?;
    if configs.is_empty() {
        return Ok(());
    }

    let existing = provider_repo.find_all().await?;
    let existing_ids: HashSet<String> = existing
        .iter()
        .map(|provider| provider.id().as_str().to_string())
        .collect();

    // Delete providers that have the same name as a built-in but a different ID
    // (e.g. from a prior version that used UUIDs)
    let builtin_by_name: HashMap<&str, &str> = configs
        .iter()
        .map(|c| (c.name.as_str(), c.id.as_str()))
        .collect();
    for provider in &existing {
        if let Some(&expected_id) = builtin_by_name.get(provider.name()) {
            if provider.id().as_str() != expected_id {
                info!(
                    "Removing stale provider '{}' (id={}) to re-seed as id={}",
                    provider.name(),
                    provider.id().as_str(),
                    expected_id
                );
                provider_repo.delete(provider.id()).await?;
            }
        }
    }

    // Rebuild the ID set after deletions
    let existing_ids: HashSet<String> = {
        let mut ids = existing_ids;
        for provider in &existing {
            if let Some(&expected_id) = builtin_by_name.get(provider.name()) {
                if provider.id().as_str() != expected_id {
                    ids.remove(provider.id().as_str());
                }
            }
        }
        ids
    };

    let has_default_nodes = configs.iter().any(|config| {
        config
            .default_nodes
            .as_ref()
            .is_some_and(|nodes| !nodes.is_empty())
    });

    let mut base_urls_by_provider: HashMap<ProviderId, HashSet<String>> = HashMap::new();
    if has_default_nodes {
        let existing_nodes = custom_node_repo.find_all().await?;
        for node in existing_nodes {
            base_urls_by_provider
                .entry(node.provider_id().clone())
                .or_default()
                .insert(node.base_url().to_string());
        }
    }

    let mut seeded_count = 0;
    for config in configs.iter() {
        if !existing_ids.contains(&config.id) {
            let provider = Provider::builtin(
                &config.id,
                ProviderConfig {
                    name: config.name.clone(),
                    domain: config.domain.clone(),
                    login_path: config.login_path.clone(),
                    sign_in_path: config.sign_in_path.clone(),
                    user_info_path: config.user_info_path.clone(),
                    token_api_path: config.token_api_path.clone(),
                    models_path: config.models_path.clone(),
                    api_user_key: config.api_user_key.clone(),
                    bypass_method: config.bypass_method.clone(),
                    supports_check_in: config.supports_check_in.unwrap_or(true),
                    check_in_bugged: config.check_in_bugged.unwrap_or(false),
                },
            );
            provider_repo.save(&provider).await?;
            seeded_count += 1;
            info!("Seeded built-in provider: {} ({})", config.name, config.id);
        }

        if let Some(default_nodes) = &config.default_nodes {
            if default_nodes.is_empty() {
                continue;
            }

            let provider_id_obj = ProviderId::from_string(&config.id);
            let base_urls = base_urls_by_provider
                .entry(provider_id_obj.clone())
                .or_default();

            for node in default_nodes.iter() {
                if node.base_url == config.domain {
                    continue;
                }
                if !base_urls.insert(node.base_url.clone()) {
                    continue;
                }

                let custom_node = CustomProviderNode::create(
                    provider_id_obj.clone(),
                    node.name.clone(),
                    node.base_url.clone(),
                );
                let _ = custom_node_repo.create(&custom_node).await?;
            }
        }
    }

    if seeded_count > 0 {
        info!(
            "Seeded {} missing built-in provider(s) from config",
            seeded_count
        );
    }

    Ok(())
}
