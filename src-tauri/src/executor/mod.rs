pub mod local;
pub mod ssh;

use crate::error::AppError;

#[derive(Debug, Clone)]
pub struct CommandOutput {
    pub status: i32,
    pub stdout: String,
    pub stderr: String,
}

#[async_trait::async_trait]
pub trait CommandExecutor: Send + Sync {
    async fn run_command(&self, cmd: &str, args: &[&str]) -> Result<CommandOutput, AppError>;
    async fn file_exists(&self, path: &str) -> Result<bool, AppError>;
    async fn read_file(&self, path: &str) -> Result<String, AppError>;
    async fn write_file(&self, path: &str, content: &str) -> Result<(), AppError>;
    async fn create_dir_all(&self, path: &str) -> Result<(), AppError>;
    fn home_dir(&self) -> String;
    fn is_local(&self) -> bool;
}
