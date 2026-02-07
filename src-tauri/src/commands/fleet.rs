use tauri::State;
use chrono::Utc;
use uuid::Uuid;

use crate::error::AppError;
use crate::fleet::{Node, NodeKind, NodeStatus, SshConfig, SshAuthMethod};
use crate::fleet::ssh_config::{SshHostEntry, discover_ssh_hosts};
use crate::fleet::status_poller::poll_node_statuses;
use crate::fleet::terminal;
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
        applied_blueprint_version: None,
        created_at: Utc::now(),
    };

    let mut manager = state.fleet_manager.lock().await;

    // Enforce: only one local node is allowed
    if kind == NodeKind::Local {
        let has_local = manager.list_nodes().iter().any(|n| n.kind == NodeKind::Local);
        if has_local {
            return Err(AppError::Fleet(
                "A local node already exists. Only one local node is allowed.".to_string(),
            ));
        }
    }

    manager.add_node(node.clone())?;
    drop(manager);

    let mut log = state.activity_log.lock().await;
    log.log(
        "node_added",
        &node.name,
        Some(&format!("Added {:?} node '{}'", node.kind, node.hostname)),
        true,
    );

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
    let node_name = manager
        .get_node(&node_id)
        .map(|n| n.name.clone())
        .unwrap_or_else(|| node_id.clone());
    manager.remove_node(&node_id)?;
    drop(manager);

    let mut log = state.activity_log.lock().await;
    log.log("node_removed", &node_name, None, true);

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

/// Probes all remote nodes for SSH connectivity and updates their status.
///
/// Returns a map of `node_id -> is_online`. Local nodes are always reported
/// as online and are not probed.
#[tauri::command]
pub async fn poll_all_node_statuses(
    state: State<'_, AppState>,
) -> Result<Vec<(String, bool)>, AppError> {
    let manager = state.fleet_manager.lock().await;
    let nodes: Vec<Node> = manager.list_nodes().to_vec();

    let mut probe_input: Vec<(String, String, u16)> = Vec::new();
    let mut results: Vec<(String, bool)> = Vec::new();

    for node in &nodes {
        match node.kind {
            NodeKind::Local => {
                results.push((node.id.clone(), true));
            }
            NodeKind::Remote => {
                if let Some(ref ssh) = node.ssh_config {
                    probe_input.push((node.id.clone(), ssh.host.clone(), ssh.port));
                } else {
                    results.push((node.id.clone(), false));
                }
            }
        }
    }

    // Drop the lock before the async network probe
    let remote_results = {
        drop(manager);
        poll_node_statuses(probe_input).await
    };

    // Update node statuses in the fleet manager
    let mut manager = state.fleet_manager.lock().await;
    for (node_id, online) in &remote_results {
        if let Some(node) = manager.get_node_mut(node_id) {
            node.status = if *online {
                NodeStatus::Online
            } else {
                NodeStatus::Offline
            };
            if *online {
                node.last_seen = Some(Utc::now());
            }
        }
    }
    manager.save()?;

    results.extend(remote_results);
    Ok(results)
}

/// Reads `~/.ssh/config` and returns discovered host entries for import.
#[tauri::command]
pub async fn discover_ssh_hosts_cmd() -> Result<Vec<SshHostEntry>, AppError> {
    Ok(discover_ssh_hosts())
}

/// Opens a terminal emulator with an SSH session to a fleet node.
#[tauri::command]
pub async fn open_node_terminal(
    node_id: String,
    state: State<'_, AppState>,
) -> Result<(), AppError> {
    let manager = state.fleet_manager.lock().await;
    let node = manager
        .get_node(&node_id)
        .ok_or_else(|| AppError::Fleet(format!("Node not found: {}", node_id)))?;

    match node.kind {
        NodeKind::Local => {
            terminal::open_local_terminal()?;
        }
        NodeKind::Remote => {
            let ssh = node.ssh_config.as_ref().ok_or_else(|| {
                AppError::Fleet(format!("Node '{}' has no SSH configuration", node.name))
            })?;

            let identity_file = match &ssh.auth_method {
                SshAuthMethod::KeyFile { private_key_path } => Some(private_key_path.as_str()),
                SshAuthMethod::Certificate { private_key_path, .. } => {
                    Some(private_key_path.as_str())
                }
                _ => None,
            };

            terminal::open_terminal_with_ssh(
                &ssh.host,
                ssh.port,
                &ssh.username,
                identity_file,
            )?;
        }
    }

    Ok(())
}
