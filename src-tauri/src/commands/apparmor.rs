use tauri::{AppHandle, State};

use crate::apparmor::{log_parser, rule_generator, rule_consolidator, profile_manager, monitor, types::*};
use crate::error::AppError;
use crate::state::AppState;

#[derive(serde::Serialize)]
pub struct DenialLogsResponse {
    pub denials: Vec<DenialEvent>,
    pub suggestions: Vec<PermissionSuggestion>,
}

#[tauri::command]
pub async fn get_denial_logs() -> Result<DenialLogsResponse, AppError> {
    let denials = log_parser::parse_audit_log().await?;
    let suggestions = rule_generator::generate_suggestions(&denials);
    Ok(DenialLogsResponse {
        denials,
        suggestions,
    })
}

#[tauri::command]
pub async fn get_profiles() -> Result<Vec<ProfileInfo>, AppError> {
    profile_manager::list_profiles().await
}

#[tauri::command]
pub async fn get_profile_detail(profile_name: String) -> Result<ProfileDetail, AppError> {
    profile_manager::get_profile_detail(&profile_name).await
}

#[tauri::command]
pub async fn apply_permission_rules(request: ApplyPermissionsRequest) -> Result<(), AppError> {
    profile_manager::apply_rules(&request.profile, &request.rules).await
}

#[tauri::command]
pub async fn start_log_monitor(
    app: AppHandle,
    state: State<'_, AppState>,
) -> Result<(), AppError> {
    let running = state.monitor_running.clone();
    monitor::start_monitoring(app, running).await
}

#[tauri::command]
pub async fn stop_log_monitor(state: State<'_, AppState>) -> Result<(), AppError> {
    let running = state.monitor_running.clone();
    monitor::stop_monitoring(running).await;
    Ok(())
}

#[tauri::command]
pub async fn consolidate_rules(
    suggestions: Vec<PermissionSuggestion>,
) -> Result<Vec<ConsolidationResult>, AppError> {
    Ok(rule_consolidator::consolidate(&suggestions))
}

#[tauri::command]
pub async fn apply_permission_rules_batch(
    requests: Vec<ApplyPermissionsRequest>,
) -> Result<(), AppError> {
    profile_manager::apply_rules_batch(&requests).await
}

#[tauri::command]
pub async fn consolidate_profile_rules(
    profile_name: String,
) -> Result<ConsolidationResult, AppError> {
    let detail = profile_manager::get_profile_detail(&profile_name).await?;
    Ok(rule_consolidator::consolidate_raw_rules(
        &profile_name,
        &detail.rules,
    ))
}

#[tauri::command]
pub async fn rewrite_profile_rules(
    profile_name: String,
    rules: Vec<String>,
) -> Result<(), AppError> {
    profile_manager::rewrite_profile_rules(&profile_name, &rules).await
}
