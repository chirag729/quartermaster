use std::collections::HashMap;

use quartermaster_lib::apparmor::template_manager::ProfileTemplateManager;
use quartermaster_lib::apparmor::template_schema::{validate_profile_template, ProfileTemplateStatus};
use quartermaster_lib::config::manager::{ConfigManager, InstalledProfileState};
use quartermaster_lib::tasks::registry::create_registry;

// ── Helpers ──────────────────────────────────────────────────────────

fn load_real_templates() -> ProfileTemplateManager {
    ProfileTemplateManager::load()
}

fn render_with_defaults(
    templates: &ProfileTemplateManager,
    template_id: &str,
    registry: &quartermaster_lib::tasks::registry::TaskRegistry,
) -> String {
    let template = templates
        .get(template_id)
        .unwrap_or_else(|| panic!("template {} not found", template_id));
    let mut ctx = ProfileTemplateManager::build_context(
        template,
        registry,
        &HashMap::new(),
        &HashMap::new(),
        "/home/testuser",
    );
    // Include empty fragments since no tasks are completed
    let config_data = quartermaster_lib::config::manager::ConfigData::default();
    let fragments = ProfileTemplateManager::resolve_fragments(
        template, registry, &config_data, "/home/testuser",
    );
    ctx.insert("fragments".to_string(), fragments);
    ProfileTemplateManager::render(template, &ctx)
}

