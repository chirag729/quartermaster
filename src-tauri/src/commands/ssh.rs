use tauri::State;

use crate::error::AppError;
use crate::state::AppState;

/// Allowed SSH key types for generation
const ALLOWED_KEY_TYPES: &[&str] = &["ed25519", "ecdsa", "rsa", "ed25519-sk", "ecdsa-sk"];

#[tauri::command]
pub async fn test_ssh_connection(
    host: String,
    port: u16,
    username: String,
    _state: State<'_, AppState>,
) -> Result<String, AppError> {
    let output = tokio::process::Command::new("ssh")
        .args([
            "-p", &port.to_string(),
            "-o", "BatchMode=yes",
            "-o", "ConnectTimeout=10",
            "-o", "StrictHostKeyChecking=accept-new",
            &format!("{}@{}", username, host),
            "echo ok",
        ])
        .output()
        .await
        .map_err(|e| AppError::Ssh(format!("Failed to run ssh: {}", e)))?;

    if output.status.success() {
        Ok(format!("Connected to {}@{}:{}", username, host, port))
    } else {
        let stderr = String::from_utf8_lossy(&output.stderr);
        Err(AppError::Ssh(format!("Connection failed: {}", stderr)))
    }
}

#[tauri::command]
pub async fn list_ssh_keys() -> Result<Vec<SshKeyInfo>, AppError> {
    let ssh_dir = dirs::home_dir()
        .unwrap_or_default()
        .join(".ssh");

    let mut keys = Vec::new();

    if !ssh_dir.exists() {
        return Ok(keys);
    }

    let entries = std::fs::read_dir(&ssh_dir)?;
    for entry in entries {
        let entry = entry?;
        let path = entry.path();
        let name = path.file_name().unwrap_or_default().to_string_lossy().to_string();

        // Only include public key files
        if name.ends_with(".pub") {
            let content = std::fs::read_to_string(&path).unwrap_or_default();
            let key_type = content.split_whitespace().next().unwrap_or("unknown").to_string();
            let is_fido2 = key_type.contains("sk-");

            keys.push(SshKeyInfo {
                name: name.trim_end_matches(".pub").to_string(),
                path: path.to_string_lossy().to_string(),
                key_type,
                is_fido2,
            });
        }
    }

    Ok(keys)
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct SshKeyInfo {
    pub name: String,
    pub path: String,
    pub key_type: String,
    pub is_fido2: bool,
}

#[tauri::command]
pub async fn generate_ssh_key(
    key_type: String,
    comment: String,
) -> Result<SshKeyInfo, AppError> {
    // Validate key_type against allowed list
    if !ALLOWED_KEY_TYPES.contains(&key_type.as_str()) {
        return Err(AppError::Ssh(format!(
            "Invalid key type '{}'. Allowed types: {}",
            key_type,
            ALLOWED_KEY_TYPES.join(", ")
        )));
    }

    let ssh_dir = dirs::home_dir()
        .unwrap_or_default()
        .join(".ssh");

    std::fs::create_dir_all(&ssh_dir)?;

    // Sanitize both key_type and comment to prevent path traversal
    let safe_type = super::yubikey::sanitize_key_name(&key_type);
    let safe_comment = super::yubikey::sanitize_key_name(
        &comment.replace(' ', "_").to_lowercase(),
    );
    let key_name = format!("id_{}_{}", safe_type, safe_comment);
    let key_path = ssh_dir.join(&key_name);

    // Don't overwrite existing keys
    if key_path.exists() {
        return Err(AppError::Ssh(format!("Key already exists: {}", key_path.display())));
    }

    let output = tokio::process::Command::new("ssh-keygen")
        .args([
            "-t", &key_type,
            "-f", &key_path.to_string_lossy(),
            "-C", &comment,
            "-N", "",  // empty passphrase for non-interactive generation
        ])
        .output()
        .await
        .map_err(|e| AppError::Ssh(format!("Failed to run ssh-keygen: {}", e)))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(AppError::Ssh(format!("ssh-keygen failed: {}", stderr)));
    }

    // Read the public key to determine type
    let pub_path = ssh_dir.join(format!("{}.pub", key_name));
    let pub_content = std::fs::read_to_string(&pub_path)
        .unwrap_or_default();
    let detected_type = pub_content.split_whitespace()
        .next()
        .unwrap_or("unknown")
        .to_string();
    let is_fido2 = detected_type.contains("sk-") || key_type.contains("sk");

    Ok(SshKeyInfo {
        name: key_name,
        path: pub_path.to_string_lossy().to_string(),
        key_type: detected_type,
        is_fido2,
    })
}

