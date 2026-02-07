//! Variable resolution module with 4-layer precedence system.
//!
//! Variables are resolved using a layered precedence model inspired by Ansible:
//!
//! 1. **Task defaults** — from task config schema `default` values (lowest priority)
//! 2. **Blueprint variables** — from `variables.yaml` in the blueprint
//! 3. **User overrides** — configured before running, stored in config
//! 4. **Node-specific overrides** — per-node config for remote machines (highest priority)
//!
//! Higher-priority layers override lower ones. A lookup walks the layers top-down
//! and returns the first match found.

use std::collections::HashMap;

use serde::{Deserialize, Serialize};

use crate::error::AppError;

/// Schema definition for a shared variable.
///
/// Describes the key, display metadata, type, default value, and documentation
/// for a variable that can be referenced across tasks.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct VariableDefinition {
    /// Unique key used to reference this variable (e.g., `"install_dir"`).
    pub key: String,

    /// Human-readable label for UI display (e.g., `"Installation Directory"`).
    pub label: String,

    /// The type of the field (e.g., `"string"`, `"path"`, `"boolean"`).
    pub field_type: String,

    /// Default value used when no higher-priority layer provides one.
    pub default: String,

    /// Description of the variable's purpose and expected values.
    pub description: String,
}

/// A store for resolving variables across a 4-layer precedence system.
///
/// Resolution order (highest to lowest priority):
/// 1. `node_overrides` — node-specific values for remote machines
/// 2. `user_overrides` — user's runtime overrides
/// 3. `blueprint_values` — blueprint-level variables
/// 4. Task defaults — from `definitions[].default`
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct VariableStore {
    /// Variable definitions (schema). Each definition carries a default value
    /// which serves as the lowest-priority layer.
    pub definitions: Vec<VariableDefinition>,

    /// Blueprint-level variable values (layer 2).
    #[serde(default)]
    pub blueprint_values: HashMap<String, String>,

    /// User-configured overrides applied before execution (layer 3).
    #[serde(default)]
    pub user_overrides: HashMap<String, String>,

    /// Per-node overrides for machines with different paths/configurations (layer 4).
    #[serde(default)]
    pub node_overrides: HashMap<String, String>,
}

impl VariableStore {
    /// Creates a new, empty `VariableStore`.
    pub fn new() -> Self {
        Self::default()
    }

    /// Creates a `VariableStore` pre-populated with the given variable definitions.
    pub fn with_definitions(definitions: Vec<VariableDefinition>) -> Self {
        Self {
            definitions,
            blueprint_values: HashMap::new(),
            user_overrides: HashMap::new(),
            node_overrides: HashMap::new(),
        }
    }

    /// Adds a variable definition to the store.
    ///
    /// If a definition with the same key already exists, it is replaced.
    pub fn add_definition(&mut self, definition: VariableDefinition) {
        if let Some(existing) = self.definitions.iter_mut().find(|d| d.key == definition.key) {
            *existing = definition;
        } else {
            self.definitions.push(definition);
        }
    }

    /// Sets a blueprint-level value for the given key.
    pub fn set_blueprint_value(&mut self, key: String, value: String) {
        self.blueprint_values.insert(key, value);
    }

    /// Sets a user override for the given key.
    pub fn set_user_override(&mut self, key: String, value: String) {
        self.user_overrides.insert(key, value);
    }

    /// Sets a node-specific override for the given key.
    pub fn set_node_override(&mut self, key: String, value: String) {
        self.node_overrides.insert(key, value);
    }

    /// Removes a blueprint-level value.
    pub fn remove_blueprint_value(&mut self, key: &str) {
        self.blueprint_values.remove(key);
    }

    /// Removes a user override.
    pub fn remove_user_override(&mut self, key: &str) {
        self.user_overrides.remove(key);
    }

    /// Removes a node-specific override.
    pub fn remove_node_override(&mut self, key: &str) {
        self.node_overrides.remove(key);
    }

    /// Resolves a single variable by key using the 4-layer precedence system.
    ///
    /// Returns `None` if the key is not defined at any layer.
    ///
    /// # Precedence (highest first)
    /// 1. Node overrides
    /// 2. User overrides
    /// 3. Blueprint values
    /// 4. Definition defaults
    pub fn resolve(&self, key: &str) -> Option<String> {
        // Layer 4 (highest): node overrides
        if let Some(value) = self.node_overrides.get(key) {
            return Some(value.clone());
        }

        // Layer 3: user overrides
        if let Some(value) = self.user_overrides.get(key) {
            return Some(value.clone());
        }

        // Layer 2: blueprint values
        if let Some(value) = self.blueprint_values.get(key) {
            return Some(value.clone());
        }

        // Layer 1 (lowest): definition defaults
        self.definitions
            .iter()
            .find(|d| d.key == key)
            .map(|d| d.default.clone())
    }

