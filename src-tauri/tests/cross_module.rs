use std::collections::HashMap;

use quartermaster_lib::apparmor::log_parser::parse_denial_line;
use quartermaster_lib::apparmor::rule_consolidator::consolidate;
use quartermaster_lib::apparmor::rule_generator::generate_suggestions;
use quartermaster_lib::blueprints::manager::BlueprintManager;
use quartermaster_lib::blueprints::package::{pack_blueprint, unpack_blueprint};
use quartermaster_lib::blueprints::yaml_schema::BlueprintDefinition;
use quartermaster_lib::blueprints::{Blueprint, BlueprintTaskEntry};
use quartermaster_lib::tasks::embedded::BUILTIN_BLUEPRINT_YAMLS;
use quartermaster_lib::tasks::registry::create_registry;
use quartermaster_lib::variables::{VariableDefinition, VariableSource, VariableStore};

#[test]
fn apparmor_denial_to_suggestion_to_consolidation_pipeline() {
    // Sample denial log lines
    let log_lines = [
        r#"type=AMP msg=audit(1700000001.000:100): apparmor="DENIED" operation="open" profile="/usr/bin/myapp" name="/home/user/.config/myapp/settings.json" pid=1234 comm="myapp" requested_mask="r" denied_mask="r""#,
        r#"type=AMP msg=audit(1700000002.000:101): apparmor="DENIED" operation="open" profile="/usr/bin/myapp" name="/home/user/.config/myapp/cache.db" pid=1234 comm="myapp" requested_mask="r" denied_mask="r""#,
        r#"type=AMP msg=audit(1700000003.000:102): apparmor="DENIED" operation="open" profile="/usr/bin/myapp" name="/home/user/.config/myapp/plugins/ext1.so" pid=1234 comm="myapp" requested_mask="r" denied_mask="r""#,
        r#"type=AMP msg=audit(1700000004.000:103): apparmor="DENIED" operation="open" profile="/usr/bin/myapp" name="/home/user/.local/share/data.db" pid=1235 comm="myapp" requested_mask="rw" denied_mask="rw""#,
    ];

    // Step 1: Parse denial lines
    let denials: Vec<_> = log_lines
        .iter()
        .filter_map(|line| parse_denial_line(line))
        .collect();
    assert_eq!(denials.len(), 4, "Should parse all 4 denial lines");
    assert_eq!(denials[0].profile, "/usr/bin/myapp");
    assert_eq!(denials[0].operation, "open");

    // Step 2: Generate suggestions
    let suggestions = generate_suggestions(&denials);
    assert!(
        !suggestions.is_empty(),
        "Should generate at least one suggestion"
    );

    // Step 3: Consolidate
    let consolidated = consolidate(&suggestions);
    assert!(
        !consolidated.is_empty(),
        "Should produce at least one consolidation result"
    );
    // All denials were for the same profile
    assert_eq!(consolidated[0].profile, "/usr/bin/myapp");
}

#[test]
fn blueprint_packaging_roundtrip_with_real_data() {
    let dir = tempfile::tempdir().unwrap();

    // Use the first builtin blueprint YAML
    let yaml_str = BUILTIN_BLUEPRINT_YAMLS[0];
    let original_def: BlueprintDefinition =
        serde_yaml::from_str(yaml_str).expect("invalid built-in blueprint YAML");
    let original_name = original_def.name.clone();
    let original_task_count = original_def.tasks.len();

    // Write as manifest.yaml
    let bp_dir = dir.path().join("blueprint_src");
    std::fs::create_dir_all(&bp_dir).unwrap();
    let manifest_content = serde_yaml::to_string(&original_def).unwrap();
    std::fs::write(bp_dir.join("manifest.yaml"), &manifest_content).unwrap();

    // Pack to .qmbp
    let output_dir = dir.path().join("output");
    let qmbp_path = pack_blueprint(&bp_dir, &output_dir).unwrap();
    assert!(qmbp_path.exists());

    // Unpack
    let unpack_dir = dir.path().join("unpacked");
    let manifest = unpack_blueprint(&qmbp_path, &unpack_dir).unwrap();

    // Verify the unpacked definition matches the original
    assert_eq!(manifest.definition.name, original_name);
    assert_eq!(manifest.definition.tasks.len(), original_task_count);
    assert_eq!(manifest.definition.id, original_def.id);
}