#[tauri::command]
pub async fn deploy_ssh_key(
    public_key_path: String,
    target_host: String,
    target_port: u16,
    target_username: String,
) -> Result<(), AppError> {
    let pub_key = std::fs::read_to_string(&public_key_path)
        .map_err(|e| AppError::Ssh(format!("Failed to read public key: {}", e)))?;

    // Validate public key format: single line starting with a known key type prefix
    let trimmed = pub_key.trim();
    let valid_prefixes = [
        "ssh-rsa", "ssh-ed25519", "ecdsa-sha2-", "ssh-dss",
        "sk-ssh-ed25519@", "sk-ecdsa-sha2-",
    ];
    if trimmed.lines().count() != 1
        || !valid_prefixes.iter().any(|p| trimmed.starts_with(p))
    {
        return Err(AppError::Ssh(
            "Invalid public key format. Expected a single-line OpenSSH public key.".to_string(),
        ));
    }

    // Pipe the public key via stdin to avoid shell injection
    let script = "mkdir -p ~/.ssh && chmod 700 ~/.ssh && cat >> ~/.ssh/authorized_keys && chmod 600 ~/.ssh/authorized_keys";

    let mut child = tokio::process::Command::new("ssh")
        .args([
            "-p", &target_port.to_string(),
            "-o", "StrictHostKeyChecking=accept-new",
            &format!("{}@{}", target_username, target_host),
            script,
        ])
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .map_err(|e| AppError::Ssh(format!("Failed to start ssh: {}", e)))?;

    // Write the public key content to stdin
    if let Some(mut stdin) = child.stdin.take() {
        use tokio::io::AsyncWriteExt;
        stdin.write_all(pub_key.trim().as_bytes()).await
            .map_err(|e| AppError::Ssh(format!("Failed to write key to stdin: {}", e)))?;
        stdin.write_all(b"\n").await
            .map_err(|e| AppError::Ssh(format!("Failed to write newline: {}", e)))?;
        drop(stdin);
    }

    let output = child.wait_with_output().await
        .map_err(|e| AppError::Ssh(format!("Failed to deploy key: {}", e)))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(AppError::Ssh(format!("Key deployment failed: {}", stderr)));
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn generate_ssh_key_rejects_invalid_key_type() {
        let result = generate_ssh_key("invalid-type".into(), "test".into()).await;
        assert!(result.is_err());
        let err = result.unwrap_err().to_string();
        assert!(err.contains("Invalid key type"), "Error: {}", err);
    }

    #[tokio::test]
    async fn generate_ssh_key_rejects_injection_in_key_type() {
        let result = generate_ssh_key("; rm -rf /".into(), "test".into()).await;
        assert!(result.is_err());
        let err = result.unwrap_err().to_string();
        assert!(err.contains("Invalid key type"), "Error: {}", err);
    }

    #[test]
    fn allowed_key_types_are_valid() {
        for kt in ALLOWED_KEY_TYPES {
            assert!(
                ["ed25519", "ecdsa", "rsa", "ed25519-sk", "ecdsa-sk"].contains(kt),
                "Unexpected allowed key type: {}",
                kt
            );
        }
    }
}
