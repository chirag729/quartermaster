use std::collections::HashMap;
use std::path::PathBuf;

use serde_json::Value;
use tauri::{AppHandle, State};

use crate::apparmor::{log_parser, rule_generator, rule_consolidator, profile_manager, monitor, types::*};
use crate::apparmor::template_manager::ProfileTemplateManager;
use crate::apparmor::template_schema::*;
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

// Profile template commands

#[tauri::command]
pub async fn list_profile_templates(
    state: State<'_, AppState>,
) -> Result<Vec<ProfileTemplateInfo>, AppError> {
    let config = state.config.lock().await;
    let templates = state.profile_templates.list();

    let infos = templates
        .iter()
        .map(|t| {
            let status = ProfileTemplateManager::get_status(t, &config, &state.registry);
            ProfileTemplateManager::to_info(t, status)
        })
        .collect();

    Ok(infos)
}

#[tauri::command]
pub async fn get_profile_template(
    template_id: String,
    state: State<'_, AppState>,
) -> Result<ProfileTemplateInfo, AppError> {
    let config = state.config.lock().await;
    let template = state
        .profile_templates
        .get(&template_id)
        .ok_or_else(|| AppError::AppArmor(format!("Unknown profile template: {}", template_id)))?;

    let status = ProfileTemplateManager::get_status(template, &config, &state.registry);
    Ok(ProfileTemplateManager::to_info(template, status))
}

#[tauri::command]
pub async fn preview_profile(
    template_id: String,
    state: State<'_, AppState>,
) -> Result<String, AppError> {
    let config = state.config.lock().await;
    let template = state
        .profile_templates
        .get(&template_id)
        .ok_or_else(|| AppError::AppArmor(format!("Unknown profile template: {}", template_id)))?;

    let home = dirs::home_dir()
        .unwrap_or_else(|| PathBuf::from("/home/unknown"))
        .to_string_lossy()
        .to_string();

    let task_config = config
        .data
        .task_configs
        .get(&template.task_id)
        .cloned()
        .unwrap_or_default();

    let profile_config = config
        .data
        .profile_configs
        .get(&template.id)
        .cloned()
        .unwrap_or_default();

    let context = ProfileTemplateManager::build_context(
        template,
        &state.registry,
        &task_config,
        &profile_config,
        &home,
    );
    let rendered = ProfileTemplateManager::render(template, &context);

    Ok(rendered)
}

#[tauri::command]
pub async fn install_profile_template(
    template_id: String,
    state: State<'_, AppState>,
) -> Result<(), AppError> {
    let mut config = state.config.lock().await;
    ProfileTemplateManager::install_profile(
        &template_id,
        &state.profile_templates,
        &state.registry,
        &mut config,
    )
    .await
}

#[tauri::command]
pub async fn uninstall_profile_template(
    template_id: String,
    state: State<'_, AppState>,
) -> Result<(), AppError> {
    let mut config = state.config.lock().await;
    ProfileTemplateManager::uninstall_profile(
        &template_id,
        &state.profile_templates,
        &mut config,
    )
    .await
}

#[tauri::command]
pub async fn sync_installed_profiles(
    state: State<'_, AppState>,
) -> Result<Vec<SyncResultInfo>, AppError> {
    // Collect IDs and profile names under lock, then drop before async loop
    let id_names: Vec<(String, String)> = {
        let config = state.config.lock().await;
        config
            .data
            .installed_profiles
            .iter()
            .map(|(id, s)| (id.clone(), s.profile_name.clone()))
            .collect()
    };

    let mut results = Vec::new();

    for (id, profile_name) in &id_names {
        // Re-acquire lock for each sync call to avoid holding across awaits
        let mut config = state.config.lock().await;
        let updated = match ProfileTemplateManager::sync_profile(
            id,
            &state.profile_templates,
            &state.registry,
            &mut config,
        )
        .await
        {
            Ok(v) => v,
            Err(e) => {
                eprintln!("Warning: sync_profile for '{}' failed: {}", id, e);
                false
            }
        };
        drop(config);

        results.push(SyncResultInfo {
            profile_id: id.clone(),
            profile_name: profile_name.clone(),
            updated,
        });
    }

    Ok(results)
}

#[tauri::command]
pub async fn get_profile_template_config(
    template_id: String,
    state: State<'_, AppState>,
) -> Result<ProfileTemplateConfig, AppError> {
    let config = state.config.lock().await;
    let template = state
        .profile_templates
        .get(&template_id)
        .ok_or_else(|| AppError::AppArmor(format!("Unknown profile template: {}", template_id)))?;

    let home = dirs::home_dir()
        .unwrap_or_else(|| PathBuf::from("/home/unknown"))
        .to_string_lossy()
        .to_string();

    let task_config = config
        .data
        .task_configs
        .get(&template.task_id)
        .cloned()
        .unwrap_or_default();

    let profile_config = config
        .data
        .profile_configs
        .get(&template.id)
        .cloned()
        .unwrap_or_default();

    Ok(ProfileTemplateManager::build_config_fields(
        template,
        &state.registry,
        &task_config,
        &profile_config,
        &home,
    ))
}

#[tauri::command]
pub async fn set_profile_template_config(
    template_id: String,
    config_values: HashMap<String, String>,
    state: State<'_, AppState>,
) -> Result<(), AppError> {
    let mut config = state.config.lock().await;

    // Verify template exists
    let template = state
        .profile_templates
        .get(&template_id)
        .ok_or_else(|| AppError::AppArmor(format!("Unknown profile template: {}", template_id)))?
        .clone();

    // Store profile-specific config values
    let profile_var_keys: Vec<String> = template.variables.iter().map(|v| v.key.clone()).collect();

    let mut profile_config_values: HashMap<String, Value> = HashMap::new();
    for (key, value) in &config_values {
        if profile_var_keys.contains(key) {
            profile_config_values.insert(key.clone(), Value::String(value.clone()));
        }
    }

    if !profile_config_values.is_empty() {
        config
            .data
            .profile_configs
            .insert(template_id.clone(), profile_config_values);
    }

    // If the profile is installed, re-install to apply changes
    if config.data.installed_profiles.contains_key(&template_id) {
        ProfileTemplateManager::install_profile(
            &template_id,
            &state.profile_templates,
            &state.registry,
            &mut config,
        )
        .await?;
    }

    config.save()?;

    Ok(())
}
