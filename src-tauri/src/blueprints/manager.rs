use std::path::PathBuf;

use chrono::Utc;
use serde_json::Value;
use uuid::Uuid;

use crate::error::AppError;
use crate::tasks::embedded;
use super::yaml_schema::{BlueprintDefinition, BlueprintTaskDef};
use super::{Blueprint, BlueprintTaskEntry};

pub struct BlueprintManager {
    blueprints: Vec<Blueprint>,
    config_dir: PathBuf,
}

impl BlueprintManager {
    /// Creates a new BlueprintManager, loading existing blueprints from disk.
    /// If no blueprints exist on disk, migrates from legacy path or creates defaults.
    pub fn load() -> Result<Self, AppError> {
        let base_dir = dirs::config_dir()
            .unwrap_or_else(|| PathBuf::from("~/.config"));
        let config_dir = base_dir.join("quartermaster").join("blueprints");

        std::fs::create_dir_all(&config_dir)?;

        let mut manager = Self {
            blueprints: Vec::new(),
            config_dir: config_dir.clone(),
        };

        manager.load_blueprints_from_disk()?;

        // Migrate from legacy JSON path if directory is empty
        if manager.blueprints.is_empty() {
            let legacy_dir = base_dir.join("machine-setup").join("blueprints");
            if legacy_dir.exists() {
                if let Ok(entries) = std::fs::read_dir(&legacy_dir) {
                    for entry in entries.flatten() {
                        let src = entry.path();
                        if src.extension().and_then(|e| e.to_str()) == Some("json") {
                            // Migrate JSON to YAML
                            if let Ok(content) = std::fs::read_to_string(&src) {
                                if let Ok(bp) = serde_json::from_str::<Blueprint>(&content) {
                                    let _ = manager.save_blueprint_yaml(&bp);
                                }
                            }
                        }
                    }
                }
                manager.load_blueprints_from_disk()?;
            }
        }

        // Migrate existing JSON files in the quartermaster directory to YAML
        manager.migrate_json_to_yaml()?;

        if manager.blueprints.is_empty() {
            let defaults = load_default_blueprints();
            for bp in defaults {
                manager.add_blueprint(bp)?;
            }
        }

        Ok(manager)
    }

    fn load_blueprints_from_disk(&mut self) -> Result<(), AppError> {
        self.blueprints.clear();

        let entries = std::fs::read_dir(&self.config_dir)?;
        for entry in entries {
            let entry = entry?;
            let path = entry.path();
            let ext = path.extension().and_then(|e| e.to_str());

            match ext {
                Some("yaml") | Some("yml") => {
                    let content = std::fs::read_to_string(&path)?;
                    match serde_yaml::from_str::<BlueprintDefinition>(&content) {
                        Ok(def) => self.blueprints.push(Blueprint::from_definition(def)),
                        Err(e) => {
                            eprintln!(
                                "Warning: failed to parse blueprint file {}: {}",
                                path.display(),
                                e
                            );
                        }
                    }
                }
                Some("json") => {
                    let content = std::fs::read_to_string(&path)?;
                    match serde_json::from_str::<Blueprint>(&content) {
                        Ok(blueprint) => self.blueprints.push(blueprint),
                        Err(e) => {
                            eprintln!(
                                "Warning: failed to parse blueprint file {}: {}",
                                path.display(),
                                e
                            );
                        }
                    }
                }
                _ => {}
            }
        }

        Ok(())
    }

    /// Migrate any remaining JSON blueprint files to YAML format.
    fn migrate_json_to_yaml(&mut self) -> Result<(), AppError> {
        let entries: Vec<_> = std::fs::read_dir(&self.config_dir)?
            .flatten()
            .collect();

        for entry in entries {
            let path = entry.path();
            if path.extension().and_then(|e| e.to_str()) == Some("json") {
                let content = std::fs::read_to_string(&path)?;
                if let Ok(bp) = serde_json::from_str::<Blueprint>(&content) {
                    if self.save_blueprint_yaml(&bp).is_ok() {
                        let _ = std::fs::remove_file(&path);
                    }
                }
            }
        }

        // Reload after migration
        self.load_blueprints_from_disk()?;
        Ok(())
    }

    fn save_blueprint_yaml(&self, blueprint: &Blueprint) -> Result<(), AppError> {
        let def = blueprint.to_definition();
        let yaml = serde_yaml::to_string(&def)
            .map_err(|e| AppError::Blueprint(format!("YAML serialization error: {}", e)))?;
        let path = self.config_dir.join(format!("{}.yaml", blueprint.id));
        std::fs::write(&path, yaml)?;
        Ok(())
    }

