use std::collections::HashMap;

use tauri::{AppHandle, Emitter, State};

use crate::blueprints::{Blueprint, BlueprintTaskEntry};
use crate::error::AppError;
use crate::executor::CommandExecutor;
use crate::executor::local::LocalExecutor;
use crate::executor::ssh::SshExecutor;
use crate::fleet::NodeKind;
use crate::state::AppState;
use crate::tasks::TaskStatus;

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
    manager.create_blueprint(name, description, icon, task_entries)
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
    let resolved = state.registry.resolve_dependencies(&task_ids);

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

    let mut manager = state.blueprint_manager.lock().await;
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
    state: State<'_, AppState>,
) -> Result<Blueprint, AppError> {
    let mut manager = state.blueprint_manager.lock().await;
    manager.create_blueprint(name, description, "Layers".to_string(), vec![])
}

#[tauri::command]
pub async fn delete_blueprint(
    id: String,
    state: State<'_, AppState>,
) -> Result<(), AppError> {
    let bp_manager = state.blueprint_manager.lock().await;

    // Check if builtin
    let bp = bp_manager
        .get_blueprint(&id)
        .ok_or_else(|| AppError::Blueprint(format!("Blueprint not found: {}", id)))?;
    if bp.is_builtin {
        return Err(AppError::Blueprint("Cannot delete a built-in blueprint".to_string()));
    }
    drop(bp_manager);

    // Unassign from any nodes that reference this blueprint
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
    drop(fleet_manager);

    let mut bp_manager = state.blueprint_manager.lock().await;
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
    bp_manager
        .get_blueprint(&blueprint_id)
        .ok_or_else(|| AppError::Blueprint(format!("Blueprint not found: {}", blueprint_id)))?;
    drop(bp_manager);

    // Validate node exists and update it
    let mut fleet_manager = state.fleet_manager.lock().await;
    let node = fleet_manager
        .get_node(&node_id)
        .cloned()
        .ok_or_else(|| AppError::Fleet(format!("Node not found: {}", node_id)))?;
    let mut updated_node = node;
    updated_node.blueprint_id = Some(blueprint_id);
    fleet_manager.update_node(updated_node)?;
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
    let mut updated_node = node;
    updated_node.blueprint_id = None;
    fleet_manager.update_node(updated_node)?;
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
    drop(bp_manager);

    // Verify the node exists and get its kind
    let fleet_manager = state.fleet_manager.lock().await;
    let node = fleet_manager
        .get_node(&node_id)
        .cloned()
        .ok_or_else(|| AppError::Fleet(format!("Node not found: {}", node_id)))?;
    drop(fleet_manager);

    // Get enabled entries sorted by order
    let mut entries: Vec<&BlueprintTaskEntry> = blueprint
        .task_entries
        .iter()
        .filter(|e| e.enabled)
        .collect();
    entries.sort_by_key(|e| e.order);

    let total = entries.len();

    // Choose executor based on node kind
    let exec: Box<dyn CommandExecutor> = match node.kind {
        NodeKind::Local => Box::new(LocalExecutor::new()),
        NodeKind::Remote => {
            let ssh_config = node.ssh_config.as_ref().ok_or_else(|| {
                AppError::Ssh(format!("Remote node '{}' has no SSH configuration", node.name))
            })?;
            Box::new(SshExecutor::new(
                ssh_config.host.clone(),
                ssh_config.port,
                ssh_config.username.clone(),
            ))
        }
    };

    for (i, entry) in entries.iter().enumerate() {
        let task = match state.registry.get(&entry.task_id) {
            Some(t) => t,
            None => continue,
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
        let status = task.detect_state(&task_config, exec.as_ref()).await;
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

        task.execute(&task_config, exec.as_ref(), &progress_cb).await?;

        let _ = app.emit(
            "task-state-changed",
            serde_json::json!({
                "node_id": node_id,
                "task_id": entry.task_id,
                "status": "completed",
            }),
        );
    }

    // Emit completion
    let _ = app.emit(
        "blueprint-apply-complete",
        serde_json::json!({
            "node_id": node_id,
            "blueprint_id": blueprint_id,
        }),
    );

    Ok(())
}
