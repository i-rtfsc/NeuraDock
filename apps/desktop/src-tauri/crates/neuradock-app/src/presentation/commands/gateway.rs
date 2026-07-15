use tauri::State;
use tokio::sync::Mutex;

use crate::presentation::error::CommandError;
use crate::presentation::state::Repositories;
use neuradock_domain::independent_key::IndependentKeyId;
use neuradock_infrastructure::gateway::{start_gateway, GatewayHandle, GatewayProfile};

/// Runtime state for the single embedded provider gateway. Only one gateway
/// runs at a time; starting a new one replaces the previous.
#[derive(Default)]
pub struct GatewayState {
    inner: Mutex<Option<RunningGateway>>,
}

struct RunningGateway {
    handle: GatewayHandle,
    key_id: i64,
    base_url: String,
}

/// Status of the embedded gateway reported to the frontend.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, specta::Type)]
pub struct GatewayStatusDto {
    pub running: bool,
    /// `http://127.0.0.1:<port>` the tools should use, when running.
    pub base_url: Option<String>,
    /// Independent key id the gateway is forwarding to, when running.
    pub key_id: Option<i64>,
}

/// Start (or restart) the embedded gateway forwarding to the given independent
/// key's relay. Returns the localhost base URL to configure into tools.
#[tauri::command]
#[specta::specta]
pub async fn start_provider_gateway(
    key_id: i64,
    model: Option<String>,
    state: State<'_, GatewayState>,
    repositories: State<'_, Repositories>,
) -> Result<GatewayStatusDto, CommandError> {
    let id = IndependentKeyId::new(key_id);
    let key = repositories
        .independent_key
        .find_by_id(&id)
        .await
        .map_err(CommandError::from)?
        .ok_or_else(|| CommandError::not_found(format!("Key with ID {} not found", key_id)))?;

    if !key.is_active() {
        return Err(CommandError::validation(
            "Cannot start gateway for an inactive API key. Please enable it first.",
        ));
    }

    let profile = GatewayProfile {
        base_url: key.base_url().to_string(),
        api_key: key.api_key().to_string(),
        model,
    };

    let mut guard = state.inner.lock().await;
    // Replace any gateway already running.
    if let Some(previous) = guard.take() {
        previous.handle.shutdown().await;
    }

    let handle = start_gateway(profile, 0)
        .await
        .map_err(|e| CommandError::infrastructure(format!("Failed to start gateway: {}", e)))?;
    let base_url = handle.base_url();

    *guard = Some(RunningGateway {
        handle,
        key_id,
        base_url: base_url.clone(),
    });

    Ok(GatewayStatusDto {
        running: true,
        base_url: Some(base_url),
        key_id: Some(key_id),
    })
}

/// Stop the embedded gateway if it is running.
#[tauri::command]
#[specta::specta]
pub async fn stop_provider_gateway(
    state: State<'_, GatewayState>,
) -> Result<GatewayStatusDto, CommandError> {
    let mut guard = state.inner.lock().await;
    if let Some(running) = guard.take() {
        running.handle.shutdown().await;
    }
    Ok(GatewayStatusDto {
        running: false,
        base_url: None,
        key_id: None,
    })
}

/// Report whether the gateway is running and its base URL.
#[tauri::command]
#[specta::specta]
pub async fn get_provider_gateway_status(
    state: State<'_, GatewayState>,
) -> Result<GatewayStatusDto, CommandError> {
    let guard = state.inner.lock().await;
    Ok(match guard.as_ref() {
        Some(running) => GatewayStatusDto {
            running: true,
            base_url: Some(running.base_url.clone()),
            key_id: Some(running.key_id),
        },
        None => GatewayStatusDto {
            running: false,
            base_url: None,
            key_id: None,
        },
    })
}
