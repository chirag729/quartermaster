use std::collections::HashMap;
use std::path::PathBuf;

use serde_json::Value;

use super::profile_manager::run_apparmor_helper;
use super::template_schema::{
    validate_profile_template, ProfileTemplate, ProfileTemplateConfig, ProfileTemplateInfo,
    ProfileTemplateStatus, ResolvedConfigField,
};
use crate::config::manager::{ConfigManager, InstalledProfileState};
use crate::error::AppError;
use crate::tasks::registry::TaskRegistry;

pub struct ProfileTemplateManager {
    templates: Vec<ProfileTemplate>,
}

impl ProfileTemplateManager {
    /// Load built-in profile templates from embedded YAML and user templates
    /// from ~/.config/quartermaster/profiles/.
    pub fn load() -> Self {
        let mut templates = Vec::new();

        // Load built-in templates
        for yaml_str in crate::tasks::embedded::BUILTIN_PROFILE_YAMLS {
            match serde_yaml::from_str::<ProfileTemplate>(yaml_str) {
                Ok(template) => {
                    let errors = validate_profile_template(&template);
                    if errors.is_empty() {
                        templates.push(template);
                    } else {
                        eprintln!(
                            "Skipping invalid built-in profile template: {}",
                            errors.join("; ")
                        );
                    }
                }
                Err(e) => {
                    eprintln!("Failed to parse built-in profile template: {}", e);
                }
            }
        }

        // Load user templates from ~/.config/quartermaster/profiles/
        let user_dir = Self::user_profiles_dir();
        if let Ok(entries) = std::fs::read_dir(&user_dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                if path
                    .extension()
                    .map_or(false, |e| e == "yaml" || e == "yml")
                {
                    match std::fs::read_to_string(&path) {
                        Ok(content) => match serde_yaml::from_str::<ProfileTemplate>(&content) {
                            Ok(template) => {
                                let errors = validate_profile_template(&template);
                                if errors.is_empty() {
                                    // Don't add if a template with same ID already exists
                                    if !templates.iter().any(|t| t.id == template.id) {
                                        templates.push(template);
                                    } else {
                                        eprintln!(
                                            "Skipping user profile template with duplicate ID '{}' from {}",
                                            template.id,
                                            path.display()
                                        );
                                    }
                                } else {
                                    eprintln!(
                                        "Skipping invalid user profile template {}: {}",
                                        path.display(),
                                        errors.join("; ")
                                    );
                                }
                            }
                            Err(e) => {
                                eprintln!(
                                    "Failed to parse user profile template {}: {}",
                                    path.display(),
                                    e
                                );
                            }
                        },
                        Err(e) => {
                            eprintln!(
                                "Failed to read user profile template {}: {}",
                                path.display(),
                                e
                            );
                        }
                    }
                }
            }
        }

