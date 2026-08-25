use super::gateway_persistence::IsolatedTarget;
use crate::application::services::{ClaudeConfigService, CodexConfigService, GatewayTakeover};
use crate::presentation::error::CommandError;
use neuradock_infrastructure::gateway::GatewayClient;
use tauri::Manager;

pub(super) fn takeover_for(app: &tauri::AppHandle) -> Result<GatewayTakeover, CommandError> {
    let app_data_dir = app.path().app_data_dir().map_err(|e| {
        CommandError::infrastructure(format!("Failed to get app data directory: {}", e))
    })?;
    Ok(GatewayTakeover::new(&app_data_dir))
}

pub(super) fn apply_gateway_config(
    takeover: &GatewayTakeover,
    client: GatewayClient,
    base_url: &str,
    local_credential: &str,
    model: Option<&str>,
    context1m: bool,
    isolated: Option<&IsolatedTarget>,
) -> Result<(), CommandError> {
    match isolated {
        Some(target) if client == GatewayClient::Codex => CodexConfigService::new()
            .configure_isolated_gateway(
                &target.slug,
                &target.display,
                base_url,
                local_credential,
                model,
                context1m,
            )
            .map(|_| ())
            .map_err(CommandError::from),
        Some(target) if client == GatewayClient::ClaudeCode => ClaudeConfigService::new()
            .configure_isolated_with_key(&target.slug, local_credential, base_url, model, context1m)
            .map(|_| ())
            .map_err(CommandError::from),
        _ => takeover
            .take_over(client, base_url, local_credential, model, context1m)
            .map_err(|e| CommandError::infrastructure(e.to_string())),
    }
}
