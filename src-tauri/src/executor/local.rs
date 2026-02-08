use std::path::Path;

use crate::error::AppError;
use super::{CommandExecutor, CommandOutput};

pub struct LocalExecutor {
    home: String,
}

impl LocalExecutor {
    pub fn new() -> Self {
        let home = dirs::home_dir()
            .map(|p| p.to_string_lossy().to_string())
            .unwrap_or_else(|| {
                std::env::var("HOME").unwrap_or_else(|_| "/root".to_string())
            });
        Self { home }
    }
}

impl Default for LocalExecutor {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait::async_trait]
impl CommandExecutor for LocalExecutor {
    async fn run_command(&self, cmd: &str, args: &[&str]) -> Result<CommandOutput, AppError> {
        let output = tokio::process::Command::new(cmd)
            .args(args)
            .current_dir(&self.home)
            .output()
            .await?;

        Ok(CommandOutput {
            status: output.status.code().unwrap_or(-1),
            stdout: String::from_utf8_lossy(&output.stdout).to_string(),
            stderr: String::from_utf8_lossy(&output.stderr).to_string(),
        })
    }

    async fn file_exists(&self, path: &str) -> Result<bool, AppError> {
        let expanded = shellexpand::tilde(path);
        Ok(Path::new(expanded.as_ref()).exists())
    }

    async fn read_file(&self, path: &str) -> Result<String, AppError> {
        let expanded = shellexpand::tilde(path);
        Ok(std::fs::read_to_string(expanded.as_ref())?)
    }

    async fn write_file(&self, path: &str, content: &str) -> Result<(), AppError> {
        let expanded = shellexpand::tilde(path);
        let p = Path::new(expanded.as_ref());
        if let Some(parent) = p.parent() {
            std::fs::create_dir_all(parent)?;
        }
        std::fs::write(p, content)?;
        Ok(())
    }

    async fn create_dir_all(&self, path: &str) -> Result<(), AppError> {
        let expanded = shellexpand::tilde(path);
        std::fs::create_dir_all(expanded.as_ref())?;
        Ok(())
    }

    fn home_dir(&self) -> String {
        self.home.clone()
    }

    fn is_local(&self) -> bool {
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn local_executor_runs_echo() {
        let exec = LocalExecutor::new();
        let result = exec.run_command("echo", &["hello"]).await.unwrap();
        assert_eq!(result.status, 0);
        assert_eq!(result.stdout.trim(), "hello");
    }

    #[tokio::test]
    async fn local_executor_file_exists() {
        let exec = LocalExecutor::new();
        assert!(exec.file_exists("/etc/hostname").await.unwrap());
        assert!(!exec.file_exists("/nonexistent/path").await.unwrap());
    }

    #[tokio::test]
    async fn local_executor_is_local() {
        let exec = LocalExecutor::new();
        assert!(exec.is_local());
    }

    #[tokio::test]
    async fn local_executor_create_and_read_file() {
        let dir = tempfile::tempdir().unwrap();
        let file_path = dir.path().join("test.txt");
        let exec = LocalExecutor::new();

        exec.write_file(file_path.to_str().unwrap(), "hello world").await.unwrap();
        let content = exec.read_file(file_path.to_str().unwrap()).await.unwrap();
        assert_eq!(content, "hello world");
    }
}