        Self { templates }
    }

    fn user_profiles_dir() -> PathBuf {
        let base = dirs::config_dir().unwrap_or_else(|| PathBuf::from("~/.config"));
        base.join("quartermaster").join("profiles")
    }

    pub fn get(&self, id: &str) -> Option<&ProfileTemplate> {
        self.templates.iter().find(|t| t.id == id)
    }

    pub fn list(&self) -> &[ProfileTemplate] {
        &self.templates
    }

    pub fn templates_for_task(&self, task_id: &str) -> Vec<&ProfileTemplate> {
        self.templates
            .iter()
            .filter(|t| t.task_id == task_id)
            .collect()
    }

    /// Build the full variable context for rendering a template.
    ///
    /// Resolution order:
    /// 1. `home` — user's home directory
    /// 2. `mode` — the profile's mode field
    /// 3. Task config fields — from user overrides → task defaults (~ expanded on path types)
    /// 4. Profile variables — from user overrides → variable defaults (~ expanded on path types)
    ///
    /// Profile variables shadow task config keys if both define the same key.
    pub fn build_context(
        template: &ProfileTemplate,
        registry: &TaskRegistry,
        task_config: &HashMap<String, Value>,
        profile_config: &HashMap<String, Value>,
        home: &str,
    ) -> HashMap<String, String> {
        let mut ctx = HashMap::new();

        // 1. Always-available variables
        ctx.insert("home".to_string(), home.to_string());

        // 2. Mode
        ctx.insert("mode".to_string(), template.mode.clone());

        // 3. Task config fields
        if let Some(task) = registry.get(&template.task_id) {
            for field in task.config_schema() {
                let value = task_config
                    .get(&field.key)
                    .and_then(|v| v.as_str())
                    .unwrap_or(&field.default_value)
                    .to_string();

                let expanded = if field.field_type == "path" {
                    value.replace('~', home)
                } else {
                    value
                };

                ctx.insert(field.key.clone(), expanded);
            }
        }

        // 4. Profile variables (shadow task config keys)
        for var in &template.variables {
            let value = profile_config
                .get(&var.key)
                .and_then(|v| v.as_str())
                .unwrap_or(&var.default)
                .to_string();

            let expanded = if var.var_type == "path" {
                value.replace('~', home)
            } else {
                value
            };

            ctx.insert(var.key.clone(), expanded);
        }

        ctx
    }

    /// Render a template's content by replacing {{var}} placeholders.
    pub fn render(template: &ProfileTemplate, context: &HashMap<String, String>) -> String {
        let mut result = template.content.clone();
        for (key, value) in context {
            let placeholder = format!("{{{{{}}}}}", key);
            result = result.replace(&placeholder, value);
        }
        result
    }

    /// Get the status of a profile template.
    pub fn get_status(
        template: &ProfileTemplate,
        config: &ConfigManager,
        registry: &TaskRegistry,
    ) -> ProfileTemplateStatus {
        // Check if linked task is completed
        if !config.data.completed_tasks.contains(&template.task_id) {
            return ProfileTemplateStatus::TaskNotCompleted;
        }

        match config.data.installed_profiles.get(&template.id) {
            None => ProfileTemplateStatus::NotInstalled,
            Some(installed) => {
                let home = dirs::home_dir()
                    .unwrap_or_else(|| PathBuf::from("/home/unknown"))
                    .to_string_lossy()
                    .to_string();

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

                let current_context =
                    Self::build_context(template, registry, &task_config, &profile_config, &home);

                if current_context == installed.config_snapshot {
                    ProfileTemplateStatus::Installed
                } else {
                    ProfileTemplateStatus::Stale
                }
            }
        }
    }

    /// Install a profile template: render, write to /etc/apparmor.d/, and track state.
    pub async fn install_profile(
        template_id: &str,
        templates: &ProfileTemplateManager,
        registry: &TaskRegistry,
        config: &mut ConfigManager,
    ) -> Result<(), AppError> {
        let template = templates
            .get(template_id)
            .ok_or_else(|| AppError::AppArmor(format!("Unknown profile template: {}", template_id)))?
            .clone();

        let home = dirs::home_dir()
            .unwrap_or_else(|| PathBuf::from("/home/unknown"))
            .to_string_lossy()
            .to_string();

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

        let context =
            Self::build_context(&template, registry, &task_config, &profile_config, &home);
        let rendered = Self::render(&template, &context);

        // Write to a secure temp file
        let temp_file = tempfile::Builder::new()
            .prefix("qm-profile-")
            .tempfile()
            .map_err(|e| AppError::AppArmor(format!("Failed to create temp file: {}", e)))?;

        std::fs::write(temp_file.path(), &rendered)?;

        let temp_str = temp_file.path().to_string_lossy().to_string();
        let dest = format!("/etc/apparmor.d/{}", template.profile_name);

        let output = run_apparmor_helper(&["copy-and-reload", &temp_str, &dest]).await?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(AppError::AppArmor(format!(
                "Failed to install profile: {}",
                stderr
            )));
        }

        // Track installed state
        config.data.installed_profiles.insert(
            template.id.clone(),
            InstalledProfileState {
                profile_id: template.id.clone(),
                profile_name: template.profile_name.clone(),
                config_snapshot: context,
                installed_at: chrono::Utc::now().to_rfc3339(),
            },
        );
        config.save()?;

        Ok(())
    }

    /// Uninstall a profile template: remove from /etc/apparmor.d/ and untrack.
    pub async fn uninstall_profile(
        template_id: &str,
        templates: &ProfileTemplateManager,
        config: &mut ConfigManager,
    ) -> Result<(), AppError> {
        // Get profile_name from template or installed state
        let profile_name = if let Some(template) = templates.get(template_id) {
            template.profile_name.clone()
        } else if let Some(installed) = config.data.installed_profiles.get(template_id) {
            installed.profile_name.clone()
        } else {
            return Err(AppError::AppArmor(format!(
                "Unknown profile template: {}",
                template_id
            )));
        };

        let profile_path = format!("/etc/apparmor.d/{}", profile_name);

        let output = run_apparmor_helper(&["remove-and-unload", &profile_path]).await?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(AppError::AppArmor(format!(
                "Failed to uninstall profile: {}",
                stderr
            )));
        }

        config.data.installed_profiles.remove(template_id);
        config.save()?;

        Ok(())
    }

    /// Sync a single profile: re-install if stale, return whether it was updated.
    pub async fn sync_profile(
        template_id: &str,
        templates: &ProfileTemplateManager,
        registry: &TaskRegistry,
        config: &mut ConfigManager,
    ) -> Result<bool, AppError> {
        let template = templates
            .get(template_id)
            .ok_or_else(|| AppError::AppArmor(format!("Unknown profile template: {}", template_id)))?;

        let status = Self::get_status(template, config, registry);

        match status {
            ProfileTemplateStatus::Stale => {
                Self::install_profile(template_id, templates, registry, config).await?;
                Ok(true)
            }
            _ => Ok(false),
        }
    }

    /// Build the resolved config fields for a template (for UI display).
    pub fn build_config_fields(
        template: &ProfileTemplate,
        registry: &TaskRegistry,
        task_config: &HashMap<String, Value>,
        profile_config: &HashMap<String, Value>,
        home: &str,
    ) -> ProfileTemplateConfig {
        let mut fields = Vec::new();

        // Task config fields
        if let Some(task) = registry.get(&template.task_id) {
            for field in task.config_schema() {
                let value = task_config
                    .get(&field.key)
                    .and_then(|v| v.as_str())
                    .unwrap_or(&field.default_value)
                    .to_string();

                let expanded = if field.field_type == "path" {
                    value.replace('~', home)
                } else {
                    value
                };

                fields.push(ResolvedConfigField {
                    key: field.key.clone(),
                    label: field.label.clone(),
                    field_type: field.field_type.clone(),
                    value: expanded,
                    default: field.default_value.clone(),
                    options: field.options.clone(),
                    source: "task".to_string(),
                });
            }
        }

        // Profile variables
        for var in &template.variables {
            let value = profile_config
                .get(&var.key)
                .and_then(|v| v.as_str())
                .unwrap_or(&var.default)
                .to_string();

            let expanded = if var.var_type == "path" {
                value.replace('~', home)
            } else {
                value
            };

            fields.push(ResolvedConfigField {
                key: var.key.clone(),
                label: var.label.clone(),
                field_type: var.var_type.clone(),
                value: expanded,
                default: var.default.clone(),
                options: var.options.clone(),
                source: "profile".to_string(),
            });
        }

        ProfileTemplateConfig {
            template_id: template.id.clone(),
            fields,
        }
    }

    /// Convert a template to ProfileTemplateInfo with status.
    pub fn to_info(
        template: &ProfileTemplate,
        status: ProfileTemplateStatus,
    ) -> ProfileTemplateInfo {
        ProfileTemplateInfo {
            id: template.id.clone(),
            name: template.name.clone(),
            description: template.description.clone(),
            task_id: template.task_id.clone(),
            profile_name: template.profile_name.clone(),
            mode: template.mode.clone(),
            variables: template.variables.clone(),
            status,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use super::super::template_schema::ProfileVariable;
    use crate::tasks::registry::create_registry;

    fn make_test_template() -> ProfileTemplate {
        ProfileTemplate {
            id: "test-profile".into(),
            name: "Test Profile".into(),
            description: "A test profile".into(),
            task_id: "flutter-sdk".into(),
            profile_name: "quartermaster.test".into(),
            mode: "complain".into(),
            variables: vec![ProfileVariable {
                key: "projects_path".into(),
                label: "Projects Directory".into(),
                var_type: "path".into(),
                default: "~/Development/Projects".into(),
                options: None,
            }],
            content: "profile test {{sdk_base_path}}/flutter flags=({{mode}}) {\n  {{home}}/.pub-cache/** rwk,\n  {{projects_path}}/** rwk,\n}".into(),
        }
    }

    #[test]
    fn build_context_with_defaults() {
        let registry = create_registry();
        let template = make_test_template();
        let task_config = HashMap::new();
        let profile_config = HashMap::new();

        let ctx =
            ProfileTemplateManager::build_context(&template, &registry, &task_config, &profile_config, "/home/user");

        assert_eq!(ctx.get("home").unwrap(), "/home/user");
        assert_eq!(ctx.get("mode").unwrap(), "complain");
        // flutter-sdk task has sdk_base_path default "~/.local/share/sdk"
        assert_eq!(ctx.get("sdk_base_path").unwrap(), "/home/user/.local/share/sdk");
        // profile variable default
        assert_eq!(
            ctx.get("projects_path").unwrap(),
            "/home/user/Development/Projects"
        );
    }

    #[test]
    fn build_context_with_task_config_overrides() {
        let registry = create_registry();
        let template = make_test_template();
        let mut task_config = HashMap::new();
        task_config.insert(
            "sdk_base_path".to_string(),
            Value::String("~/CustomSDK".into()),
        );
        let profile_config = HashMap::new();

        let ctx =
            ProfileTemplateManager::build_context(&template, &registry, &task_config, &profile_config, "/home/user");

        assert_eq!(ctx.get("sdk_base_path").unwrap(), "/home/user/CustomSDK");
    }

    #[test]
    fn build_context_with_profile_config_overrides() {
        let registry = create_registry();
        let template = make_test_template();
        let task_config = HashMap::new();
        let mut profile_config = HashMap::new();
        profile_config.insert(
            "projects_path".to_string(),
            Value::String("~/MyProjects".into()),
        );

        let ctx =
            ProfileTemplateManager::build_context(&template, &registry, &task_config, &profile_config, "/home/user");

        assert_eq!(
            ctx.get("projects_path").unwrap(),
            "/home/user/MyProjects"
        );
    }

    #[test]
    fn build_context_profile_var_shadows_task_config() {
        let registry = create_registry();
        // Create a template where a profile variable has the same key as a task config field
        let mut template = make_test_template();
        template.variables.push(ProfileVariable {
            key: "sdk_base_path".into(),
            label: "SDK Path Override".into(),
            var_type: "path".into(),
            default: "~/OverriddenSDK".into(),
            options: None,
        });

        let task_config = HashMap::new();
        let profile_config = HashMap::new();

        let ctx =
            ProfileTemplateManager::build_context(&template, &registry, &task_config, &profile_config, "/home/user");

        // Profile variable should shadow the task config field
        assert_eq!(
            ctx.get("sdk_base_path").unwrap(),
            "/home/user/OverriddenSDK"
        );
    }

    #[test]
    fn build_context_tilde_expansion_on_path_types() {
        let registry = create_registry();
        let template = make_test_template();
        let task_config = HashMap::new();
        let profile_config = HashMap::new();

        let ctx =
            ProfileTemplateManager::build_context(&template, &registry, &task_config, &profile_config, "/home/user");

        // sdk_base_path (task config, path type, default "~/.local/share/sdk") should be expanded
        assert_eq!(ctx.get("sdk_base_path").unwrap(), "/home/user/.local/share/sdk");
        // projects_path (profile variable, path type, default "~/Development/Projects") should be expanded
        assert_eq!(
            ctx.get("projects_path").unwrap(),
            "/home/user/Development/Projects"
        );
    }

    #[test]
    fn render_template_replaces_variables() {
        let template = make_test_template();
        let mut context = HashMap::new();
        context.insert("home".to_string(), "/home/user".to_string());
        context.insert("mode".to_string(), "complain".to_string());
        context.insert("sdk_base_path".to_string(), "/home/user/SDK".to_string());
        context.insert(
            "projects_path".to_string(),
            "/home/user/Projects".to_string(),
        );

        let rendered = ProfileTemplateManager::render(&template, &context);

        assert!(rendered.contains("/home/user/SDK/flutter flags=(complain)"));
        assert!(rendered.contains("/home/user/.pub-cache/** rwk,"));
        assert!(rendered.contains("/home/user/Projects/** rwk,"));
        assert!(!rendered.contains("{{"));
    }

    #[test]
    fn status_task_not_completed() {
        let registry = create_registry();
        let template = make_test_template();

        let config = ConfigManager::default();
        let status = ProfileTemplateManager::get_status(&template, &config, &registry);

        assert!(matches!(status, ProfileTemplateStatus::TaskNotCompleted));
    }

    #[test]
    fn status_not_installed() {
        let registry = create_registry();
        let template = make_test_template();

        let mut config = ConfigManager::default();
        config
            .data
            .completed_tasks
            .push("flutter-sdk".to_string());

        let status = ProfileTemplateManager::get_status(&template, &config, &registry);

        assert!(matches!(status, ProfileTemplateStatus::NotInstalled));
    }

    #[test]
    fn status_installed_matches_snapshot() {
        let registry = create_registry();
        let template = make_test_template();

        let mut config = ConfigManager::default();
        config
            .data
            .completed_tasks
            .push("flutter-sdk".to_string());

        // Build context and install with matching snapshot
        let home = dirs::home_dir()
            .unwrap_or_else(|| PathBuf::from("/home/unknown"))
            .to_string_lossy()
            .to_string();
        let context = ProfileTemplateManager::build_context(
            &template,
            &registry,
            &HashMap::new(),
            &HashMap::new(),
            &home,
        );

        config.data.installed_profiles.insert(
            "test-profile".to_string(),
            InstalledProfileState {
                profile_id: "test-profile".to_string(),
                profile_name: "quartermaster.test".to_string(),
                config_snapshot: context,
                installed_at: "2025-01-01T00:00:00Z".to_string(),
            },
        );

        let status = ProfileTemplateManager::get_status(&template, &config, &registry);

        assert!(matches!(status, ProfileTemplateStatus::Installed));
    }

    #[test]
    fn status_stale_when_config_changed() {
        let registry = create_registry();
        let template = make_test_template();

        let mut config = ConfigManager::default();
        config
            .data
            .completed_tasks
            .push("flutter-sdk".to_string());

        // Install with a different snapshot than current context
        let mut old_snapshot = HashMap::new();
        old_snapshot.insert("home".to_string(), "/home/user".to_string());
        old_snapshot.insert("mode".to_string(), "complain".to_string());
        old_snapshot.insert("sdk_base_path".to_string(), "/home/user/OldSDK".to_string());
        old_snapshot.insert(
            "projects_path".to_string(),
            "/home/user/OldProjects".to_string(),
        );

        config.data.installed_profiles.insert(
            "test-profile".to_string(),
            InstalledProfileState {
                profile_id: "test-profile".to_string(),
                profile_name: "quartermaster.test".to_string(),
                config_snapshot: old_snapshot,
                installed_at: "2025-01-01T00:00:00Z".to_string(),
            },
        );

        let status = ProfileTemplateManager::get_status(&template, &config, &registry);

        assert!(matches!(status, ProfileTemplateStatus::Stale));
    }

    #[test]
    fn build_config_fields_includes_task_and_profile_fields() {
        let registry = create_registry();
        let template = make_test_template();

        let home = "/home/user";
        let config_fields = ProfileTemplateManager::build_config_fields(
            &template,
            &registry,
            &HashMap::new(),
            &HashMap::new(),
            home,
        );

        assert_eq!(config_fields.template_id, "test-profile");

        // Should have task config fields (from flutter-sdk: channel, sdk_base_path)
        // plus profile variable (projects_path)
        let task_fields: Vec<_> = config_fields
            .fields
            .iter()
            .filter(|f| f.source == "task")
            .collect();
        let profile_fields: Vec<_> = config_fields
            .fields
            .iter()
            .filter(|f| f.source == "profile")
            .collect();

        assert!(task_fields.len() >= 2); // channel, sdk_base_path
        assert_eq!(profile_fields.len(), 1); // projects_path
        assert_eq!(profile_fields[0].key, "projects_path");
        assert_eq!(profile_fields[0].source, "profile");
    }

    #[test]
    fn to_info_preserves_fields() {
        let template = make_test_template();
        let info = ProfileTemplateManager::to_info(&template, ProfileTemplateStatus::Installed);

        assert_eq!(info.id, "test-profile");
        assert_eq!(info.name, "Test Profile");
        assert_eq!(info.task_id, "flutter-sdk");
        assert_eq!(info.profile_name, "quartermaster.test");
        assert_eq!(info.mode, "complain");
        assert_eq!(info.variables.len(), 1);
        assert!(matches!(info.status, ProfileTemplateStatus::Installed));
    }
}
