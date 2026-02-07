use tauri::State;

use crate::config::manager::ConfigData;
use crate::error::AppError;
use crate::state::AppState;

#[tauri::command]
pub async fn get_config(state: State<'_, AppState>) -> Result<ConfigData, AppError> {
    let config = state.config.lock().await;
    Ok(config.data.clone())
}

/// Partially merges the provided config fields into the stored config.
/// Only fields that differ from defaults are considered "set" by the caller.
/// For safety, we only update the subset of fields that the frontend knows about:
/// theme, task_configs, and completed_tasks.
#[tauri::command]
pub async fn set_config(
    config: ConfigData,
    state: State<'_, AppState>,
) -> Result<ConfigData, AppError> {
    let mut mgr = state.config.lock().await;
    // Merge only frontend-safe fields, preserving backend-managed data
    mgr.data.theme = config.theme;
    mgr.data.task_configs = config.task_configs;
    mgr.data.completed_tasks = config.completed_tasks;
    mgr.save()?;
    Ok(mgr.data.clone())
}