    /// Resolves all defined variables, merging all layers.
    ///
    /// The returned map contains every key from the definitions, with the
    /// highest-priority value for each. Keys present only in override layers
    /// (without a definition) are also included.
    pub fn resolve_all(&self) -> HashMap<String, String> {
        let mut result = HashMap::new();

        // Start with definition defaults (layer 1, lowest)
        for def in &self.definitions {
            result.insert(def.key.clone(), def.default.clone());
        }

        // Apply blueprint values (layer 2)
        for (key, value) in &self.blueprint_values {
            result.insert(key.clone(), value.clone());
        }

        // Apply user overrides (layer 3)
        for (key, value) in &self.user_overrides {
            result.insert(key.clone(), value.clone());
        }

        // Apply node overrides (layer 4, highest)
        for (key, value) in &self.node_overrides {
            result.insert(key.clone(), value.clone());
        }

        result
    }

    /// Resolves a variable by key, returning an error if the key is not found.
    pub fn resolve_required(&self, key: &str) -> Result<String, AppError> {
        self.resolve(key).ok_or_else(|| {
            AppError::Variable(format!("Required variable '{}' is not defined", key))
        })
    }

    /// Returns the definition for a given key, if one exists.
    pub fn get_definition(&self, key: &str) -> Option<&VariableDefinition> {
        self.definitions.iter().find(|d| d.key == key)
    }

    /// Returns a list of all variable keys that have definitions.
    pub fn defined_keys(&self) -> Vec<&str> {
        self.definitions.iter().map(|d| d.key.as_str()).collect()
    }

    /// Returns the layer that provided the resolved value for a key.
    ///
    /// Useful for UI display to show where a value originates.
    pub fn resolve_source(&self, key: &str) -> Option<VariableSource> {
        if self.node_overrides.contains_key(key) {
            return Some(VariableSource::NodeOverride);
        }
        if self.user_overrides.contains_key(key) {
            return Some(VariableSource::UserOverride);
        }
        if self.blueprint_values.contains_key(key) {
            return Some(VariableSource::Blueprint);
        }
        if self.definitions.iter().any(|d| d.key == key) {
            return Some(VariableSource::Default);
        }
        None
    }

    /// Clears all override layers, leaving only definitions (and their defaults).
    pub fn clear_overrides(&mut self) {
        self.blueprint_values.clear();
        self.user_overrides.clear();
        self.node_overrides.clear();
    }
}