/// Build a full context snapshot including resolved fragments (matching what get_status() computes).
fn build_full_context(
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

// ── Group 1: Static Cross-SDK Content Verification ──────────────────

#[test]
fn intellij_profile_includes_all_sdk_rules() {
    let templates = load_real_templates();
    let registry = create_registry();
    let rendered = render_with_defaults(&templates, "intellij-idea", &registry);

    assert!(rendered.contains(".rustup/**"), "IntelliJ profile missing .rustup/** rule");
    assert!(rendered.contains(".cargo/bin/*"), "IntelliJ profile missing .cargo/bin/* rule");
    assert!(rendered.contains(".nvm/**"), "IntelliJ profile missing .nvm/** rule");
    assert!(rendered.contains(".gradle/**"), "IntelliJ profile missing .gradle/** rule");
    assert!(rendered.contains(".gitconfig"), "IntelliJ profile missing .gitconfig rule");
    assert!(
        rendered.contains("deny /home/testuser/.ssh/id_*"),
        "IntelliJ profile missing deny .ssh/id_* rule"
    );
    assert!(
        rendered.contains("owner /home/testuser/"),
        "IntelliJ profile missing owner qualifier"
    );
}

#[test]
fn claude_code_profile_includes_all_sdk_rules() {
    let templates = load_real_templates();
    let registry = create_registry();
    let rendered = render_with_defaults(&templates, "claude-code", &registry);

    assert!(rendered.contains(".rustup/**"), "Claude Code profile missing .rustup/** rule");
    assert!(rendered.contains(".cargo/bin/*"), "Claude Code profile missing .cargo/bin/* rule");
    assert!(rendered.contains(".nvm/**"), "Claude Code profile missing .nvm/** rule");
    assert!(rendered.contains(".gitconfig"), "Claude Code profile missing .gitconfig rule");
    assert!(
        rendered.contains("deny /home/testuser/.ssh/id_*"),
        "Claude Code profile missing deny .ssh/id_* rule"
    );
    assert!(
        rendered.contains("deny /home/testuser/.gnupg/**"),
        "Claude Code profile missing deny .gnupg/** rule"
    );
    assert!(
        rendered.contains("deny /etc/shadow"),
        "Claude Code profile missing deny /etc/shadow rule"
    );
}

#[test]
fn codex_cli_profile_includes_all_sdk_rules() {
    let templates = load_real_templates();
    let registry = create_registry();
    let rendered = render_with_defaults(&templates, "codex-cli", &registry);

    assert!(rendered.contains(".rustup/**"), "Codex CLI profile missing .rustup/** rule");
    assert!(rendered.contains(".cargo/bin/*"), "Codex CLI profile missing .cargo/bin/* rule");
    assert!(rendered.contains(".nvm/**"), "Codex CLI profile missing .nvm/** rule");
    assert!(rendered.contains(".gitconfig"), "Codex CLI profile missing .gitconfig rule");
    assert!(
        rendered.contains("deny /home/testuser/.ssh/id_*"),
        "Codex CLI profile missing deny .ssh/id_* rule"
    );
}

#[test]
fn flutter_profile_includes_all_sdk_rules() {
    let templates = load_real_templates();
    let registry = create_registry();
    let rendered = render_with_defaults(&templates, "flutter-sdk", &registry);

    assert!(rendered.contains(".rustup/**"), "Flutter profile missing .rustup/** rule");
    assert!(rendered.contains(".cargo/bin/*"), "Flutter profile missing .cargo/bin/* rule");
    assert!(rendered.contains(".nvm/**"), "Flutter profile missing .nvm/** rule");
    assert!(rendered.contains(".gradle/**"), "Flutter profile missing .gradle/** rule");
    assert!(rendered.contains(".gitconfig"), "Flutter profile missing .gitconfig rule");
    assert!(
        rendered.contains("deny /home/testuser/.ssh/id_*"),
        "Flutter profile missing deny .ssh/id_* rule"
    );
}

#[test]
fn all_profiles_contain_deny_rules() {
    let templates = load_real_templates();
    let registry = create_registry();

    let deny_rules = [
        ("deny /home/testuser/.ssh/id_*", "deny .ssh/id_*"),
        ("deny /home/testuser/.gnupg/**", "deny .gnupg/**"),
        ("deny /etc/shadow", "deny /etc/shadow"),
    ];

    for template in templates.list() {
        let ctx = ProfileTemplateManager::build_context(
            template,
            &registry,
            &HashMap::new(),
            &HashMap::new(),
            "/home/testuser",
        );
        let rendered = ProfileTemplateManager::render(template, &ctx);

        for (rule, label) in &deny_rules {
            assert!(
                rendered.contains(rule),
                "Profile '{}' missing {} rule",
                template.id,
                label
            );
        }
    }
}

// ── Group 2: Render Order Independence ──────────────────────────────

#[test]
fn render_output_independent_of_completed_tasks() {
    let templates = load_real_templates();
    let registry = create_registry();

    let all_task_ids: Vec<String> = registry.all().iter().map(|t| t.id().to_string()).collect();

    for template in templates.list() {
        let ctx_empty = ProfileTemplateManager::build_context(
            template,
            &registry,
            &HashMap::new(),
            &HashMap::new(),
            "/home/testuser",
        );
        let rendered_empty = ProfileTemplateManager::render(template, &ctx_empty);

        let ctx_full = ProfileTemplateManager::build_context(
            template,
            &registry,
            &HashMap::new(),
            &HashMap::new(),
            "/home/testuser",
        );
        let rendered_full = ProfileTemplateManager::render(template, &ctx_full);

        assert_eq!(
            rendered_empty, rendered_full,
            "Profile '{}' rendered differently with different completed_tasks state",
            template.id
        );

        assert_eq!(
            ctx_empty, ctx_full,
            "Profile '{}' context differs between render calls",
            template.id
        );
    }

    assert!(templates.list().len() >= 5, "Expected at least 5 built-in profile templates, found {}", templates.list().len());
    assert!(!all_task_ids.is_empty(), "Registry returned no tasks — test is vacuous");
}

#[test]
fn all_templates_render_without_unresolved_placeholders() {
    let templates = load_real_templates();
    let registry = create_registry();

    for template in templates.list() {
        let rendered = render_with_defaults(&templates, &template.id, &registry);

        assert!(
            !rendered.contains("{{"),
            "Profile '{}' has unresolved placeholder: {}",
            template.id,
            rendered
                .lines()
                .find(|l| l.contains("{{"))
                .unwrap_or("(not found)")
        );
    }
}

// ── Group 3: Status Lifecycle Through Installation Scenarios ─────────

#[test]
fn completing_unrelated_task_does_not_affect_profile_status() {
    let templates = load_real_templates();
    let registry = create_registry();
    let template = templates.get("intellij-idea").unwrap();

    let mut config = ConfigManager::default();
    config
        .data
        .completed_tasks
        .push("intellij-idea".to_string());

    let real_home = dirs::home_dir()
        .unwrap_or_else(|| std::path::PathBuf::from("/home/unknown"))
        .to_string_lossy()
        .to_string();
    let real_context = build_full_context(template, &registry, &config, &real_home);
    config.data.installed_profiles.insert(
        "intellij-idea".to_string(),
        InstalledProfileState {
            profile_id: "intellij-idea".to_string(),
            profile_name: "quartermaster.intellij-idea".to_string(),
            config_snapshot: real_context,
            installed_at: "2025-06-01T00:00:00Z".to_string(),
        },
    );

    // Complete an unrelated task (no fragments matching intellij's subscriptions)
    config
        .data
        .completed_tasks
        .push("rust-toolchain".to_string());

    let status = ProfileTemplateManager::get_status(template, &config, &registry);
    assert!(
        matches!(status, ProfileTemplateStatus::Installed),
        "Completing unrelated task 'rust-toolchain' changed IntelliJ profile status to {:?}",
        status
    );
}

#[test]
fn profile_status_task_not_completed_until_linked_task_done() {
    let templates = load_real_templates();
    let registry = create_registry();
    let template = templates.get("intellij-idea").unwrap();

    let mut config = ConfigManager::default();

    let status = ProfileTemplateManager::get_status(template, &config, &registry);
    assert!(
        matches!(status, ProfileTemplateStatus::TaskNotCompleted),
        "Expected TaskNotCompleted when no tasks done, got {:?}",
        status
    );

    config
        .data
        .completed_tasks
        .push("rust-toolchain".to_string());
    config.data.completed_tasks.push("flutter-sdk".to_string());

    let status = ProfileTemplateManager::get_status(template, &config, &registry);
    assert!(
        matches!(status, ProfileTemplateStatus::TaskNotCompleted),
        "Expected TaskNotCompleted when only unrelated tasks done, got {:?}",
        status
    );

    config
        .data
        .completed_tasks
        .push("intellij-idea".to_string());

    let status = ProfileTemplateManager::get_status(template, &config, &registry);
    assert!(
        matches!(status, ProfileTemplateStatus::NotInstalled),
        "Expected NotInstalled when linked task done but profile not installed, got {:?}",
        status
    );
}

#[test]
fn profile_becomes_stale_on_profile_config_change() {
    let templates = load_real_templates();
    let registry = create_registry();
    let template = templates.get("intellij-idea").unwrap();

    let mut config = ConfigManager::default();
    config
        .data
        .completed_tasks
        .push("intellij-idea".to_string());

    let home = dirs::home_dir()
        .unwrap_or_else(|| std::path::PathBuf::from("/home/unknown"))
        .to_string_lossy()
        .to_string();
    let context = build_full_context(template, &registry, &config, &home);
    config.data.installed_profiles.insert(
        "intellij-idea".to_string(),
        InstalledProfileState {
            profile_id: "intellij-idea".to_string(),
            profile_name: "quartermaster.intellij-idea".to_string(),
            config_snapshot: context,
            installed_at: "2025-06-01T00:00:00Z".to_string(),
        },
    );

    let status = ProfileTemplateManager::get_status(template, &config, &registry);
    assert!(matches!(status, ProfileTemplateStatus::Installed));

    let mut profile_overrides = HashMap::new();
    profile_overrides.insert(
        "projects_path".to_string(),
        serde_json::Value::String("~/NewProjects".into()),
    );
    config
        .data
        .profile_configs
        .insert("intellij-idea".to_string(), profile_overrides);

    let status = ProfileTemplateManager::get_status(template, &config, &registry);
    assert!(
        matches!(status, ProfileTemplateStatus::Stale),
        "Expected Stale after profile config change, got {:?}",
        status
    );
}

#[test]
fn profile_becomes_stale_on_task_config_change() {
    let templates = load_real_templates();
    let registry = create_registry();
    let template = templates.get("flutter-sdk").unwrap();

    let mut config = ConfigManager::default();
    config.data.completed_tasks.push("flutter-sdk".to_string());

    let home = dirs::home_dir()
        .unwrap_or_else(|| std::path::PathBuf::from("/home/unknown"))
        .to_string_lossy()
        .to_string();
    let context = build_full_context(template, &registry, &config, &home);
    config.data.installed_profiles.insert(
        "flutter-sdk".to_string(),
        InstalledProfileState {
            profile_id: "flutter-sdk".to_string(),
            profile_name: "quartermaster.flutter".to_string(),
            config_snapshot: context,
            installed_at: "2025-06-01T00:00:00Z".to_string(),
        },
    );

    let status = ProfileTemplateManager::get_status(template, &config, &registry);
    assert!(matches!(status, ProfileTemplateStatus::Installed));

    let mut task_overrides = HashMap::new();
    task_overrides.insert(
        "sdk_base_path".to_string(),
        serde_json::Value::String("~/CustomSDK".into()),
    );
    config
        .data
        .task_configs
        .insert("flutter-sdk".to_string(), task_overrides);

    let status = ProfileTemplateManager::get_status(template, &config, &registry);
    assert!(
        matches!(status, ProfileTemplateStatus::Stale),
        "Expected Stale after task config change, got {:?}",
        status
    );
}

#[test]
fn profile_stays_installed_after_unrelated_config_change() {
    let templates = load_real_templates();
    let registry = create_registry();
    let template = templates.get("intellij-idea").unwrap();

    let mut config = ConfigManager::default();
    config
        .data
        .completed_tasks
        .push("intellij-idea".to_string());

    let home = dirs::home_dir()
        .unwrap_or_else(|| std::path::PathBuf::from("/home/unknown"))
        .to_string_lossy()
        .to_string();
    let context = build_full_context(template, &registry, &config, &home);
    config.data.installed_profiles.insert(
        "intellij-idea".to_string(),
        InstalledProfileState {
            profile_id: "intellij-idea".to_string(),
            profile_name: "quartermaster.intellij-idea".to_string(),
            config_snapshot: context,
            installed_at: "2025-06-01T00:00:00Z".to_string(),
        },
    );

    // Change Flutter's sdk_base_path — this is unrelated to IntelliJ
    let mut flutter_overrides = HashMap::new();
    flutter_overrides.insert(
        "sdk_base_path".to_string(),
        serde_json::Value::String("~/NewFlutterSDK".into()),
    );
    config
        .data
        .task_configs
        .insert("flutter-sdk".to_string(), flutter_overrides);

    let status = ProfileTemplateManager::get_status(template, &config, &registry);
    assert!(
        matches!(status, ProfileTemplateStatus::Installed),
        "Changing Flutter's task config should not affect IntelliJ profile status, got {:?}",
        status
    );
}

// ── Group 4: Full Multi-Component Installation Scenarios ────────────

#[test]
fn install_order_a_vs_b_produces_identical_profiles() {
    let templates = load_real_templates();
    let registry = create_registry();

    let profile_ids = ["intellij-idea", "claude-code", "flutter-sdk"];

    for profile_id in &profile_ids {
        let template = templates.get(profile_id).unwrap();

        let ctx_a = ProfileTemplateManager::build_context(
            template,
            &registry,
            &HashMap::new(),
            &HashMap::new(),
            "/home/testuser",
        );
        let rendered_a = ProfileTemplateManager::render(template, &ctx_a);

        let ctx_b = ProfileTemplateManager::build_context(
            template,
            &registry,
            &HashMap::new(),
            &HashMap::new(),
            "/home/testuser",
        );
        let rendered_b = ProfileTemplateManager::render(template, &ctx_b);

        assert_eq!(
            rendered_a, rendered_b,
            "Profile '{}' renders differently between order A and order B",
            profile_id
        );
    }
}

#[test]
fn all_profiles_renderable_with_only_their_own_task_completed() {
    let templates = load_real_templates();
    let registry = create_registry();

    for template in templates.list() {
        let config_data = quartermaster_lib::config::manager::ConfigData::default();
        let mut ctx = ProfileTemplateManager::build_context(
            template,
            &registry,
            &HashMap::new(),
            &HashMap::new(),
            "/home/testuser",
        );
        let fragments = ProfileTemplateManager::resolve_fragments(
            template, &registry, &config_data, "/home/testuser",
        );
        ctx.insert("fragments".to_string(), fragments);
        let rendered = ProfileTemplateManager::render(template, &ctx);

        assert!(
            !rendered.contains("{{"),
            "Profile '{}' has unresolved placeholders when rendered with only its own task: {}",
            template.id,
            rendered
                .lines()
                .find(|l| l.contains("{{"))
                .unwrap_or("(not found)")
        );

        assert!(
            rendered.contains("deny /home/testuser/.ssh/id_*"),
            "Profile '{}' missing deny .ssh/id_* when rendered standalone",
            template.id
        );
        assert!(
            rendered.contains("deny /etc/shadow"),
            "Profile '{}' missing deny /etc/shadow when rendered standalone",
            template.id
        );
    }
}

// ── New tests ────────────────────────────────────────────────────────

#[test]
fn all_profile_template_task_ids_exist_in_registry() {
    let templates = load_real_templates();
    let registry = create_registry();
    let all_task_ids: Vec<&str> = registry.tasks().iter().map(|t| t.id()).collect();

    for template in templates.list() {
        assert!(
            all_task_ids.contains(&template.task_id.as_str()),
            "Profile template '{}' references task '{}', which does not exist in the registry",
            template.id,
            template.task_id
        );
    }
}

#[test]
fn all_profile_templates_pass_validation() {
    let templates = load_real_templates();

    for template in templates.list() {
        let errors = validate_profile_template(template);
        assert!(
            errors.is_empty(),
            "Profile template '{}' fails validation: {:?}",
            template.id,
            errors
        );
    }
}
