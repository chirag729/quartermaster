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

// ── Uninstall feature tests ───────────────────────────────────────

#[test]
fn tasks_with_uninstall_steps_report_supports_uninstall() {
    let registry = create_registry();
    let tasks_with_uninstall = [
        "rust-toolchain",
        "intellij-idea",
        "claude-code",
        "codex-cli",
        "flutter-sdk",
        "android-sdk",
        "git-ssh",
    ];
    for id in &tasks_with_uninstall {
        let task = registry.get(id).unwrap_or_else(|| panic!("Task '{}' not found", id));
        assert!(
            task.supports_uninstall(),
            "Task '{}' has uninstall steps but supports_uninstall() returns false",
            id
        );
        assert!(
            !task.uninstall_steps().is_empty(),
            "Task '{}' supports uninstall but has no uninstall_steps()",
            id
        );
    }
}

#[test]
fn tasks_without_uninstall_steps_report_no_support() {
    let registry = create_registry();
    let task = registry.get("create-development-folder").unwrap();
    assert!(
        !task.supports_uninstall(),
        "create-development-folder should not support uninstall"
    );
    assert!(
        task.uninstall_steps().is_empty(),
        "create-development-folder should have no uninstall steps"
    );
}

#[test]
fn uninstall_steps_have_valid_metadata() {
    let registry = create_registry();
    for task in registry.tasks() {
        if task.supports_uninstall() {
            for step in task.uninstall_steps() {
                assert!(
                    !step.name.is_empty(),
                    "Task '{}' has an uninstall step with an empty name",
                    task.id()
                );
                assert!(
                    step.progress > 0 && step.progress <= 100,
                    "Task '{}' uninstall step '{}' has invalid progress {}",
                    task.id(),
                    step.name,
                    step.progress
                );
            }
            // Last uninstall step should have progress 100
            let steps = task.uninstall_steps();
            let last = steps.last().unwrap();
            assert_eq!(
                last.progress, 100,
                "Task '{}' last uninstall step should have progress 100, got {}",
                task.id(),
                last.progress
            );
        }
    }
}

#[test]
fn to_info_includes_uninstall_metadata() {
    use quartermaster_lib::tasks::TaskStatus;
    let registry = create_registry();
    let rust = registry.get("rust-toolchain").unwrap();
    let info = rust.to_info(TaskStatus::Completed, None);
    assert!(info.supports_uninstall);
    assert!(!info.uninstall_steps.is_empty());

    let dev_folder = registry.get("create-development-folder").unwrap();
    let info2 = dev_folder.to_info(TaskStatus::NotStarted, None);
    assert!(!info2.supports_uninstall);
    assert!(info2.uninstall_steps.is_empty());
}

#[test]
fn yaml_uninstall_steps_deserialize_correctly() {
    use quartermaster_lib::tasks::yaml_schema::TaskDefinition;

    let yaml = r#"
id: test-uninstall
name: Test Uninstall
description: Task with uninstall steps
icon: Trash
category: Test
privilege: user
target: any
detect: "test -f /tmp/installed"
steps:
  - name: Install
    progress: 100
    run: "touch /tmp/installed"
uninstall:
  - name: Remove files
    progress: 50
    run: "rm -f /tmp/installed"
  - name: Clean up config
    progress: 100
    run: "rm -rf ~/.config/test-uninstall"
"#;

    let def: TaskDefinition = serde_yaml::from_str(yaml).unwrap();
    assert_eq!(def.id, "test-uninstall");
    assert_eq!(def.uninstall.len(), 2);
    assert_eq!(def.uninstall[0].name, "Remove files");
    assert_eq!(def.uninstall[0].progress, 50);
    assert!(def.uninstall[0].run.contains("rm -f /tmp/installed"));
    assert_eq!(def.uninstall[1].name, "Clean up config");
    assert_eq!(def.uninstall[1].progress, 100);
}

