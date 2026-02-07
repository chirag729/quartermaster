use std::collections::HashMap;

use serde_json::Value;

use super::yaml_schema::TaskDefinition;
use super::{
    AppArmorInfo, ConfigField, DesktopInfo, DownloadInfo, ExecutionTarget, PrivilegeLevel,
    ProgressCallback, SetupTask, TaskStatus,
};
use crate::error::AppError;
use crate::executor::CommandExecutor;

pub struct ScriptTask {
    definition: TaskDefinition,
}

impl ScriptTask {
    pub fn new(definition: TaskDefinition) -> Self {
        Self { definition }
    }

    /// Escape a value for safe use inside a shell single-quoted string.
    /// Wraps in single quotes with internal `'` escaped as `'\''`.
    fn shell_escape_value(value: &str) -> String {
        let escaped = value.replace('\'', "'\\''");
        format!("'{}'", escaped)
    }

    /// Build template context from config overrides, falling back to definition defaults.
    /// Also adds `home` from the executor.
    /// All values are shell-escaped to prevent injection.
    fn build_context(
        &self,
        config: &HashMap<String, Value>,
        home: &str,
    ) -> HashMap<String, String> {
        let mut ctx = HashMap::new();
        ctx.insert("home".to_string(), Self::shell_escape_value(home));

        // Inject version if defined
        if let Some(ref version) = self.definition.version {
            ctx.insert("version".to_string(), Self::shell_escape_value(version));
        }

        for field in &self.definition.config {
            let value = config
                .get(&field.key)
                .map(|v| match v {
                    Value::String(s) => s.clone(),
                    other => other.to_string(),
                })
                .unwrap_or_else(|| field.default.clone());

            // Expand leading ~ in path-type values (only ~/... or standalone ~)
            let expanded = if field.field_type == "path" {
                if value == "~" {
                    home.to_string()
                } else if let Some(rest) = value.strip_prefix("~/") {
                    format!("{}/{}", home, rest)
                } else {
                    value
                }
            } else {
                value
            };

            ctx.insert(field.key.clone(), Self::shell_escape_value(&expanded));
        }

        ctx
    }

    /// Replace `{{var}}` placeholders in a script string.
    fn expand_template(script: &str, ctx: &HashMap<String, String>) -> String {
        let mut result = script.to_string();
        for (key, value) in ctx {
            let placeholder = format!("{{{{{}}}}}", key);
            result = result.replace(&placeholder, value);
        }
        result
    }
}

#[async_trait::async_trait]
impl SetupTask for ScriptTask {
    fn id(&self) -> &str {
        &self.definition.id
    }

    fn name(&self) -> &str {
        &self.definition.name
    }

    fn description(&self) -> &str {
        &self.definition.description
    }

    fn icon(&self) -> &str {
        &self.definition.icon
    }

    fn category(&self) -> &str {
        &self.definition.category
    }

    fn tags(&self) -> Vec<String> {
        self.definition.tags.clone()
    }

    fn privilege_level(&self) -> PrivilegeLevel {
        match self.definition.privilege.as_str() {
            "admin" => PrivilegeLevel::Admin,
            _ => PrivilegeLevel::User,
        }
    }

    fn execution_target(&self) -> ExecutionTarget {
        match self.definition.target.as_str() {
            "local_only" => ExecutionTarget::LocalOnly,
            "remote_only" => ExecutionTarget::RemoteOnly,
            _ => ExecutionTarget::Any,
        }
    }

    fn depends_on(&self) -> Vec<String> {
        self.definition.depends_on.clone()
    }

    fn version(&self) -> Option<String> {
        self.definition.version.clone()
    }

    fn variables(&self) -> Vec<String> {
        self.definition.variables.clone()
    }

    fn download_info(&self) -> Option<DownloadInfo> {
        self.definition.download.as_ref().map(|d| DownloadInfo {
            url: d.url.clone(),
            extract: d.extract.clone(),
            checksum_sha256: d.checksum_sha256.clone(),
        })
    }

    fn desktop_info(&self) -> Option<DesktopInfo> {
        self.definition.desktop.as_ref().map(|d| DesktopInfo {
            name: d.name.clone(),
            exec: d.exec.clone(),
            categories: d.categories.clone(),
        })
    }

    fn apparmor_info(&self) -> Option<AppArmorInfo> {
        self.definition.apparmor.as_ref().map(|a| AppArmorInfo {
            profile: a.profile.clone(),
            abstractions: a.abstractions.clone(),
        })
    }

    fn config_schema(&self) -> Vec<ConfigField> {
        self.definition
            .config
            .iter()
            .map(|f| ConfigField {
                key: f.key.clone(),
                label: f.label.clone(),
                field_type: f.field_type.clone(),
                default_value: f.default.clone(),
                options: f.options.clone(),
                required: f.required.unwrap_or(false),
            })
            .collect()
    }

