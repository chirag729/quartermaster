use std::collections::HashMap;
use std::path::PathBuf;

use quartermaster_lib::apparmor::template_manager::ProfileTemplateManager;
use quartermaster_lib::apparmor::template_schema::ProfileTemplateStatus;
use quartermaster_lib::config::manager::{ConfigManager, InstalledProfileState};
use quartermaster_lib::tasks::registry::create_registry;

// ── Helpers ──────────────────────────────────────────────────────────

fn load_real_templates() -> ProfileTemplateManager {
    ProfileTemplateManager::load()
}

fn real_home() -> String {
    dirs::home_dir()
        .unwrap_or_else(|| PathBuf::from("/home/unknown"))
        .to_string_lossy()
        .to_string()
}

/// Build a full context snapshot including resolved fragments (matches what get_status() computes).
fn build_full_snapshot(
    template: &quartermaster_lib::apparmor::template_schema::ProfileTemplate,
    registry: &quartermaster_lib::tasks::registry::TaskRegistry,
    config: &ConfigManager,
    home: &str,
) -> HashMap<String, String> {
    let task_config = config
        .data
        .task_configs
        .get(&template.task_id)
        .cloned()
        .unwrap_or_default();
    let profile_config = config
        .data
        .profile_configs
        .get(&template.id)
        .cloned()
        .unwrap_or_default();
    let mut ctx = ProfileTemplateManager::build_context(
        template, registry, &task_config, &profile_config, home,
    );
    let fragments = ProfileTemplateManager::resolve_fragments(
        template, registry, &config.data, home,
    );
    ctx.insert("fragments".to_string(), fragments);
    ctx
}

// ── Fragment Composition Integration Tests ───────────────────────────

#[test]
fn context_snapshot_includes_fragments_key() {
    let templates = load_real_templates();
    let registry = create_registry();
    let template = templates.get("intellij-idea").unwrap();

    let mut config = ConfigManager::default();
    config.data.completed_tasks.push("intellij-idea".into());

    let home = real_home();
    let snapshot = build_full_snapshot(template, &registry, &config, &home);

    assert!(
        snapshot.contains_key("fragments"),
        "Snapshot should contain 'fragments' key"
    );
}

#[test]
fn staleness_on_task_install() {
    let templates = load_real_templates();
    let registry = create_registry();
    let template = templates.get("intellij-idea").unwrap();

    let mut config = ConfigManager::default();
    config.data.completed_tasks.push("intellij-idea".into());

    let home = real_home();
    // Install profile WITHOUT flutter completed
    let snapshot = build_full_snapshot(template, &registry, &config, &home);
    assert!(
        snapshot.get("fragments").unwrap().is_empty(),
        "Fragments should be empty without Flutter completed"
    );

    config.data.installed_profiles.insert(
        "intellij-idea".into(),
        InstalledProfileState {
            profile_id: "intellij-idea".into(),
            profile_name: "quartermaster.intellij-idea".into(),
            config_snapshot: snapshot,
            installed_at: "2025-01-01T00:00:00Z".into(),
        },
    );

    // Verify it's Installed initially
    let status = ProfileTemplateManager::get_status(template, &config, &registry);
    assert!(
        matches!(status, ProfileTemplateStatus::Installed),
        "Should be Installed before Flutter is completed, got {:?}",
        status
    );

    // Now "install" flutter-sdk (mark it completed)
    config.data.completed_tasks.push("flutter-sdk".into());

    // Profile should now be Stale because fragments changed
    let status = ProfileTemplateManager::get_status(template, &config, &registry);
    assert!(
        matches!(status, ProfileTemplateStatus::Stale),
        "Should be Stale after Flutter is completed (new fragments available), got {:?}",
        status
    );
}

