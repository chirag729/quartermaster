use tauri::State;

use crate::error::AppError;
use crate::state::AppState;

#[tauri::command]
pub async fn vault_exists(state: State<'_, AppState>) -> Result<bool, AppError> {
    let vault = state.vault.lock().await;
    Ok(vault.exists())
}

#[tauri::command]
pub async fn vault_is_unlocked(state: State<'_, AppState>) -> Result<bool, AppError> {
    let vault = state.vault.lock().await;
    Ok(vault.is_unlocked())
}

#[tauri::command]
pub async fn vault_create(
    password: String,
    state: State<'_, AppState>,
) -> Result<(), AppError> {
    let mut vault = state.vault.lock().await;
    if vault.exists() {
        return Err(AppError::Vault(
            "Vault already exists. Use vault_change_password to change the master password."
                .to_string(),
        ));
    }
    vault.create(&password)
}

#[tauri::command]
pub async fn vault_unlock(
    password: String,
    state: State<'_, AppState>,
) -> Result<(), AppError> {
    let mut vault = state.vault.lock().await;
    vault.unlock(&password)
}

#[tauri::command]
pub async fn vault_lock(state: State<'_, AppState>) -> Result<(), AppError> {
    let mut vault = state.vault.lock().await;
    vault.lock();
    Ok(())
}

#[tauri::command]
pub async fn vault_get(key: String, state: State<'_, AppState>) -> Result<Option<String>, AppError> {
    let vault = state.vault.lock().await;
    vault.get(&key)
}

#[tauri::command]
pub async fn vault_set(
    key: String,
    value: String,
    state: State<'_, AppState>,
) -> Result<(), AppError> {
    let mut vault = state.vault.lock().await;
    vault.set(key, value)
}

#[tauri::command]
pub async fn vault_remove(key: String, state: State<'_, AppState>) -> Result<bool, AppError> {
    let mut vault = state.vault.lock().await;
    vault.remove(&key)
}

#[tauri::command]
pub async fn vault_list_keys(state: State<'_, AppState>) -> Result<Vec<String>, AppError> {
    let vault = state.vault.lock().await;
    vault.list_keys()
}

#[tauri::command]
pub async fn vault_change_password(
    current_password: String,
    new_password: String,
    state: State<'_, AppState>,
) -> Result<(), AppError> {
    let mut vault = state.vault.lock().await;
    vault.change_password(&current_password, &new_password)
}
