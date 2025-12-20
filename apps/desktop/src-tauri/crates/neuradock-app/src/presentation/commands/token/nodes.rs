use crate::presentation::error::CommandError;
use crate::application::dtos::ProviderNodeDto;
use crate::presentation::state::AppState;
use tauri::State;

#[tauri::command]
#[specta::specta]
pub async fn get_provider_nodes(
    provider_id: String,
    state: State<'_, AppState>,
) -> Result<Vec<ProviderNodeDto>, CommandError> {
    let provider_id_obj = neuradock_domain::shared::ProviderId::from_string(&provider_id);
    let provider = state
        .provider_repo
        .find_by_id(&provider_id_obj)
        .await
        .map_err(CommandError::from)?
        .ok_or_else(|| CommandError::not_found(format!("Provider not found: {}", provider_id)))?;

    let mut nodes = vec![ProviderNodeDto {
        id: provider_id.clone(),
        name: provider.name().to_string(),
        base_url: provider.domain().to_string(),
    }];

    // Add custom nodes
    let custom_nodes = state
        .custom_node_repo
        .find_by_provider(&provider_id_obj)
        .await
        .map_err(CommandError::from)?;

    for custom_node in custom_nodes {
        nodes.push(ProviderNodeDto {
            id: format!("custom_{}", custom_node.id().value()),
            name: custom_node.name().to_string(),
            base_url: custom_node.base_url().to_string(),
        });
    }

    Ok(nodes)
}

#[tauri::command]
#[specta::specta]
pub async fn add_custom_node(
    provider_id: String,
    name: String,
    base_url: String,
    state: State<'_, AppState>,
) -> Result<String, CommandError> {
    let provider_id_obj = neuradock_domain::shared::ProviderId::from_string(&provider_id);
    let node = neuradock_domain::custom_node::CustomProviderNode::create(
        provider_id_obj,
        name.clone(),
        base_url.clone(),
    );

    state
        .custom_node_repo
        .create(&node)
        .await
        .map_err(CommandError::from)?;

    Ok(format!("Custom node '{}' added successfully", name))
}

#[tauri::command]
#[specta::specta]
pub async fn delete_custom_node(
    node_id: i64,
    state: State<'_, AppState>,
) -> Result<String, CommandError> {
    let id = neuradock_domain::custom_node::CustomNodeId::new(node_id);

    state
        .custom_node_repo
        .delete(&id)
        .await
        .map_err(CommandError::from)?;

    Ok("Custom node deleted successfully".to_string())
}
