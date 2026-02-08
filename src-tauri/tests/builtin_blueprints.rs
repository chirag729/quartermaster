use quartermaster_lib::blueprints::yaml_schema::BlueprintDefinition;
use quartermaster_lib::blueprints::Blueprint;
use quartermaster_lib::tasks::embedded::BUILTIN_BLUEPRINT_YAMLS;
use quartermaster_lib::tasks::registry::create_registry;

/// Helper: load builtin blueprints by parsing embedded YAML directly.
fn load_builtin_blueprints() -> Vec<Blueprint> {
    BUILTIN_BLUEPRINT_YAMLS
        .iter()
        .map(|yaml_str| {
            let def: BlueprintDefinition =
                serde_yaml::from_str(yaml_str).expect("invalid built-in blueprint YAML");
            Blueprint::from_definition(def)
        })
        .collect()
}

// ── Moved from blueprints/manager.rs ─────────────────────────────────

#[test]
fn default_blueprints_load_from_yaml() {
    let defaults = load_builtin_blueprints();
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
    let defaults = load_builtin_blueprints();
    let ws = defaults
        .iter()
        .find(|b| b.name == "Development Workstation")
        .unwrap();
    assert_eq!(ws.task_entries.len(), 6);
    assert_eq!(ws.icon, "Monitor");
    let task_ids: Vec<&str> = ws.task_entries.iter().map(|t| t.task_id.as_str()).collect();
    assert!(task_ids.contains(&"flutter-sdk"));
    assert!(task_ids.contains(&"claude-code"));
}

#[test]
fn mobile_dev_has_expected_tasks() {
    let defaults = load_builtin_blueprints();
    let mobile = defaults
        .iter()
        .find(|b| b.name == "Mobile Development")
        .unwrap();
    assert_eq!(mobile.task_entries.len(), 4);
    assert_eq!(mobile.icon, "Smartphone");
}

#[test]
fn minimal_server_has_empty_tasks() {
    let defaults = load_builtin_blueprints();
    let server = defaults
        .iter()
        .find(|b| b.name == "Minimal Server")
        .unwrap();
    assert!(server.task_entries.is_empty());
    assert_eq!(server.icon, "Server");
}

#[test]
fn task_entries_are_ordered() {
    let defaults = load_builtin_blueprints();
    for bp in &defaults {
        for (i, entry) in bp.task_entries.iter().enumerate() {
            assert_eq!(entry.order, i as u32);
        }
    }
}

// ── New tests ────────────────────────────────────────────────────────

#[test]
fn all_blueprint_task_ids_exist_in_registry() {
    let registry = create_registry();
    let all_task_ids: Vec<&str> = registry.tasks().iter().map(|t| t.id()).collect();
    let blueprints = load_builtin_blueprints();

    for bp in &blueprints {
        for entry in &bp.task_entries {
            assert!(
                all_task_ids.contains(&entry.task_id.as_str()),
                "Blueprint '{}' references task '{}', which does not exist in the registry",
                bp.name,
                entry.task_id
            );
        }
    }
}

#[test]
fn builtin_blueprints_have_valid_ids_and_names() {
    let blueprints = load_builtin_blueprints();
    for bp in &blueprints {
        assert!(!bp.id.is_empty(), "Blueprint has empty id");
        assert!(
            !bp.name.is_empty(),
            "Blueprint '{}' has empty name",
            bp.id
        );
        assert!(
            !bp.description.is_empty(),
            "Blueprint '{}' has empty description",
            bp.id
        );
        assert!(
            !bp.icon.is_empty(),
            "Blueprint '{}' has empty icon",
            bp.id
        );
    }
}

#[test]
fn builtin_blueprint_versions_are_valid_semver() {
    let semver_re = regex::Regex::new(r"^\d+\.\d+\.\d+$").unwrap();
    let blueprints = load_builtin_blueprints();
    for bp in &blueprints {
        assert!(
            semver_re.is_match(&bp.version),
            "Blueprint '{}' has invalid version '{}' (expected X.Y.Z)",
            bp.name,
            bp.version
        );
    }
}
