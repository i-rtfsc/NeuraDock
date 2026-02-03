use log::{info, warn};
use std::collections::{HashMap, HashSet};
use std::sync::Arc;

use neuradock_domain::account::{Account, AccountRepository};
use neuradock_domain::check_in::{Provider, ProviderRepository};
use neuradock_domain::custom_node::{CustomProviderNode, CustomProviderNodeRepository};
use neuradock_domain::provider_models::ProviderModelsRepository;
use neuradock_domain::proxy_config::ProxyConfigRepository;
use neuradock_domain::shared::{DomainError, ProviderId};
use neuradock_domain::waf_cookies::WafCookiesRepository;
use neuradock_infrastructure::http::HttpClient;

pub struct OrphanAccountRepairService {
    account_repo: Arc<dyn AccountRepository>,
    provider_repo: Arc<dyn ProviderRepository>,
    custom_node_repo: Arc<dyn CustomProviderNodeRepository>,
    provider_models_repo: Arc<dyn ProviderModelsRepository>,
    waf_cookies_repo: Arc<dyn WafCookiesRepository>,
    proxy_config_repo: Arc<dyn ProxyConfigRepository>,
}

impl OrphanAccountRepairService {
    pub fn new(
        account_repo: Arc<dyn AccountRepository>,
        provider_repo: Arc<dyn ProviderRepository>,
        custom_node_repo: Arc<dyn CustomProviderNodeRepository>,
        provider_models_repo: Arc<dyn ProviderModelsRepository>,
        waf_cookies_repo: Arc<dyn WafCookiesRepository>,
        proxy_config_repo: Arc<dyn ProxyConfigRepository>,
    ) -> Self {
        Self {
            account_repo,
            provider_repo,
            custom_node_repo,
            provider_models_repo,
            waf_cookies_repo,
            proxy_config_repo,
        }
    }

    pub async fn repair_orphaned_accounts(&self) -> Result<usize, DomainError> {
        let providers = self.provider_repo.find_all().await?;
        if providers.is_empty() {
            return Ok(0);
        }

        let mut providers_by_id: HashMap<String, Provider> = HashMap::new();
        for provider in &providers {
            providers_by_id.insert(provider.id().as_str().to_string(), provider.clone());
        }

        let mut candidate_providers = providers.clone();
        candidate_providers
            .sort_by_key(|provider| (!provider.is_builtin(), provider.name().to_lowercase()));

        let accounts = self.account_repo.find_all().await?;
        let mut orphaned_by_provider: HashMap<String, Vec<Account>> = HashMap::new();
        for account in accounts {
            if !providers_by_id.contains_key(account.provider_id().as_str()) {
                orphaned_by_provider
                    .entry(account.provider_id().as_str().to_string())
                    .or_default()
                    .push(account);
            }
        }

        if orphaned_by_provider.is_empty() {
            return Ok(0);
        }

        let orphaned_accounts_count: usize = orphaned_by_provider.values().map(|v| v.len()).sum();
        info!(
            "Detected {} orphaned account(s) across {} missing provider ID(s)",
            orphaned_accounts_count,
            orphaned_by_provider.len()
        );

        let proxy_url = self.proxy_config_repo.get().await?.proxy_url();
        let http_client = HttpClient::with_proxy(proxy_url).map_err(|e| {
            DomainError::Infrastructure(format!("Failed to create HTTP client: {e}"))
        })?;

        let mut total_fixed = 0usize;
        let mut resolved_mappings: HashMap<String, ProviderId> = HashMap::new();

        for (old_provider_id, mut accounts) in orphaned_by_provider {
            if accounts.is_empty() {
                continue;
            }

            let mapping = if let Some(existing) = resolved_mappings.get(&old_provider_id) {
                Some(existing.clone())
            } else {
                let probe_account = &accounts[0];
                let mut matches: Vec<Provider> = Vec::new();

                for provider in &candidate_providers {
                    match http_client
                        .get_user_info(
                            &provider.user_info_url(),
                            probe_account.credentials().cookies(),
                            provider.api_user_key(),
                            probe_account.credentials().api_user(),
                        )
                        .await
                    {
                        Ok(_) => {
                            matches.push(provider.clone());
                            if matches.len() > 1 {
                                break;
                            }
                        }
                        Err(err) => {
                            let message = err.to_string();
                            if message.contains("WAF_CHALLENGE") {
                                warn!(
                                    "WAF challenge while probing provider {} for orphaned account {}",
                                    provider.name(),
                                    probe_account.name()
                                );
                            }
                        }
                    }
                }

                if matches.len() == 1 {
                    let matched = matches.remove(0);
                    let new_id = matched.id().clone();
                    resolved_mappings.insert(old_provider_id.clone(), new_id.clone());
                    Some(new_id)
                } else {
                    warn!(
                        "Unable to resolve provider for orphaned provider_id {} (matches found: {})",
                        old_provider_id,
                        matches.len()
                    );
                    None
                }
            };

            let Some(new_provider_id) = mapping else {
                continue;
            };

            let old_id = ProviderId::from_string(&old_provider_id);
            if let Err(e) = self
                .migrate_related_provider_data(&old_id, &new_provider_id)
                .await
            {
                warn!(
                    "Failed to migrate related data for provider {} -> {}: {}",
                    old_provider_id,
                    new_provider_id.as_str(),
                    e
                );
            }

            for mut account in accounts.drain(..) {
                account.update_provider_id(new_provider_id.clone());
                self.account_repo.save(&account).await?;
                total_fixed += 1;
            }
        }

        if total_fixed > 0 {
            info!("Repaired {} orphaned account(s)", total_fixed);
        }

        Ok(total_fixed)
    }

    async fn migrate_related_provider_data(
        &self,
        old_id: &ProviderId,
        new_id: &ProviderId,
    ) -> Result<(), DomainError> {
        let old_nodes = self.custom_node_repo.find_by_provider(old_id).await?;
        if !old_nodes.is_empty() {
            let new_nodes = self.custom_node_repo.find_by_provider(new_id).await?;
            let mut existing_urls: HashSet<String> = new_nodes
                .iter()
                .map(|node| node.base_url().to_string())
                .collect();

            for node in old_nodes {
                if existing_urls.insert(node.base_url().to_string()) {
                    let new_node = CustomProviderNode::create(
                        new_id.clone(),
                        node.name().to_string(),
                        node.base_url().to_string(),
                    );
                    let _ = self.custom_node_repo.create(&new_node).await?;
                }
                self.custom_node_repo.delete(node.id()).await?;
            }
        }

        let old_models = self
            .provider_models_repo
            .find_by_provider(old_id.as_str())
            .await?;
        if let Some(old_models) = old_models {
            let new_models = self
                .provider_models_repo
                .find_by_provider(new_id.as_str())
                .await?;
            let should_copy = match new_models {
                None => true,
                Some(current) => old_models.fetched_at > current.fetched_at,
            };

            if should_copy {
                self.provider_models_repo
                    .save(new_id.as_str(), &old_models.models)
                    .await?;
            }
            self.provider_models_repo
                .delete_by_provider(old_id.as_str())
                .await?;
        }

        let old_waf = self.waf_cookies_repo.get_valid(old_id.as_str()).await?;
        if let Some(old_waf) = old_waf {
            let new_waf = self.waf_cookies_repo.get_valid(new_id.as_str()).await?;
            if new_waf.is_none() {
                self.waf_cookies_repo
                    .save(new_id.as_str(), &old_waf.cookies)
                    .await?;
            }
        }
        self.waf_cookies_repo.delete(old_id.as_str()).await?;

        Ok(())
    }
}