    async fn detect_installed_version(
        &self,
        config: &HashMap<String, Value>,
        exec: &dyn CommandExecutor,
    ) -> Option<String> {
        let detect_cmd = self.definition.version_detect.as_ref()?;
        let ctx = self.build_context(config, &exec.home_dir());
        let script = Self::expand_template(detect_cmd, &ctx);

        match exec.run_command("sh", &["-c", &script]).await {
            Ok(output) if output.status == 0 => {
                let version = output.stdout.trim().to_string();
                if version.is_empty() { None } else { Some(version) }
            }
            _ => None,
        }
    }

    async fn detect_state(
        &self,
        config: &HashMap<String, Value>,
        exec: &dyn CommandExecutor,
    ) -> TaskStatus {
        let ctx = self.build_context(config, &exec.home_dir());
        let script = Self::expand_template(&self.definition.detect, &ctx);

        match exec.run_command("sh", &["-c", &script]).await {
            Ok(output) if output.status == 0 => TaskStatus::Completed,
            Ok(_) => TaskStatus::NotStarted, // Non-zero exit = not installed
            Err(e) => {
                eprintln!("Warning: detect_state for '{}' failed: {}", self.definition.id, e);
                TaskStatus::NotStarted
            }
        }
    }

    async fn execute(
        &self,
        config: &HashMap<String, Value>,
        exec: &dyn CommandExecutor,
        on_progress: &ProgressCallback,
    ) -> Result<(), AppError> {
        let ctx = self.build_context(config, &exec.home_dir());
        let total_steps = self.definition.steps.len();

        for (i, step) in self.definition.steps.iter().enumerate() {
            let progress = step.progress as f32 / 100.0;
            on_progress(progress, step.name.clone());

            let script = Self::expand_template(&step.run, &ctx);
            let result = exec.run_command("sh", &["-c", &script]).await?;

            if result.status != 0 {
                return Err(AppError::Task(format!(
                    "Step {} of {} failed (\"{}\"): {}",
                    i + 1,
                    total_steps,
                    step.name,
                    result.stderr.trim()
                )));
            }
        }

        on_progress(1.0, "Completed".to_string());
        Ok(())
    }

    fn supports_uninstall(&self) -> bool {
        !self.definition.uninstall.is_empty()
    }

