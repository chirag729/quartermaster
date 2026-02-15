//! YubiKey/SSH setup commands for remote node provisioning.
//!
//! Provides commands for checking remote SSH versions and hardening
//! remote sshd configuration as part of the YubiKey conversion wizard.

use serde::{Deserialize, Serialize};
use tauri::State;

use crate::error::AppError;
use crate::fleet::{SshAuthMethod, SshConfig};
use crate::state::AppState;

// ---------------------------------------------------------------------------
// SSH command builder helper
// ---------------------------------------------------------------------------

/// Build a `tokio::process::Command` that will execute a single remote command
/// over SSH, respecting all authentication methods.
///
/// This mirrors the logic in `SshExecutor::ssh_command()` but returns a
/// standalone command suitable for one-shot remote invocations.
fn build_ssh_command(
    ssh_config: &SshConfig,
    vault_password: Option<&str>,
    remote_command: &str,
) -> tokio::process::Command {
    let use_sshpass = matches!(ssh_config.auth_method, SshAuthMethod::Password { .. });

    let mut cmd = if use_sshpass {
        let mut c = tokio::process::Command::new("sshpass");
        c.arg("-e"); // read password from SSHPASS env
        c.arg("ssh");
        c
    } else {
        tokio::process::Command::new("ssh")
    };

    cmd.args(["-p", &ssh_config.port.to_string()]);

    // Auth-specific flags
    match &ssh_config.auth_method {
        SshAuthMethod::Password { .. } => {
            let pw = vault_password.unwrap_or("");
            cmd.env("SSHPASS", pw);
        }
        SshAuthMethod::KeyFile { private_key_path } => {
            cmd.args(["-o", "BatchMode=yes"]);
            cmd.args(["-i", private_key_path]);
        }
        SshAuthMethod::Certificate {
            certificate_path,
            private_key_path,
        } => {
            cmd.args(["-o", "BatchMode=yes"]);
            cmd.args(["-i", certificate_path]);
            cmd.args(["-i", private_key_path]);
        }
        SshAuthMethod::Fido2Resident { .. } | SshAuthMethod::Agent => {
            cmd.args(["-o", "BatchMode=yes"]);
        }
    }

    // ProxyJump
    if let Some(ref jump) = ssh_config.proxy_jump {
        cmd.args(["-J", jump]);
    }

    cmd.args([
        "-o", "ConnectTimeout=10",
        "-o", "StrictHostKeyChecking=accept-new",
        &format!("{}@{}", ssh_config.username, ssh_config.host),
        remote_command,
    ]);

    cmd
}

/// Execute a single remote command via SSH and return (stdout, stderr, success).
async fn run_remote_command(
    ssh_config: &SshConfig,
    vault_password: Option<&str>,
    remote_command: &str,
) -> Result<(String, String, bool), AppError> {
    let output = build_ssh_command(ssh_config, vault_password, remote_command)
        .output()
        .await
        .map_err(|e| AppError::Ssh(format!("Failed to execute SSH command: {}", e)))?;

    let stdout = String::from_utf8_lossy(&output.stdout).to_string();
    let stderr = String::from_utf8_lossy(&output.stderr).to_string();
    let success = output.status.success();

    Ok((stdout, stderr, success))
}

/// Resolve the vault password for a node's SSH config, if it uses password auth.
async fn resolve_vault_password(
    ssh_config: &SshConfig,
    state: &State<'_, AppState>,
) -> Result<Option<String>, AppError> {
    if let SshAuthMethod::Password { ref vault_key } = ssh_config.auth_method {
        if let Some(ref key) = vault_key {
            let vault = state.vault.lock().await;
            return vault.get(key);
        }
    }
    Ok(None)
}

// ---------------------------------------------------------------------------
// Remote SSH version check
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RemoteSshVersionInfo {
    pub version_string: String,
    pub major: u32,
    pub minor: u32,
    pub supports_sk_keys: bool,
}

