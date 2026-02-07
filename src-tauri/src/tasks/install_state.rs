//! Installation state tracking for tasks.
//!
//! Records what was installed, with which config, so we can detect:
//! - Whether a task is already installed for a given node
//! - Whether the config has drifted since installation
//! - Whether a version upgrade is available

use std::collections::HashMap;

use sha2::{Digest, Sha256};
use serde_json::Value;

use crate::config::manager::InstalledTaskState;

/// Computes a deterministic hash of a task's configuration.
///
/// The hash is based on sorted key-value pairs, ensuring the same config
/// always produces the same hash regardless of insertion order.
pub fn compute_config_hash(config: &HashMap<String, Value>) -> String {
    let mut hasher = Sha256::new();

    // Sort keys for deterministic hashing
    let mut keys: Vec<&String> = config.keys().collect();
    keys.sort();

    for key in keys {
        hasher.update(key.as_bytes());
        hasher.update(b"=");
        let value_str = match &config[key] {
            Value::String(s) => s.clone(),
            other => other.to_string(),
        };
        hasher.update(value_str.as_bytes());
        hasher.update(b"\n");
    }

    hex::encode(hasher.finalize())
}

/// Checks whether the task's config has drifted since installation.
///
/// Returns `true` if the current config hash differs from the stored one,
/// meaning the user changed settings after the task was installed.
pub fn has_config_drifted(
    state: &InstalledTaskState,
    current_config: &HashMap<String, Value>,
) -> bool {
    let current_hash = compute_config_hash(current_config);
    state.config_hash != current_hash
}

/// Checks whether a version upgrade is available.
///
/// Returns `true` if the task defines a newer version than what was installed.
pub fn has_version_changed(
    state: &InstalledTaskState,
    current_version: Option<&str>,
) -> bool {
    match (&state.version, current_version) {
        (Some(installed), Some(current)) => installed != current,
        (None, Some(_)) => true,  // version added after install
        (Some(_), None) => false,  // version removed — treat as no change
        (None, None) => false,
    }
}

/// Creates a new `InstalledTaskState` snapshot for the current installation.
pub fn record_installation(
    task_id: &str,
    node_id: &str,
    version: Option<&str>,
    blueprint_id: Option<&str>,
    config: &HashMap<String, Value>,
) -> InstalledTaskState {
    InstalledTaskState {
        task_id: task_id.to_string(),
        node_id: node_id.to_string(),
        version: version.map(|v| v.to_string()),
        blueprint_id: blueprint_id.map(|b| b.to_string()),
        config_hash: compute_config_hash(config),
        installed_at: chrono::Utc::now().to_rfc3339(),
    }
}

/// Determines the composite key for storing installation state.
///
/// Format: `{task_id}@{node_id}` — each task can be installed on
/// multiple nodes independently.
pub fn state_key(task_id: &str, node_id: &str) -> String {
    format!("{}@{}", task_id, node_id)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn config_hash_is_deterministic() {
        let mut config = HashMap::new();
        config.insert("b".to_string(), Value::String("two".to_string()));
        config.insert("a".to_string(), Value::String("one".to_string()));

        let hash1 = compute_config_hash(&config);

        // Same data, inserted in different order
        let mut config2 = HashMap::new();
        config2.insert("a".to_string(), Value::String("one".to_string()));
        config2.insert("b".to_string(), Value::String("two".to_string()));

        let hash2 = compute_config_hash(&config2);
        assert_eq!(hash1, hash2);
    }

    #[test]
    fn config_hash_differs_for_different_values() {
        let mut config1 = HashMap::new();
        config1.insert("key".to_string(), Value::String("value1".to_string()));

        let mut config2 = HashMap::new();
        config2.insert("key".to_string(), Value::String("value2".to_string()));

        assert_ne!(compute_config_hash(&config1), compute_config_hash(&config2));
    }

    #[test]
    fn config_hash_empty_config() {
        let config = HashMap::new();
        let hash = compute_config_hash(&config);
        // Should be the hash of empty input
        assert!(!hash.is_empty());
        assert_eq!(hash.len(), 64); // SHA-256 hex length
    }

    #[test]
    fn drift_detected_when_config_changes() {
        let mut original = HashMap::new();
        original.insert("path".to_string(), Value::String("~/dev".to_string()));

        let state = record_installation("test", "local", None, None, &original);

        let mut modified = original.clone();
        modified.insert("path".to_string(), Value::String("~/projects".to_string()));

        assert!(has_config_drifted(&state, &modified));
    }

    #[test]
    fn no_drift_when_config_unchanged() {
        let mut config = HashMap::new();
        config.insert("path".to_string(), Value::String("~/dev".to_string()));

        let state = record_installation("test", "local", None, None, &config);
        assert!(!has_config_drifted(&state, &config));
    }

    #[test]
    fn version_changed_detects_upgrade() {
        let state = InstalledTaskState {
            task_id: "test".to_string(),
            node_id: "local".to_string(),
            version: Some("1.0".to_string()),
            blueprint_id: None,
            config_hash: String::new(),
            installed_at: String::new(),
        };

        assert!(has_version_changed(&state, Some("2.0")));
        assert!(!has_version_changed(&state, Some("1.0")));
    }

    #[test]
    fn version_changed_new_version_added() {
        let state = InstalledTaskState {
            task_id: "test".to_string(),
            node_id: "local".to_string(),
            version: None,
            blueprint_id: None,
            config_hash: String::new(),
            installed_at: String::new(),
        };

        assert!(has_version_changed(&state, Some("1.0")));
        assert!(!has_version_changed(&state, None));
    }

    #[test]
    fn record_installation_captures_all_fields() {
        let mut config = HashMap::new();
        config.insert("key".to_string(), Value::String("val".to_string()));

        let state = record_installation(
            "flutter-sdk",
            "node-1",
            Some("3.24.0"),
            Some("dev-workstation"),
            &config,
        );

        assert_eq!(state.task_id, "flutter-sdk");
        assert_eq!(state.node_id, "node-1");
        assert_eq!(state.version.as_deref(), Some("3.24.0"));
        assert_eq!(state.blueprint_id.as_deref(), Some("dev-workstation"));
        assert!(!state.config_hash.is_empty());
        assert!(!state.installed_at.is_empty());
    }

    #[test]
    fn state_key_format() {
        assert_eq!(state_key("flutter-sdk", "local"), "flutter-sdk@local");
        assert_eq!(state_key("a", "b"), "a@b");
    }

    #[test]
    fn config_hash_handles_non_string_values() {
        let mut config = HashMap::new();
        config.insert("count".to_string(), Value::Number(42.into()));
        config.insert("flag".to_string(), Value::Bool(true));

        let hash = compute_config_hash(&config);
        assert_eq!(hash.len(), 64);

        // Different non-string value should produce different hash
        let mut config2 = HashMap::new();
        config2.insert("count".to_string(), Value::Number(99.into()));
        config2.insert("flag".to_string(), Value::Bool(true));

        assert_ne!(hash, compute_config_hash(&config2));
    }
}
