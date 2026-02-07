use std::sync::Mutex;

use serde::{Deserialize, Serialize};

use crate::error::AppError;
use super::{CommandExecutor, CommandOutput};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", tag = "type")]
pub enum DryRunAction {
    RunCommand { command: String, args: Vec<String> },
    WriteFile { path: String, content_length: usize },
    CreateDir { path: String },
}

pub struct DryRunExecutor {
    inner: Box<dyn CommandExecutor>,
    actions: Mutex<Vec<DryRunAction>>,
}

impl DryRunExecutor {
    pub fn new(inner: Box<dyn CommandExecutor>) -> Self {
        Self {
            inner,
            actions: Mutex::new(Vec::new()),
        }
    }

    /// Drains the recorded actions and returns them.
    pub fn take_actions(&self) -> Vec<DryRunAction> {
        let mut actions = self.actions.lock().unwrap();
        std::mem::take(&mut *actions)
    }
}

#[async_trait::async_trait]
impl CommandExecutor for DryRunExecutor {
    async fn run_command(&self, cmd: &str, args: &[&str]) -> Result<CommandOutput, AppError> {
        let action = DryRunAction::RunCommand {
            command: cmd.to_string(),
            args: args.iter().map(|a| a.to_string()).collect(),
        };
        self.actions.lock().unwrap().push(action);
        Ok(CommandOutput {
            status: 0,
            stdout: String::new(),
            stderr: String::new(),
        })
    }

    async fn file_exists(&self, path: &str) -> Result<bool, AppError> {
        self.inner.file_exists(path).await
    }

    async fn read_file(&self, path: &str) -> Result<String, AppError> {
        self.inner.read_file(path).await
    }

    async fn write_file(&self, path: &str, content: &str) -> Result<(), AppError> {
        let action = DryRunAction::WriteFile {
            path: path.to_string(),
            content_length: content.len(),
        };
        self.actions.lock().unwrap().push(action);
        Ok(())
    }

    async fn create_dir_all(&self, path: &str) -> Result<(), AppError> {
        let action = DryRunAction::CreateDir {
            path: path.to_string(),
        };
        self.actions.lock().unwrap().push(action);
        Ok(())
    }

    fn home_dir(&self) -> String {
        self.inner.home_dir()
    }

    fn is_local(&self) -> bool {
        self.inner.is_local()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::executor::local::LocalExecutor;

    #[tokio::test]
    async fn read_operations_pass_through() {
        let inner = Box::new(LocalExecutor::new());
        let dry_run = DryRunExecutor::new(inner);

        // file_exists should delegate to the real executor
        let exists = dry_run.file_exists("/etc/hostname").await.unwrap();
        assert!(exists);

        let not_exists = dry_run.file_exists("/nonexistent/path/xyz").await.unwrap();
        assert!(!not_exists);

        // home_dir should delegate
        let home = dry_run.home_dir();
        assert!(!home.is_empty());

        // is_local should delegate
        assert!(dry_run.is_local());

        // No actions should have been recorded for read operations
        let actions = dry_run.take_actions();
        assert!(actions.is_empty());
    }

    #[tokio::test]
    async fn write_file_is_intercepted() {
        let dir = tempfile::tempdir().unwrap();
        let file_path = dir.path().join("should_not_exist.txt");
        let path_str = file_path.to_str().unwrap();

        let inner = Box::new(LocalExecutor::new());
        let dry_run = DryRunExecutor::new(inner);

        let result = dry_run.write_file(path_str, "hello world").await;
        assert!(result.is_ok());

        // The file should NOT actually be created
        assert!(!file_path.exists());

        let actions = dry_run.take_actions();
        assert_eq!(actions.len(), 1);
        match &actions[0] {
            DryRunAction::WriteFile { path, content_length } => {
                assert_eq!(path, path_str);
                assert_eq!(*content_length, 11);
            }
            _ => panic!("Expected WriteFile action"),
        }
    }

    #[tokio::test]
    async fn run_command_is_intercepted() {
        let inner = Box::new(LocalExecutor::new());
        let dry_run = DryRunExecutor::new(inner);

        let output = dry_run.run_command("apt-get", &["install", "-y", "curl"]).await.unwrap();
        assert_eq!(output.status, 0);
        assert!(output.stdout.is_empty());
        assert!(output.stderr.is_empty());

        let actions = dry_run.take_actions();
        assert_eq!(actions.len(), 1);
        match &actions[0] {
            DryRunAction::RunCommand { command, args } => {
                assert_eq!(command, "apt-get");
                assert_eq!(args, &["install", "-y", "curl"]);
            }
            _ => panic!("Expected RunCommand action"),
        }
    }

    #[tokio::test]
    async fn create_dir_is_intercepted() {
        let dir = tempfile::tempdir().unwrap();
        let new_dir = dir.path().join("should/not/exist");
        let path_str = new_dir.to_str().unwrap();

        let inner = Box::new(LocalExecutor::new());
        let dry_run = DryRunExecutor::new(inner);

        let result = dry_run.create_dir_all(path_str).await;
        assert!(result.is_ok());

        // The directory should NOT actually be created
        assert!(!new_dir.exists());

        let actions = dry_run.take_actions();
        assert_eq!(actions.len(), 1);
        match &actions[0] {
            DryRunAction::CreateDir { path } => {
                assert_eq!(path, path_str);
            }
            _ => panic!("Expected CreateDir action"),
        }
    }

    #[tokio::test]
    async fn take_actions_clears_the_log() {
        let inner = Box::new(LocalExecutor::new());
        let dry_run = DryRunExecutor::new(inner);

        dry_run.run_command("echo", &["hello"]).await.unwrap();
        dry_run.write_file("/tmp/test", "data").await.unwrap();
        dry_run.create_dir_all("/tmp/newdir").await.unwrap();

        let actions = dry_run.take_actions();
        assert_eq!(actions.len(), 3);

        // After take_actions, the log should be empty
        let actions_after = dry_run.take_actions();
        assert!(actions_after.is_empty());
    }
}
