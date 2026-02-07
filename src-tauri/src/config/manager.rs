use std::collections::HashMap;
use std::path::PathBuf;
use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::error::AppError;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InstalledProfileState {
    pub profile_id: String,
    pub profile_name: String,
    pub config_snapshot: HashMap<String, String>,
    pub installed_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConfigData {
    #[serde(default = "default_theme")]
    pub theme: String,
    #[serde(default)]
    pub task_configs: HashMap<String, HashMap<String, Value>>,
    #[serde(default)]
    pub completed_tasks: Vec<String>,
    #[serde(default)]
    pub profile_configs: HashMap<String, HashMap<String, Value>>,
    #[serde(default)]
    pub installed_profiles: HashMap<String, InstalledProfileState>,

    /// Shared variables — user-level defaults that tasks can reference.
    /// Keys are variable names (e.g., "dev_folder"), values are user-configured paths/strings.
    #[serde(default)]
    pub shared_variables: HashMap<String, String>,

    /// Per-node variable overrides. Outer key is node_id, inner is variable_name → value.
    #[serde(default)]
    pub node_variable_overrides: HashMap<String, HashMap<String, String>>,

    /// Installation state tracking: task_id → InstalledTaskState.
    #[serde(default)]
    pub installed_tasks: HashMap<String, InstalledTaskState>,

    /// Fleet status polling interval in seconds (default: 30).
    #[serde(default = "default_poll_interval")]
    pub fleet_poll_interval: u64,

    // Legacy field for migration
    #[serde(default, skip_serializing)]
    pub module_configs: Option<HashMap<String, HashMap<String, Value>>>,
    #[serde(default, skip_serializing)]
    pub completed_modules: Option<Vec<String>>,
}

fn default_poll_interval() -> u64 {
    30
}

/// Tracks the installed state of a task on a specific node.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InstalledTaskState {
    pub task_id: String,
    pub node_id: String,
    pub version: Option<String>,
    pub blueprint_id: Option<String>,
    pub config_hash: String,
    pub installed_at: String,
}

fn default_theme() -> String {
    "system".to_string()
}

impl Default for ConfigData {
    fn default() -> Self {
        Self {
            theme: default_theme(),
            task_configs: HashMap::new(),
            completed_tasks: Vec::new(),
            profile_configs: HashMap::new(),
            installed_profiles: HashMap::new(),
            shared_variables: HashMap::new(),
            node_variable_overrides: HashMap::new(),
            installed_tasks: HashMap::new(),
            fleet_poll_interval: default_poll_interval(),
            module_configs: None,
            completed_modules: None,
        }
    }
}

pub struct ConfigManager {
    pub data: ConfigData,
    path: PathBuf,
}

impl Default for ConfigManager {
    fn default() -> Self {
        Self {
            data: ConfigData::default(),
            path: Self::config_path(),
        }
    }
}

impl ConfigManager {
    fn config_path() -> PathBuf {
        let config_dir = dirs::config_dir()
            .unwrap_or_else(|| PathBuf::from("~/.config"))
            .join("quartermaster");
        config_dir.join("config.json")
    }

    fn legacy_config_path() -> PathBuf {
        let config_dir = dirs::config_dir()
            .unwrap_or_else(|| PathBuf::from("~/.config"))
            .join("machine-setup");
        config_dir.join("config.json")
    }

    pub fn load() -> Result<Self, AppError> {
        let path = Self::config_path();
        let legacy_path = Self::legacy_config_path();

        if path.exists() {
            let content = std::fs::read_to_string(&path)?;
            let mut data: ConfigData = serde_json::from_str(&content)?;
            Self::migrate_fields(&mut data);
            Ok(Self { data, path })
        } else if legacy_path.exists() {
            // Migrate from legacy path
            let content = std::fs::read_to_string(&legacy_path)?;
            let mut data: ConfigData = serde_json::from_str(&content)?;
            Self::migrate_fields(&mut data);
            let mgr = Self { data, path };
            mgr.save()?;
            Ok(mgr)
        } else {
            Ok(Self {
                data: ConfigData::default(),
                path,
            })
        }
    }

    fn migrate_fields(data: &mut ConfigData) {
        // Migrate module_configs → task_configs
        if let Some(module_configs) = data.module_configs.take() {
            if data.task_configs.is_empty() {
                data.task_configs = module_configs;
            }
        }
        // Migrate completed_modules → completed_tasks
        if let Some(completed_modules) = data.completed_modules.take() {
            if data.completed_tasks.is_empty() {
                data.completed_tasks = completed_modules;
            }
        }
    }

    pub fn save(&self) -> Result<(), AppError> {
        if let Some(parent) = self.path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let content = serde_json::to_string_pretty(&self.data)?;
        std::fs::write(&self.path, content)?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::tempdir;

    #[test]
    fn default_config_has_system_theme() {
        let config = ConfigData::default();
        assert_eq!(config.theme, "system");
        assert!(config.task_configs.is_empty());
        assert!(config.completed_tasks.is_empty());
    }

    #[test]
    fn config_roundtrip() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("config.json");

        let manager = ConfigManager {
            data: ConfigData {
                theme: "dark".to_string(),
                completed_tasks: vec!["flutter-sdk".to_string()],
                ..ConfigData::default()
            },
            path: path.clone(),
        };

        manager.save().unwrap();
        assert!(path.exists());

        let content = fs::read_to_string(&path).unwrap();
        let loaded: ConfigData = serde_json::from_str(&content).unwrap();
        assert_eq!(loaded.theme, "dark");
        assert_eq!(loaded.completed_tasks, vec!["flutter-sdk"]);
    }

    #[test]
    fn load_nonexistent_returns_default() {
        let data: ConfigData = serde_json::from_str("{}").unwrap();
        assert_eq!(data.theme, "system");
        assert!(data.completed_tasks.is_empty());
    }

    #[test]
    fn deserialize_partial_config() {
        let json = r#"{"theme": "light"}"#;
        let data: ConfigData = serde_json::from_str(json).unwrap();
        assert_eq!(data.theme, "light");
        assert!(data.task_configs.is_empty());
    }

    #[test]
    fn migrate_legacy_fields() {
        let json = r#"{"theme": "dark", "module_configs": {"test": {"key": "val"}}, "completed_modules": ["flutter-sdk"]}"#;
        let mut data: ConfigData = serde_json::from_str(json).unwrap();
        ConfigManager::migrate_fields(&mut data);
        assert_eq!(data.task_configs.len(), 1);
        assert!(data.task_configs.contains_key("test"));
        assert_eq!(data.completed_tasks, vec!["flutter-sdk"]);
    }
}
