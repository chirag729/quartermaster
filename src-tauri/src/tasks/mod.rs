pub mod registry;
pub mod yaml_schema;
pub mod validate;
pub mod script_task;
pub mod embedded;

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
}

pub type ProgressCallback = Box<dyn Fn(f32, String) + Send + Sync>;

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
    async fn detect_state(&self, config: &HashMap<String, Value>, exec: &dyn CommandExecutor) -> TaskStatus;
    async fn execute(
        &self,
        config: &HashMap<String, Value>,
        exec: &dyn CommandExecutor,
        on_progress: &ProgressCallback,
    ) -> Result<(), AppError>;

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
        }
    }
}
