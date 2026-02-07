use std::collections::HashMap;

use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Emitter, State};

use crate::error::AppError;
use crate::executor::CommandExecutor;
use crate::executor::local::LocalExecutor;
use crate::executor::ssh::SshExecutor;
use crate::fleet::NodeKind;
use crate::tasks::install_state;
use crate::tasks::TaskInfo;
use crate::state::AppState;

/// Extended task info returned to the frontend, including installation state.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskStateInfo {
    #[serde(flatten)]
    pub info: TaskInfo,
    /// Whether config has changed since the task was last installed on this node.
    pub config_drifted: bool,
    /// Whether the task version has been upgraded since last installation.
    pub version_changed: bool,
    /// When the task was last installed on this node, if ever.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub installed_at: Option<String>,
    /// The version that was installed, if any.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub installed_version: Option<String>,
}

#[tauri::command]
pub async fn list_tasks(state: State<'_, AppState>) -> Result<Vec<TaskInfo>, AppError> {
    let config = state.config.lock().await;
    let task_configs = config.data.task_configs.clone();
    drop(config);

    let exec = LocalExecutor::new();
    let mut infos = Vec::new();
    for task in state.registry.tasks() {
        let task_config = task_configs
            .get(task.id())
            .cloned()
            .unwrap_or_default();
        let status = task.detect_state(&task_config, &exec).await;
        infos.push(task.to_info(status, None));
    }
    Ok(infos)
}

#[tauri::command]
pub async fn detect_all_states(state: State<'_, AppState>) -> Result<Vec<TaskInfo>, AppError> {
    list_tasks(state).await
}

/// Returns extended task state info for a specific node, including drift detection.
#[tauri::command]
pub async fn list_tasks_for_node(
    node_id: String,
    state: State<'_, AppState>,
) -> Result<Vec<TaskStateInfo>, AppError> {
    let config = state.config.lock().await;
    let task_configs = config.data.task_configs.clone();
    let installed_tasks = config.data.installed_tasks.clone();
    drop(config);

    // Determine executor for this node
    let exec: Box<dyn CommandExecutor> = {
        let fleet_manager = state.fleet_manager.lock().await;
        let node = fleet_manager.get_node(&node_id).cloned().ok_or_else(|| {
            AppError::Fleet(format!("Node not found: {}", node_id))
        })?;
        drop(fleet_manager);
        match node.kind {
            NodeKind::Local => Box::new(LocalExecutor::new()),
            NodeKind::Remote => {
                let ssh_config = node.ssh_config.as_ref().ok_or_else(|| {
                    AppError::Ssh(format!(
                        "Remote node '{}' has no SSH configuration",
                        node.name
                    ))
                })?;
                let vault_password: Option<String> = match &ssh_config.auth_method {
                    crate::fleet::SshAuthMethod::Password { vault_key } => {
                        if let Some(key) = vault_key {
                            let vault = state.vault.lock().await;
                            vault.get(key)?
                        } else {
                            None
                        }
                    }
                    _ => None,
                };
                Box::new(SshExecutor::from_ssh_config(ssh_config, vault_password.as_deref()))
            }
        }
    };

    let mut results = Vec::new();
    for task in state.registry.tasks() {
        let task_config = task_configs
            .get(task.id())
            .cloned()
            .unwrap_or_default();
        let status = task.detect_state(&task_config, exec.as_ref()).await;
        let info = task.to_info(status, None);

        let key = install_state::state_key(task.id(), &node_id);
        let (config_drifted, version_changed, installed_at, installed_version) =
            if let Some(install) = installed_tasks.get(&key) {
                (
                    install_state::has_config_drifted(install, &task_config),
                    install_state::has_version_changed(install, task.version().as_deref()),
                    Some(install.installed_at.clone()),
                    install.version.clone(),
                )
            } else {
                (false, false, None, None)
            };

        results.push(TaskStateInfo {
            info,
            config_drifted,
            version_changed,
            installed_at,
            installed_version,
        });
    }

    Ok(results)
}

