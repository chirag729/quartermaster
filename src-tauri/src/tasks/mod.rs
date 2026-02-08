pub mod embedded;
pub mod execution_log;
pub mod install_state;
pub mod registry;
pub mod script_task;
pub mod validate;
pub mod yaml_schema;

use std::collections::HashMap;
use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::error::AppError;
use crate::executor::CommandExecutor;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum TaskStatus {
    NotStarted,
    InProgress,
    Completed,
    Failed,
    Skipped,
    Unknown,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum PrivilegeLevel {
    User,
    Admin,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum ExecutionTarget {
    LocalOnly,
    RemoteOnly,
    Any,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConfigField {
    pub key: String,
    pub label: String,
    pub field_type: String,
    pub default_value: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub options: Option<Vec<String>>,
    pub required: bool,
}

/// V2 metadata about a task's download specification (exposed to frontend).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DownloadInfo {
    pub url: String,
    pub extract: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub checksum_sha256: Option<String>,
}

/// V2 metadata about a task's desktop entry (exposed to frontend).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DesktopInfo {
    pub name: String,
    pub exec: String,
    pub categories: Vec<String>,
}

/// V2 metadata about a task's AppArmor profile (exposed to frontend).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppArmorInfo {
    pub profile: String,
    pub abstractions: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskInfo {
    pub id: String,
    pub name: String,
    pub description: String,
    pub icon: String,
    pub category: String,
    pub tags: Vec<String>,
    pub privilege_level: PrivilegeLevel,
    pub execution_target: ExecutionTarget,
    pub depends_on: Vec<String>,
    pub config_schema: Vec<ConfigField>,
    pub status: TaskStatus,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error_message: Option<String>,

    // V2 fields
    #[serde(skip_serializing_if = "Option::is_none")]
    pub version: Option<String>,
    #[serde(default)]
    pub variables: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub download: Option<DownloadInfo>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub desktop: Option<DesktopInfo>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub apparmor: Option<AppArmorInfo>,
    pub supports_uninstall: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub installed_version: Option<String>,
    #[serde(default)]
    pub update_available: bool,
}

/// Output from a single step within a task execution.
/// Emitted to the frontend as a `task-output` event after each command completes.
#[derive(Debug, Clone, Serialize)]
pub struct StepOutput {
    pub step_index: usize,
    pub step_total: usize,
    pub step_name: String,
    pub command: String,
    pub stdout: String,
    pub stderr: String,
    pub exit_code: i32,
    pub duration_ms: u64,
}

pub type ProgressCallback = Box<dyn Fn(f32, String) + Send + Sync>;
pub type OutputCallback = Box<dyn Fn(StepOutput) + Send + Sync>;

#[async_trait::async_trait]
pub trait SetupTask: Send + Sync {
    fn id(&self) -> &str;
    fn name(&self) -> &str;
    fn description(&self) -> &str;
    fn icon(&self) -> &str;
    fn category(&self) -> &str;
    fn tags(&self) -> Vec<String> { vec![] }
    fn privilege_level(&self) -> PrivilegeLevel;
    fn execution_target(&self) -> ExecutionTarget { ExecutionTarget::Any }
    fn depends_on(&self) -> Vec<String> { vec![] }
    fn config_schema(&self) -> Vec<ConfigField>;
    fn version(&self) -> Option<String> { None }
    fn variables(&self) -> Vec<String> { vec![] }
    fn download_info(&self) -> Option<DownloadInfo> { None }
    fn desktop_info(&self) -> Option<DesktopInfo> { None }
    fn apparmor_info(&self) -> Option<AppArmorInfo> { None }
    async fn detect_state(&self, config: &HashMap<String, Value>, exec: &dyn CommandExecutor) -> TaskStatus;
    async fn execute(
        &self,
        config: &HashMap<String, Value>,
        exec: &dyn CommandExecutor,
        on_progress: &ProgressCallback,
        on_output: Option<&OutputCallback>,
    ) -> Result<(), AppError>;

    /// Returns the currently installed version of this task's software, if detectable.
    /// Default implementation returns None (version detection not supported).
    async fn detect_installed_version(
        &self,
        config: &HashMap<String, Value>,
        exec: &dyn CommandExecutor,
    ) -> Option<String> {
        let _ = (config, exec);
        None
    }

    async fn uninstall(
        &self,
        _config: &HashMap<String, Value>,
        _exec: &dyn CommandExecutor,
        _on_progress: &ProgressCallback,
        _on_output: Option<&OutputCallback>,
    ) -> Result<(), AppError> {
        Err(AppError::Task(format!(
            "Uninstall not supported for task '{}'",
            self.name()
        )))
    }

    fn supports_uninstall(&self) -> bool {
        false
    }

    fn to_info(&self, status: TaskStatus, error: Option<String>) -> TaskInfo {
        TaskInfo {
            id: self.id().to_string(),
            name: self.name().to_string(),
            description: self.description().to_string(),
            icon: self.icon().to_string(),
            category: self.category().to_string(),
            tags: self.tags(),
            privilege_level: self.privilege_level(),
            execution_target: self.execution_target(),
            depends_on: self.depends_on(),
            config_schema: self.config_schema(),
            status,
            error_message: error,
            version: self.version(),
            variables: self.variables(),
            download: self.download_info(),
            desktop: self.desktop_info(),
            apparmor: self.apparmor_info(),
            supports_uninstall: self.supports_uninstall(),
            installed_version: None,
            update_available: false,
        }
    }
}
