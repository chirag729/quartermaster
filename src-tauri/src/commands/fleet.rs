use tauri::State;
use chrono::Utc;
use uuid::Uuid;

use crate::error::AppError;
use crate::fleet::{Node, NodeKind, NodeStatus, SshConfig};
use crate::state::AppState;

#[tauri::command]
pub async fn list_nodes(state: State<'_, AppState>) -> Result<Vec<Node>, AppError> {
    let manager = state.fleet_manager.lock().await;
    Ok(manager.list_nodes().to_vec())
}

#[tauri::command]
pub async fn get_node(node_id: String, state: State<'_, AppState>) -> Result<Node, AppError> {
    let manager = state.fleet_manager.lock().await;
    manager
        .get_node(&node_id)
        .cloned()
        .ok_or_else(|| AppError::Fleet(format!("Node not found: {}", node_id)))
}

#[tauri::command]
pub async fn add_node(
    name: String,
    hostname: String,
    kind: NodeKind,
    tags: Vec<String>,
    ssh_config: Option<SshConfig>,
    state: State<'_, AppState>,
) -> Result<Node, AppError> {
    let node = Node {
        id: Uuid::new_v4().to_string(),
        name,
        kind: kind.clone(),
        hostname,
        os: None,
        tags,
        ssh_config,
        status: if kind == NodeKind::Local {
            NodeStatus::Online
        } else {
            NodeStatus::Unknown
        },
        last_seen: None,
        blueprint_id: None,
        created_at: Utc::now(),
    };

    let mut manager = state.fleet_manager.lock().await;
    manager.add_node(node.clone())?;

    Ok(node)
}

#[tauri::command]
pub async fn update_node(node: Node, state: State<'_, AppState>) -> Result<Node, AppError> {
    let mut manager = state.fleet_manager.lock().await;
    manager.update_node(node.clone())?;
    Ok(node)
}

#[tauri::command]
pub async fn remove_node(node_id: String, state: State<'_, AppState>) -> Result<(), AppError> {
    let mut manager = state.fleet_manager.lock().await;
    manager.remove_node(&node_id)?;
    Ok(())
}

#[tauri::command]
pub async fn get_node_status(
    node_id: String,
    state: State<'_, AppState>,
) -> Result<NodeStatus, AppError> {
    let manager = state.fleet_manager.lock().await;
    let node = manager
        .get_node(&node_id)
        .ok_or_else(|| AppError::Fleet(format!("Node not found: {}", node_id)))?;
    Ok(node.status.clone())
}
