pub struct GatewayBindingService;

pub struct GatewayBindingConfigInput {
    pub model: Option<String>,
    pub context1m: bool,
    pub impersonate: Option<bool>,
    pub isolated: bool,
}

pub struct NormalizedGatewayBindingConfig {
    pub model: Option<String>,
    pub context1m: bool,
    pub impersonate: bool,
    pub isolated: bool,
}

#[derive(Debug, thiserror::Error)]
pub enum GatewayBindingConfigError {
    #[error("1M context requires selecting a model.")]
    ContextRequiresModel,
}

impl GatewayBindingService {
    pub fn normalize_config(
        config: GatewayBindingConfigInput,
    ) -> Result<NormalizedGatewayBindingConfig, GatewayBindingConfigError> {
        let model = config.model;
        let context1m = config.context1m;
        if context1m && model.is_none() {
            return Err(GatewayBindingConfigError::ContextRequiresModel);
        }
        let impersonate = config.impersonate.unwrap_or(true);
        Ok(NormalizedGatewayBindingConfig {
            model,
            context1m,
            impersonate,
            isolated: config.isolated,
        })
    }

    pub fn effective_model(model: Option<&str>) -> Option<String> {
        model.map(str::to_string)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn context_requires_a_model() {
        let result = GatewayBindingService::normalize_config(GatewayBindingConfigInput {
            model: None,
            context1m: true,
            impersonate: None,
            isolated: false,
        });
        assert!(matches!(
            result,
            Err(GatewayBindingConfigError::ContextRequiresModel)
        ));
    }

    #[test]
    fn normalization_applies_defaults_and_preserves_flags() {
        let result = GatewayBindingService::normalize_config(GatewayBindingConfigInput {
            model: Some("gpt-5".into()),
            context1m: true,
            impersonate: None,
            isolated: true,
        })
        .unwrap();
        assert_eq!(result.model.as_deref(), Some("gpt-5"));
        assert!(result.context1m);
        assert!(result.impersonate);
        assert!(result.isolated);
    }
}