#[tauri::command]
pub async fn execute_task(
    task_id: String,
    node_id: Option<String>,
    blueprint_id: Option<String>,
    app: AppHandle,
    state: State<'_, AppState>,
) -> Result<(), AppError> {
    let task = state
        .registry
        .get(&task_id)
        .ok_or_else(|| AppError::Task(format!("Task not found: {}", task_id)))?;

    let config = state.config.lock().await;
    let task_config = config
        .data
        .task_configs
        .get(&task_id)
        .cloned()
        .unwrap_or_default();
    drop(config);

    let effective_node_id = node_id.clone().unwrap_or_else(|| "local".to_string());

    // Choose executor based on node kind
    let exec: Box<dyn CommandExecutor> = if let Some(ref nid) = node_id {
        let fleet_manager = state.fleet_manager.lock().await;
        let node = fleet_manager
            .get_node(nid)
            .cloned()
            .ok_or_else(|| AppError::Fleet(format!("Node not found: {}", nid)))?;
        drop(fleet_manager);

        match node.kind {
            NodeKind::Local => Box::new(LocalExecutor::new()),
            NodeKind::Remote => {
                let ssh_config = node.ssh_config.as_ref().ok_or_else(|| {
                    AppError::Ssh(format!("Remote node '{}' has no SSH configuration", node.name))
                })?;
                // For password auth, retrieve from vault
                let vault_password: Option<String> = match &ssh_config.auth_method {
                    crate::fleet::SshAuthMethod::Password { vault_key } => {
                        if let Some(key) = vault_key {
                            let vault = state.vault.lock().await;
                            vault.get(key)?
                        } else {
                            None
                        }
                    }
                    _ => None,
                };
                Box::new(SshExecutor::from_ssh_config(
                    ssh_config,
                    vault_password.as_deref(),
                ))
            }
        }
    } else {
        Box::new(LocalExecutor::new())
    };

    let app_handle = app.clone();
    let tid = task_id.clone();
    let progress_cb: crate::tasks::ProgressCallback = Box::new(move |progress, message| {
        let _ = app_handle.emit("task-progress", serde_json::json!({
            "task_id": tid,
            "progress": progress,
            "message": message,
        }));
    });

    let result = task.execute(&task_config, exec.as_ref(), &progress_cb).await;

    // Log the task execution result
    {
        let mut log = state.activity_log.lock().await;
        let detail = Some(format!("node: {}", effective_node_id));
        match &result {
            Ok(()) => {
                log.log("task_executed", &task_id, detail.as_deref(), true);
            }
            Err(e) => {
                let err_detail = format!("node: {}, error: {}", effective_node_id, e);
                log.log("task_executed", &task_id, Some(&err_detail), false);
            }
        }
    }

    result?;

    let _ = app.emit("task-state-changed", serde_json::json!({
        "task_id": task_id,
        "status": "completed",
    }));

    // Record installation state
    let mut config = state.config.lock().await;
    if !config.data.completed_tasks.contains(&task_id) {
        config.data.completed_tasks.push(task_id.clone());
    }

    let install_record = install_state::record_installation(
        &task_id,
        &effective_node_id,
        task.version().as_deref(),
        blueprint_id.as_deref(),
        &task_config,
    );
    let key = install_state::state_key(&task_id, &effective_node_id);
    config.data.installed_tasks.insert(key, install_record);
    config.save()?;

    Ok(())
}

