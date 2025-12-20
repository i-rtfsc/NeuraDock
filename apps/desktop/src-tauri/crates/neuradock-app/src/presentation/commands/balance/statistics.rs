use crate::application::dtos::BalanceStatisticsDto;
use crate::presentation::error::CommandError;
use crate::presentation::state::Queries;
use tauri::State;

/// Get balance statistics by provider
#[tauri::command]
#[specta::specta]
pub async fn get_balance_statistics(
    state: State<'_, Queries>,
) -> Result<BalanceStatisticsDto, CommandError> {
    state
        .balance_statistics
        .get_balance_statistics()
        .await
        .map_err(CommandError::from)
}
