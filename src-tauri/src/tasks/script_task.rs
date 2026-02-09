use std::collections::HashMap;

use serde_json::Value;

use super::yaml_schema::TaskDefinition;
use super::{
    AppArmorInfo, ConfigField, DesktopInfo, DownloadInfo, ExecutionTarget, OutputCallback,
    PrivilegeLevel, ProgressCallback, SetupTask, StepInfo, StepOutput, TaskStatus,
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

    fn steps(&self) -> Vec<StepInfo> {
        self.definition.steps.iter().map(|s| StepInfo {
            name: s.name.clone(),
            progress: s.progress,
        }).collect()
    }

    fn uninstall_steps(&self) -> Vec<StepInfo> {
        self.definition.uninstall.iter().map(|s| StepInfo {
            name: s.name.clone(),
            progress: s.progress,
        }).collect()
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
        on_output: Option<&OutputCallback>,
    ) -> Result<(), AppError> {
        let ctx = self.build_context(config, &exec.home_dir());
        let total_steps = self.definition.steps.len();

        for (i, step) in self.definition.steps.iter().enumerate() {
            let progress = step.progress as f32 / 100.0;
            on_progress(progress, step.name.clone());

            let script = Self::expand_template(&step.run, &ctx);
            let start = std::time::Instant::now();
            let result = exec.run_command("sh", &["-c", &script]).await?;
            let duration_ms = start.elapsed().as_millis() as u64;

            if let Some(cb) = on_output {
                cb(StepOutput {
                    step_index: i,
                    step_total: total_steps,
                    step_name: step.name.clone(),
                    command: script.clone(),
                    stdout: result.stdout.clone(),
                    stderr: result.stderr.clone(),
                    exit_code: result.status,
                    duration_ms,
                });
            }

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
        on_output: Option<&OutputCallback>,
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
            let start = std::time::Instant::now();
            let result = exec.run_command("sh", &["-c", &script]).await?;
            let duration_ms = start.elapsed().as_millis() as u64;

            if let Some(cb) = on_output {
                cb(StepOutput {
                    step_index: i,
                    step_total: total_steps,
                    step_name: step.name.clone(),
                    command: script.clone(),
                    stdout: result.stdout.clone(),
                    stderr: result.stderr.clone(),
                    exit_code: result.status,
                    duration_ms,
                });
            }

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

    #[test]
    fn step_info_metadata() {
        let mut def = make_definition();
        def.steps = vec![
            StepDef { name: "Download".into(), progress: 30, run: "curl ...".into() },
            StepDef { name: "Install".into(), progress: 80, run: "dpkg -i ...".into() },
            StepDef { name: "Configure".into(), progress: 100, run: "echo done".into() },
        ];
        def.uninstall = vec![
            StepDef { name: "Remove files".into(), progress: 50, run: "rm -rf ...".into() },
            StepDef { name: "Clean config".into(), progress: 100, run: "rm ...".into() },
        ];
        let task = ScriptTask::new(def);

        let steps = task.steps();
        assert_eq!(steps.len(), 3);
        assert_eq!(steps[0].name, "Download");
        assert_eq!(steps[0].progress, 30);
        assert_eq!(steps[2].name, "Configure");
        assert_eq!(steps[2].progress, 100);

        let uninstall = task.uninstall_steps();
        assert_eq!(uninstall.len(), 2);
        assert_eq!(uninstall[0].name, "Remove files");
        assert_eq!(uninstall[1].progress, 100);
    }

    #[test]
    fn to_info_includes_steps() {
        let mut def = make_definition();
        def.steps = vec![
            StepDef { name: "Step A".into(), progress: 50, run: "echo a".into() },
            StepDef { name: "Step B".into(), progress: 100, run: "echo b".into() },
        ];
        def.uninstall = vec![
            StepDef { name: "Undo".into(), progress: 100, run: "echo undo".into() },
        ];
        let task = ScriptTask::new(def);
        let info = task.to_info(TaskStatus::NotStarted, None);

        assert_eq!(info.steps.len(), 2);
        assert_eq!(info.steps[0].name, "Step A");
        assert_eq!(info.steps[1].progress, 100);
        assert_eq!(info.uninstall_steps.len(), 1);
        assert_eq!(info.uninstall_steps[0].name, "Undo");
    }

    // ── Mock executor for async tests ─────────────────────────────────

    use std::sync::{Arc, Mutex as StdMutex};

    /// Records commands that were executed and returns configurable results.
    struct MockExecutor {
        home: String,
        /// Each call to run_command consumes the next result from this queue.
        /// If empty, returns success (exit code 0).
        results: StdMutex<Vec<crate::executor::CommandOutput>>,
        /// Records all commands that were executed.
        commands: StdMutex<Vec<(String, Vec<String>)>>,
    }

    impl MockExecutor {
        fn new() -> Self {
            Self {
                home: "/home/testuser".into(),
                results: StdMutex::new(vec![]),
                commands: StdMutex::new(vec![]),
            }
        }

        fn with_results(results: Vec<crate::executor::CommandOutput>) -> Self {
            Self {
                home: "/home/testuser".into(),
                results: StdMutex::new(results),
                commands: StdMutex::new(vec![]),
            }
        }

        fn executed_commands(&self) -> Vec<(String, Vec<String>)> {
            self.commands.lock().unwrap().clone()
        }
    }

    #[async_trait::async_trait]
    impl crate::executor::CommandExecutor for MockExecutor {
        async fn run_command(&self, cmd: &str, args: &[&str]) -> Result<crate::executor::CommandOutput, AppError> {
            self.commands.lock().unwrap().push((
                cmd.to_string(),
                args.iter().map(|a| a.to_string()).collect(),
            ));
            let mut results = self.results.lock().unwrap();
            if results.is_empty() {
                Ok(crate::executor::CommandOutput {
                    status: 0,
                    stdout: String::new(),
                    stderr: String::new(),
                })
            } else {
                Ok(results.remove(0))
            }
        }

        async fn file_exists(&self, _path: &str) -> Result<bool, AppError> {
            Ok(false)
        }

        async fn read_file(&self, _path: &str) -> Result<String, AppError> {
            Ok(String::new())
        }

        async fn write_file(&self, _path: &str, _content: &str) -> Result<(), AppError> {
            Ok(())
        }

        async fn create_dir_all(&self, _path: &str) -> Result<(), AppError> {
            Ok(())
        }

        fn home_dir(&self) -> String {
            self.home.clone()
        }

        fn is_local(&self) -> bool {
            true
        }
    }

    fn make_uninstallable_definition() -> TaskDefinition {
        let mut def = make_definition();
        def.uninstall = vec![
            StepDef { name: "Remove files".into(), progress: 50, run: "rm -rf {{path}}".into() },
            StepDef { name: "Clean config".into(), progress: 100, run: "rm ~/.config/test".into() },
        ];
        def
    }

    // ── Async uninstall tests ─────────────────────────────────────────

    #[tokio::test]
    async fn uninstall_succeeds_with_all_steps_passing() {
        let def = make_uninstallable_definition();
        let task = ScriptTask::new(def);
        let exec = MockExecutor::new();

        let progress_messages = Arc::new(StdMutex::new(vec![]));
        let pm = progress_messages.clone();
        let progress_cb: ProgressCallback = Box::new(move |p, msg| {
            pm.lock().unwrap().push((p, msg));
        });

        let result = task.uninstall(&HashMap::new(), &exec, &progress_cb, None).await;
        assert!(result.is_ok(), "Uninstall should succeed: {:?}", result);

        // Verify both steps were executed
        let cmds = exec.executed_commands();
        assert_eq!(cmds.len(), 2, "Should execute 2 uninstall commands");
        assert_eq!(cmds[0].0, "sh");
        assert_eq!(cmds[1].0, "sh");

        // Verify progress callbacks were called
        let messages = progress_messages.lock().unwrap();
        assert!(messages.len() >= 3, "Should have at least 3 progress calls (2 steps + completion)");
        // Last progress should be 1.0 with "Uninstall completed"
        let last = messages.last().unwrap();
        assert_eq!(last.0, 1.0);
        assert_eq!(last.1, "Uninstall completed");
    }

    #[tokio::test]
    async fn uninstall_fails_on_nonzero_exit_code() {
        let def = make_uninstallable_definition();
        let task = ScriptTask::new(def);
        let exec = MockExecutor::with_results(vec![
            // First step succeeds
            crate::executor::CommandOutput { status: 0, stdout: String::new(), stderr: String::new() },
            // Second step fails
            crate::executor::CommandOutput { status: 1, stdout: String::new(), stderr: "permission denied".into() },
        ]);

        let progress_cb: ProgressCallback = Box::new(|_, _| {});
        let result = task.uninstall(&HashMap::new(), &exec, &progress_cb, None).await;

        assert!(result.is_err());
        let err = result.unwrap_err().to_string();
        assert!(err.contains("Uninstall step 2 of 2 failed"), "Error should mention step 2: {}", err);
        assert!(err.contains("permission denied"), "Error should include stderr: {}", err);
    }

    #[tokio::test]
    async fn uninstall_returns_error_when_no_steps() {
        let def = make_definition(); // no uninstall steps
        let task = ScriptTask::new(def);
        let exec = MockExecutor::new();

        let progress_cb: ProgressCallback = Box::new(|_, _| {});
        let result = task.uninstall(&HashMap::new(), &exec, &progress_cb, None).await;

        assert!(result.is_err());
        let err = result.unwrap_err().to_string();
        assert!(err.contains("Uninstall not supported"), "Error should say not supported: {}", err);

        // No commands should have been executed
        assert!(exec.executed_commands().is_empty());
    }

    #[tokio::test]
    async fn uninstall_expands_config_variables_in_scripts() {
        let mut def = make_definition();
        def.uninstall = vec![
            StepDef { name: "Remove".into(), progress: 100, run: "rm -rf {{path}}".into() },
        ];
        let task = ScriptTask::new(def);

        let mut config = HashMap::new();
        config.insert("path".to_string(), serde_json::json!("~/custom"));

        let exec = MockExecutor::new();
        let progress_cb: ProgressCallback = Box::new(|_, _| {});
        let result = task.uninstall(&config, &exec, &progress_cb, None).await;
        assert!(result.is_ok());

        let cmds = exec.executed_commands();
        assert_eq!(cmds.len(), 1);
        // The script should have the expanded and shell-escaped path
        let script = &cmds[0].1[1]; // args[1] is the script passed to sh -c
        assert!(
            script.contains("/home/testuser/custom"),
            "Script should contain expanded path: {}",
            script
        );
        // Should NOT contain the raw template variable
        assert!(
            !script.contains("{{path}}"),
            "Script should not contain raw template variable: {}",
            script
        );
    }

    #[tokio::test]
    async fn uninstall_invokes_output_callback_for_each_step() {
        let def = make_uninstallable_definition();
        let task = ScriptTask::new(def);
        let exec = MockExecutor::new();

        let progress_cb: ProgressCallback = Box::new(|_, _| {});

        let outputs = Arc::new(StdMutex::new(vec![]));
        let outputs_clone = outputs.clone();
        let output_cb: OutputCallback = Box::new(move |step| {
            outputs_clone.lock().unwrap().push((
                step.step_index,
                step.step_total,
                step.step_name.clone(),
                step.exit_code,
            ));
        });

        let result = task.uninstall(&HashMap::new(), &exec, &progress_cb, Some(&output_cb)).await;
        assert!(result.is_ok());

        let recorded = outputs.lock().unwrap();
        assert_eq!(recorded.len(), 2, "Should have output for 2 steps");
        assert_eq!(recorded[0].0, 0); // step_index
        assert_eq!(recorded[0].1, 2); // step_total
        assert_eq!(recorded[0].2, "Remove files");
        assert_eq!(recorded[0].3, 0); // exit_code
        assert_eq!(recorded[1].0, 1);
        assert_eq!(recorded[1].2, "Clean config");
    }

    #[tokio::test]
    async fn uninstall_stops_at_first_failure() {
        let mut def = make_definition();
        def.uninstall = vec![
            StepDef { name: "Step 1".into(), progress: 33, run: "echo step1".into() },
            StepDef { name: "Step 2 (fails)".into(), progress: 66, run: "bad-cmd".into() },
            StepDef { name: "Step 3 (never runs)".into(), progress: 100, run: "echo step3".into() },
        ];
        let task = ScriptTask::new(def);
        let exec = MockExecutor::with_results(vec![
            crate::executor::CommandOutput { status: 0, stdout: "ok".into(), stderr: String::new() },
            crate::executor::CommandOutput { status: 127, stdout: String::new(), stderr: "command not found".into() },
            // Step 3 would succeed but should never be reached
            crate::executor::CommandOutput { status: 0, stdout: "ok".into(), stderr: String::new() },
        ]);

        let progress_cb: ProgressCallback = Box::new(|_, _| {});
        let result = task.uninstall(&HashMap::new(), &exec, &progress_cb, None).await;

        assert!(result.is_err());
        // Only 2 commands should have been executed (stops at failure)
        assert_eq!(exec.executed_commands().len(), 2);
    }

    #[test]
    fn supports_uninstall_false_without_steps() {
        let task = ScriptTask::new(make_definition());
        assert!(!task.supports_uninstall());
    }

    #[test]
    fn supports_uninstall_true_with_steps() {
        let task = ScriptTask::new(make_uninstallable_definition());
        assert!(task.supports_uninstall());
    }
}