/// Indicates which layer provided the resolved value for a variable.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum VariableSource {
    /// Value comes from the variable definition's default.
    Default,
    /// Value comes from blueprint-level variables.
    Blueprint,
    /// Value comes from user overrides.
    UserOverride,
    /// Value comes from node-specific overrides.
    NodeOverride,
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_definitions() -> Vec<VariableDefinition> {
        vec![
            VariableDefinition {
                key: "install_dir".to_string(),
                label: "Installation Directory".to_string(),
                field_type: "path".to_string(),
                default: "/opt/tools".to_string(),
                description: "Base directory for tool installations".to_string(),
            },
            VariableDefinition {
                key: "java_version".to_string(),
                label: "Java Version".to_string(),
                field_type: "string".to_string(),
                default: "21".to_string(),
                description: "Java JDK version to install".to_string(),
            },
            VariableDefinition {
                key: "flutter_channel".to_string(),
                label: "Flutter Channel".to_string(),
                field_type: "string".to_string(),
                default: "stable".to_string(),
                description: "Flutter release channel".to_string(),
            },
        ]
    }

    #[test]
    fn new_store_is_empty() {
        let store = VariableStore::new();
        assert!(store.definitions.is_empty());
        assert!(store.blueprint_values.is_empty());
        assert!(store.user_overrides.is_empty());
        assert!(store.node_overrides.is_empty());
    }

    #[test]
    fn with_definitions_populates_schema() {
        let defs = sample_definitions();
        let store = VariableStore::with_definitions(defs.clone());
        assert_eq!(store.definitions.len(), 3);
        assert_eq!(store.definitions[0].key, "install_dir");
    }

    #[test]
    fn resolve_returns_default_when_no_overrides() {
        let store = VariableStore::with_definitions(sample_definitions());
        assert_eq!(store.resolve("install_dir"), Some("/opt/tools".to_string()));
        assert_eq!(store.resolve("java_version"), Some("21".to_string()));
    }

    #[test]
    fn resolve_returns_none_for_unknown_key() {
        let store = VariableStore::with_definitions(sample_definitions());
        assert_eq!(store.resolve("nonexistent"), None);
    }

    #[test]
    fn blueprint_overrides_default() {
        let mut store = VariableStore::with_definitions(sample_definitions());
        store.set_blueprint_value("install_dir".to_string(), "/usr/local".to_string());

        assert_eq!(
            store.resolve("install_dir"),
            Some("/usr/local".to_string())
        );
        // Other keys still use defaults
        assert_eq!(store.resolve("java_version"), Some("21".to_string()));
    }

    #[test]
    fn user_override_beats_blueprint() {
        let mut store = VariableStore::with_definitions(sample_definitions());
        store.set_blueprint_value("install_dir".to_string(), "/usr/local".to_string());
        store.set_user_override("install_dir".to_string(), "/home/user/tools".to_string());

        assert_eq!(
            store.resolve("install_dir"),
            Some("/home/user/tools".to_string())
        );
    }

    #[test]
    fn node_override_beats_user_override() {
        let mut store = VariableStore::with_definitions(sample_definitions());
        store.set_blueprint_value("install_dir".to_string(), "/usr/local".to_string());
        store.set_user_override("install_dir".to_string(), "/home/user/tools".to_string());
        store.set_node_override("install_dir".to_string(), "/srv/node1/tools".to_string());

        assert_eq!(
            store.resolve("install_dir"),
            Some("/srv/node1/tools".to_string())
        );
    }

    #[test]
    fn full_precedence_chain() {
        let mut store = VariableStore::with_definitions(sample_definitions());

        // Set different values at every layer for install_dir
        store.set_blueprint_value("install_dir".to_string(), "/bp".to_string());
        store.set_user_override("install_dir".to_string(), "/user".to_string());
        store.set_node_override("install_dir".to_string(), "/node".to_string());

        // Set only blueprint for java_version
        store.set_blueprint_value("java_version".to_string(), "17".to_string());

        // flutter_channel has no overrides — uses default
        assert_eq!(store.resolve("install_dir"), Some("/node".to_string()));
        assert_eq!(store.resolve("java_version"), Some("17".to_string()));
        assert_eq!(
            store.resolve("flutter_channel"),
            Some("stable".to_string())
        );
    }

    #[test]
    fn resolve_all_merges_all_layers() {
        let mut store = VariableStore::with_definitions(sample_definitions());
        store.set_blueprint_value("install_dir".to_string(), "/bp".to_string());
        store.set_user_override("java_version".to_string(), "17".to_string());
        store.set_node_override("install_dir".to_string(), "/node".to_string());

        let resolved = store.resolve_all();
        assert_eq!(resolved.get("install_dir"), Some(&"/node".to_string()));
        assert_eq!(resolved.get("java_version"), Some(&"17".to_string()));
        assert_eq!(
            resolved.get("flutter_channel"),
            Some(&"stable".to_string())
        );
    }

    #[test]
    fn resolve_all_includes_override_only_keys() {
        let mut store = VariableStore::with_definitions(sample_definitions());
        store.set_user_override("custom_key".to_string(), "custom_value".to_string());

        let resolved = store.resolve_all();
        assert_eq!(
            resolved.get("custom_key"),
            Some(&"custom_value".to_string())
        );
    }

    #[test]
    fn resolve_required_returns_error_for_missing_key() {
        let store = VariableStore::with_definitions(sample_definitions());
        let result = store.resolve_required("nonexistent");
        assert!(result.is_err());
        let err = result.unwrap_err().to_string();
        assert!(err.contains("nonexistent"));
    }

    #[test]
    fn resolve_required_returns_value_for_existing_key() {
        let store = VariableStore::with_definitions(sample_definitions());
        let result = store.resolve_required("install_dir");
        assert_eq!(result.unwrap(), "/opt/tools");
    }

    #[test]
    fn add_definition_appends_new() {
        let mut store = VariableStore::new();
        store.add_definition(VariableDefinition {
            key: "new_key".to_string(),
            label: "New Key".to_string(),
            field_type: "string".to_string(),
            default: "default_val".to_string(),
            description: "A new variable".to_string(),
        });

        assert_eq!(store.definitions.len(), 1);
        assert_eq!(store.resolve("new_key"), Some("default_val".to_string()));
    }

    #[test]
    fn add_definition_replaces_existing() {
        let mut store = VariableStore::with_definitions(sample_definitions());
        assert_eq!(store.definitions.len(), 3);

        store.add_definition(VariableDefinition {
            key: "install_dir".to_string(),
            label: "Install Path".to_string(),
            field_type: "path".to_string(),
            default: "/new/default".to_string(),
            description: "Updated description".to_string(),
        });

        assert_eq!(store.definitions.len(), 3);
        assert_eq!(
            store.resolve("install_dir"),
            Some("/new/default".to_string())
        );
    }

    #[test]
    fn remove_overrides() {
        let mut store = VariableStore::with_definitions(sample_definitions());
        store.set_blueprint_value("install_dir".to_string(), "/bp".to_string());
        store.set_user_override("install_dir".to_string(), "/user".to_string());
        store.set_node_override("install_dir".to_string(), "/node".to_string());

        store.remove_node_override("install_dir");
        assert_eq!(store.resolve("install_dir"), Some("/user".to_string()));

        store.remove_user_override("install_dir");
        assert_eq!(store.resolve("install_dir"), Some("/bp".to_string()));

        store.remove_blueprint_value("install_dir");
        assert_eq!(store.resolve("install_dir"), Some("/opt/tools".to_string()));
    }

    #[test]
    fn clear_overrides_resets_all_layers() {
        let mut store = VariableStore::with_definitions(sample_definitions());
        store.set_blueprint_value("install_dir".to_string(), "/bp".to_string());
        store.set_user_override("java_version".to_string(), "17".to_string());
        store.set_node_override("flutter_channel".to_string(), "beta".to_string());

        store.clear_overrides();

        assert_eq!(store.resolve("install_dir"), Some("/opt/tools".to_string()));
        assert_eq!(store.resolve("java_version"), Some("21".to_string()));
        assert_eq!(
            store.resolve("flutter_channel"),
            Some("stable".to_string())
        );
    }

    #[test]
    fn resolve_source_identifies_correct_layer() {
        let mut store = VariableStore::with_definitions(sample_definitions());
        assert_eq!(
            store.resolve_source("install_dir"),
            Some(VariableSource::Default)
        );

        store.set_blueprint_value("install_dir".to_string(), "/bp".to_string());
        assert_eq!(
            store.resolve_source("install_dir"),
            Some(VariableSource::Blueprint)
        );

        store.set_user_override("install_dir".to_string(), "/user".to_string());
        assert_eq!(
            store.resolve_source("install_dir"),
            Some(VariableSource::UserOverride)
        );

        store.set_node_override("install_dir".to_string(), "/node".to_string());
        assert_eq!(
            store.resolve_source("install_dir"),
            Some(VariableSource::NodeOverride)
        );
    }

    #[test]
    fn resolve_source_returns_none_for_unknown() {
        let store = VariableStore::new();
        assert_eq!(store.resolve_source("unknown"), None);
    }

    #[test]
    fn get_definition_returns_matching() {
        let store = VariableStore::with_definitions(sample_definitions());
        let def = store.get_definition("java_version").unwrap();
        assert_eq!(def.label, "Java Version");
        assert_eq!(def.default, "21");
    }

    #[test]
    fn get_definition_returns_none_for_missing() {
        let store = VariableStore::with_definitions(sample_definitions());
        assert!(store.get_definition("nonexistent").is_none());
    }

    #[test]
    fn defined_keys_lists_all() {
        let store = VariableStore::with_definitions(sample_definitions());
        let keys = store.defined_keys();
        assert_eq!(keys.len(), 3);
        assert!(keys.contains(&"install_dir"));
        assert!(keys.contains(&"java_version"));
        assert!(keys.contains(&"flutter_channel"));
    }

    #[test]
    fn serialization_roundtrip() {
        let mut store = VariableStore::with_definitions(sample_definitions());
        store.set_blueprint_value("install_dir".to_string(), "/bp".to_string());
        store.set_user_override("java_version".to_string(), "17".to_string());
        store.set_node_override("flutter_channel".to_string(), "beta".to_string());

        let json = serde_json::to_string(&store).unwrap();
        let deserialized: VariableStore = serde_json::from_str(&json).unwrap();

        assert_eq!(deserialized.definitions.len(), 3);
        assert_eq!(
            deserialized.blueprint_values.get("install_dir"),
            Some(&"/bp".to_string())
        );
        assert_eq!(
            deserialized.user_overrides.get("java_version"),
            Some(&"17".to_string())
        );
        assert_eq!(
            deserialized.node_overrides.get("flutter_channel"),
            Some(&"beta".to_string())
        );

        // Verify resolution still works after deserialization
        assert_eq!(deserialized.resolve("install_dir"), Some("/bp".to_string()));
        assert_eq!(deserialized.resolve("java_version"), Some("17".to_string()));
        assert_eq!(
            deserialized.resolve("flutter_channel"),
            Some("beta".to_string())
        );
    }

    #[test]
    fn empty_store_resolve_all_is_empty() {
        let store = VariableStore::new();
        assert!(store.resolve_all().is_empty());
    }

    #[test]
    fn override_for_undefined_key_still_resolves() {
        let mut store = VariableStore::new();
        store.set_user_override("ad_hoc".to_string(), "value".to_string());

        assert_eq!(store.resolve("ad_hoc"), Some("value".to_string()));
        assert!(store.get_definition("ad_hoc").is_none());
    }
}