    pub fn save_blueprint(&self, blueprint: &Blueprint) -> Result<(), AppError> {
        self.save_blueprint_yaml(blueprint)
    }

    pub fn delete_blueprint(&self, id: &str) -> Result<(), AppError> {
        // Remove both possible formats
        let yaml_path = self.config_dir.join(format!("{}.yaml", id));
        let json_path = self.config_dir.join(format!("{}.json", id));
        if yaml_path.exists() {
            std::fs::remove_file(&yaml_path)?;
        }
        if json_path.exists() {
            std::fs::remove_file(&json_path)?;
        }
        Ok(())
    }

    pub fn add_blueprint(&mut self, blueprint: Blueprint) -> Result<(), AppError> {
        self.save_blueprint(&blueprint)?;
        self.blueprints.push(blueprint);
        Ok(())
    }

    pub fn update_blueprint(&mut self, blueprint: Blueprint) -> Result<(), AppError> {
        self.save_blueprint(&blueprint)?;
        if let Some(existing) = self.blueprints.iter_mut().find(|b| b.id == blueprint.id) {
            *existing = blueprint;
        } else {
            return Err(AppError::Blueprint(format!(
                "Blueprint not found: {}",
                blueprint.id
            )));
        }
        Ok(())
    }

    pub fn remove_blueprint(&mut self, id: &str) -> Result<(), AppError> {
        let idx = self
            .blueprints
            .iter()
            .position(|b| b.id == id)
            .ok_or_else(|| AppError::Blueprint(format!("Blueprint not found: {}", id)))?;

        let blueprint = &self.blueprints[idx];
        if blueprint.is_builtin {
            return Err(AppError::Blueprint(
                "Cannot remove a built-in blueprint".to_string(),
            ));
        }

        self.blueprints.remove(idx);
        self.delete_blueprint(id)?;
        Ok(())
    }

    pub fn get_blueprint(&self, id: &str) -> Option<&Blueprint> {
        self.blueprints.iter().find(|b| b.id == id)
    }

    pub fn list_blueprints(&self) -> &[Blueprint] {
        &self.blueprints
    }

    pub fn create_blueprint(
        &mut self,
        name: String,
        description: String,
        icon: String,
        task_entries: Vec<BlueprintTaskEntry>,
    ) -> Result<Blueprint, AppError> {
        let now = Utc::now();
        let blueprint = Blueprint {
            id: Uuid::new_v4().to_string(),
            name,
            description,
            icon,
            is_builtin: false,
            task_entries,
            created_at: now,
            updated_at: now,
        };
        self.add_blueprint(blueprint.clone())?;
        Ok(blueprint)
    }

    pub fn clone_blueprint(&mut self, id: &str, new_name: String) -> Result<Blueprint, AppError> {
        let source = self
            .get_blueprint(id)
            .ok_or_else(|| AppError::Blueprint(format!("Blueprint not found: {}", id)))?
            .clone();

        let now = Utc::now();
        let cloned = Blueprint {
            id: Uuid::new_v4().to_string(),
            name: new_name,
            description: source.description,
            icon: source.icon,
            is_builtin: false,
            task_entries: source.task_entries,
            created_at: now,
            updated_at: now,
        };
        self.add_blueprint(cloned.clone())?;
        Ok(cloned)
    }
}

/// Load built-in blueprints from embedded YAML.
fn load_default_blueprints() -> Vec<Blueprint> {
    embedded::BUILTIN_BLUEPRINT_YAMLS
        .iter()
        .map(|yaml_str| {
            let def: BlueprintDefinition =
                serde_yaml::from_str(yaml_str).expect("invalid built-in blueprint YAML");
            Blueprint::from_definition(def)
        })
        .collect()
}

impl Blueprint {
    pub fn from_definition(def: BlueprintDefinition) -> Self {
        let now = Utc::now();
        Blueprint {
            id: def.id,
            name: def.name,
            description: def.description,
            icon: def.icon,
            is_builtin: def.builtin,
            task_entries: def
                .tasks
                .into_iter()
                .enumerate()
                .map(|(i, t)| BlueprintTaskEntry {
                    task_id: t.id,
                    enabled: t.enabled,
                    config_overrides: t
                        .config
                        .unwrap_or_default()
                        .into_iter()
                        .map(|(k, v)| (k, Value::String(v)))
                        .collect(),
                    order: i as u32,
                })
                .collect(),
            created_at: now,
            updated_at: now,
        }
    }

