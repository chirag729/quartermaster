use std::collections::HashMap;

use tauri::State;

use crate::error::AppError;
use crate::state::AppState;

#[tauri::command]
pub async fn get_shared_variables(
    state: State<'_, AppState>,
) -> Result<HashMap<String, String>, AppError> {
    let config = state.config.lock().await;
    Ok(config.data.shared_variables.clone())
}

#[tauri::command]
pub async fn set_shared_variable(
    key: String,
    value: String,
    state: State<'_, AppState>,
) -> Result<(), AppError> {
    let mut config = state.config.lock().await;
    config.data.shared_variables.insert(key, value);
    config.save()?;
    Ok(())
}

#[tauri::command]
pub async fn remove_shared_variable(
    key: String,
    state: State<'_, AppState>,
) -> Result<(), AppError> {
    let mut config = state.config.lock().await;
    config.data.shared_variables.remove(&key);
    config.save()?;
    Ok(())
}

#[tauri::command]
pub async fn get_node_variable_overrides(
    node_id: String,
    state: State<'_, AppState>,
) -> Result<HashMap<String, String>, AppError> {
    let config = state.config.lock().await;
    let overrides = config
        .data
        .node_variable_overrides
        .get(&node_id)
        .cloned()
        .unwrap_or_default();
    Ok(overrides)
}

#[tauri::command]
pub async fn set_node_variable_override(
    node_id: String,
    key: String,
    value: String,
    state: State<'_, AppState>,
) -> Result<(), AppError> {
    let mut config = state.config.lock().await;
    config
        .data
        .node_variable_overrides
        .entry(node_id)
        .or_default()
        .insert(key, value);
    config.save()?;
    Ok(())
}

#[tauri::command]
pub async fn remove_node_variable_override(
    node_id: String,
    key: String,
    state: State<'_, AppState>,
) -> Result<(), AppError> {
    let mut config = state.config.lock().await;
    if let Some(overrides) = config.data.node_variable_overrides.get_mut(&node_id) {
        overrides.remove(&key);
        // Clean up empty override maps to keep config tidy
        if overrides.is_empty() {
            config.data.node_variable_overrides.remove(&node_id);
        }
    }
    config.save()?;
    Ok(())
}
