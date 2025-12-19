use anyhow::{Context, Result};
use neuradock_domain::shared::AccountId;
use neuradock_domain::token::{ApiToken, ApiTokenConfig, ModelLimits, TokenId, TokenStatus};
use neuradock_infrastructure::http::token::TokenData;

impl super::TokenService {
    pub(super) fn convert_to_domain(
        &self,
        data: TokenData,
        account_id: AccountId,
    ) -> Result<ApiToken> {
        let expired_time = if data.expired_time == -1 {
            None
        } else {
            Some(
                chrono::DateTime::from_timestamp(data.expired_time, 0)
                    .context("Invalid expired_time")?,
            )
        };

        let model_limits = if data.model_limits_enabled {
            Some(self.parse_model_limits(&data.model_limits))
        } else {
            None
        };

        let status = TokenStatus::from_i32(data.status)
            .context(format!("Invalid token status: {}", data.status))?;

        Ok(ApiToken::new(
            TokenId::new(data.id),
            account_id,
            ApiTokenConfig {
                name: data.name,
                key: data.key,
                status,
                used_quota: data.used_quota,
                remain_quota: data.remain_quota,
                unlimited_quota: data.unlimited_quota,
                expired_time,
                model_limits_enabled: data.model_limits_enabled,
                model_limits,
            },
        ))
    }

    pub(super) fn parse_model_limits(&self, limits_json: &serde_json::Value) -> ModelLimits {
        let mut allowed = Vec::new();
        let denied = Vec::new(); // API doesn't provide denied list

        // API returns model_limits as comma-separated string or JSON object
        if let Some(limits_str) = limits_json.as_str() {
            // Parse comma-separated string: "model1,model2,model3"
            if !limits_str.is_empty() {
                allowed = limits_str
                    .split(',')
                    .map(|s| s.trim().to_string())
                    .filter(|s| !s.is_empty())
                    .collect();
            }
        } else if let Some(obj) = limits_json.as_object() {
            // Parse JSON object format (for backwards compatibility)
            for (model, config) in obj {
                if let Some(allow) = config.get("allow").and_then(|v| v.as_bool()) {
                    if allow {
                        allowed.push(model.clone());
                    }
                }
            }
        }

        log::debug!("Parsed model limits: {} allowed models", allowed.len());

        ModelLimits { allowed, denied }
    }
}
