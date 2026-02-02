use async_trait::async_trait;

use crate::error::AppError;
use super::{CommandExecutor, CommandOutput};

pub struct SshExecutor {
    host: String,
    port: u16,
    username: String,
    home_dir: String,
}

impl SshExecutor {
    pub fn new(host: String, port: u16, username: String) -> Self {
        let home_dir = if username == "root" {
            "/root".to_string()
        } else {
            format!("/home/{}", username)
        };
        Self { host, port, username, home_dir }
    }

    /// Build the base ssh command with standard options.
    fn ssh_command(&self) -> tokio::process::Command {
        let mut cmd = tokio::process::Command::new("ssh");
        cmd.args([
            "-p", &self.port.to_string(),
            "-o", "BatchMode=yes",
            "-o", "ConnectTimeout=10",
            "-o", "StrictHostKeyChecking=accept-new",
            &format!("{}@{}", self.username, self.host),
        ]);
        cmd
    }
}

/// Escape a string for safe use in a remote shell command.
fn shell_escape(s: &str) -> String {
    format!("'{}'", s.replace('\'', "'\\''"))
}

#[async_trait]
impl CommandExecutor for SshExecutor {
    async fn run_command(&self, cmd: &str, args: &[&str]) -> Result<CommandOutput, AppError> {
        let remote_cmd = if args.is_empty() {
            cmd.to_string()
        } else {
            let escaped_args: Vec<String> = args.iter().map(|a| shell_escape(a)).collect();
            format!("{} {}", cmd, escaped_args.join(" "))
        };

        let output = self.ssh_command()
            .arg(&remote_cmd)
            .output()
            .await
            .map_err(|e| AppError::Ssh(format!(
                "SSH command failed for {}@{}:{}: {}",
                self.username, self.host, self.port, e
            )))?;

        Ok(CommandOutput {
            status: output.status.code().unwrap_or(-1),
            stdout: String::from_utf8_lossy(&output.stdout).to_string(),
            stderr: String::from_utf8_lossy(&output.stderr).to_string(),
        })
    }

    async fn file_exists(&self, path: &str) -> Result<bool, AppError> {
        let output = self.run_command("test", &["-e", path]).await?;
        Ok(output.status == 0)
    }

    async fn read_file(&self, path: &str) -> Result<String, AppError> {
        let output = self.run_command("cat", &[path]).await?;
        if output.status != 0 {
            return Err(AppError::Ssh(format!("Failed to read file: {}", output.stderr)));
        }
        Ok(output.stdout)
    }

    async fn write_file(&self, path: &str, content: &str) -> Result<(), AppError> {
        // Pipe content via stdin to avoid shell escaping issues
        use tokio::io::AsyncWriteExt;

        let remote_cmd = format!("cat > {}", shell_escape(path));
        let mut child = self.ssh_command()
            .arg(&remote_cmd)
            .stdin(std::process::Stdio::piped())
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::piped())
            .spawn()
            .map_err(|e| AppError::Ssh(format!("Failed to start ssh: {}", e)))?;

        if let Some(mut stdin) = child.stdin.take() {
            stdin.write_all(content.as_bytes()).await
                .map_err(|e| AppError::Ssh(format!("Failed to write to stdin: {}", e)))?;
            drop(stdin);
        }

        let output = child.wait_with_output().await
            .map_err(|e| AppError::Ssh(format!("SSH write_file failed: {}", e)))?;

        if !output.status.success() {
            return Err(AppError::Ssh(format!("Failed to write file: {}", String::from_utf8_lossy(&output.stderr))));
        }
        Ok(())
    }

    async fn create_dir_all(&self, path: &str) -> Result<(), AppError> {
        let output = self.run_command("mkdir", &["-p", path]).await?;
        if output.status != 0 {
            return Err(AppError::Ssh(format!("Failed to create directory: {}", output.stderr)));
        }
        Ok(())
    }

    fn home_dir(&self) -> String {
        self.home_dir.clone()
    }

    fn is_local(&self) -> bool {
        false
    }
}
