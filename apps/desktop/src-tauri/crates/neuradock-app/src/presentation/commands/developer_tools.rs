use tauri::State;

use crate::application::services::{
    DeveloperToolCapabilityDto, DeveloperToolConnectionDto, GerritChangeDto, JiraIssueDto,
    JiraMetadataDto, SaveDeveloperToolConnectionInput, SavedDeveloperToolQueryDto,
};
use crate::presentation::error::CommandError;
use crate::presentation::state::Services;

#[tauri::command]
#[specta::specta]
pub async fn list_developer_tool_connections(
    services: State<'_, Services>,
) -> Result<Vec<DeveloperToolConnectionDto>, CommandError> {
    services
        .developer_tools
        .list_connections()
        .await
        .map_err(CommandError::from)
}

#[tauri::command]
#[specta::specta]
pub async fn save_developer_tool_connection(
    input: SaveDeveloperToolConnectionInput,
    services: State<'_, Services>,
) -> Result<DeveloperToolConnectionDto, CommandError> {
    services
        .developer_tools
        .save_connection(input)
        .await
        .map_err(CommandError::from)
}

#[tauri::command]
#[specta::specta]
pub async fn delete_developer_tool_connection(
    connection_id: String,
    services: State<'_, Services>,
) -> Result<(), CommandError> {
    services
        .developer_tools
        .delete_connection(&connection_id)
        .await
        .map_err(CommandError::from)
}

#[tauri::command]
#[specta::specta]
pub async fn test_developer_tool_connection(
    connection_id: String,
    services: State<'_, Services>,
) -> Result<DeveloperToolCapabilityDto, CommandError> {
    services
        .developer_tools
        .test_connection(&connection_id)
        .await
        .map_err(CommandError::from)
}

#[tauri::command]
#[specta::specta]
pub async fn get_jira_metadata(
    connection_id: String,
    services: State<'_, Services>,
) -> Result<JiraMetadataDto, CommandError> {
    services
        .developer_tools
        .jira_metadata(&connection_id)
        .await
        .map_err(CommandError::from)
}

#[tauri::command]
#[specta::specta]
pub async fn query_jira_issues(
    connection_id: String,
    query: String,
    services: State<'_, Services>,
) -> Result<Vec<JiraIssueDto>, CommandError> {
    services
        .developer_tools
        .query_jira(&connection_id, &query)
        .await
        .map_err(CommandError::from)
}

#[tauri::command]
#[specta::specta]
pub async fn query_gerrit_changes(
    connection_id: String,
    query: String,
    services: State<'_, Services>,
) -> Result<Vec<GerritChangeDto>, CommandError> {
    services
        .developer_tools
        .query_gerrit(&connection_id, &query)
        .await
        .map_err(CommandError::from)
}

#[tauri::command]
#[specta::specta]
pub async fn list_developer_tool_saved_queries(
    connection_id: String,
    services: State<'_, Services>,
) -> Result<Vec<SavedDeveloperToolQueryDto>, CommandError> {
    services
        .developer_tools
        .list_saved_queries(&connection_id)
        .await
        .map_err(CommandError::from)
}

#[tauri::command]
#[specta::specta]
pub async fn save_developer_tool_query(
    connection_id: String,
    query_id: Option<String>,
    name: String,
    query: String,
    services: State<'_, Services>,
) -> Result<SavedDeveloperToolQueryDto, CommandError> {
    services
        .developer_tools
        .save_query(&connection_id, query_id, &name, &query)
        .await
        .map_err(CommandError::from)
}

#[tauri::command]
#[specta::specta]
pub async fn delete_developer_tool_query(
    query_id: String,
    services: State<'_, Services>,
) -> Result<(), CommandError> {
    services
        .developer_tools
        .delete_query(&query_id)
        .await
        .map_err(CommandError::from)
}
