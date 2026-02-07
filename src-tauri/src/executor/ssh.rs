use std::time::Duration;

use async_trait::async_trait;
use tokio::time::timeout;

use crate::error::AppError;
use crate::fleet::{SshAuthMethod, SshConfig};
use super::{CommandExecutor, CommandOutput};

/// Default timeout for SSH command execution (5 minutes).
const SSH_COMMAND_TIMEOUT: Duration = Duration::from_secs(300);

/// How the SSH connection should authenticate.
#[derive(Debug, Clone)]
enum AuthMode {
    /// Rely on the SSH agent or default keys (`BatchMode=yes`).
    AgentOrDefault,
    /// Explicit private key file (`-i <path>`, `BatchMode=yes`).
    KeyFile(String),
    /// Certificate-based (`-i <cert> -i <key>`, `BatchMode=yes`).
    Certificate { cert_path: String, key_path: String },
    /// Password via `sshpass -e` (password set in `SSHPASS` env).
    Password(String),
}

pub struct SshExecutor {
    host: String,
    port: u16,
    username: String,
    home_dir: String,
    auth_mode: AuthMode,
    proxy_jump: Option<String>,
}

impl SshExecutor {
    /// Basic constructor — agent/default key auth, no proxy jump.
    pub fn new(host: String, port: u16, username: String) -> Self {
        let home_dir = if username == "root" {
            "/root".to_string()
        } else {
            format!("/home/{}", username)
        };
        Self {
            host,
            port,
            username,
            home_dir,
            auth_mode: AuthMode::AgentOrDefault,
            proxy_jump: None,
        }
    }

    /// Construct from the fleet `SshConfig` model.
    ///
    /// For `Password` auth, `vault_password` must be provided (retrieved from
    /// the vault by the caller). For all other methods the vault is not needed.
    pub fn from_ssh_config(config: &SshConfig, vault_password: Option<&str>) -> Self {
        let home_dir = if config.username == "root" {
            "/root".to_string()
        } else {
            format!("/home/{}", config.username)
        };

        let auth_mode = match &config.auth_method {
            SshAuthMethod::Password { .. } => {
                let pw = vault_password.unwrap_or("").to_string();
                AuthMode::Password(pw)
            }
            SshAuthMethod::KeyFile { private_key_path } => {
                AuthMode::KeyFile(private_key_path.clone())
            }
            SshAuthMethod::Certificate {
                certificate_path,
                private_key_path,
            } => AuthMode::Certificate {
                cert_path: certificate_path.clone(),
                key_path: private_key_path.clone(),
            },
            SshAuthMethod::Fido2Resident { .. } | SshAuthMethod::Agent => {
                AuthMode::AgentOrDefault
            }
        };

        Self {
            host: config.host.clone(),
            port: config.port,
            username: config.username.clone(),
            home_dir,
            auth_mode,
            proxy_jump: config.proxy_jump.clone(),
        }
    }

    /// Build the base ssh command with standard options and auth-specific flags.
    fn ssh_command(&self) -> tokio::process::Command {
        let use_sshpass = matches!(self.auth_mode, AuthMode::Password(_));

        let mut cmd = if use_sshpass {
            let mut c = tokio::process::Command::new("sshpass");
            c.arg("-e"); // read password from SSHPASS env
            c.arg("ssh");
            c
        } else {
            tokio::process::Command::new("ssh")
        };

        cmd.args(["-p", &self.port.to_string()]);

        // Auth-specific flags
        match &self.auth_mode {
            AuthMode::AgentOrDefault => {
                cmd.args(["-o", "BatchMode=yes"]);
            }
            AuthMode::KeyFile(path) => {
                cmd.args(["-o", "BatchMode=yes"]);
                cmd.args(["-i", path]);
            }
            AuthMode::Certificate { cert_path, key_path } => {
                cmd.args(["-o", "BatchMode=yes"]);
                cmd.args(["-i", cert_path]);
                cmd.args(["-i", key_path]);
            }
            AuthMode::Password(pw) => {
                cmd.env("SSHPASS", pw);
            }
        }

        // ProxyJump
        if let Some(ref jump) = self.proxy_jump {
            cmd.args(["-J", jump]);
        }

        cmd.args([
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

        let output = timeout(SSH_COMMAND_TIMEOUT, self.ssh_command()
            .arg(&remote_cmd)
            .output())
            .await
            .map_err(|_| AppError::Ssh(format!(
                "SSH command timed out after {}s for {}@{}:{}",
                SSH_COMMAND_TIMEOUT.as_secs(), self.username, self.host, self.port
            )))?
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
