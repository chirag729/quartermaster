use std::path::PathBuf;

use chrono::Utc;
use uuid::Uuid;

use crate::error::AppError;
use super::{Node, NodeKind, NodeStatus};

pub struct FleetManager {
    nodes: Vec<Node>,
    config_dir: PathBuf,
}

impl FleetManager {
    /// Creates a new FleetManager, loading existing nodes from disk.
    /// If no local node exists, auto-creates one representing this machine.
    /// Migrates from legacy ~/.config/machine-setup/nodes/ if the new path is empty.
    pub fn load() -> Result<Self, AppError> {
        let base_dir = dirs::config_dir()
            .unwrap_or_else(|| PathBuf::from("~/.config"));
        let config_dir = base_dir.join("quartermaster").join("nodes");

        std::fs::create_dir_all(&config_dir)?;

        let mut manager = Self {
            nodes: Vec::new(),
            config_dir: config_dir.clone(),
        };

        manager.load_nodes_from_disk()?;

        // Migrate from legacy path if new directory is empty
        if manager.nodes.is_empty() {
            let legacy_dir = base_dir.join("machine-setup").join("nodes");
            if legacy_dir.exists() {
                if let Ok(entries) = std::fs::read_dir(&legacy_dir) {
                    for entry in entries.flatten() {
                        let src = entry.path();
                        if src.extension().and_then(|e| e.to_str()) == Some("json") {
                            let dest = config_dir.join(entry.file_name());
                            if !dest.exists() {
                                let _ = std::fs::copy(&src, &dest);
                            }
                        }
                    }
                }
                manager.load_nodes_from_disk()?;
            }
        }

        // Auto-create local node if none exists
        let has_local = manager.nodes.iter().any(|n| n.kind == NodeKind::Local);
        if !has_local {
            let local_node = Self::detect_local_node();
            manager.add_node(local_node)?;
        }

        Ok(manager)
    }

    /// Loads all node JSON files from the config directory.
    fn load_nodes_from_disk(&mut self) -> Result<(), AppError> {
        self.nodes.clear();

        let entries = std::fs::read_dir(&self.config_dir)?;
        for entry in entries {
            let entry = entry?;
            let path = entry.path();
            if path.extension().and_then(|e| e.to_str()) == Some("json") {
                let content = std::fs::read_to_string(&path)?;
                match serde_json::from_str::<Node>(&content) {
                    Ok(node) => self.nodes.push(node),
                    Err(e) => {
                        eprintln!(
                            "Warning: failed to parse node file {}: {}",
                            path.display(),
                            e
                        );
                    }
                }
            }
        }

        Ok(())
    }

    /// Persists a single node to disk as a JSON file.
    pub fn save_node(&self, node: &Node) -> Result<(), AppError> {
        let path = self.config_dir.join(format!("{}.json", node.id));
        let content = serde_json::to_string_pretty(node)?;
        std::fs::write(&path, content)?;
        Ok(())
    }

    /// Deletes a node's JSON file from disk.
    pub fn delete_node(&self, id: &str) -> Result<(), AppError> {
        let path = self.config_dir.join(format!("{}.json", id));
        if path.exists() {
            std::fs::remove_file(&path)?;
        }
        Ok(())
    }

    /// Adds a node to the in-memory list and saves it to disk.
    pub fn add_node(&mut self, node: Node) -> Result<(), AppError> {
        self.save_node(&node)?;
        self.nodes.push(node);
        Ok(())
    }

    /// Updates an existing node in the list and on disk.
    pub fn update_node(&mut self, node: Node) -> Result<(), AppError> {
        self.save_node(&node)?;
        if let Some(existing) = self.nodes.iter_mut().find(|n| n.id == node.id) {
            *existing = node;
        } else {
            return Err(AppError::Fleet(format!("Node not found: {}", node.id)));
        }
        Ok(())
    }

    /// Removes a node from the in-memory list and deletes it from disk.
    pub fn remove_node(&mut self, id: &str) -> Result<(), AppError> {
        let idx = self
            .nodes
            .iter()
            .position(|n| n.id == id)
            .ok_or_else(|| AppError::Fleet(format!("Node not found: {}", id)))?;
        self.nodes.remove(idx);
        self.delete_node(id)?;
        Ok(())
    }

    /// Returns a reference to a node by ID.
    pub fn get_node(&self, id: &str) -> Option<&Node> {
        self.nodes.iter().find(|n| n.id == id)
    }

