use std::collections::HashMap;

use quartermaster_lib::config::manager::{ConfigData, ConfigManager, InstalledProfileState};

#[test]
fn config_roundtrip_all_fields() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("config.json");

    let mut task_configs = HashMap::new();
    let mut flutter_cfg = HashMap::new();
    flutter_cfg.insert(
        "sdk_base_path".to_string(),
        serde_json::Value::String("~/CustomSDK".into()),
    );
    task_configs.insert("flutter-sdk".to_string(), flutter_cfg);

    let mut shared_variables = HashMap::new();
    shared_variables.insert("dev_folder".to_string(), "~/Dev".to_string());

    let mut node_overrides = HashMap::new();
    let mut node1 = HashMap::new();
    node1.insert("dev_folder".to_string(), "/srv/dev".to_string());
    node_overrides.insert("node-1".to_string(), node1);

    let mut installed_profiles = HashMap::new();
    installed_profiles.insert(
        "intellij-idea".to_string(),
        InstalledProfileState {
            profile_id: "intellij-idea".to_string(),
            profile_name: "quartermaster.intellij-idea".to_string(),
            config_snapshot: HashMap::new(),
            installed_at: "2025-01-01T00:00:00Z".to_string(),
        },
    );

    let mut manager = ConfigManager::with_path(path.clone());
    manager.data.theme = "dark".to_string();
    manager.data.task_configs = task_configs;
    manager.data.completed_tasks = vec!["flutter-sdk".to_string(), "git-ssh".to_string()];
    manager.data.shared_variables = shared_variables;
    manager.data.node_variable_overrides = node_overrides;
    manager.data.installed_profiles = installed_profiles;
    manager.data.fleet_poll_interval = 60;

    manager.save().unwrap();

    // Reload from the file
    let content = std::fs::read_to_string(&path).unwrap();
    let loaded: ConfigData = serde_json::from_str(&content).unwrap();

    assert_eq!(loaded.theme, "dark");
    assert_eq!(loaded.completed_tasks, vec!["flutter-sdk", "git-ssh"]);
    assert_eq!(loaded.fleet_poll_interval, 60);
    assert!(loaded.task_configs.contains_key("flutter-sdk"));
    assert_eq!(
        loaded.shared_variables.get("dev_folder"),
        Some(&"~/Dev".to_string())
    );
    assert!(loaded.node_variable_overrides.contains_key("node-1"));
    assert!(loaded.installed_profiles.contains_key("intellij-idea"));
}

#[test]
fn config_default_values_from_empty_json() {
    let data: ConfigData = serde_json::from_str("{}").unwrap();
    assert_eq!(data.theme, "system");
    assert_eq!(data.fleet_poll_interval, 30);
    assert!(data.task_configs.is_empty());
    assert!(data.completed_tasks.is_empty());
    assert!(data.profile_configs.is_empty());
    assert!(data.installed_profiles.is_empty());
    assert!(data.shared_variables.is_empty());
    assert!(data.node_variable_overrides.is_empty());
    assert!(data.installed_tasks.is_empty());
}

#[test]
fn config_partial_json_preserves_present_fields() {
    let json = r#"{"theme": "light", "completed_tasks": ["flutter-sdk"]}"#;
    let data: ConfigData = serde_json::from_str(json).unwrap();
    assert_eq!(data.theme, "light");
    assert_eq!(data.completed_tasks, vec!["flutter-sdk"]);
    // Everything else defaults
    assert!(data.task_configs.is_empty());
    assert_eq!(data.fleet_poll_interval, 30);
}

#[test]
fn config_migration_legacy_module_configs() {
    let json = r#"{
        "theme": "dark",
        "module_configs": {"test": {"key": "val"}},
        "completed_modules": ["flutter-sdk"]
    }"#;
    let mut data: ConfigData = serde_json::from_str(json).unwrap();

    // Simulate migration (same logic as ConfigManager::migrate_fields)
    if let Some(module_configs) = data.module_configs.take() {
        if data.task_configs.is_empty() {
            data.task_configs = module_configs;
        }
    }
    if let Some(completed_modules) = data.completed_modules.take() {
        if data.completed_tasks.is_empty() {
            data.completed_tasks = completed_modules;
        }
    }

    assert_eq!(data.task_configs.len(), 1);
    assert!(data.task_configs.contains_key("test"));
    assert_eq!(data.completed_tasks, vec!["flutter-sdk"]);
}

#[test]
fn config_installed_profile_state_roundtrip() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("config.json");

    let mut config_snapshot = HashMap::new();
    config_snapshot.insert("home".to_string(), "/home/user".to_string());
    config_snapshot.insert("mode".to_string(), "complain".to_string());
    config_snapshot.insert("sdk_base_path".to_string(), "/home/user/SDK".to_string());

    let mut manager = ConfigManager::with_path(path.clone());
    manager.data.installed_profiles.insert(
        "flutter-sdk".to_string(),
        InstalledProfileState {
            profile_id: "flutter-sdk".to_string(),
            profile_name: "quartermaster.flutter".to_string(),
            config_snapshot: config_snapshot.clone(),
            installed_at: "2025-06-01T12:00:00Z".to_string(),
        },
    );

    manager.save().unwrap();

    // Reload
    let content = std::fs::read_to_string(&path).unwrap();
    let loaded: ConfigData = serde_json::from_str(&content).unwrap();

    let state = loaded.installed_profiles.get("flutter-sdk").unwrap();
    assert_eq!(state.profile_id, "flutter-sdk");
    assert_eq!(state.profile_name, "quartermaster.flutter");
    assert_eq!(state.installed_at, "2025-06-01T12:00:00Z");
    assert_eq!(state.config_snapshot, config_snapshot);
}