#[test]
fn yaml_without_uninstall_section_defaults_to_empty() {
    use quartermaster_lib::tasks::yaml_schema::TaskDefinition;

    let yaml = r#"
id: test-no-uninstall
name: No Uninstall
description: Task without uninstall
icon: Box
category: Test
privilege: user
target: any
detect: "test -f /tmp/installed"
steps:
  - name: Install
    progress: 100
    run: "touch /tmp/installed"
"#;

    let def: TaskDefinition = serde_yaml::from_str(yaml).unwrap();
    assert!(def.uninstall.is_empty(), "Tasks without uninstall section should default to empty vec");
}

#[test]
fn all_builtin_yaml_tasks_parse_uninstall_sections() {
    // These are embedded via include_str! in the registry, so we load them
    // through the registry and verify the round-trip
    let registry = create_registry();
    let tasks_with_expected_uninstall = [
        ("flutter-sdk", 2),     // 2 steps: remove PATH, remove dir
        ("android-sdk", 2),     // 2 steps: remove PATH/env, remove dir
        ("rust-toolchain", 1),  // 1 step: rustup self uninstall
        ("intellij-idea", 1),   // 1 step: snap remove
        ("claude-code", 1),     // 1 step: npm uninstall
        ("codex-cli", 1),       // 1 step: npm uninstall
        ("git-ssh", 3),         // 3 steps: remove keys, unset name, unset email
    ];

    for (task_id, expected_count) in &tasks_with_expected_uninstall {
        let task = registry.get(task_id).unwrap();
        let steps = task.uninstall_steps();
        assert_eq!(
            steps.len(),
            *expected_count,
            "Task '{}' expected {} uninstall steps, got {}",
            task_id,
            expected_count,
            steps.len()
        );
    }
}

#[test]
fn blueprint_uninstall_ordering_is_reverse_of_apply() {
    use quartermaster_lib::blueprints::BlueprintTaskEntry;
    use std::collections::HashMap;

    // Simulate what uninstall_blueprint does: sort entries by Reverse(order)
    let entries = vec![
        BlueprintTaskEntry {
            task_id: "first".into(),
            enabled: true,
            config_overrides: HashMap::new(),
            order: 0,
        },
        BlueprintTaskEntry {
            task_id: "second".into(),
            enabled: true,
            config_overrides: HashMap::new(),
            order: 1,
        },
        BlueprintTaskEntry {
            task_id: "third".into(),
            enabled: true,
            config_overrides: HashMap::new(),
            order: 2,
        },
    ];

    // Apply order: sorted by order ascending
    let mut apply_order = entries.clone();
    apply_order.sort_by_key(|e| e.order);
    assert_eq!(apply_order[0].task_id, "first");
    assert_eq!(apply_order[1].task_id, "second");
    assert_eq!(apply_order[2].task_id, "third");

    // Uninstall order: sorted by order DESCENDING (reverse)
    let mut uninstall_order = entries.clone();
    uninstall_order.sort_by_key(|e| std::cmp::Reverse(e.order));
    assert_eq!(uninstall_order[0].task_id, "third");
    assert_eq!(uninstall_order[1].task_id, "second");
    assert_eq!(uninstall_order[2].task_id, "first");
}

#[test]
fn blueprint_uninstall_skips_disabled_entries() {
    use quartermaster_lib::blueprints::BlueprintTaskEntry;
    use std::collections::HashMap;

    let entries = vec![
        BlueprintTaskEntry {
            task_id: "enabled-task".into(),
            enabled: true,
            config_overrides: HashMap::new(),
            order: 0,
        },
        BlueprintTaskEntry {
            task_id: "disabled-task".into(),
            enabled: false,
            config_overrides: HashMap::new(),
            order: 1,
        },
    ];

    // Simulate the filter from uninstall_blueprint
    let filtered: Vec<_> = entries.into_iter().filter(|e| e.enabled).collect();
    assert_eq!(filtered.len(), 1);
    assert_eq!(filtered[0].task_id, "enabled-task");
}