    /// Returns a mutable reference to a node by ID.
    pub fn get_node_mut(&mut self, id: &str) -> Option<&mut Node> {
        self.nodes.iter_mut().find(|n| n.id == id)
    }

    /// Returns a slice of all nodes.
    pub fn list_nodes(&self) -> &[Node] {
        &self.nodes
    }

    /// Persists all nodes to disk.
    pub fn save(&self) -> Result<(), AppError> {
        for node in &self.nodes {
            self.save_node(node)?;
        }
        Ok(())
    }

    /// Auto-detects the local machine as a node.
    pub fn detect_local_node() -> Node {
        let hostname = hostname::get()
            .map(|h| h.to_string_lossy().to_string())
            .unwrap_or_else(|_| "localhost".to_string());

        let username = std::env::var("USER")
            .or_else(|_| std::env::var("USERNAME"))
            .unwrap_or_else(|_| "unknown".to_string());

        let os_info = format!("{} {}", std::env::consts::OS, std::env::consts::ARCH);

        Node {
            id: Uuid::new_v4().to_string(),
            name: format!("{}@{} (local)", username, hostname),
            kind: NodeKind::Local,
            hostname,
            os: Some(os_info),
            tags: vec!["local".to_string()],
            ssh_config: None,
            status: NodeStatus::Online,
            last_seen: Some(Utc::now()),
            blueprint_id: None,
            applied_blueprint_version: None,
            created_at: Utc::now(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    fn test_manager(dir: &std::path::Path) -> FleetManager {
        FleetManager {
            nodes: Vec::new(),
            config_dir: dir.to_path_buf(),
        }
    }

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
    fn add_and_list_nodes() {
        let dir = tempdir().unwrap();
        let mut mgr = test_manager(dir.path());

        let node = make_node("test-node", NodeKind::Remote);
        let id = node.id.clone();
        mgr.add_node(node).unwrap();

        assert_eq!(mgr.list_nodes().len(), 1);
        assert_eq!(mgr.get_node(&id).unwrap().name, "test-node");
    }

    #[test]
    fn update_node_changes_data() {
        let dir = tempdir().unwrap();
        let mut mgr = test_manager(dir.path());

        let mut node = make_node("original", NodeKind::Remote);
        let id = node.id.clone();
        mgr.add_node(node.clone()).unwrap();

        node.name = "updated".to_string();
        mgr.update_node(node).unwrap();

        assert_eq!(mgr.get_node(&id).unwrap().name, "updated");
    }

    #[test]
    fn remove_node_deletes_from_list_and_disk() {
        let dir = tempdir().unwrap();
        let mut mgr = test_manager(dir.path());

        let node = make_node("to-remove", NodeKind::Remote);
        let id = node.id.clone();
        mgr.add_node(node).unwrap();

        assert!(dir.path().join(format!("{}.json", id)).exists());

        mgr.remove_node(&id).unwrap();
        assert_eq!(mgr.list_nodes().len(), 0);
        assert!(!dir.path().join(format!("{}.json", id)).exists());
    }

    #[test]
    fn save_and_reload_from_disk() {
        let dir = tempdir().unwrap();
        let mut mgr = test_manager(dir.path());

        let node = make_node("persistent", NodeKind::Remote);
        let id = node.id.clone();
        mgr.add_node(node).unwrap();

        // Reload from disk
        let mut mgr2 = test_manager(dir.path());
        mgr2.load_nodes_from_disk().unwrap();

        assert_eq!(mgr2.list_nodes().len(), 1);
        assert_eq!(mgr2.get_node(&id).unwrap().name, "persistent");
    }

    #[test]
    fn detect_local_node_has_correct_kind() {
        let node = FleetManager::detect_local_node();
        assert_eq!(node.kind, NodeKind::Local);
        assert_eq!(node.status, NodeStatus::Online);
        assert!(!node.hostname.is_empty());
    }

    #[test]
    fn update_nonexistent_node_returns_error() {
        let dir = tempdir().unwrap();
        let mut mgr = test_manager(dir.path());

        let node = make_node("ghost", NodeKind::Remote);
        let result = mgr.update_node(node);
        assert!(result.is_err());
    }

    #[test]
    fn remove_nonexistent_node_returns_error() {
        let dir = tempdir().unwrap();
        let mut mgr = test_manager(dir.path());

        let result = mgr.remove_node("nonexistent-id");
        assert!(result.is_err());
    }
}
