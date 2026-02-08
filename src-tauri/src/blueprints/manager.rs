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
    /// Creates a BlueprintManager with a custom config directory (for testing).
    pub fn with_dir(config_dir: PathBuf) -> Self {
        Self { blueprints: Vec::new(), config_dir }
    }

    /// Creates a new BlueprintManager, loading existing blueprints from disk.
    /// If no blueprints exist on disk, migrates from legacy path or creates defaults.
    pub fn load() -> Result<Self, AppError> {
        let base_dir = crate::dirs::config_dir();
        let config_dir = base_dir.join("blueprints");

        std::fs::create_dir_all(&config_dir)?;

        let mut manager = Self {
            blueprints: Vec::new(),
            config_dir: config_dir.clone(),
        };

        manager.load_blueprints_from_disk()?;

        // Migrate from legacy JSON path if directory is empty
        if manager.blueprints.is_empty() {
            let legacy_base = dirs::config_dir().unwrap_or_else(|| {
                dirs::home_dir()
                    .unwrap_or_else(|| PathBuf::from("/tmp"))
                    .join(".config")
            });
            let legacy_dir = legacy_base.join("machine-setup").join("blueprints");
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
        } else {
            // Sync builtin blueprints: overwrite stale on-disk copies with the
            // latest embedded definitions so that task list changes (additions,
            // removals, renames) are picked up on app restart.
            manager.sync_builtin_blueprints()?;
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

    /// Overwrite on-disk builtin blueprints with the latest embedded definitions.
    /// This ensures task additions/removals/renames are picked up on app restart.
    /// User-created blueprints (is_builtin == false) are left untouched.
    fn sync_builtin_blueprints(&mut self) -> Result<(), AppError> {
        let defaults = load_default_blueprints();
        let mut changed = false;

        for fresh in &defaults {
            if let Some(idx) = self.blueprints.iter().position(|b| b.id == fresh.id && b.is_builtin) {
                // Preserve timestamps from the on-disk version
                let mut updated = fresh.clone();
                updated.created_at = self.blueprints[idx].created_at;
                updated.updated_at = self.blueprints[idx].updated_at;
                self.save_blueprint_yaml(&updated)?;
                self.blueprints[idx] = updated;
                changed = true;
            } else if !self.blueprints.iter().any(|b| b.id == fresh.id) {
                // New builtin that doesn't exist on disk yet
                self.save_blueprint_yaml(fresh)?;
                self.blueprints.push(fresh.clone());
                changed = true;
            }
        }

        if changed {
            // Re-sort to maintain consistent ordering
            self.blueprints.sort_by(|a, b| a.id.cmp(&b.id));
        }

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
        // Check in-memory existence first (immutable), before writing to disk
        if !self.blueprints.iter().any(|b| b.id == blueprint.id) {
            return Err(AppError::Blueprint(format!(
                "Blueprint not found: {}",
                blueprint.id
            )));
        }
        self.save_blueprint(&blueprint)?;
        if let Some(existing) = self.blueprints.iter_mut().find(|b| b.id == blueprint.id) {
            *existing = blueprint;
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
            version: "1.0.0".to_string(),
            extends: None,
            task_entries,
            created_at: now,
            updated_at: now,
        };
        self.add_blueprint(blueprint.clone())?;
        Ok(blueprint)
    }

    /// Import a blueprint from a `BlueprintDefinition` (e.g. from a `.qmbp` package).
    /// Assigns a new unique ID, marks it as non-builtin, saves it, and returns the new Blueprint.
    pub fn import_from_definition(&mut self, def: BlueprintDefinition) -> Result<Blueprint, AppError> {
        let mut blueprint = Blueprint::from_definition(def);
        blueprint.id = Uuid::new_v4().to_string();
        blueprint.is_builtin = false;
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
            version: "1.0.0".to_string(),
            extends: source.extends,
            task_entries: source.task_entries,
            created_at: now,
            updated_at: now,
        };
        self.add_blueprint(cloned.clone())?;
        Ok(cloned)
    }

    /// Resolves a blueprint's full task list by merging parent entries.
    /// Child entries override parent entries with the same task_id.
    /// Returns the merged entries sorted by order.
    ///
    /// Detects circular inheritance by tracking visited blueprint IDs.
    pub fn resolve_task_entries(&self, blueprint_id: &str) -> Result<Vec<BlueprintTaskEntry>, AppError> {
        let mut entries = Vec::new();
        let mut visited = std::collections::HashSet::new();
        let mut current_id = blueprint_id.to_string();

        // Walk the inheritance chain collecting parent entries (deepest ancestor first)
        let mut chain_entries: Vec<Vec<BlueprintTaskEntry>> = Vec::new();

        loop {
            if !visited.insert(current_id.clone()) {
                return Err(AppError::Blueprint(format!(
                    "Circular blueprint inheritance detected involving '{}'",
                    current_id
                )));
            }

            let bp = self.get_blueprint(&current_id)
                .ok_or_else(|| AppError::Blueprint(format!("Blueprint not found: {}", current_id)))?;

            chain_entries.push(bp.task_entries.clone());

            match bp.extends {
                Some(ref parent_id) => {
                    current_id = parent_id.clone();
                }
                None => break,
            }
        }

        // Merge from deepest ancestor to the target blueprint (last entry is the deepest)
        for layer in chain_entries.into_iter().rev() {
            for entry in layer {
                if let Some(existing) = entries.iter_mut().find(|e: &&mut BlueprintTaskEntry| e.task_id == entry.task_id) {
                    *existing = entry;
                } else {
                    entries.push(entry);
                }
            }
        }

        // Re-sort by order
        entries.sort_by_key(|e| e.order);

        Ok(entries)
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
            version: def.version,
            extends: def.extends,
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
            version: self.version.clone(),
            extends: self.extends.clone(),
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
                                    (k.clone(), match v {
                                        Value::String(s) => s.clone(),
                                        other => other.to_string(),
                                    })
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
            version: "1.0.0".to_string(),
            extends: None,
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
    fn roundtrip_definition_conversion() {
        let bp = make_blueprint("roundtrip", false);
        let def = bp.to_definition();
        let bp2 = Blueprint::from_definition(def);
        assert_eq!(bp.id, bp2.id);
        assert_eq!(bp.name, bp2.name);
        assert_eq!(bp.version, bp2.version);
        assert_eq!(bp.task_entries.len(), bp2.task_entries.len());
        assert_eq!(bp.task_entries[0].task_id, bp2.task_entries[0].task_id);
    }

    #[test]
    fn new_blueprint_starts_at_version_1_0_0() {
        let dir = tempdir().unwrap();
        let mut mgr = test_manager(dir.path());

        let bp = mgr
            .create_blueprint(
                "Versioned".to_string(),
                "Test version".to_string(),
                "Star".to_string(),
                vec![],
            )
            .unwrap();

        assert_eq!(bp.version, "1.0.0");
    }

    #[test]
    fn cloned_blueprint_starts_at_version_1_0_0() {
        let dir = tempdir().unwrap();
        let mut mgr = test_manager(dir.path());

        let mut bp = make_blueprint("source", false);
        bp.version = "3.2.1".to_string();
        let original_id = bp.id.clone();
        mgr.add_blueprint(bp).unwrap();

        let cloned = mgr.clone_blueprint(&original_id, "Clone".to_string()).unwrap();
        assert_eq!(cloned.version, "1.0.0");
    }

    #[test]
    fn roundtrip_preserves_version() {
        let mut bp = make_blueprint("versioned-roundtrip", false);
        bp.version = "2.5.3".to_string();
        let def = bp.to_definition();
        assert_eq!(def.version, "2.5.3");
        let bp2 = Blueprint::from_definition(def);
        assert_eq!(bp2.version, "2.5.3");
    }

    #[test]
    fn resolve_entries_without_parent() {
        let dir = tempdir().unwrap();
        let mut mgr = test_manager(dir.path());
        let bp = make_blueprint("standalone", false);
        let id = bp.id.clone();
        mgr.add_blueprint(bp).unwrap();

        let resolved = mgr.resolve_task_entries(&id).unwrap();
        assert_eq!(resolved.len(), 1);
        assert_eq!(resolved[0].task_id, "test-task");
    }

    #[test]
    fn resolve_entries_with_parent() {
        let dir = tempdir().unwrap();
        let mut mgr = test_manager(dir.path());

        // Create parent
        let parent = make_blueprint("parent", false);
        let parent_id = parent.id.clone();
        mgr.add_blueprint(parent).unwrap();

        // Create child that extends parent and adds a task
        let now = Utc::now();
        let child = Blueprint {
            id: Uuid::new_v4().to_string(),
            name: "child".to_string(),
            description: "Child blueprint".to_string(),
            icon: "Star".to_string(),
            is_builtin: false,
            version: "1.0.0".to_string(),
            extends: Some(parent_id.clone()),
            task_entries: vec![BlueprintTaskEntry {
                task_id: "extra-task".to_string(),
                enabled: true,
                config_overrides: HashMap::new(),
                order: 1,
            }],
            created_at: now,
            updated_at: now,
        };
        let child_id = child.id.clone();
        mgr.add_blueprint(child).unwrap();

        let resolved = mgr.resolve_task_entries(&child_id).unwrap();
        assert_eq!(resolved.len(), 2); // parent's test-task + child's extra-task
    }

    #[test]
    fn resolve_entries_child_overrides_parent() {
        let dir = tempdir().unwrap();
        let mut mgr = test_manager(dir.path());

        let parent = make_blueprint("parent", false);
        let parent_id = parent.id.clone();
        mgr.add_blueprint(parent).unwrap();

        let now = Utc::now();
        let child = Blueprint {
            id: Uuid::new_v4().to_string(),
            name: "child".to_string(),
            description: "Overrides parent".to_string(),
            icon: "Star".to_string(),
            is_builtin: false,
            version: "1.0.0".to_string(),
            extends: Some(parent_id.clone()),
            task_entries: vec![BlueprintTaskEntry {
                task_id: "test-task".to_string(), // Same as parent
                enabled: false,                   // But disabled
                config_overrides: HashMap::new(),
                order: 0,
            }],
            created_at: now,
            updated_at: now,
        };
        let child_id = child.id.clone();
        mgr.add_blueprint(child).unwrap();

        let resolved = mgr.resolve_task_entries(&child_id).unwrap();
        assert_eq!(resolved.len(), 1);
        assert!(!resolved[0].enabled); // Child's override wins
    }
}