/// Parse an SSH version string and extract (major, minor, full_version_string).
///
/// Handles formats like:
/// - `OpenSSH_8.9p1 Ubuntu-3ubuntu0.6, OpenSSL 3.0.2 15 Mar 2022`
/// - `SSH-2.0-OpenSSH_8.9`
/// - `sshd: OpenSSH_8.2, ...`
pub fn parse_ssh_version(version_output: &str) -> Option<(u32, u32, String)> {
    for line in version_output.lines() {
        let line = line.trim();
        if let Some(idx) = line.find("OpenSSH_") {
            let after_prefix = &line[idx + "OpenSSH_".len()..];
            // Extract version part (digits, dots, optional suffix like "p1")
            let version_part: String = after_prefix
                .chars()
                .take_while(|c| c.is_ascii_digit() || *c == '.' || c.is_ascii_alphabetic())
                .collect();

            // Parse major.minor from the version part
            let numeric_part: String = version_part
                .chars()
                .take_while(|c| c.is_ascii_digit() || *c == '.')
                .collect();

            let parts: Vec<&str> = numeric_part.split('.').collect();
            if parts.len() >= 2 {
                if let (Ok(major), Ok(minor)) = (parts[0].parse::<u32>(), parts[1].parse::<u32>()) {
                    let full_version = format!("OpenSSH_{}", version_part);
                    return Some((major, minor, full_version));
                }
            }
        }
    }

    None
}

/// Check a remote node's SSH server version to verify it supports ed25519-sk keys.
///
/// OpenSSH 8.2+ is required for `ed25519-sk` key type support.
#[tauri::command]
pub async fn check_remote_ssh_version(
    node_id: String,
    state: State<'_, AppState>,
) -> Result<RemoteSshVersionInfo, AppError> {
    // Look up the node
    let node = {
        let fleet = state.fleet_manager.lock().await;
        fleet
            .get_node(&node_id)
            .cloned()
            .ok_or_else(|| AppError::Fleet(format!("Node not found: {}", node_id)))?
    };

    let ssh_config = node
        .ssh_config
        .as_ref()
        .ok_or_else(|| AppError::Ssh("Node has no SSH configuration".to_string()))?;

    let vault_password = resolve_vault_password(ssh_config, &state).await?;

    // sshd -V outputs to stderr, so capture both; fall back to ssh -V
    let remote_cmd = "sshd -V 2>&1 || ssh -V 2>&1";

    let mut cmd = build_ssh_command(ssh_config, vault_password.as_deref(), remote_cmd);

    let output = tokio::time::timeout(
        std::time::Duration::from_secs(30),
        cmd.output(),
    )
    .await
    .map_err(|_| {
        AppError::Ssh(format!(
            "SSH version check timed out after 30s for {}:{}",
            ssh_config.host, ssh_config.port
        ))
    })?
    .map_err(|e| {
        AppError::Ssh(format!(
            "Failed to run SSH command for {}:{}: {}",
            ssh_config.host, ssh_config.port, e
        ))
    })?;

    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    let combined = format!("{}\n{}", stdout, stderr);

    let (major, minor, version_string) = parse_ssh_version(&combined).ok_or_else(|| {
        AppError::Ssh(format!(
            "Could not parse SSH version from remote output: {}",
            combined.trim().chars().take(200).collect::<String>()
        ))
    })?;

    let supports_sk_keys = major > 8 || (major == 8 && minor >= 2);

    Ok(RemoteSshVersionInfo {
        version_string,
        major,
        minor,
        supports_sk_keys,
    })
}

// ---------------------------------------------------------------------------
// sshd hardening
// ---------------------------------------------------------------------------

/// Configuration for which sshd settings to harden.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SshdHardenConfig {
    pub disable_password_auth: bool,
    pub disable_challenge_response: bool,
    pub disable_pam: bool,
}

