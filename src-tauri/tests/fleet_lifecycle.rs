use chrono::Utc;
use uuid::Uuid;

use quartermaster_lib::fleet::manager::FleetManager;
use quartermaster_lib::fleet::{Node, NodeKind, NodeStatus};

fn make_node(name: &str, kind: NodeKind) -> Node {
    Node {
        id: Uuid::new_v4().to_string(),
        name: name.to_string(),
        kind,
        hostname: "test-host".to_string(),
        os: Some("linux x86_64".to_string()),
        tags: vec![],
        ssh_config: None,
        status: NodeStatus::Unknown,
        last_seen: None,
        blueprint_id: None,
        applied_blueprint_version: None,
        created_at: Utc::now(),
    }
}

#[test]
fn fleet_crud_lifecycle() {
    let dir = tempfile::tempdir().unwrap();
    let mut mgr = FleetManager::with_dir(dir.path().to_path_buf());

    // Add
    let node = make_node("test-node", NodeKind::Remote);
    let id = node.id.clone();
    mgr.add_node(node).unwrap();
    assert_eq!(mgr.list_nodes().len(), 1);

    // Get
    let retrieved = mgr.get_node(&id).unwrap();
    assert_eq!(retrieved.name, "test-node");

    // Update
    let mut updated = mgr.get_node(&id).unwrap().clone();
    updated.name = "updated-node".to_string();
    mgr.update_node(updated).unwrap();
    assert_eq!(mgr.get_node(&id).unwrap().name, "updated-node");

    // Remove
    mgr.remove_node(&id).unwrap();
    assert!(mgr.get_node(&id).is_none());
    assert_eq!(mgr.list_nodes().len(), 0);
}

#[test]
fn fleet_persist_and_reload() {
    let dir = tempfile::tempdir().unwrap();
    let mut mgr = FleetManager::with_dir(dir.path().to_path_buf());

    let node1 = make_node("node-1", NodeKind::Remote);
    let node2 = make_node("node-2", NodeKind::Remote);
    let node3 = make_node("node-3", NodeKind::Remote);
    let id1 = node1.id.clone();
    let id2 = node2.id.clone();
    let id3 = node3.id.clone();

    mgr.add_node(node1).unwrap();
    mgr.add_node(node2).unwrap();
    mgr.add_node(node3).unwrap();

    // Verify the files exist on disk
    assert!(dir.path().join(format!("{}.json", id1)).exists());
    assert!(dir.path().join(format!("{}.json", id2)).exists());
    assert!(dir.path().join(format!("{}.json", id3)).exists());

    // Read the node files directly to verify persistence
    let content = std::fs::read_to_string(dir.path().join(format!("{}.json", id1))).unwrap();
    let loaded_node: Node = serde_json::from_str(&content).unwrap();
    assert_eq!(loaded_node.name, "node-1");
}

#[test]
fn fleet_remove_nonexistent_returns_error() {
    let dir = tempfile::tempdir().unwrap();
    let mut mgr = FleetManager::with_dir(dir.path().to_path_buf());

    let result = mgr.remove_node("bogus-id");
    assert!(result.is_err());
}

#[test]
fn fleet_update_nonexistent_returns_error() {
    let dir = tempfile::tempdir().unwrap();
    let mut mgr = FleetManager::with_dir(dir.path().to_path_buf());

    let ghost = make_node("ghost", NodeKind::Remote);
    let result = mgr.update_node(ghost);
    assert!(result.is_err());
}

#[test]
fn fleet_detect_local_node_structure() {
    let node = FleetManager::detect_local_node();
    assert_eq!(node.kind, NodeKind::Local);
    assert_eq!(node.status, NodeStatus::Online);
    assert!(!node.hostname.is_empty());
    assert!(!node.name.is_empty());
    assert!(node.os.is_some());
    assert!(node.tags.contains(&"local".to_string()));
}