    pub fn to_definition(&self) -> BlueprintDefinition {
        BlueprintDefinition {
            id: self.id.clone(),
            name: self.name.clone(),
            description: self.description.clone(),
            icon: self.icon.clone(),
            builtin: self.is_builtin,
            tasks: self
                .task_entries
                .iter()
                .map(|e| BlueprintTaskDef {
                    id: e.task_id.clone(),
                    enabled: e.enabled,
                    config: if e.config_overrides.is_empty() {
                        None
                    } else {
                        Some(
                            e.config_overrides
                                .iter()
                                .map(|(k, v)| {
                                    (k.clone(), v.as_str().unwrap_or_default().to_string())
                                })
                                .collect(),
                        )
                    },
                })
                .collect(),
        }
    }
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;
    use super::*;
    use tempfile::tempdir;

    fn test_manager(dir: &std::path::Path) -> BlueprintManager {
        BlueprintManager {
            blueprints: Vec::new(),
            config_dir: dir.to_path_buf(),
        }
    }

    fn make_blueprint(name: &str, is_builtin: bool) -> Blueprint {
        let now = Utc::now();
        Blueprint {
            id: Uuid::new_v4().to_string(),
            name: name.to_string(),
            description: format!("Test blueprint: {}", name),
            icon: "TestIcon".to_string(),
            is_builtin,
            task_entries: vec![BlueprintTaskEntry {
                task_id: "test-task".to_string(),
                enabled: true,
                config_overrides: HashMap::new(),
                order: 0,
            }],
            created_at: now,
            updated_at: now,
        }
    }

    #[test]
    fn add_and_list_blueprints() {
        let dir = tempdir().unwrap();
        let mut mgr = test_manager(dir.path());

        let bp = make_blueprint("test-bp", false);
        let id = bp.id.clone();
        mgr.add_blueprint(bp).unwrap();

        assert_eq!(mgr.list_blueprints().len(), 1);
        assert_eq!(mgr.get_blueprint(&id).unwrap().name, "test-bp");
    }

    #[test]
    fn update_blueprint_changes_data() {
        let dir = tempdir().unwrap();
        let mut mgr = test_manager(dir.path());

        let mut bp = make_blueprint("original", false);
        let id = bp.id.clone();
        mgr.add_blueprint(bp.clone()).unwrap();

        bp.name = "updated".to_string();
        bp.updated_at = Utc::now();
        mgr.update_blueprint(bp).unwrap();

        assert_eq!(mgr.get_blueprint(&id).unwrap().name, "updated");
    }

    #[test]
    fn remove_blueprint_deletes_from_list_and_disk() {
        let dir = tempdir().unwrap();
        let mut mgr = test_manager(dir.path());

        let bp = make_blueprint("to-remove", false);
        let id = bp.id.clone();
        mgr.add_blueprint(bp).unwrap();

        assert!(dir.path().join(format!("{}.yaml", id)).exists());

        mgr.remove_blueprint(&id).unwrap();
        assert_eq!(mgr.list_blueprints().len(), 0);
        assert!(!dir.path().join(format!("{}.yaml", id)).exists());
    }

    #[test]
    fn cannot_remove_builtin_blueprint() {
        let dir = tempdir().unwrap();
        let mut mgr = test_manager(dir.path());

        let bp = make_blueprint("builtin", true);
        let id = bp.id.clone();
        mgr.add_blueprint(bp).unwrap();

        let result = mgr.remove_blueprint(&id);
        assert!(result.is_err());
        assert_eq!(mgr.list_blueprints().len(), 1);
    }

    #[test]
    fn save_and_reload_from_disk() {
        let dir = tempdir().unwrap();
        let mut mgr = test_manager(dir.path());

        let bp = make_blueprint("persistent", false);
        let id = bp.id.clone();
        mgr.add_blueprint(bp).unwrap();

        let mut mgr2 = test_manager(dir.path());
        mgr2.load_blueprints_from_disk().unwrap();

        assert_eq!(mgr2.list_blueprints().len(), 1);
        assert_eq!(mgr2.get_blueprint(&id).unwrap().name, "persistent");
    }