#[test]
fn staleness_on_task_uninstall() {
    let templates = load_real_templates();
    let registry = create_registry();
    let template = templates.get("intellij-idea").unwrap();

    let mut config = ConfigManager::default();
    config.data.completed_tasks.push("intellij-idea".into());
    config.data.completed_tasks.push("flutter-sdk".into());

    let home = real_home();
    // Install profile WITH flutter completed (fragments included)
    let snapshot = build_full_snapshot(template, &registry, &config, &home);
    assert!(
        !snapshot.get("fragments").unwrap().is_empty(),
        "Fragments should include Flutter rules when Flutter is completed"
    );

    config.data.installed_profiles.insert(
        "intellij-idea".into(),
        InstalledProfileState {
            profile_id: "intellij-idea".into(),
            profile_name: "quartermaster.intellij-idea".into(),
            config_snapshot: snapshot,
            installed_at: "2025-01-01T00:00:00Z".into(),
        },
    );

    // Verify Installed
    let status = ProfileTemplateManager::get_status(template, &config, &registry);
    assert!(matches!(status, ProfileTemplateStatus::Installed));

    // "Uninstall" flutter-sdk
    config.data.completed_tasks.retain(|t| t != "flutter-sdk");

    // Profile should be Stale because fragments changed (Flutter removed)
    let status = ProfileTemplateManager::get_status(template, &config, &registry);
    assert!(
        matches!(status, ProfileTemplateStatus::Stale),
        "Should be Stale after Flutter is uninstalled (fragments removed), got {:?}",
        status
    );
}

#[test]
fn no_staleness_when_unrelated_task_installed() {
    let templates = load_real_templates();
    let registry = create_registry();
    let template = templates.get("intellij-idea").unwrap();

    let mut config = ConfigManager::default();
    config.data.completed_tasks.push("intellij-idea".into());

    let home = real_home();
    let snapshot = build_full_snapshot(template, &registry, &config, &home);

    config.data.installed_profiles.insert(
        "intellij-idea".into(),
        InstalledProfileState {
            profile_id: "intellij-idea".into(),
            profile_name: "quartermaster.intellij-idea".into(),
            config_snapshot: snapshot,
            installed_at: "2025-01-01T00:00:00Z".into(),
        },
    );

    // Complete a task that does NOT have sdk/mobile-sdk fragment tags
    config.data.completed_tasks.push("rust-toolchain".into());

    let status = ProfileTemplateManager::get_status(template, &config, &registry);
    assert!(
        matches!(status, ProfileTemplateStatus::Installed),
        "Installing unrelated task should NOT make IntelliJ profile stale, got {:?}",
        status
    );
}

#[test]
fn fragments_empty_when_no_tasks_completed() {
    let templates = load_real_templates();
    let registry = create_registry();
    let template = templates.get("intellij-idea").unwrap();

    let config_data = quartermaster_lib::config::manager::ConfigData::default();
    let fragments = ProfileTemplateManager::resolve_fragments(
        template, &registry, &config_data, "/home/testuser",
    );

    assert!(
        fragments.is_empty(),
        "Fragments should be empty when no tasks are completed, got: '{}'",
        fragments
    );
}

#[test]
fn intellij_template_parses_with_subscriptions() {
    let templates = load_real_templates();
    let template = templates.get("intellij-idea").unwrap();

    assert!(
        template.subscribes_to.contains(&"sdk".to_string()),
        "IntelliJ template should subscribe to 'sdk' tag"
    );
    assert!(
        template.subscribes_to.contains(&"mobile-sdk".to_string()),
        "IntelliJ template should subscribe to 'mobile-sdk' tag"
    );
    assert!(
        template.content.contains("{{fragments}}"),
        "IntelliJ template should contain {{fragments}} placeholder"
    );
}

#[test]
fn flutter_task_has_fragments() {
    let registry = create_registry();
    let flutter = registry.get("flutter-sdk").expect("flutter-sdk task should exist");

    let fragments = flutter.fragment_infos();
    assert!(
        !fragments.is_empty(),
        "Flutter SDK task should have at least one fragment"
    );

    let first = &fragments[0];
    assert!(
        first.tags.contains(&"sdk".to_string()),
        "Flutter fragment should be tagged with 'sdk'"
    );
    assert!(
        first.tags.contains(&"mobile-sdk".to_string()),
        "Flutter fragment should be tagged with 'mobile-sdk'"
    );
    assert!(
        first.content.contains("flutter"),
        "Flutter fragment content should mention flutter"
    );
}
