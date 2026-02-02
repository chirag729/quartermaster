use tauri::State;

use crate::config::manager::ConfigData;
use crate::error::AppError;
use crate::state::AppState;

#[tauri::command]
pub async fn get_config(state: State<'_, AppState>) -> Result<ConfigData, AppError> {
    let config = state.config.lock().await;
    Ok(config.data.clone())
}

#[tauri::command]
pub async fn set_config(
    config: ConfigData,
    state: State<'_, AppState>,
) -> Result<ConfigData, AppError> {
    let mut mgr = state.config.lock().await;
    mgr.data = config;
    mgr.save()?;
    Ok(mgr.data.clone())
}