    async fn uninstall(
        &self,
        config: &HashMap<String, Value>,
        exec: &dyn CommandExecutor,
        on_progress: &ProgressCallback,
    ) -> Result<(), AppError> {
        if self.definition.uninstall.is_empty() {
            return Err(AppError::Task(format!(
                "Uninstall not supported for task '{}'",
                self.name()
            )));
        }

        let ctx = self.build_context(config, &exec.home_dir());
        let total_steps = self.definition.uninstall.len();

        for (i, step) in self.definition.uninstall.iter().enumerate() {
            let progress = step.progress as f32 / 100.0;
            on_progress(progress, step.name.clone());

            let script = Self::expand_template(&step.run, &ctx);
            let result = exec.run_command("sh", &["-c", &script]).await?;

            if result.status != 0 {
                return Err(AppError::Task(format!(
                    "Uninstall step {} of {} failed (\"{}\"): {}",
                    i + 1,
                    total_steps,
                    step.name,
                    result.stderr.trim()
                )));
            }
        }

        on_progress(1.0, "Uninstall completed".to_string());
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tasks::yaml_schema::{ConfigFieldDef, StepDef, TaskDefinition};

    fn make_definition() -> TaskDefinition {
        TaskDefinition {
            id: "test-task".into(),
            name: "Test Task".into(),
            description: "A test".into(),
            icon: "Star".into(),
            category: "Testing".into(),
            tags: vec!["test".into()],
            privilege: "user".into(),
            target: "any".into(),
            depends_on: vec!["other".into()],
            config: vec![ConfigFieldDef {
                key: "path".into(),
                label: "Path".into(),
                field_type: "path".into(),
                default: "~/test".into(),
                options: None,
                required: Some(true),
            }],
            detect: "test -d {{path}}".into(),
            steps: vec![StepDef {
                name: "Create dir".into(),
                progress: 100,
                run: "mkdir -p {{path}}".into(),
            }],
            version: None,
            variables: vec![],
            download: None,
            desktop: None,
            apparmor: None,
            uninstall: vec![],
            version_detect: None,
        }
    }

    #[test]
    fn script_task_metadata() {
        let task = ScriptTask::new(make_definition());
        assert_eq!(task.id(), "test-task");
        assert_eq!(task.name(), "Test Task");
        assert_eq!(task.category(), "Testing");
        assert_eq!(task.tags(), vec!["test".to_string()]);
        assert_eq!(task.privilege_level(), PrivilegeLevel::User);
        assert_eq!(task.execution_target(), ExecutionTarget::Any);
        assert_eq!(task.depends_on(), vec!["other".to_string()]);
    }

    #[test]
    fn config_schema_mapping() {
        let task = ScriptTask::new(make_definition());
        let schema = task.config_schema();
        assert_eq!(schema.len(), 1);
        assert_eq!(schema[0].key, "path");
        assert_eq!(schema[0].field_type, "path");
        assert_eq!(schema[0].default_value, "~/test");
        assert!(schema[0].required);
    }

    #[test]
    fn template_expansion() {
        let mut ctx = HashMap::new();
        ctx.insert("home".to_string(), "'/home/user'".to_string());
        ctx.insert("path".to_string(), "'/home/user/test'".to_string());

        let result = ScriptTask::expand_template("test -d {{path}} && echo {{home}}", &ctx);
        assert_eq!(result, "test -d '/home/user/test' && echo '/home/user'");
    }

    #[test]
    fn build_context_uses_defaults() {
        let task = ScriptTask::new(make_definition());
        let config = HashMap::new();
        let ctx = task.build_context(&config, "/home/user");
        assert_eq!(ctx.get("home").unwrap(), "'/home/user'");
        // path default is ~/test, ~ expanded to /home/user/test, then shell-escaped
        assert_eq!(ctx.get("path").unwrap(), "'/home/user/test'");
    }

    #[test]
    fn build_context_uses_overrides() {
        let task = ScriptTask::new(make_definition());
        let mut config = HashMap::new();
        config.insert(
            "path".to_string(),
            Value::String("~/custom".to_string()),
        );
        let ctx = task.build_context(&config, "/home/user");
        assert_eq!(ctx.get("path").unwrap(), "'/home/user/custom'");
    }

    #[test]
    fn shell_escape_basic() {
        assert_eq!(ScriptTask::shell_escape_value("hello"), "'hello'");
        assert_eq!(ScriptTask::shell_escape_value("/home/user"), "'/home/user'");
    }

    #[test]
    fn shell_escape_single_quotes() {
        assert_eq!(
            ScriptTask::shell_escape_value("it's a test"),
            "'it'\\''s a test'"
        );
    }

    #[test]
    fn shell_escape_special_chars() {
        assert_eq!(
            ScriptTask::shell_escape_value("$(whoami)"),
            "'$(whoami)'"
        );
        assert_eq!(
            ScriptTask::shell_escape_value("foo; rm -rf /"),
            "'foo; rm -rf /'"
        );
    }

    #[test]
    fn build_context_non_string_json_values() {
        let task = ScriptTask::new(make_definition());
        let mut config = HashMap::new();
        // Non-string value (number) should be converted to string
        config.insert("path".to_string(), serde_json::json!(42));
        let ctx = task.build_context(&config, "/home/user");
        // 42 is not a path starting with ~, so no tilde expansion, just shell-escaped
        assert_eq!(ctx.get("path").unwrap(), "'42'");
    }

    #[test]
    fn build_context_boolean_json_value() {
        let mut def = make_definition();
        def.config.push(ConfigFieldDef {
            key: "enabled".into(),
            label: "Enabled".into(),
            field_type: "boolean".into(),
            default: "true".into(),
            options: None,
            required: None,
        });
        let task = ScriptTask::new(def);
        let mut config = HashMap::new();
        config.insert("enabled".to_string(), serde_json::json!(false));
        let ctx = task.build_context(&config, "/home/user");
        assert_eq!(ctx.get("enabled").unwrap(), "'false'");
    }

    #[test]
    fn tilde_expansion_only_leading() {
        let task = ScriptTask::new(make_definition());
        let mut config = HashMap::new();
        // Tilde in the middle should NOT be expanded
        config.insert(
            "path".to_string(),
            Value::String("/some/path/~file".to_string()),
        );
        let ctx = task.build_context(&config, "/home/user");
        assert_eq!(ctx.get("path").unwrap(), "'/some/path/~file'");
    }

    #[test]
    fn tilde_expansion_standalone() {
        let task = ScriptTask::new(make_definition());
        let mut config = HashMap::new();
        config.insert("path".to_string(), Value::String("~".to_string()));
        let ctx = task.build_context(&config, "/home/user");
        assert_eq!(ctx.get("path").unwrap(), "'/home/user'");
    }

    #[test]
    fn admin_privilege() {
        let mut def = make_definition();
        def.privilege = "admin".into();
        let task = ScriptTask::new(def);
        assert_eq!(task.privilege_level(), PrivilegeLevel::Admin);
    }

    #[test]
    fn local_only_target() {
        let mut def = make_definition();
        def.target = "local_only".into();
        let task = ScriptTask::new(def);
        assert_eq!(task.execution_target(), ExecutionTarget::LocalOnly);
    }
}
