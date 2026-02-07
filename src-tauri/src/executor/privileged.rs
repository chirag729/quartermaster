use crate::error::AppError;
use super::{CommandExecutor, CommandOutput};
use super::local::LocalExecutor;

/// Wraps a LocalExecutor to run commands through pkexec for privilege escalation.
///
/// Used for tasks with `PrivilegeLevel::Admin` running on local nodes.
/// - `run_command` calls are executed via `pkexec <cmd> <args...>`
/// - `write_file` and `create_dir_all` use pkexec with shell commands
/// - `file_exists`, `read_file`, `home_dir` delegate to the inner executor
///   since they don't require elevated privileges
pub struct PrivilegedLocalExecutor {
    inner: LocalExecutor,
}

impl PrivilegedLocalExecutor {
    pub fn new() -> Self {
        Self {
            inner: LocalExecutor::new(),
        }
    }
}

#[async_trait::async_trait]
impl CommandExecutor for PrivilegedLocalExecutor {
    async fn run_command(&self, cmd: &str, args: &[&str]) -> Result<CommandOutput, AppError> {
        let mut pkexec_args = vec![cmd];
        pkexec_args.extend_from_slice(args);

        let output = tokio::process::Command::new("pkexec")
            .args(&pkexec_args)
            .output()
            .await?;

        Ok(CommandOutput {
            status: output.status.code().unwrap_or(-1),
            stdout: String::from_utf8_lossy(&output.stdout).to_string(),
            stderr: String::from_utf8_lossy(&output.stderr).to_string(),
        })
    }

    async fn file_exists(&self, path: &str) -> Result<bool, AppError> {
        self.inner.file_exists(path).await
    }

    async fn read_file(&self, path: &str) -> Result<String, AppError> {
        self.inner.read_file(path).await
    }

    async fn write_file(&self, path: &str, content: &str) -> Result<(), AppError> {
        // Write to a temp file first, then use pkexec to move it into place
        let tmp_dir = crate::dirs::cache_dir();
        std::fs::create_dir_all(&tmp_dir)?;
        let tmp_path = format!("{}/privileged_write.tmp", tmp_dir.display());
        std::fs::write(&tmp_path, content)?;

        let escaped_tmp = tmp_path.replace('\'', "'\\''");
        let escaped_dest = path.replace('\'', "'\\''");
        let script = format!(
            "mkdir -p \"$(dirname '{}')\" && cp -- '{}' '{}' && rm -f '{}'",
            escaped_dest, escaped_tmp, escaped_dest, escaped_tmp
        );

        let output = tokio::process::Command::new("pkexec")
            .args(["sh", "-c", &script])
            .output()
            .await?;

        if output.status.success() {
            Ok(())
        } else {
            // Clean up temp file on failure
            let _ = std::fs::remove_file(&tmp_path);
            let stderr = String::from_utf8_lossy(&output.stderr);
            Err(AppError::Task(format!("Privileged write failed: {}", stderr)))
        }
    }

    async fn create_dir_all(&self, path: &str) -> Result<(), AppError> {
        let escaped = path.replace('\'', "'\\''");
        let output = tokio::process::Command::new("pkexec")
            .args(["mkdir", "-p", &escaped])
            .output()
            .await?;

        if output.status.success() {
            Ok(())
        } else {
            let stderr = String::from_utf8_lossy(&output.stderr);
            Err(AppError::Task(format!("Privileged mkdir failed: {}", stderr)))
        }
    }

    fn home_dir(&self) -> String {
        self.inner.home_dir()
    }

    fn is_local(&self) -> bool {
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn privileged_executor_is_local() {
        let exec = PrivilegedLocalExecutor::new();
        assert!(exec.is_local());
    }

    #[test]
    fn privileged_executor_home_dir_matches_local() {
        let privileged = PrivilegedLocalExecutor::new();
        let local = LocalExecutor::new();
        assert_eq!(privileged.home_dir(), local.home_dir());
    }
}
