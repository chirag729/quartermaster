use quartermaster_lib::tasks::registry::create_registry;

// ── Moved from tasks/registry.rs ─────────────────────────────────────

#[test]
fn registry_loads_all_builtin_tasks() {
    let registry = create_registry();
    let ids: Vec<&str> = registry.tasks().iter().map(|t| t.id()).collect();
    assert!(ids.contains(&"create-development-folder"));
    assert!(ids.contains(&"flutter-sdk"));
    assert!(ids.contains(&"android-sdk"));
    assert!(ids.contains(&"intellij-idea"));
    assert!(ids.contains(&"claude-code"));
    assert!(ids.contains(&"git-ssh"));
}

#[test]
fn builtin_task_metadata_preserved() {
    let registry = create_registry();
    let flutter = registry.get("flutter-sdk").unwrap();
    assert_eq!(flutter.name(), "Flutter SDK");
    assert_eq!(flutter.category(), "SDKs");
    assert_eq!(
        flutter.tags(),
        vec!["mobile".to_string(), "flutter".to_string()]
    );
    assert!(flutter.depends_on().is_empty());
}

#[test]
fn resolve_dependencies_no_deps_flutter() {
    let registry = create_registry();
    let result = registry
        .resolve_dependencies(&["flutter-sdk".to_string()])
        .unwrap();
    assert_eq!(result, vec!["flutter-sdk".to_string()]);
}

#[test]
fn resolve_dependencies_multiple_independent() {
    let registry = create_registry();
    let result = registry
        .resolve_dependencies(&["flutter-sdk".to_string(), "android-sdk".to_string()])
        .unwrap();
    assert!(result.contains(&"flutter-sdk".to_string()));
    assert!(result.contains(&"android-sdk".to_string()));
    assert_eq!(result.len(), 2);
}

#[test]
fn resolve_dependencies_no_deps() {
    let registry = create_registry();
    let result = registry
        .resolve_dependencies(&["create-development-folder".to_string()])
        .unwrap();
    assert_eq!(result, vec!["create-development-folder".to_string()]);
}

#[test]
fn resolve_dependencies_empty_input() {
    let registry = create_registry();
    let result = registry.resolve_dependencies(&[]).unwrap();
    assert!(result.is_empty());
}

// ── New tests ────────────────────────────────────────────────────────

#[test]
fn all_builtin_tasks_have_valid_metadata() {
    let registry = create_registry();
    for task in registry.tasks() {
        assert!(!task.id().is_empty(), "Task has empty id");
        assert!(
            !task.name().is_empty(),
            "Task '{}' has empty name",
            task.id()
        );
        assert!(
            !task.description().is_empty(),
            "Task '{}' has empty description",
            task.id()
        );
        assert!(
            !task.icon().is_empty(),
            "Task '{}' has empty icon",
            task.id()
        );
        assert!(
            !task.category().is_empty(),
            "Task '{}' has empty category",
            task.id()
        );

        // Validate config schema fields
        for field in task.config_schema() {
            assert!(
                !field.key.is_empty(),
                "Task '{}' has config field with empty key",
                task.id()
            );
            assert!(
                !field.label.is_empty(),
                "Task '{}' config field '{}' has empty label",
                task.id(),
                field.key
            );
            assert!(
                !field.field_type.is_empty(),
                "Task '{}' config field '{}' has empty field_type",
                task.id(),
                field.key
            );
        }
    }
}

#[test]
fn all_task_dependencies_reference_existing_tasks() {
    let registry = create_registry();
    let all_ids: Vec<&str> = registry.tasks().iter().map(|t| t.id()).collect();

    for task in registry.tasks() {
        for dep in task.depends_on() {
            assert!(
                all_ids.contains(&dep.as_str()),
                "Task '{}' depends on '{}', which does not exist in the registry",
                task.id(),
                dep
            );
        }
    }
}
