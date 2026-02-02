use std::collections::{HashMap, HashSet};
use std::sync::Arc;

use neuradock_domain::account::AccountRepository;
use neuradock_domain::check_in::{Provider, ProviderConfig, ProviderRepository};
use neuradock_domain::custom_node::{
    CustomProviderNode, CustomProviderNodeRepository,
};
use neuradock_domain::provider_models::ProviderModelsRepository;
use neuradock_domain::shared::{DomainError, ProviderId};
use neuradock_domain::waf_cookies::WafCookiesRepository;
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

#[derive(Debug, Clone)]
struct StaleProviderMapping {
    name: String,
    old_id: ProviderId,
    new_id: ProviderId,
}

async fn migrate_provider_references(
    mappings: &[StaleProviderMapping],
    account_repo: Arc<dyn AccountRepository>,
    custom_node_repo: Arc<dyn CustomProviderNodeRepository>,
    provider_models_repo: Arc<dyn ProviderModelsRepository>,
    waf_cookies_repo: Arc<dyn WafCookiesRepository>,
) -> Result<(), DomainError> {
    if mappings.is_empty() {
        return Ok(());
    }

    let mut mapping_by_old: HashMap<String, ProviderId> = HashMap::new();
    for mapping in mappings {
        mapping_by_old.insert(mapping.old_id.as_str().to_string(), mapping.new_id.clone());
    }

    let accounts = account_repo.find_all().await?;
    let mut updated_accounts = 0;
    for mut account in accounts {
        if let Some(new_id) = mapping_by_old.get(account.provider_id().as_str()) {
            account.update_provider_id(new_id.clone());
            account_repo.save(&account).await?;
            updated_accounts += 1;
        }
    }
    if updated_accounts > 0 {
        info!(
            "Migrated {} account(s) to updated provider IDs",
            updated_accounts
        );
    }

    for mapping in mappings {
        let old_nodes = custom_node_repo.find_by_provider(&mapping.old_id).await?;
        if !old_nodes.is_empty() {
            let new_nodes = custom_node_repo.find_by_provider(&mapping.new_id).await?;
            let mut existing_urls: HashSet<String> = new_nodes
                .iter()
                .map(|node| node.base_url().to_string())
                .collect();

            for node in old_nodes {
                if existing_urls.insert(node.base_url().to_string()) {
                    let new_node = CustomProviderNode::create(
                        mapping.new_id.clone(),
                        node.name().to_string(),
                        node.base_url().to_string(),
                    );
                    let _ = custom_node_repo.create(&new_node).await?;
                }
                custom_node_repo.delete(node.id()).await?;
            }
        }

        let old_models = provider_models_repo
            .find_by_provider(mapping.old_id.as_str())
            .await?;
        if let Some(old_models) = old_models {
            let new_models = provider_models_repo
                .find_by_provider(mapping.new_id.as_str())
                .await?;
            let should_copy = match new_models {
                None => true,
                Some(current) => old_models.fetched_at > current.fetched_at,
            };

            if should_copy {
                provider_models_repo
                    .save(mapping.new_id.as_str(), &old_models.models)
                    .await?;
            }
            provider_models_repo
                .delete_by_provider(mapping.old_id.as_str())
                .await?;
        }

        let old_waf = waf_cookies_repo
            .get_valid(mapping.old_id.as_str())
            .await?;
        if let Some(old_waf) = old_waf {
            let new_waf = waf_cookies_repo
                .get_valid(mapping.new_id.as_str())
                .await?;
            if new_waf.is_none() {
                waf_cookies_repo
                    .save(mapping.new_id.as_str(), &old_waf.cookies)
                    .await?;
            }
        }
        waf_cookies_repo.delete(mapping.old_id.as_str()).await?;
    }

    Ok(())
}

/// Ensure built-in providers from embedded configuration exist in the database.
pub async fn seed_builtin_providers(
    provider_repo: Arc<dyn ProviderRepository>,
    custom_node_repo: Arc<dyn CustomProviderNodeRepository>,
    account_repo: Arc<dyn AccountRepository>,
    provider_models_repo: Arc<dyn ProviderModelsRepository>,
    waf_cookies_repo: Arc<dyn WafCookiesRepository>,
) -> Result<(), DomainError> {
    let configs = builtin_provider_configs()?;
    if configs.is_empty() {
        return Ok(());
    }

    let existing = provider_repo.find_all().await?;

    // Delete providers that have the same name as a built-in but a different ID
    // (e.g. from a prior version that used UUIDs)
    let builtin_by_name: HashMap<&str, &str> = configs
        .iter()
        .map(|c| (c.name.as_str(), c.id.as_str()))
        .collect();
    let mut stale_mappings = Vec::new();
    for provider in &existing {
        if let Some(&expected_id) = builtin_by_name.get(provider.name()) {
            if provider.id().as_str() != expected_id {
                stale_mappings.push(StaleProviderMapping {
                    name: provider.name().to_string(),
                    old_id: provider.id().clone(),
                    new_id: ProviderId::from_string(expected_id),
                });
            }
        }
    }

    if !stale_mappings.is_empty() {
        for mapping in &stale_mappings {
            info!(
                "Migrating provider '{}' from id={} to id={}",
                mapping.name,
                mapping.old_id.as_str(),
                mapping.new_id.as_str()
            );
        }
        migrate_provider_references(
            &stale_mappings,
            account_repo,
            custom_node_repo.clone(),
            provider_models_repo,
            waf_cookies_repo,
        )
        .await?;

        for mapping in &stale_mappings {
            info!(
                "Removing stale provider '{}' (id={}) to re-seed as id={}",
                mapping.name,
                mapping.old_id.as_str(),
                mapping.new_id.as_str()
            );
            provider_repo.delete(&mapping.old_id).await?;
        }
    }

    let existing = provider_repo.find_all().await?;
    let existing_ids: HashSet<String> = existing
        .iter()
        .map(|provider| provider.id().as_str().to_string())
        .collect();

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
