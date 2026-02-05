use serde::Serialize;
use tauri::Manager;
use crate::error::AppError;
use crate::polkit::auth;

#[derive(Debug, Serialize)]
pub struct SystemInfo {
    pub os: String,
    pub username: String,
    pub hostname: String,
}

#[tauri::command]
pub async fn get_system_info() -> Result<SystemInfo, AppError> {
    let username = std::env::var("USER")
        .or_else(|_| std::env::var("USERNAME"))
        .unwrap_or_else(|_| "unknown".to_string());

    let hostname = hostname::get()
        .map(|h| h.to_string_lossy().to_string())
        .unwrap_or_else(|_| "unknown".to_string());

    Ok(SystemInfo {
        os: std::env::consts::OS.to_string(),
        username,
        hostname,
    })
}

#[tauri::command]
pub async fn check_polkit_auth(action_id: String) -> Result<bool, AppError> {
    let output = tokio::process::Command::new("pkaction")
        .arg("--verbose")
        .arg("--action-id")
        .arg(&action_id)
        .output()
        .await;

    match output {
        Ok(o) => Ok(o.status.success()),
        Err(_) => Ok(false),
    }
}

#[tauri::command]
pub async fn is_polkit_policy_installed() -> Result<bool, AppError> {
    Ok(auth::is_policy_installed())
}

#[tauri::command]
pub async fn is_apparmor_helper_installed() -> Result<bool, AppError> {
    Ok(auth::is_helper_installed())
}

#[tauri::command]
pub async fn install_polkit_policy(app: tauri::AppHandle) -> Result<(), AppError> {
    let resource_dir = app
        .path()
        .resource_dir()
        .map_err(|e| AppError::Other(format!("Failed to get resource dir: {}", e)))?
        .join("resources");

    // Install PolicyKit policy
    let policy_source = resource_dir.join("com.quartermaster.policy");
    if !policy_source.exists() {
        return Err(AppError::Other(format!(
            "Policy file not found at: {}",
            policy_source.display()
        )));
    }

    let policy_dest = "/usr/share/polkit-1/actions/com.quartermaster.policy";
    auth::execute_privileged("cp", &[&policy_source.to_string_lossy(), policy_dest]).await?;

    // Install AppArmor helper script (single pkexec call)
    let helper_source = resource_dir.join("quartermaster-apparmor-helper");
    if helper_source.exists() {
        let script = format!(
            "mkdir -p /usr/lib/quartermaster && cp -- '{}' /usr/lib/quartermaster/quartermaster-apparmor-helper && chmod 755 /usr/lib/quartermaster/quartermaster-apparmor-helper && chown root:root /usr/lib/quartermaster/quartermaster-apparmor-helper",
            helper_source.to_string_lossy().replace('\'', "'\\''")
        );
        auth::execute_privileged("bash", &["-c", &script]).await?;
    }

    Ok(())
}
