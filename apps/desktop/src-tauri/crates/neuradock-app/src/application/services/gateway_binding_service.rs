use crate::presentation::commands::gateway::{GatewayBindingConfigDto, GatewayClientDto};
use crate::presentation::error::CommandError;

pub struct GatewayBindingService;

impl GatewayBindingService {
    pub fn normalize_config(
        client: GatewayClientDto,
        config: GatewayBindingConfigDto,
    ) -> Result<(Option<String>, bool, bool, bool), CommandError> {
        let model = config.model;
        let context1m = config.context_1m;
        if context1m && model.is_none() {
            return Err(CommandError::validation(
                "1M context requires selecting a model.",
            ));
        }
        let impersonate = config.impersonate.unwrap_or(true);
        let _ = client;
        Ok((model, context1m, impersonate, config.isolated))
    }
}
