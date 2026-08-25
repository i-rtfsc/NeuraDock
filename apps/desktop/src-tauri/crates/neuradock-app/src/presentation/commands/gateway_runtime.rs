use super::gateway::{GatewayClientDto, GatewayStatusDto};
use super::gateway_state::GatewayState;
use super::gateway_status::status_of;
use crate::presentation::error::CommandError;
use neuradock_infrastructure::gateway::{GatewayClient, GatewayRoute};
use tauri::State;

#[tauri::command]
#[specta::specta]
pub async fn get_gateway_status(
    state: State<'_, GatewayState>,
) -> Result<GatewayStatusDto, CommandError> {
    Ok(status_of(&state).await)
}

#[tauri::command]
#[specta::specta]
pub async fn recover_gateway_requests(
    client: GatewayClientDto,
    state: State<'_, GatewayState>,
) -> Result<GatewayStatusDto, CommandError> {
    let gateway_client: GatewayClient = client.into();
    let _operation = state.operation_lock(gateway_client).lock().await;
    let upstream = state
        .bindings
        .read()
        .await
        .get(&GatewayRoute::default(gateway_client))
        .cloned()
        .ok_or_else(|| CommandError::validation("The Agent has no gateway binding."))?;
    state.runtime.recover(gateway_client, &upstream.id).await;
    Ok(status_of(&state).await)
}

#[tauri::command]
#[specta::specta]
pub async fn clear_gateway_statistics(
    client: GatewayClientDto,
    state: State<'_, GatewayState>,
) -> Result<GatewayStatusDto, CommandError> {
    let gateway_client: GatewayClient = client.into();
    let _operation = state.operation_lock(gateway_client).lock().await;
    let upstream = state
        .bindings
        .read()
        .await
        .get(&GatewayRoute::default(gateway_client))
        .cloned()
        .ok_or_else(|| CommandError::validation("The Agent has no gateway binding."))?;
    state
        .runtime
        .clear_statistics(gateway_client, &upstream.id)
        .await;
    Ok(status_of(&state).await)
}
