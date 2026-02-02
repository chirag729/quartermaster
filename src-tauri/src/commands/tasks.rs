use tauri::{AppHandle, Emitter, State};

use crate::error::AppError;
use crate::executor::CommandExecutor;
use crate::executor::local::LocalExecutor;
use crate::executor::ssh::SshExecutor;
use crate::fleet::NodeKind;
use crate::tasks::TaskInfo;
use crate::state::AppState;

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

#[tauri::command]
pub async fn execute_task(
    task_id: String,
    node_id: Option<String>,
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
                Box::new(SshExecutor::new(
                    ssh_config.host.clone(),
                    ssh_config.port,
                    ssh_config.username.clone(),
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

    task.execute(&task_config, exec.as_ref(), &progress_cb).await?;

    let _ = app.emit("task-state-changed", serde_json::json!({
        "task_id": task_id,
        "status": "completed",
    }));

    let mut config = state.config.lock().await;
    if !config.data.completed_tasks.contains(&task_id) {
        config.data.completed_tasks.push(task_id);
    }
    config.save()?;

    Ok(())
}