#[test]
fn variable_resolution_with_real_task_config_schemas() {
    let registry = create_registry();

    // Pick a task that has config fields (flutter-sdk has sdk_base_path, channel)
    let flutter = registry.get("flutter-sdk").unwrap();
    let schema = flutter.config_schema();
    assert!(!schema.is_empty(), "flutter-sdk should have config schema fields");

    // Build a VariableStore from the task's config schema
    let definitions: Vec<VariableDefinition> = schema
        .iter()
        .map(|f| VariableDefinition {
            key: f.key.clone(),
            label: f.label.clone(),
            field_type: f.field_type.clone(),
            default: f.default_value.clone(),
            description: format!("From task config: {}", f.key),
        })
        .collect();

    let mut store = VariableStore::with_definitions(definitions);

    // Layer 1: defaults come from task config schema
    let default_sdk = store.resolve("sdk_base_path");
    assert!(default_sdk.is_some(), "sdk_base_path should have a default");
    assert_eq!(store.resolve_source("sdk_base_path"), Some(VariableSource::Default));

    // Layer 2: blueprint value
    store.set_blueprint_value("sdk_base_path".to_string(), "/bp/sdk".to_string());
    assert_eq!(store.resolve("sdk_base_path"), Some("/bp/sdk".to_string()));
    assert_eq!(store.resolve_source("sdk_base_path"), Some(VariableSource::Blueprint));

    // Layer 3: user override
    store.set_user_override("sdk_base_path".to_string(), "/user/sdk".to_string());
    assert_eq!(store.resolve("sdk_base_path"), Some("/user/sdk".to_string()));
    assert_eq!(store.resolve_source("sdk_base_path"), Some(VariableSource::UserOverride));

    // Layer 4: node override
    store.set_node_override("sdk_base_path".to_string(), "/node/sdk".to_string());
    assert_eq!(store.resolve("sdk_base_path"), Some("/node/sdk".to_string()));
    assert_eq!(store.resolve_source("sdk_base_path"), Some(VariableSource::NodeOverride));
}

#[test]
fn all_builtin_blueprint_tasks_exist_in_registry() {
    let registry = create_registry();
    let all_task_ids: Vec<&str> = registry.tasks().iter().map(|t| t.id()).collect();

    for yaml_str in BUILTIN_BLUEPRINT_YAMLS {
        let def: BlueprintDefinition =
            serde_yaml::from_str(yaml_str).expect("invalid built-in blueprint YAML");
        for task in &def.tasks {
            assert!(
                all_task_ids.contains(&task.id.as_str()),
                "Builtin blueprint '{}' references task '{}', which does not exist in registry",
                def.name,
                task.id
            );
        }
    }
}

#[test]
fn task_config_schema_fields_are_valid() {
    let registry = create_registry();
    let valid_field_types = ["text", "path", "select", "boolean", "string", "number"];

    for task in registry.tasks() {
        for field in task.config_schema() {
            assert!(
                valid_field_types.contains(&field.field_type.as_str()),
                "Task '{}' config field '{}' has invalid field_type '{}' (expected one of {:?})",
                task.id(),
                field.key,
                field.field_type,
                valid_field_types
            );
        }
    }
}

#[test]
fn blueprint_resolve_entries_with_parent_child() {
    let dir = tempfile::tempdir().unwrap();
    let mut mgr = BlueprintManager::with_dir(dir.path().to_path_buf());

    let now = chrono::Utc::now();

    // Create parent blueprint
    let parent = Blueprint {
        id: "parent-bp".to_string(),
        name: "Parent".to_string(),
        description: "Parent blueprint".to_string(),
        icon: "Star".to_string(),
        is_builtin: false,
        version: "1.0.0".to_string(),
        extends: None,
        task_entries: vec![
            BlueprintTaskEntry {
                task_id: "task-a".to_string(),
                enabled: true,
                config_overrides: HashMap::new(),
                order: 0,
            },
            BlueprintTaskEntry {
                task_id: "task-b".to_string(),
                enabled: true,
                config_overrides: HashMap::new(),
                order: 1,
            },
        ],
        created_at: now,
        updated_at: now,
    };
    mgr.add_blueprint(parent).unwrap();

    // Create child that extends parent, overrides task-b and adds task-c
    let child = Blueprint {
        id: "child-bp".to_string(),
        name: "Child".to_string(),
        description: "Child blueprint".to_string(),
        icon: "Star".to_string(),
        is_builtin: false,
        version: "1.0.0".to_string(),
        extends: Some("parent-bp".to_string()),
        task_entries: vec![
            BlueprintTaskEntry {
                task_id: "task-b".to_string(),
                enabled: false, // Override: disable task-b
                config_overrides: HashMap::new(),
                order: 1,
            },
            BlueprintTaskEntry {
                task_id: "task-c".to_string(),
                enabled: true,
                config_overrides: HashMap::new(),
                order: 2,
            },
        ],
        created_at: now,
        updated_at: now,
    };
    mgr.add_blueprint(child).unwrap();

    let resolved = mgr.resolve_task_entries("child-bp").unwrap();

    // Should have 3 tasks: task-a from parent, task-b (overridden/disabled), task-c from child
    assert_eq!(resolved.len(), 3);

    let task_a = resolved.iter().find(|e| e.task_id == "task-a").unwrap();
    assert!(task_a.enabled, "task-a should be enabled (inherited from parent)");

    let task_b = resolved.iter().find(|e| e.task_id == "task-b").unwrap();
    assert!(!task_b.enabled, "task-b should be disabled (child override)");

    let task_c = resolved.iter().find(|e| e.task_id == "task-c").unwrap();
    assert!(task_c.enabled, "task-c should be enabled (child addition)");
}