    #[test]
    fn create_blueprint_generates_id() {
        let dir = tempdir().unwrap();
        let mut mgr = test_manager(dir.path());

        let bp = mgr
            .create_blueprint(
                "New Blueprint".to_string(),
                "A new blueprint".to_string(),
                "Star".to_string(),
                vec![],
            )
            .unwrap();

        assert!(!bp.id.is_empty());
        assert_eq!(bp.name, "New Blueprint");
        assert!(!bp.is_builtin);
        assert_eq!(mgr.list_blueprints().len(), 1);
    }

    #[test]
    fn update_nonexistent_blueprint_returns_error() {
        let dir = tempdir().unwrap();
        let mut mgr = test_manager(dir.path());

        let bp = make_blueprint("ghost", false);
        let result = mgr.update_blueprint(bp);
        assert!(result.is_err());
    }

    #[test]
    fn remove_nonexistent_blueprint_returns_error() {
        let dir = tempdir().unwrap();
        let mut mgr = test_manager(dir.path());

        let result = mgr.remove_blueprint("nonexistent-id");
        assert!(result.is_err());
    }

    #[test]
    fn clone_blueprint_creates_independent_copy() {
        let dir = tempdir().unwrap();
        let mut mgr = test_manager(dir.path());

        let bp = make_blueprint("original", true);
        let original_id = bp.id.clone();
        mgr.add_blueprint(bp).unwrap();

        let cloned = mgr.clone_blueprint(&original_id, "My Copy".to_string()).unwrap();
        assert_ne!(cloned.id, original_id);
        assert_eq!(cloned.name, "My Copy");
        assert!(!cloned.is_builtin);
        assert_eq!(mgr.list_blueprints().len(), 2);
    }

    #[test]
    fn saves_as_yaml_format() {
        let dir = tempdir().unwrap();
        let mut mgr = test_manager(dir.path());

        let bp = make_blueprint("yaml-test", false);
        let id = bp.id.clone();
        mgr.add_blueprint(bp).unwrap();

        // Should save as .yaml, not .json
        assert!(dir.path().join(format!("{}.yaml", id)).exists());
        assert!(!dir.path().join(format!("{}.json", id)).exists());
    }

    #[test]
    fn default_blueprints_load_from_yaml() {
        let defaults = load_default_blueprints();
        assert_eq!(defaults.len(), 3);

        let names: Vec<&str> = defaults.iter().map(|b| b.name.as_str()).collect();
        assert!(names.contains(&"Development Workstation"));
        assert!(names.contains(&"Mobile Development"));
        assert!(names.contains(&"Minimal Server"));

        for bp in &defaults {
            assert!(bp.is_builtin);
        }
    }

    #[test]
    fn dev_workstation_has_expected_tasks() {
        let defaults = load_default_blueprints();
        let ws = defaults.iter().find(|b| b.name == "Development Workstation").unwrap();
        assert_eq!(ws.task_entries.len(), 7);
        assert_eq!(ws.icon, "Monitor");
        let task_ids: Vec<&str> = ws.task_entries.iter().map(|t| t.task_id.as_str()).collect();
        assert!(task_ids.contains(&"flutter-sdk"));
        assert!(task_ids.contains(&"claude-code"));
    }

    #[test]
    fn mobile_dev_has_expected_tasks() {
        let defaults = load_default_blueprints();
        let mobile = defaults.iter().find(|b| b.name == "Mobile Development").unwrap();
        assert_eq!(mobile.task_entries.len(), 5);
        assert_eq!(mobile.icon, "Smartphone");
    }

    #[test]
    fn minimal_server_has_empty_tasks() {
        let defaults = load_default_blueprints();
        let server = defaults.iter().find(|b| b.name == "Minimal Server").unwrap();
        assert!(server.task_entries.is_empty());
        assert_eq!(server.icon, "Server");
    }

    #[test]
    fn task_entries_are_ordered() {
        let defaults = load_default_blueprints();
        for bp in &defaults {
            for (i, entry) in bp.task_entries.iter().enumerate() {
                assert_eq!(entry.order, i as u32);
            }
        }
    }

    #[test]
    fn roundtrip_definition_conversion() {
        let bp = make_blueprint("roundtrip", false);
        let def = bp.to_definition();
        let bp2 = Blueprint::from_definition(def);
        assert_eq!(bp.id, bp2.id);
        assert_eq!(bp.name, bp2.name);
        assert_eq!(bp.task_entries.len(), bp2.task_entries.len());
        assert_eq!(bp.task_entries[0].task_id, bp2.task_entries[0].task_id);
    }
}
