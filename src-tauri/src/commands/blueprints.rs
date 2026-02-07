use std::collections::HashMap;

use serde::Serialize;
use tauri::{AppHandle, Emitter, State};

use crate::blueprints::{Blueprint, BlueprintTaskEntry};
use crate::error::AppError;
use crate::executor::CommandExecutor;
use crate::executor::dry_run::{DryRunAction, DryRunExecutor};
use crate::executor::local::LocalExecutor;
use crate::executor::privileged::PrivilegedLocalExecutor;
use crate::executor::ssh::SshExecutor;
use crate::fleet::NodeKind;
use crate::state::AppState;
use crate::tasks::{ExecutionTarget, PrivilegeLevel, TaskStatus};
use crate::tasks::install_state;

fn bump_version(current: &str, bump_type: &str) -> String {
    let parts: Vec<u32> = current
        .split('.')
        .map(|p| p.parse().unwrap_or(0))
        .collect();
    let (major, minor, patch) = (
        parts.first().copied().unwrap_or(1),
        parts.get(1).copied().unwrap_or(0),
        parts.get(2).copied().unwrap_or(0),
    );
    match bump_type {
        "major" => format!("{}.0.0", major + 1),
        "minor" => format!("{}.{}.0", major, minor + 1),
        _ => format!("{}.{}.{}", major, minor, patch + 1),
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct BulkApplyResult {
    pub node_id: String,
    pub node_name: String,
    pub success: bool,
    pub error: Option<String>,
}

#[tauri::command]
pub async fn list_blueprints(state: State<'_, AppState>) -> Result<Vec<Blueprint>, AppError> {
    let manager = state.blueprint_manager.lock().await;
    Ok(manager.list_blueprints().to_vec())
}

#[tauri::command]
pub async fn get_blueprint(
    blueprint_id: String,
    state: State<'_, AppState>,
) -> Result<Blueprint, AppError> {
    let manager = state.blueprint_manager.lock().await;
    manager
        .get_blueprint(&blueprint_id)
        .cloned()
        .ok_or_else(|| AppError::Blueprint(format!("Blueprint not found: {}", blueprint_id)))
}

#[tauri::command]
pub async fn create_blueprint(
    name: String,
    description: String,
    icon: String,
    task_entries: Vec<BlueprintTaskEntry>,
    state: State<'_, AppState>,
) -> Result<Blueprint, AppError> {
    let mut manager = state.blueprint_manager.lock().await;
    let blueprint = manager.create_blueprint(name, description, icon, task_entries)?;
    drop(manager);

    let mut log = state.activity_log.lock().await;
    log.log(
        "blueprint_created",
        &blueprint.name,
        Some(&format!("{} tasks", blueprint.task_entries.len())),
        true,
    );

    Ok(blueprint)
}

#[tauri::command]
pub async fn update_blueprint(
    blueprint: Blueprint,
    state: State<'_, AppState>,
) -> Result<Blueprint, AppError> {
    // Auto-resolve dependencies: if a task with depends_on is present,
    // ensure its dependencies are also in the entry list
    let mut updated = blueprint;
    let task_ids: Vec<String> = updated.task_entries.iter().map(|e| e.task_id.clone()).collect();
    let resolved = state.registry.resolve_dependencies(&task_ids)
        .map_err(|e| AppError::Blueprint(e))?;

    // Add any missing dependency tasks
    let existing_ids: std::collections::HashSet<String> = task_ids.into_iter().collect();
    let max_order = updated.task_entries.iter().map(|e| e.order).max().unwrap_or(0);
    let mut next_order = max_order + 1;

    for dep_id in &resolved {
        if !existing_ids.contains(dep_id) {
            updated.task_entries.push(BlueprintTaskEntry {
                task_id: dep_id.clone(),
                enabled: true,
                config_overrides: HashMap::new(),
                order: next_order,
            });
            next_order += 1;
        }
    }

    // Re-sort entries to match dependency order
    let order_map: HashMap<&str, usize> = resolved
        .iter()
        .enumerate()
        .map(|(i, id)| (id.as_str(), i))
        .collect();

    updated.task_entries.sort_by_key(|e| {
        order_map.get(e.task_id.as_str()).copied().unwrap_or(usize::MAX)
    });

    // Reassign order values
    for (i, entry) in updated.task_entries.iter_mut().enumerate() {
        entry.order = i as u32;
    }

    // Single lock for version comparison + write (prevents TOCTOU)
    let mut manager = state.blueprint_manager.lock().await;

    // Reject updates to built-in blueprints (check the stored version, not the incoming struct)
    if let Some(old) = manager.get_blueprint(&updated.id) {
        if old.is_builtin {
            return Err(AppError::Blueprint(
                "Cannot modify a built-in blueprint. Clone it first.".to_string(),
            ));
        }
    }

    // Auto-bump version based on what changed
    if let Some(old) = manager.get_blueprint(&updated.id) {
        let old_task_ids: Vec<&str> = old.task_entries.iter().map(|e| e.task_id.as_str()).collect();
        let new_task_ids: Vec<&str> = updated.task_entries.iter().map(|e| e.task_id.as_str()).collect();

        if old_task_ids != new_task_ids {
            // Tasks added or removed: bump minor version
            updated.version = bump_version(&old.version, "minor");
        } else {
            // Check if any config_overrides or enabled state changed
            let configs_changed = old.task_entries.iter().zip(updated.task_entries.iter()).any(
                |(old_entry, new_entry)| {
                    old_entry.config_overrides != new_entry.config_overrides
                        || old_entry.enabled != new_entry.enabled
                },
            );
            if configs_changed {
                updated.version = bump_version(&old.version, "patch");
            }
            // If nothing changed, keep the same version
        }
    }

    manager.update_blueprint(updated.clone())?;
    Ok(updated)
}

#[tauri::command]
pub async fn remove_blueprint(
    blueprint_id: String,
    state: State<'_, AppState>,
) -> Result<(), AppError> {
    let mut manager = state.blueprint_manager.lock().await;
    manager.remove_blueprint(&blueprint_id)?;
    Ok(())
}

#[tauri::command]
pub async fn clone_blueprint(
    id: String,
    new_name: String,
    state: State<'_, AppState>,
) -> Result<Blueprint, AppError> {
    let mut manager = state.blueprint_manager.lock().await;
    manager.clone_blueprint(&id, new_name)
}

#[tauri::command]
pub async fn create_blank_blueprint(
    name: String,
    description: String,
    icon: Option<String>,
    state: State<'_, AppState>,
) -> Result<Blueprint, AppError> {
    let mut manager = state.blueprint_manager.lock().await;
    manager.create_blueprint(name, description, icon.unwrap_or_else(|| "Layers".to_string()), vec![])
}

#[tauri::command]
pub async fn delete_blueprint(
    id: String,
    state: State<'_, AppState>,
) -> Result<(), AppError> {
    // Hold bp_manager across the entire operation (check + remove)
    let mut bp_manager = state.blueprint_manager.lock().await;

    // Check if builtin
    let bp = bp_manager
        .get_blueprint(&id)
        .ok_or_else(|| AppError::Blueprint(format!("Blueprint not found: {}", id)))?;
    if bp.is_builtin {
        return Err(AppError::Blueprint("Cannot delete a built-in blueprint".to_string()));
    }

    // Unassign from any nodes that reference this blueprint
    {
        let mut fleet_manager = state.fleet_manager.lock().await;
        let nodes_to_update: Vec<String> = fleet_manager
            .list_nodes()
            .iter()
            .filter(|n| n.blueprint_id.as_deref() == Some(&id))
            .map(|n| n.id.clone())
            .collect();
        for node_id in nodes_to_update {
            if let Some(node) = fleet_manager.get_node(&node_id).cloned() {
                let mut updated_node = node;
                updated_node.blueprint_id = None;
                fleet_manager.update_node(updated_node)?;
            }
        }
    }

    bp_manager.remove_blueprint(&id)?;
    Ok(())
}

#[tauri::command]
pub async fn assign_blueprint(
    node_id: String,
    blueprint_id: String,
    state: State<'_, AppState>,
) -> Result<(), AppError> {
    // Validate blueprint exists
    let bp_manager = state.blueprint_manager.lock().await;
    let bp_name = bp_manager
        .get_blueprint(&blueprint_id)
        .map(|bp| bp.name.clone())
        .ok_or_else(|| AppError::Blueprint(format!("Blueprint not found: {}", blueprint_id)))?;
    drop(bp_manager);

    // Validate node exists and update it
    let mut fleet_manager = state.fleet_manager.lock().await;
    let node = fleet_manager
        .get_node(&node_id)
        .cloned()
        .ok_or_else(|| AppError::Fleet(format!("Node not found: {}", node_id)))?;
    let node_name = node.name.clone();
    let mut updated_node = node;
    updated_node.blueprint_id = Some(blueprint_id);
    fleet_manager.update_node(updated_node)?;
    drop(fleet_manager);

    let mut log = state.activity_log.lock().await;
    log.log(
        "blueprint_assigned",
        &bp_name,
        Some(&format!("Assigned to node '{}'", node_name)),
        true,
    );

    Ok(())
}

#[tauri::command]
pub async fn unassign_blueprint(
    node_id: String,
    state: State<'_, AppState>,
) -> Result<(), AppError> {
    let mut fleet_manager = state.fleet_manager.lock().await;
    let node = fleet_manager
        .get_node(&node_id)
        .cloned()
        .ok_or_else(|| AppError::Fleet(format!("Node not found: {}", node_id)))?;
    let node_name = node.name.clone();
    let mut updated_node = node;
    updated_node.blueprint_id = None;
    fleet_manager.update_node(updated_node)?;
    drop(fleet_manager);

    let mut log = state.activity_log.lock().await;
    log.log(
        "blueprint_unassigned",
        &node_name,
        None,
        true,
    );

    Ok(())
}

#[tauri::command]
pub async fn apply_blueprint(
    node_id: String,
    blueprint_id: String,
    app: AppHandle,
    state: State<'_, AppState>,
) -> Result<(), AppError> {
    // Get the blueprint
    let bp_manager = state.blueprint_manager.lock().await;
    let blueprint = bp_manager
        .get_blueprint(&blueprint_id)
        .cloned()
        .ok_or_else(|| AppError::Blueprint(format!("Blueprint not found: {}", blueprint_id)))?;

    // Resolve inherited entries (merges parent + child)
    let resolved_entries = bp_manager.resolve_task_entries(&blueprint_id)?;
    drop(bp_manager);

    // Verify the node exists and get its kind
    let fleet_manager = state.fleet_manager.lock().await;
    let node = fleet_manager
        .get_node(&node_id)
        .cloned()
        .ok_or_else(|| AppError::Fleet(format!("Node not found: {}", node_id)))?;
    drop(fleet_manager);

    // Get enabled entries sorted by order
    let mut entries: Vec<BlueprintTaskEntry> = resolved_entries
        .into_iter()
        .filter(|e| e.enabled)
        .collect();
    entries.sort_by_key(|e| e.order);

    let total = entries.len();

    // For remote nodes, build the SSH executor once (shared across tasks)
    let ssh_exec: Option<Box<dyn CommandExecutor>> = match node.kind {
        NodeKind::Local => None,
        NodeKind::Remote => {
            let ssh_config = node.ssh_config.as_ref().ok_or_else(|| {
                AppError::Ssh(format!("Remote node '{}' has no SSH configuration", node.name))
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
            Some(Box::new(SshExecutor::from_ssh_config(ssh_config, vault_password.as_deref())))
        }
    };

    for (i, entry) in entries.iter().enumerate() {
        let task = match state.registry.get(&entry.task_id) {
            Some(t) => t,
            None => {
                let _ = app.emit("blueprint-task-warning", serde_json::json!({
                    "blueprint_id": blueprint_id,
                    "task_id": entry.task_id,
                    "warning": format!("Unknown task '{}' was skipped", entry.task_id),
                }));
                continue;
            }
        };

        // Enforce execution target constraints
        match (task.execution_target(), &node.kind) {
            (ExecutionTarget::LocalOnly, NodeKind::Remote) | (ExecutionTarget::RemoteOnly, NodeKind::Local) => {
                let _ = app.emit("blueprint-task-warning", serde_json::json!({
                    "blueprint_id": blueprint_id,
                    "task_id": entry.task_id,
                    "warning": format!("Task '{}' skipped: incompatible execution target", entry.task_id),
                }));
                continue;
            }
            _ => {}
        }

        // Enforce privilege level constraints for local execution
        if task.privilege_level() == PrivilegeLevel::Admin && node.kind == NodeKind::Local {
            if !crate::polkit::auth::is_policy_installed() {
                let _ = app.emit("blueprint-task-warning", serde_json::json!({
                    "blueprint_id": blueprint_id,
                    "task_id": entry.task_id,
                    "warning": format!("Task '{}' skipped: requires admin privileges but PolicyKit policy is not installed", entry.task_id),
                }));
                continue;
            }
        }

        // Choose executor per task: privileged local for Admin tasks, regular for others
        let exec: &dyn CommandExecutor = if let Some(ref ssh) = ssh_exec {
            ssh.as_ref()
        } else if task.privilege_level() == PrivilegeLevel::Admin {
            // For local Admin tasks, we need a temporary PrivilegedLocalExecutor
            // We use a nested block below to handle this
            &PrivilegedLocalExecutor::new()
        } else {
            &LocalExecutor::new()
        };

        // Merge config: base task config + blueprint overrides
        let config = state.config.lock().await;
        let mut task_config: HashMap<String, serde_json::Value> = config
            .data
            .task_configs
            .get(&entry.task_id)
            .cloned()
            .unwrap_or_default();
        drop(config);

        for (k, v) in &entry.config_overrides {
            task_config.insert(k.clone(), v.clone());
        }

        // Emit blueprint progress
        let _ = app.emit(
            "blueprint-apply-progress",
            serde_json::json!({
                "node_id": node_id,
                "blueprint_id": blueprint_id,
                "completed": i,
                "total": total,
                "current_task_id": entry.task_id,
            }),
        );

        // Detect state - skip if already completed
        let status = task.detect_state(&task_config, exec).await;
        if status == TaskStatus::Completed {
            continue;
        }

        // Execute with progress callback
        let app_handle = app.clone();
        let tid = entry.task_id.clone();
        let nid = node_id.clone();
        let progress_cb: crate::tasks::ProgressCallback =
            Box::new(move |progress, message| {
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

        if let Err(e) = task.execute(&task_config, exec, &progress_cb).await {
            let _ = app.emit(
                "blueprint-task-failed",
                serde_json::json!({
                    "node_id": node_id,
                    "blueprint_id": blueprint_id,
                    "task_id": entry.task_id,
                    "error": e.to_string(),
                }),
            );
            return Err(e);
        }

        // Record installation state for this task
        {
            let install_record = install_state::record_installation(
                &entry.task_id,
                &node_id,
                task.version().as_deref(),
                Some(&blueprint_id),
                &task_config,
            );
            let key = install_state::state_key(&entry.task_id, &node_id);
            let mut config = state.config.lock().await;
            config.data.installed_tasks.insert(key, install_record);
            config.save().map_err(|e| AppError::Config(format!(
                "Failed to save install state for task '{}': {}", entry.task_id, e
            )))?;
        }

        let _ = app.emit(
            "task-state-changed",
            serde_json::json!({
                "node_id": node_id,
                "task_id": entry.task_id,
                "status": "completed",
            }),
        );
    }

    // Record applied blueprint version on the node
    let mut fleet_manager = state.fleet_manager.lock().await;
    if let Some(node) = fleet_manager.get_node(&node_id).cloned() {
        let mut updated = node;
        updated.applied_blueprint_version = Some(blueprint.version.clone());
        fleet_manager.update_node(updated)?;
    }
    drop(fleet_manager);

    // Emit completion
    let _ = app.emit(
        "blueprint-apply-complete",
        serde_json::json!({
            "node_id": node_id,
            "blueprint_id": blueprint_id,
        }),
    );

    // Send desktop notification
    crate::notifications::notify_blueprint_applied(&app, &blueprint.name, &node.name);

    Ok(())
}

#[derive(Debug, Clone, Serialize)]
pub struct DryRunResult {
    pub node_id: String,
    pub blueprint_id: String,
    pub task_actions: Vec<DryRunTaskResult>,
}

#[derive(Debug, Clone, Serialize)]
pub struct DryRunTaskResult {
    pub task_id: String,
    pub task_name: String,
    pub status: String,
    pub actions: Vec<DryRunAction>,
}

#[tauri::command]
pub async fn dry_run_blueprint(
    node_id: String,
    blueprint_id: String,
    state: State<'_, AppState>,
) -> Result<DryRunResult, AppError> {
    // Get the blueprint
    let bp_manager = state.blueprint_manager.lock().await;
    let _blueprint = bp_manager
        .get_blueprint(&blueprint_id)
        .cloned()
        .ok_or_else(|| AppError::Blueprint(format!("Blueprint not found: {}", blueprint_id)))?;

    // Resolve inherited entries (merges parent + child)
    let resolved_entries = bp_manager.resolve_task_entries(&blueprint_id)?;
    drop(bp_manager);

    // Verify the node exists and get its kind
    let fleet_manager = state.fleet_manager.lock().await;
    let node = fleet_manager
        .get_node(&node_id)
        .cloned()
        .ok_or_else(|| AppError::Fleet(format!("Node not found: {}", node_id)))?;
    drop(fleet_manager);

    // Get enabled entries sorted by order
    let mut entries: Vec<BlueprintTaskEntry> = resolved_entries
        .into_iter()
        .filter(|e| e.enabled)
        .collect();
    entries.sort_by_key(|e| e.order);

    // Choose the real executor based on node kind (used as inner for dry-run)
    let real_exec: Box<dyn CommandExecutor> = match node.kind {
        NodeKind::Local => Box::new(LocalExecutor::new()),
        NodeKind::Remote => {
            let ssh_config = node.ssh_config.as_ref().ok_or_else(|| {
                AppError::Ssh(format!("Remote node '{}' has no SSH configuration", node.name))
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
    };

    // Wrap with dry-run executor
    let dry_exec = DryRunExecutor::new(real_exec);

    let mut task_actions = Vec::new();

    for entry in &entries {
        let task = match state.registry.get(&entry.task_id) {
            Some(t) => t,
            None => {
                task_actions.push(DryRunTaskResult {
                    task_id: entry.task_id.clone(),
                    task_name: entry.task_id.clone(),
                    status: "skipped".to_string(),
                    actions: vec![],
                });
                continue;
            }
        };

        // Enforce execution target constraints
        match (task.execution_target(), &node.kind) {
            (ExecutionTarget::LocalOnly, NodeKind::Remote) | (ExecutionTarget::RemoteOnly, NodeKind::Local) => {
                task_actions.push(DryRunTaskResult {
                    task_id: entry.task_id.clone(),
                    task_name: task.name().to_string(),
                    status: "skipped".to_string(),
                    actions: vec![],
                });
                continue;
            }
            _ => {}
        }

        // Merge config: base task config + blueprint overrides
        let config = state.config.lock().await;
        let mut task_config: HashMap<String, serde_json::Value> = config
            .data
            .task_configs
            .get(&entry.task_id)
            .cloned()
            .unwrap_or_default();
        drop(config);

        for (k, v) in &entry.config_overrides {
            task_config.insert(k.clone(), v.clone());
        }

        // Detect state using the dry-run executor (reads pass through to inner)
        let status = task.detect_state(&task_config, &dry_exec).await;

        if status == TaskStatus::Completed {
            // Clear any actions that accumulated during detect_state
            dry_exec.take_actions();
            task_actions.push(DryRunTaskResult {
                task_id: entry.task_id.clone(),
                task_name: task.name().to_string(),
                status: "already_completed".to_string(),
                actions: vec![],
            });
            continue;
        }

        // Clear any actions that may have accumulated from detect_state
        dry_exec.take_actions();

        // Execute with dry-run executor (mutations are intercepted)
        let no_op_progress: crate::tasks::ProgressCallback =
            Box::new(|_progress, _message| {});

        let exec_result = task.execute(&task_config, &dry_exec, &no_op_progress).await;

        let actions = dry_exec.take_actions();

        match exec_result {
            Ok(()) => {
                task_actions.push(DryRunTaskResult {
                    task_id: entry.task_id.clone(),
                    task_name: task.name().to_string(),
                    status: "would_run".to_string(),
                    actions,
                });
            }
            Err(_) => {
                task_actions.push(DryRunTaskResult {
                    task_id: entry.task_id.clone(),
                    task_name: task.name().to_string(),
                    status: "skipped".to_string(),
                    actions,
                });
            }
        }
    }

    Ok(DryRunResult {
        node_id,
        blueprint_id,
        task_actions,
    })
}

#[tauri::command]
pub async fn apply_blueprint_bulk(
    node_ids: Vec<String>,
    blueprint_id: String,
    app: AppHandle,
    state: State<'_, AppState>,
) -> Result<Vec<BulkApplyResult>, AppError> {
    // Validate the blueprint exists first
    let bp_manager = state.blueprint_manager.lock().await;
    let blueprint = bp_manager
        .get_blueprint(&blueprint_id)
        .cloned()
        .ok_or_else(|| AppError::Blueprint(format!("Blueprint not found: {}", blueprint_id)))?;

    // Resolve inherited entries (merges parent + child)
    let resolved_entries = bp_manager.resolve_task_entries(&blueprint_id)?;
    drop(bp_manager);

    let total = node_ids.len();
    let mut results: Vec<BulkApplyResult> = Vec::with_capacity(total);

    // Get enabled entries sorted by order (shared across all nodes)
    let mut entries: Vec<BlueprintTaskEntry> = resolved_entries
        .into_iter()
        .filter(|e| e.enabled)
        .collect();
    entries.sort_by_key(|e| e.order);

    // Iterate through each node sequentially to avoid overwhelming SSH connections
    for (i, node_id) in node_ids.iter().enumerate() {
        // Look up the node
        let fleet_manager = state.fleet_manager.lock().await;
        let node = match fleet_manager.get_node(node_id).cloned() {
            Some(n) => n,
            None => {
                results.push(BulkApplyResult {
                    node_id: node_id.clone(),
                    node_name: "Unknown".to_string(),
                    success: false,
                    error: Some(format!("Node not found: {}", node_id)),
                });
                continue;
            }
        };
        drop(fleet_manager);

        let node_name = node.name.clone();

        // Emit bulk progress event
        let _ = app.emit(
            "bulk-blueprint-progress",
            serde_json::json!({
                "completed": i,
                "total": total,
                "current_node_id": node_id,
                "current_node_name": node_name,
            }),
        );

        // Build the SSH executor for remote nodes (shared across tasks for this node)
        let ssh_exec: Option<Box<dyn CommandExecutor>> = match node.kind {
            NodeKind::Local => None,
            NodeKind::Remote => {
                match node.ssh_config.as_ref() {
                    Some(ssh_config) => {
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
                        Some(Box::new(SshExecutor::from_ssh_config(ssh_config, vault_password.as_deref())))
                    }
                    None => {
                        results.push(BulkApplyResult {
                            node_id: node_id.clone(),
                            node_name,
                            success: false,
                            error: Some(format!(
                                "Remote node '{}' has no SSH configuration",
                                node.name
                            )),
                        });
                        continue;
                    }
                }
            }
        };

        // Run the blueprint tasks for this node, catching errors per-node
        let mut node_error: Option<String> = None;
        for entry in &entries {
            let task = match state.registry.get(&entry.task_id) {
                Some(t) => t,
                None => {
                    let _ = app.emit("blueprint-task-warning", serde_json::json!({
                        "blueprint_id": blueprint_id,
                        "task_id": entry.task_id,
                        "warning": format!("Unknown task '{}' was skipped", entry.task_id),
                    }));
                    continue;
                }
            };

            // Enforce execution target constraints
            match (task.execution_target(), &node.kind) {
                (ExecutionTarget::LocalOnly, NodeKind::Remote) | (ExecutionTarget::RemoteOnly, NodeKind::Local) => {
                    let _ = app.emit("blueprint-task-warning", serde_json::json!({
                        "blueprint_id": blueprint_id,
                        "task_id": entry.task_id,
                        "warning": format!("Task '{}' skipped: incompatible execution target", entry.task_id),
                    }));
                    continue;
                }
                _ => {}
            }

            // Enforce privilege level constraints for local execution
            if task.privilege_level() == PrivilegeLevel::Admin && node.kind == NodeKind::Local {
                if !crate::polkit::auth::is_policy_installed() {
                    let _ = app.emit("blueprint-task-warning", serde_json::json!({
                        "blueprint_id": blueprint_id,
                        "task_id": entry.task_id,
                        "warning": format!("Task '{}' skipped: requires admin privileges but PolicyKit policy is not installed", entry.task_id),
                    }));
                    continue;
                }
            }

            // Choose executor per task: privileged local for Admin tasks, regular for others
            let exec: &dyn CommandExecutor = if let Some(ref ssh) = ssh_exec {
                ssh.as_ref()
            } else if task.privilege_level() == PrivilegeLevel::Admin {
                &PrivilegedLocalExecutor::new()
            } else {
                &LocalExecutor::new()
            };

            // Merge config: base task config + blueprint overrides
            let config = state.config.lock().await;
            let mut task_config: HashMap<String, serde_json::Value> = config
                .data
                .task_configs
                .get(&entry.task_id)
                .cloned()
                .unwrap_or_default();
            drop(config);

            for (k, v) in &entry.config_overrides {
                task_config.insert(k.clone(), v.clone());
            }

            // Detect state - skip if already completed
            let status = task.detect_state(&task_config, exec).await;
            if status == TaskStatus::Completed {
                continue;
            }

            // Execute with progress callback
            let app_handle = app.clone();
            let tid = entry.task_id.clone();
            let nid = node_id.clone();
            let progress_cb: crate::tasks::ProgressCallback =
                Box::new(move |progress, message| {
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

            match task.execute(&task_config, exec, &progress_cb).await {
                Ok(_) => {
                    // Record installation state for this task
                    {
                        let install_record = install_state::record_installation(
                            &entry.task_id,
                            node_id,
                            task.version().as_deref(),
                            Some(&blueprint_id),
                            &task_config,
                        );
                        let key = install_state::state_key(&entry.task_id, node_id);
                        let mut config = state.config.lock().await;
                        config.data.installed_tasks.insert(key, install_record);
                        if let Err(e) = config.save() {
                            node_error = Some(format!(
                                "Failed to save install state for task '{}': {}",
                                entry.task_id, e
                            ));
                            break;
                        }
                    }

                    let _ = app.emit(
                        "task-state-changed",
                        serde_json::json!({
                            "node_id": node_id,
                            "task_id": entry.task_id,
                            "status": "completed",
                        }),
                    );
                }
                Err(e) => {
                    node_error = Some(format!("Task '{}' failed: {}", entry.task_id, e));
                    break;
                }
            }
        }

        // Update applied_blueprint_version on successful nodes (matching apply_blueprint)
        if node_error.is_none() {
            let mut fleet_manager = state.fleet_manager.lock().await;
            if let Some(n) = fleet_manager.get_node(node_id).cloned() {
                let mut updated = n;
                updated.applied_blueprint_version = Some(blueprint.version.clone());
                let _ = fleet_manager.update_node(updated);
            }
            drop(fleet_manager);
        }

        results.push(BulkApplyResult {
            node_id: node_id.clone(),
            node_name: node_name.clone(),
            success: node_error.is_none(),
            error: node_error,
        });
    }

    // Emit final progress (all completed)
    let _ = app.emit(
        "bulk-blueprint-progress",
        serde_json::json!({
            "completed": total,
            "total": total,
            "current_node_id": null,
            "current_node_name": null,
        }),
    );

    // Log bulk operation to activity log
    let succeeded = results.iter().filter(|r| r.success).count();
    let failed = results.iter().filter(|r| !r.success).count();
    let mut log = state.activity_log.lock().await;
    log.log(
        "bulk_blueprint_applied",
        &blueprint.name,
        Some(&format!(
            "Applied to {} nodes: {} succeeded, {} failed",
            total, succeeded, failed
        )),
        failed == 0,
    );

    // Send desktop notification
    crate::notifications::notify_bulk_complete(&app, &blueprint.name, succeeded, failed);

    Ok(results)
}

#[tauri::command]
pub async fn import_blueprint_package(
    path: String,
    state: State<'_, AppState>,
) -> Result<Blueprint, AppError> {
    use crate::blueprints::package;

    let manifest = package::import_blueprint(std::path::Path::new(&path))?;

    // Convert the imported definition to a Blueprint and register it
    let mut manager = state.blueprint_manager.lock().await;
    let blueprint = manager.import_from_definition(manifest.definition)?;
    drop(manager);

    let mut log = state.activity_log.lock().await;
    log.log(
        "blueprint_imported",
        &blueprint.name,
        Some(&format!("Imported from {}", path)),
        true,
    );

    Ok(blueprint)
}

#[tauri::command]
pub async fn export_blueprint_package(
    blueprint_id: String,
    output_dir: String,
    state: State<'_, AppState>,
) -> Result<String, AppError> {
    use crate::blueprints::package;

    // Verify blueprint exists
    let manager = state.blueprint_manager.lock().await;
    let bp = manager
        .get_blueprint(&blueprint_id)
        .ok_or_else(|| AppError::Blueprint(format!("Blueprint not found: {}", blueprint_id)))?;
    let bp_name = bp.name.clone();
    drop(manager);

    let blueprints_dir = crate::dirs::config_dir().join("blueprints");
    let output_path = package::export_blueprint(
        &blueprint_id,
        &blueprints_dir,
        std::path::Path::new(&output_dir),
    )?;

    let mut log = state.activity_log.lock().await;
    log.log(
        "blueprint_exported",
        &bp_name,
        Some(&format!("Exported to {}", output_path.display())),
        true,
    );

    Ok(output_path.to_string_lossy().to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bump_patch_version() {
        assert_eq!(bump_version("1.0.0", "patch"), "1.0.1");
        assert_eq!(bump_version("1.2.3", "patch"), "1.2.4");
        assert_eq!(bump_version("0.0.0", "patch"), "0.0.1");
    }

    #[test]
    fn bump_minor_version() {
        assert_eq!(bump_version("1.0.0", "minor"), "1.1.0");
        assert_eq!(bump_version("1.2.3", "minor"), "1.3.0");
        assert_eq!(bump_version("0.0.0", "minor"), "0.1.0");
    }

    #[test]
    fn bump_major_version() {
        assert_eq!(bump_version("1.0.0", "major"), "2.0.0");
        assert_eq!(bump_version("1.2.3", "major"), "2.0.0");
        assert_eq!(bump_version("0.0.0", "major"), "1.0.0");
    }

    #[test]
    fn bump_version_handles_malformed_input() {
        assert_eq!(bump_version("", "patch"), "0.0.1");
        assert_eq!(bump_version("1", "minor"), "1.1.0");
        assert_eq!(bump_version("abc", "major"), "1.0.0");
    }
}