/// Result of an sshd hardening operation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SshdHardenResult {
    pub backup_path: String,
    pub changes_made: Vec<String>,
    pub sshd_restarted: bool,
    pub warnings: Vec<String>,
}

/// Harden sshd_config on a remote node.
///
/// Backs up the existing configuration, applies the requested changes,
/// validates the config with `sshd -t`, and restarts the SSH service.
/// If validation fails, the backup is automatically restored.
#[tauri::command]
pub async fn harden_remote_sshd(
    node_id: String,
    config: SshdHardenConfig,
    state: State<'_, AppState>,
) -> Result<SshdHardenResult, AppError> {
    // Look up the node
    let node = {
        let fleet = state.fleet_manager.lock().await;
        fleet
            .get_node(&node_id)
            .cloned()
            .ok_or_else(|| AppError::Fleet(format!("Node not found: {}", node_id)))?
    };

    let ssh_config = node
        .ssh_config
        .as_ref()
        .ok_or_else(|| {
            AppError::Ssh("Node has no SSH configuration".to_string())
        })?;

    let vault_password = resolve_vault_password(ssh_config, &state).await?;
    let vault_pw_ref = vault_password.as_deref();

    // Verify at least one hardening option is selected
    if !config.disable_password_auth && !config.disable_challenge_response && !config.disable_pam {
        return Err(AppError::Ssh(
            "No hardening options selected. Enable at least one option.".to_string(),
        ));
    }

    let mut changes_made: Vec<String> = Vec::new();
    let mut warnings: Vec<String> = Vec::new();

    // Step 1: Create backup of sshd_config
    let backup_cmd =
        "sudo cp /etc/ssh/sshd_config /etc/ssh/sshd_config.bak.$(date +%s) && echo BACKUP_OK";
    let (stdout, stderr, success) =
        run_remote_command(ssh_config, vault_pw_ref, backup_cmd).await?;

    if !success || !stdout.contains("BACKUP_OK") {
        if stderr.contains("sudo:")
            && (stderr.contains("password") || stderr.contains("not allowed"))
        {
            return Err(AppError::Ssh(
                "Remote user lacks sudo privileges. Cannot modify sshd_config.".to_string(),
            ));
        }
        return Err(AppError::Ssh(format!(
            "Failed to create sshd_config backup: {}",
            stderr.trim()
        )));
    }

    // Find the backup path
    let find_backup_cmd = "ls -t /etc/ssh/sshd_config.bak.* 2>/dev/null | head -1";
    let (backup_path_out, _, backup_found) =
        run_remote_command(ssh_config, vault_pw_ref, find_backup_cmd).await?;

    let backup_path = backup_path_out.trim().to_string();
    if !backup_found || backup_path.is_empty() {
        return Err(AppError::Ssh(
            "Backup was created but could not determine backup path.".to_string(),
        ));
    }

    // Step 2: Apply hardening changes

    if config.disable_password_auth {
        let sed_cmd = "sudo sed -i 's/^#\\?PasswordAuthentication.*/PasswordAuthentication no/' /etc/ssh/sshd_config";
        let (_, stderr, success) =
            run_remote_command(ssh_config, vault_pw_ref, sed_cmd).await?;

        if !success {
            restore_backup(ssh_config, vault_pw_ref, &backup_path).await?;
            return Err(AppError::Ssh(format!(
                "Failed to disable PasswordAuthentication: {}",
                stderr.trim()
            )));
        }

        // Verify the setting was applied (it might not exist in the file)
        let verify_cmd = "grep -c '^PasswordAuthentication no' /etc/ssh/sshd_config";
        let (count_out, _, _) =
            run_remote_command(ssh_config, vault_pw_ref, verify_cmd).await?;
        if count_out.trim() == "0" {
            let append_cmd =
                "echo 'PasswordAuthentication no' | sudo tee -a /etc/ssh/sshd_config > /dev/null";
            let (_, stderr, success) =
                run_remote_command(ssh_config, vault_pw_ref, append_cmd).await?;
            if !success {
                restore_backup(ssh_config, vault_pw_ref, &backup_path).await?;
                return Err(AppError::Ssh(format!(
                    "Failed to append PasswordAuthentication directive: {}",
                    stderr.trim()
                )));
            }
        }

        changes_made.push("PasswordAuthentication set to no".to_string());
    }

    if config.disable_challenge_response {
        // ChallengeResponseAuthentication (older OpenSSH)
        let sed_cmd = "sudo sed -i 's/^#\\?ChallengeResponseAuthentication.*/ChallengeResponseAuthentication no/' /etc/ssh/sshd_config";
        let (_, stderr, success) =
            run_remote_command(ssh_config, vault_pw_ref, sed_cmd).await?;
        if !success {
            restore_backup(ssh_config, vault_pw_ref, &backup_path).await?;
            return Err(AppError::Ssh(format!(
                "Failed to disable ChallengeResponseAuthentication: {}",
                stderr.trim()
            )));
        }

        // KbdInteractiveAuthentication (newer OpenSSH >= 8.7)
        let sed_cmd_kbd = "sudo sed -i 's/^#\\?KbdInteractiveAuthentication.*/KbdInteractiveAuthentication no/' /etc/ssh/sshd_config";
        let (_, stderr_kbd, success_kbd) =
            run_remote_command(ssh_config, vault_pw_ref, sed_cmd_kbd).await?;
        if !success_kbd {
            restore_backup(ssh_config, vault_pw_ref, &backup_path).await?;
            return Err(AppError::Ssh(format!(
                "Failed to disable KbdInteractiveAuthentication: {}",
                stderr_kbd.trim()
            )));
        }

        // Verify/append ChallengeResponseAuthentication
        let verify_cmd =
            "grep -c '^ChallengeResponseAuthentication no' /etc/ssh/sshd_config";
        let (count_out, _, _) =
            run_remote_command(ssh_config, vault_pw_ref, verify_cmd).await?;
        if count_out.trim() == "0" {
            let append_cmd = "echo 'ChallengeResponseAuthentication no' | sudo tee -a /etc/ssh/sshd_config > /dev/null";
            let (_, stderr, success) =
                run_remote_command(ssh_config, vault_pw_ref, append_cmd).await?;
            if !success {
                restore_backup(ssh_config, vault_pw_ref, &backup_path).await?;
                return Err(AppError::Ssh(format!(
                    "Failed to append ChallengeResponseAuthentication: {}",
                    stderr.trim()
                )));
            }
        }

        // Verify/append KbdInteractiveAuthentication
        let verify_kbd_cmd =
            "grep -c '^KbdInteractiveAuthentication no' /etc/ssh/sshd_config";
        let (count_out_kbd, _, _) =
            run_remote_command(ssh_config, vault_pw_ref, verify_kbd_cmd).await?;
        if count_out_kbd.trim() == "0" {
            let append_cmd = "echo 'KbdInteractiveAuthentication no' | sudo tee -a /etc/ssh/sshd_config > /dev/null";
            let (_, stderr, success) =
                run_remote_command(ssh_config, vault_pw_ref, append_cmd).await?;
            if !success {
                restore_backup(ssh_config, vault_pw_ref, &backup_path).await?;
                return Err(AppError::Ssh(format!(
                    "Failed to append KbdInteractiveAuthentication: {}",
                    stderr.trim()
                )));
            }
        }

        changes_made.push("ChallengeResponseAuthentication set to no".to_string());
        changes_made.push("KbdInteractiveAuthentication set to no".to_string());
    }

    if config.disable_pam {
        let sed_cmd =
            "sudo sed -i 's/^#\\?UsePAM.*/UsePAM no/' /etc/ssh/sshd_config";
        let (_, stderr, success) =
            run_remote_command(ssh_config, vault_pw_ref, sed_cmd).await?;
        if !success {
            restore_backup(ssh_config, vault_pw_ref, &backup_path).await?;
            return Err(AppError::Ssh(format!(
                "Failed to disable UsePAM: {}",
                stderr.trim()
            )));
        }

        let verify_cmd = "grep -c '^UsePAM no' /etc/ssh/sshd_config";
        let (count_out, _, _) =
            run_remote_command(ssh_config, vault_pw_ref, verify_cmd).await?;
        if count_out.trim() == "0" {
            let append_cmd =
                "echo 'UsePAM no' | sudo tee -a /etc/ssh/sshd_config > /dev/null";
            let (_, stderr, success) =
                run_remote_command(ssh_config, vault_pw_ref, append_cmd).await?;
            if !success {
                restore_backup(ssh_config, vault_pw_ref, &backup_path).await?;
                return Err(AppError::Ssh(format!(
                    "Failed to append UsePAM directive: {}",
                    stderr.trim()
                )));
            }
        }

        changes_made.push("UsePAM set to no".to_string());
        warnings.push(
            "Disabling PAM may affect 2FA, session limits, and other PAM-dependent features."
                .to_string(),
        );
    }

    // Step 3: Validate the configuration with sshd -t
    let test_cmd = "sudo sshd -t 2>&1";
    let (test_stdout, test_stderr, test_success) =
        run_remote_command(ssh_config, vault_pw_ref, test_cmd).await?;

    if !test_success {
        let combined_output = format!("{}{}", test_stdout.trim(), test_stderr.trim());
        restore_backup(ssh_config, vault_pw_ref, &backup_path).await?;
        return Err(AppError::Ssh(format!(
            "sshd config validation failed (backup restored): {}",
            combined_output
        )));
    }

    // Step 4: Restart the SSH service (try 'ssh' then 'sshd')
    let restart_cmd =
        "sudo systemctl restart ssh 2>/dev/null || sudo systemctl restart sshd 2>/dev/null";
    let (_, restart_stderr, restart_success) =
        run_remote_command(ssh_config, vault_pw_ref, restart_cmd).await?;

    let sshd_restarted = if restart_success {
        true
    } else {
        warnings.push(format!(
            "Failed to restart SSH service: {}. You may need to restart it manually.",
            restart_stderr.trim()
        ));
        false
    };

    Ok(SshdHardenResult {
        backup_path,
        changes_made,
        sshd_restarted,
        warnings,
    })
}

