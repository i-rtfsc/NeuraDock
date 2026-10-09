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
        Some(target) if client == GatewayClient::Codex && !target.models.is_empty() => {
            CodexConfigService::new()
                .configure_named_profile(
                    &target.slug,
                    &target.display,
                    local_credential,
                    base_url,
                    model.ok_or_else(|| CommandError::validation("A default model is required"))?,
                    &target.models,
                    target.restore_direct,
                )
                .map(|_| ())
                .map_err(CommandError::from)
        }
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

pub(super) fn clear_gateway_config(
    client: GatewayClient,
    isolated: Option<&IsolatedTarget>,
) -> Result<(), CommandError> {
    let Some(target) = isolated else {
        return Ok(());
    };
    match client {
        GatewayClient::Codex => CodexConfigService::new().clear_isolated(&target.slug),
        GatewayClient::ClaudeCode => ClaudeConfigService::new().clear_isolated(&target.slug),
    }
    .map(|_| ())
    .map_err(CommandError::from)
}
