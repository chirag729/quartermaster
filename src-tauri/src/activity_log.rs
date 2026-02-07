use std::collections::VecDeque;
use std::path::{Path, PathBuf};

use chrono::Utc;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::error::AppError;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ActivityEntry {
    pub id: String,
    pub timestamp: String,
    pub action: String,
    pub target: String,
    pub detail: Option<String>,
    pub success: bool,
}

pub struct ActivityLog {
    entries: VecDeque<ActivityEntry>,
    max_entries: usize,
    path: PathBuf,
}

impl ActivityLog {
    pub fn new(path: PathBuf, max_entries: usize) -> Self {
        let entries = Self::load(&path);
        let mut deque = VecDeque::from(entries);
        // Trim if the loaded data exceeds the current limit
        while deque.len() > max_entries {
            deque.pop_front();
        }
        Self {
            entries: deque,
            max_entries,
            path,
        }
    }

    pub fn log(
        &mut self,
        action: &str,
        target: &str,
        detail: Option<&str>,
        success: bool,
    ) {
        let entry = ActivityEntry {
            id: Uuid::new_v4().to_string(),
            timestamp: Utc::now().to_rfc3339(),
            action: action.to_string(),
            target: target.to_string(),
            detail: detail.map(|s| s.to_string()),
            success,
        };

        if self.entries.len() >= self.max_entries {
            self.entries.pop_front();
        }
        self.entries.push_back(entry);

        // Best-effort save; callers should not fail on log persistence errors
        let _ = self.save();
    }

    pub fn entries(&self) -> &VecDeque<ActivityEntry> {
        &self.entries
    }

    pub fn recent(&self, count: usize) -> Vec<&ActivityEntry> {
        self.entries.iter().rev().take(count).collect()
    }

    pub fn clear(&mut self) {
        self.entries.clear();
        let _ = self.save();
    }

    pub fn save(&self) -> Result<(), AppError> {
        if let Some(parent) = self.path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let entries_vec: Vec<&ActivityEntry> = self.entries.iter().collect();
        let content = serde_json::to_string_pretty(&entries_vec)?;
        std::fs::write(&self.path, content)?;
        Ok(())
    }

    pub fn load(path: &Path) -> Vec<ActivityEntry> {
        if !path.exists() {
            return Vec::new();
        }
        match std::fs::read_to_string(path) {
            Ok(content) => serde_json::from_str(&content).unwrap_or_default(),
            Err(_) => Vec::new(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn add_entries_and_read_back() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("activity.json");
        let mut log = ActivityLog::new(path, 100);

        log.log("task_executed", "flutter-sdk", Some("Installed Flutter"), true);
        log.log("node_added", "my-server", None, true);

        assert_eq!(log.entries().len(), 2);

        let first = &log.entries()[0];
        assert_eq!(first.action, "task_executed");
        assert_eq!(first.target, "flutter-sdk");
        assert_eq!(first.detail.as_deref(), Some("Installed Flutter"));
        assert!(first.success);

        let second = &log.entries()[1];
        assert_eq!(second.action, "node_added");
        assert_eq!(second.target, "my-server");
        assert!(second.detail.is_none());
        assert!(second.success);
    }

    #[test]
    fn ring_buffer_evicts_oldest() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("activity.json");
        let mut log = ActivityLog::new(path, 3);

        log.log("action_1", "target_1", None, true);
        log.log("action_2", "target_2", None, true);
        log.log("action_3", "target_3", None, true);
        assert_eq!(log.entries().len(), 3);

        // Adding a 4th entry should evict the first
        log.log("action_4", "target_4", None, true);
        assert_eq!(log.entries().len(), 3);
        assert_eq!(log.entries()[0].action, "action_2");
        assert_eq!(log.entries()[2].action, "action_4");
    }

    #[test]
    fn save_and_load_round_trip() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("activity.json");

        {
            let mut log = ActivityLog::new(path.clone(), 100);
            log.log("task_executed", "flutter-sdk", Some("v3.19"), true);
            log.log("node_removed", "old-server", None, false);
        }

        // Load from disk into a new instance
        let log = ActivityLog::new(path, 100);
        assert_eq!(log.entries().len(), 2);
        assert_eq!(log.entries()[0].action, "task_executed");
        assert_eq!(log.entries()[0].target, "flutter-sdk");
        assert!(!log.entries()[1].success);
    }

    #[test]
    fn clear_removes_all_entries_and_persists() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("activity.json");

        let mut log = ActivityLog::new(path.clone(), 100);
        log.log("action_1", "target_1", None, true);
        log.log("action_2", "target_2", None, true);
        assert_eq!(log.entries().len(), 2);

        log.clear();
        assert_eq!(log.entries().len(), 0);

        // Verify cleared state persists
        let log2 = ActivityLog::new(path, 100);
        assert_eq!(log2.entries().len(), 0);
    }

    #[test]
    fn recent_returns_newest_first() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("activity.json");
        let mut log = ActivityLog::new(path, 100);

        log.log("action_1", "target_1", None, true);
        log.log("action_2", "target_2", None, true);
        log.log("action_3", "target_3", None, true);

        let recent = log.recent(2);
        assert_eq!(recent.len(), 2);
        assert_eq!(recent[0].action, "action_3");
        assert_eq!(recent[1].action, "action_2");
    }

    #[test]
    fn recent_with_count_larger_than_entries() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("activity.json");
        let mut log = ActivityLog::new(path, 100);

        log.log("action_1", "target_1", None, true);

        let recent = log.recent(10);
        assert_eq!(recent.len(), 1);
    }

    #[test]
    fn load_nonexistent_file_returns_empty() {
        let entries = ActivityLog::load(Path::new("/tmp/nonexistent_activity_log_test.json"));
        assert!(entries.is_empty());
    }

    #[test]
    fn load_corrupted_file_returns_empty() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("activity.json");
        std::fs::write(&path, "not valid json!!!").unwrap();

        let entries = ActivityLog::load(&path);
        assert!(entries.is_empty());
    }

    #[test]
    fn entries_have_unique_ids() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("activity.json");
        let mut log = ActivityLog::new(path, 100);

        log.log("action_1", "target_1", None, true);
        log.log("action_2", "target_2", None, true);

        let id1 = &log.entries()[0].id;
        let id2 = &log.entries()[1].id;
        assert_ne!(id1, id2);
    }

    #[test]
    fn load_trims_to_max_entries() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("activity.json");

        // Write 5 entries with a limit of 100
        {
            let mut log = ActivityLog::new(path.clone(), 100);
            for i in 0..5 {
                log.log(&format!("action_{}", i), "target", None, true);
            }
        }

        // Reload with a smaller limit
        let log = ActivityLog::new(path, 3);
        assert_eq!(log.entries().len(), 3);
        // Should keep the 3 newest (action_2, action_3, action_4)
        assert_eq!(log.entries()[0].action, "action_2");
        assert_eq!(log.entries()[2].action, "action_4");
    }
}