/// Restore the sshd_config backup file after a failure.
async fn restore_backup(
    ssh_config: &SshConfig,
    vault_password: Option<&str>,
    backup_path: &str,
) -> Result<(), AppError> {
    let restore_cmd = format!("sudo cp {} /etc/ssh/sshd_config", backup_path);
    let (_, stderr, success) =
        run_remote_command(ssh_config, vault_password, &restore_cmd).await?;

    if !success {
        return Err(AppError::Ssh(format!(
            "CRITICAL: Failed to restore sshd_config backup from {}. \
             Manual intervention required on the remote host. Error: {}",
            backup_path,
            stderr.trim()
        )));
    }

    Ok(())
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    // -- Version parsing tests --

    #[test]
    fn parse_standard_openssh_version() {
        let output = "OpenSSH_8.9p1 Ubuntu-3ubuntu0.6, OpenSSL 3.0.2 15 Mar 2022";
        let (major, minor, version) = parse_ssh_version(output).unwrap();
        assert_eq!(major, 8);
        assert_eq!(minor, 9);
        assert_eq!(version, "OpenSSH_8.9p1");
    }

    #[test]
    fn parse_ssh_banner_format() {
        let output = "SSH-2.0-OpenSSH_8.9";
        let (major, minor, version) = parse_ssh_version(output).unwrap();
        assert_eq!(major, 8);
        assert_eq!(minor, 9);
        assert_eq!(version, "OpenSSH_8.9");
    }

    #[test]
    fn parse_openssh_9_x() {
        let output = "OpenSSH_9.6p1 Ubuntu-3ubuntu13.5, OpenSSL 3.2.2 4 Jun 2024";
        let (major, minor, version) = parse_ssh_version(output).unwrap();
        assert_eq!(major, 9);
        assert_eq!(minor, 6);
        assert_eq!(version, "OpenSSH_9.6p1");
    }

    #[test]
    fn parse_old_openssh_version() {
        let output = "OpenSSH_7.4p1, OpenSSL 1.0.2k-fips  26 Jan 2017";
        let (major, minor, version) = parse_ssh_version(output).unwrap();
        assert_eq!(major, 7);
        assert_eq!(minor, 4);
        assert_eq!(version, "OpenSSH_7.4p1");
    }

    #[test]
    fn parse_openssh_8_2_boundary() {
        let output = "OpenSSH_8.2p1 Ubuntu-4ubuntu0.11, OpenSSL 1.1.1f  31 Mar 2020";
        let (major, minor, version) = parse_ssh_version(output).unwrap();
        assert_eq!(major, 8);
        assert_eq!(minor, 2);
        assert_eq!(version, "OpenSSH_8.2p1");
    }

    #[test]
    fn parse_multiline_output_finds_version() {
        let output = "some preamble\nOpenSSH_8.9p1 Ubuntu-3ubuntu0.6, OpenSSL 3.0.2\nsome trailer";
        let (major, minor, _) = parse_ssh_version(output).unwrap();
        assert_eq!(major, 8);
        assert_eq!(minor, 9);
    }

    #[test]
    fn parse_sshd_dash_v_stderr_format() {
        let output = "sshd: OpenSSH_8.9, OpenSSL 3.0.2";
        let (major, minor, version) = parse_ssh_version(output).unwrap();
        assert_eq!(major, 8);
        assert_eq!(minor, 9);
        assert_eq!(version, "OpenSSH_8.9");
    }

    #[test]
    fn parse_returns_none_for_garbage() {
        assert!(parse_ssh_version("").is_none());
        assert!(parse_ssh_version("not a version string").is_none());
        assert!(parse_ssh_version("dropbear SSH server").is_none());
    }

    #[test]
    fn parse_returns_none_for_incomplete_version() {
        assert!(parse_ssh_version("OpenSSH_8").is_none());
    }

    #[test]
    fn parse_ssh_version_with_leading_whitespace() {
        let output = "  \t  OpenSSH_8.9p1 Ubuntu-3ubuntu0.6";
        let (major, minor, _) = parse_ssh_version(output).unwrap();
        assert_eq!(major, 8);
        assert_eq!(minor, 9);
    }

    #[test]
    fn parse_ssh_version_debian_format() {
        let output = "OpenSSH_9.2p1 Debian-2+deb12u3, OpenSSL 3.0.13 30 Jan 2024";
        let (major, minor, version) = parse_ssh_version(output).unwrap();
        assert_eq!(major, 9);
        assert_eq!(minor, 2);
        assert_eq!(version, "OpenSSH_9.2p1");
    }

    #[test]
    fn supports_sk_keys_boundary_check() {
        // 8.1 - should NOT support
        let (major, minor, _) = parse_ssh_version("OpenSSH_8.1p1").unwrap();
        assert!(!(major > 8 || (major == 8 && minor >= 2)));

        // 8.2 - first version with support
        let (major, minor, _) = parse_ssh_version("OpenSSH_8.2p1").unwrap();
        assert!(major > 8 || (major == 8 && minor >= 2));

        // 9.0 - should support
        let (major, minor, _) = parse_ssh_version("OpenSSH_9.0p1").unwrap();
        assert!(major > 8 || (major == 8 && minor >= 2));

        // 7.9 - should NOT support
        let (major, minor, _) = parse_ssh_version("OpenSSH_7.9p1").unwrap();
        assert!(!(major > 8 || (major == 8 && minor >= 2)));
    }

    // -- SSH command builder tests --

    #[test]
    fn build_ssh_command_with_agent_auth() {
        let ssh_config = SshConfig {
            host: "example.com".to_string(),
            port: 22,
            username: "user".to_string(),
            auth_method: SshAuthMethod::Agent,
            fingerprint: None,
            proxy_jump: None,
        };

        let cmd = build_ssh_command(&ssh_config, None, "ssh -V");
        let program = cmd.as_std().get_program().to_string_lossy().to_string();
        assert_eq!(program, "ssh");

        let args: Vec<String> = cmd
            .as_std()
            .get_args()
            .map(|a| a.to_string_lossy().to_string())
            .collect();
        assert!(args.contains(&"BatchMode=yes".to_string()));
        assert!(args.contains(&"user@example.com".to_string()));
        assert!(args.contains(&"ssh -V".to_string()));
    }

    #[test]
    fn build_ssh_command_with_password_auth() {
        let ssh_config = SshConfig {
            host: "example.com".to_string(),
            port: 2222,
            username: "admin".to_string(),
            auth_method: SshAuthMethod::Password {
                vault_key: Some("ssh:example".to_string()),
            },
            fingerprint: None,
            proxy_jump: None,
        };

        let cmd = build_ssh_command(&ssh_config, Some("s3cret"), "whoami");
        let program = cmd.as_std().get_program().to_string_lossy().to_string();
        assert_eq!(program, "sshpass");

        let args: Vec<String> = cmd
            .as_std()
            .get_args()
            .map(|a| a.to_string_lossy().to_string())
            .collect();
        assert!(args.contains(&"-e".to_string()));
        assert!(args.contains(&"ssh".to_string()));
        assert!(args.contains(&"2222".to_string()));
        assert!(args.contains(&"admin@example.com".to_string()));
    }

    #[test]
    fn build_ssh_command_with_keyfile_auth() {
        let ssh_config = SshConfig {
            host: "server.local".to_string(),
            port: 22,
            username: "deploy".to_string(),
            auth_method: SshAuthMethod::KeyFile {
                private_key_path: "/home/user/.ssh/id_ed25519".to_string(),
            },
            fingerprint: None,
            proxy_jump: None,
        };

        let cmd = build_ssh_command(&ssh_config, None, "sshd -V 2>&1");
        let args: Vec<String> = cmd
            .as_std()
            .get_args()
            .map(|a| a.to_string_lossy().to_string())
            .collect();
        assert!(args.contains(&"-i".to_string()));
        assert!(args.contains(&"/home/user/.ssh/id_ed25519".to_string()));
        assert!(args.contains(&"BatchMode=yes".to_string()));
    }

    #[test]
    fn build_ssh_command_with_certificate_auth() {
        let ssh_config = SshConfig {
            host: "prod.example.com".to_string(),
            port: 22,
            username: "ops".to_string(),
            auth_method: SshAuthMethod::Certificate {
                certificate_path: "/home/user/.ssh/id_ed25519-cert.pub".to_string(),
                private_key_path: "/home/user/.ssh/id_ed25519".to_string(),
            },
            fingerprint: None,
            proxy_jump: None,
        };

        let cmd = build_ssh_command(&ssh_config, None, "ssh -V");
        let args: Vec<String> = cmd
            .as_std()
            .get_args()
            .map(|a| a.to_string_lossy().to_string())
            .collect();
        let i_indices: Vec<usize> = args
            .iter()
            .enumerate()
            .filter(|(_, a)| a.as_str() == "-i")
            .map(|(i, _)| i)
            .collect();
        assert_eq!(i_indices.len(), 2);
        assert_eq!(args[i_indices[0] + 1], "/home/user/.ssh/id_ed25519-cert.pub");
        assert_eq!(args[i_indices[1] + 1], "/home/user/.ssh/id_ed25519");
    }

    #[test]
    fn build_ssh_command_with_proxy_jump() {
        let ssh_config = SshConfig {
            host: "internal.host".to_string(),
            port: 22,
            username: "user".to_string(),
            auth_method: SshAuthMethod::Agent,
            fingerprint: None,
            proxy_jump: Some("bastion.example.com".to_string()),
        };

        let cmd = build_ssh_command(&ssh_config, None, "ssh -V");
        let args: Vec<String> = cmd
            .as_std()
            .get_args()
            .map(|a| a.to_string_lossy().to_string())
            .collect();
        assert!(args.contains(&"-J".to_string()));
        assert!(args.contains(&"bastion.example.com".to_string()));
    }

    #[test]
    fn build_ssh_command_with_fido2_resident_auth() {
        let ssh_config = SshConfig {
            host: "secure.example.com".to_string(),
            port: 22,
            username: "admin".to_string(),
            auth_method: SshAuthMethod::Fido2Resident {
                application: Some("ssh:quartermaster".to_string()),
            },
            fingerprint: None,
            proxy_jump: None,
        };

        let cmd = build_ssh_command(&ssh_config, None, "id");
        let program = cmd.as_std().get_program().to_string_lossy().to_string();
        assert_eq!(program, "ssh");

        let args: Vec<String> = cmd
            .as_std()
            .get_args()
            .map(|a| a.to_string_lossy().to_string())
            .collect();
        assert!(args.contains(&"BatchMode=yes".to_string()));
        assert!(args.contains(&"admin@secure.example.com".to_string()));
    }

    #[test]
    fn build_ssh_command_includes_common_options() {
        let ssh_config = SshConfig {
            host: "example.com".to_string(),
            port: 22,
            username: "user".to_string(),
            auth_method: SshAuthMethod::Agent,
            fingerprint: None,
            proxy_jump: None,
        };

        let cmd = build_ssh_command(&ssh_config, None, "test");
        let args: Vec<String> = cmd
            .as_std()
            .get_args()
            .map(|a| a.to_string_lossy().to_string())
            .collect();
        assert!(args.contains(&"ConnectTimeout=10".to_string()));
        assert!(args.contains(&"StrictHostKeyChecking=accept-new".to_string()));
    }

    // -- Serialization tests --

    #[test]
    fn sshd_harden_config_serialization_roundtrip() {
        let config = SshdHardenConfig {
            disable_password_auth: true,
            disable_challenge_response: true,
            disable_pam: false,
        };
        let json = serde_json::to_string(&config).unwrap();
        let deserialized: SshdHardenConfig = serde_json::from_str(&json).unwrap();
        assert!(deserialized.disable_password_auth);
        assert!(deserialized.disable_challenge_response);
        assert!(!deserialized.disable_pam);
    }

    #[test]
    fn sshd_harden_result_serialization_roundtrip() {
        let result = SshdHardenResult {
            backup_path: "/etc/ssh/sshd_config.bak.1700000000".to_string(),
            changes_made: vec![
                "PasswordAuthentication set to no".to_string(),
                "UsePAM set to no".to_string(),
            ],
            sshd_restarted: true,
            warnings: vec!["Disabling PAM may affect 2FA".to_string()],
        };
        let json = serde_json::to_string(&result).unwrap();
        let deserialized: SshdHardenResult = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.backup_path, "/etc/ssh/sshd_config.bak.1700000000");
        assert_eq!(deserialized.changes_made.len(), 2);
        assert!(deserialized.sshd_restarted);
        assert_eq!(deserialized.warnings.len(), 1);
    }

    #[test]
    fn remote_ssh_version_info_serialization_roundtrip() {
        let info = RemoteSshVersionInfo {
            version_string: "OpenSSH_8.9p1".to_string(),
            major: 8,
            minor: 9,
            supports_sk_keys: true,
        };
        let json = serde_json::to_string(&info).unwrap();
        let deserialized: RemoteSshVersionInfo = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.version_string, "OpenSSH_8.9p1");
        assert!(deserialized.supports_sk_keys);
    }
}