#[tauri::command]
pub async fn uninstall_task(
    task_id: String,
    node_id: Option<String>,
    app: AppHandle,
    state: State<'_, AppState>,
) -> Result<(), AppError> {
    let task = state
        .registry
        .get(&task_id)
        .ok_or_else(|| AppError::Task(format!("Task not found: {}", task_id)))?;

    if !task.supports_uninstall() {
        return Err(AppError::Task(format!(
            "Task '{}' does not support uninstall",
            task.name()
        )));
    }

    // Build executor
    let exec: Box<dyn CommandExecutor> = if let Some(ref nid) = node_id {
        let fleet_manager = state.fleet_manager.lock().await;
        let node = fleet_manager
            .get_node(nid)
            .cloned()
            .ok_or_else(|| AppError::Fleet(format!("Node not found: {}", nid)))?;
        drop(fleet_manager);

        match node.kind {
            NodeKind::Local => Box::new(LocalExecutor::new()),
            NodeKind::Remote => {
                let ssh_config = node.ssh_config.as_ref().ok_or_else(|| {
                    AppError::Ssh(format!(
                        "Remote node '{}' has no SSH configuration",
                        node.name
                    ))
                })?;
                // For password auth, retrieve from vault
                let vault_password: Option<String> = match &ssh_config.auth_method {
                    crate::fleet::SshAuthMethod::Password { vault_key } => {
                        if let Some(key) = vault_key {
                            let vault = state.vault.lock().await;
                            vault.get(key)?
                        } else {
                            None
                        }
                    }
                    _ => None,
                };
                Box::new(SshExecutor::from_ssh_config(
                    ssh_config,
                    vault_password.as_deref(),
                ))
            }
        }
    } else {
        Box::new(LocalExecutor::new())
    };

    // Get task config
    let config = state.config.lock().await;
    let task_config = config
        .data
        .task_configs
        .get(&task_id)
        .cloned()
        .unwrap_or_default();
    drop(config);

    let effective_node_id = node_id.clone().unwrap_or_else(|| "local".to_string());

    // Progress callback
    let app_handle = app.clone();
    let tid = task_id.clone();
    let nid = effective_node_id.clone();
    let progress_cb: crate::tasks::ProgressCallback = Box::new(move |progress, message| {
        let _ = app_handle.emit(
            "task-progress",
            serde_json::json!({
                "node_id": nid,
                "task_id": tid,
                "progress": progress,
                "message": message,
            }),
        );
    });

    let result = task
        .uninstall(&task_config, exec.as_ref(), &progress_cb)
        .await;

    // Log the uninstall result
    {
        let mut log = state.activity_log.lock().await;
        let detail = Some(format!("node: {}", effective_node_id));
        match &result {
            Ok(()) => {
                log.log("task_uninstalled", &task_id, detail.as_deref(), true);
            }
            Err(e) => {
                let err_detail = format!("node: {}, error: {}", effective_node_id, e);
                log.log("task_uninstalled", &task_id, Some(&err_detail), false);
            }
        }
    }

    result?;

    // Emit state change
    let _ = app.emit(
        "task-state-changed",
        serde_json::json!({
            "node_id": effective_node_id,
            "task_id": task_id,
            "status": "not_started",
        }),
    );

    // Remove installation state
    let mut config = state.config.lock().await;
    let key = crate::tasks::install_state::state_key(&task_id, &effective_node_id);
    config.data.installed_tasks.remove(&key);
    config.data.completed_tasks.retain(|t| t != &task_id);
    config.save()?;

    Ok(())
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct TaskUpdateInfo {
    pub task_id: String,
    pub task_name: String,
    pub defined_version: Option<String>,
    pub installed_version: Option<String>,
    pub update_available: bool,
}

#[tauri::command]
pub async fn check_task_updates(
    node_id: Option<String>,
    state: State<'_, AppState>,
) -> Result<Vec<TaskUpdateInfo>, AppError> {
    let exec: Box<dyn CommandExecutor> = if let Some(ref nid) = node_id {
        let fleet_manager = state.fleet_manager.lock().await;
        let node = fleet_manager
            .get_node(nid)
            .cloned()
            .ok_or_else(|| AppError::Fleet(format!("Node not found: {}", nid)))?;
        drop(fleet_manager);
        match node.kind {
            NodeKind::Local => Box::new(LocalExecutor::new()),
            NodeKind::Remote => {
                let ssh = node.ssh_config.as_ref().ok_or_else(|| {
                    AppError::Ssh(format!("No SSH config for node '{}'", node.name))
                })?;
                let vault_password: Option<String> = match &ssh.auth_method {
                    crate::fleet::SshAuthMethod::Password { vault_key } => {
                        if let Some(key) = vault_key {
                            let vault = state.vault.lock().await;
                            vault.get(key)?
                        } else {
                            None
                        }
                    }
                    _ => None,
                };
                Box::new(SshExecutor::from_ssh_config(ssh, vault_password.as_deref()))
            }
        }
    } else {
        Box::new(LocalExecutor::new())
    };

    let mut results = Vec::new();
    let config = state.config.lock().await;

    for task in state.registry.all() {
        let task_config = config.data.task_configs
            .get(task.id())
            .cloned()
            .unwrap_or_default();

        let defined_version = task.version();
        let installed_version = task.detect_installed_version(&task_config, exec.as_ref()).await;

        let update_available = match (&defined_version, &installed_version) {
            (Some(defined), Some(installed)) => defined != installed,
            _ => false,
        };

        if defined_version.is_some() || installed_version.is_some() {
            results.push(TaskUpdateInfo {
                task_id: task.id().to_string(),
                task_name: task.name().to_string(),
                defined_version,
                installed_version,
                update_available,
            });
        }
    }

    drop(config);
    Ok(results)
}

/// Returns installation state for all tasks on a given node.
#[tauri::command]
pub async fn get_installation_states(
    node_id: String,
    state: State<'_, AppState>,
) -> Result<HashMap<String, crate::config::manager::InstalledTaskState>, AppError> {
    let config = state.config.lock().await;
    let mut result = HashMap::new();

    for (key, install) in &config.data.installed_tasks {
        if install.node_id == node_id {
            result.insert(key.clone(), install.clone());
        }
    }

    Ok(result)
}
