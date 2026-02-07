use tauri::State;

use crate::activity_log::ActivityEntry;
use crate::error::AppError;
use crate::state::AppState;

#[tauri::command]
pub async fn get_activity_log(
    count: Option<usize>,
    state: State<'_, AppState>,
) -> Result<Vec<ActivityEntry>, AppError> {
    let log = state.activity_log.lock().await;
    match count {
        Some(n) => Ok(log.recent(n).into_iter().cloned().collect()),
        None => Ok(log.entries().iter().cloned().collect()),
    }
}

#[tauri::command]
pub async fn clear_activity_log(state: State<'_, AppState>) -> Result<(), AppError> {
    let mut log = state.activity_log.lock().await;
    log.clear();
    Ok(())
}
