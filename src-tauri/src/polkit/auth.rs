use crate::error::AppError;

pub async fn execute_privileged(command: &str, args: &[&str]) -> Result<String, AppError> {
    let output = tokio::process::Command::new("pkexec")
        .arg(command)
        .args(args)
        .output()
        .await?;

    if output.status.success() {
        Ok(String::from_utf8_lossy(&output.stdout).to_string())
    } else {
        let stderr = String::from_utf8_lossy(&output.stderr);
        Err(AppError::Polkit(format!("Privileged command failed: {}", stderr)))
    }
}

pub fn is_policy_installed() -> bool {
    std::path::Path::new("/usr/share/polkit-1/actions/com.anvil.policy").exists()
}

pub fn is_helper_installed() -> bool {
    std::path::Path::new("/usr/lib/anvil/anvil-apparmor-helper").exists()
}
