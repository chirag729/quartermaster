use std::path::PathBuf;
use super::types::{ApplyPermissionsRequest, ProfileInfo, ProfileDetail, ProfileMode};
use crate::error::AppError;

const HELPER_PATH: &str = "/usr/lib/anvil/anvil-apparmor-helper";

/// Validate a profile name to prevent path traversal and injection attacks.
fn validate_profile_name(name: &str) -> Result<(), AppError> {
    if name.is_empty() || name.contains("..") || name.starts_with('-') {
        return Err(AppError::AppArmor(format!("Invalid profile name: {}", name)));
    }
    if !name.chars().all(|c| c.is_alphanumeric() || "._/-".contains(c)) {
        return Err(AppError::AppArmor(format!(
            "Invalid characters in profile name: {}",
            name
        )));
    }
    Ok(())
}

/// Run a command through the AppArmor helper script via pkexec.
///
/// If the helper is installed at /usr/lib/anvil/anvil-apparmor-helper, uses it
/// directly with pkexec. This triggers the com.anvil.apparmor-manage PolicyKit
/// action which has auth_admin_keep — credentials are cached for ~5 minutes.
///
/// If the helper is not installed, falls back to direct pkexec calls.
async fn run_apparmor_helper(args: &[&str]) -> Result<std::process::Output, AppError> {
    if std::path::Path::new(HELPER_PATH).exists() {
        tokio::process::Command::new("pkexec")
            .arg(HELPER_PATH)
            .args(args)
            .output()
            .await
            .map_err(|e| AppError::AppArmor(format!("Helper execution failed: {}", e)))
    } else {
        run_apparmor_helper_fallback(args).await
    }
}

/// Fallback when the helper script is not installed.
/// Consolidates operations into single pkexec calls where possible.
async fn run_apparmor_helper_fallback(args: &[&str]) -> Result<std::process::Output, AppError> {
    match args.first().copied() {
        Some("aa-status") => {
            tokio::process::Command::new("pkexec")
                .args(["aa-status", "--json"])
                .output()
                .await
                .map_err(|e| AppError::AppArmor(format!("aa-status failed: {}", e)))
        }
        Some("copy-and-reload") => {
            // args: ["copy-and-reload", source, dest]
            if args.len() != 3 {
                return Err(AppError::AppArmor("copy-and-reload requires source and dest".into()));
            }
            let source = args[1];
            let dest = args[2];
            // Consolidate cp + apparmor_parser into a single pkexec bash -c call
            let script = format!(
                "cp -- {} {} && chmod 644 {} && apparmor_parser -r {}",
                shell_escape(source),
                shell_escape(dest),
                shell_escape(dest),
                shell_escape(dest),
            );
            tokio::process::Command::new("pkexec")
                .args(["bash", "-c", &script])
                .output()
                .await
                .map_err(|e| AppError::AppArmor(format!("copy-and-reload failed: {}", e)))
        }
        Some("batch-copy-and-reload") => {
            // args: ["batch-copy-and-reload", src1, dest1, src2, dest2, ...]
            let pairs = &args[1..];
            if pairs.len() % 2 != 0 {
                return Err(AppError::AppArmor("batch-copy-and-reload requires source/dest pairs".into()));
            }

            let mut script_parts = Vec::new();
            let mut dests = Vec::new();

            for chunk in pairs.chunks(2) {
                let source = chunk[0];
                let dest = chunk[1];
                script_parts.push(format!(
                    "cp -- {} {} && chmod 644 {}",
                    shell_escape(source),
                    shell_escape(dest),
                    shell_escape(dest),
                ));
                dests.push(dest);
            }

            for dest in &dests {
                script_parts.push(format!("apparmor_parser -r {}", shell_escape(dest)));
            }

            let script = script_parts.join(" && ");
            tokio::process::Command::new("pkexec")
                .args(["bash", "-c", &script])
                .output()
                .await
                .map_err(|e| AppError::AppArmor(format!("batch-copy-and-reload failed: {}", e)))
        }
        _ => Err(AppError::AppArmor(format!("Unknown helper command: {:?}", args.first()))),
    }
}

/// Escape a string for safe use in a shell command.
fn shell_escape(s: &str) -> String {
    format!("'{}'", s.replace('\'', "'\\''"))
}

pub async fn list_profiles() -> Result<Vec<ProfileInfo>, AppError> {
    let output = tokio::process::Command::new("aa-status")
        .arg("--json")
        .output()
        .await;

    match output {
        Ok(o) if o.status.success() => {
            let stdout = String::from_utf8_lossy(&o.stdout);
            parse_aa_status_json(&stdout)
        }
        _ => {
            // Try with helper (uses auth_admin_keep caching)
            let output = run_apparmor_helper(&["aa-status"]).await?;
            if output.status.success() {
                let stdout = String::from_utf8_lossy(&output.stdout);
                parse_aa_status_json(&stdout)
            } else {
                Err(AppError::AppArmor("Failed to get AppArmor status".into()))
            }
        }
    }
}

fn parse_aa_status_json(json_str: &str) -> Result<Vec<ProfileInfo>, AppError> {
    let parsed: serde_json::Value = serde_json::from_str(json_str)
        .map_err(|e| AppError::AppArmor(format!("Failed to parse aa-status JSON: {}", e)))?;

    let mut profiles = Vec::new();

    if let Some(profs) = parsed.get("profiles").and_then(|p| p.as_object()) {
        for (name, mode) in profs {
            let mode_str = mode.as_str().unwrap_or("unknown");
            let mode = match mode_str {
                "enforce" => ProfileMode::Enforce,
                "complain" => ProfileMode::Complain,
                _ => ProfileMode::Unconfined,
            };
            profiles.push(ProfileInfo {
                name: name.clone(),
                mode,
                pid_count: 0,
            });
        }
    }

    if let Some(processes) = parsed.get("processes").and_then(|p| p.as_object()) {
        for (name, pids) in processes {
            if let Some(pid_array) = pids.as_array() {
                if let Some(profile) = profiles.iter_mut().find(|p| &p.name == name) {
                    profile.pid_count = pid_array.len() as u32;
                }
            }
        }
    }

    Ok(profiles)
}

pub async fn get_profile_detail(profile_name: &str) -> Result<ProfileDetail, AppError> {
    validate_profile_name(profile_name)?;

    let profile_path = find_profile_path(profile_name)?;

    let content = std::fs::read_to_string(&profile_path)
        .map_err(|e| AppError::AppArmor(format!("Failed to read profile: {}", e)))?;

    let rules: Vec<String> = content
        .lines()
        .filter(|l| {
            let trimmed = l.trim();
            !trimmed.is_empty()
                && !trimmed.starts_with('#')
                && !trimmed.starts_with("profile ")
                && !trimmed.starts_with("include ")
                && trimmed != "{"
                && trimmed != "}"
        })
        .map(|l| l.trim().to_string())
        .collect();

    let profiles = list_profiles().await.unwrap_or_default();
    let mode = profiles
        .iter()
        .find(|p| p.name == profile_name)
        .map(|p| p.mode.clone())
        .unwrap_or(ProfileMode::Enforce);

    Ok(ProfileDetail {
        name: profile_name.to_string(),
        mode,
        raw_content: content,
        rules,
    })
}

fn find_profile_path(profile_name: &str) -> Result<PathBuf, AppError> {
    let search_dirs = ["/etc/apparmor.d", "/etc/apparmor.d/local"];
    let sanitized_name = profile_name.replace('/', ".");

    for dir in &search_dirs {
        let path = PathBuf::from(dir).join(&sanitized_name);
        if path.exists() {
            return Ok(path);
        }
        let path = PathBuf::from(dir).join(profile_name);
        if path.exists() {
            return Ok(path);
        }
    }

    let apparmor_dir = PathBuf::from("/etc/apparmor.d");
    if apparmor_dir.exists() {
        if let Ok(entries) = std::fs::read_dir(&apparmor_dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.is_file() {
                    if let Ok(content) = std::fs::read_to_string(&path) {
                        if content.contains(&format!("profile {} ", profile_name))
                            || content.contains(&format!("/{} ", profile_name))
                        {
                            return Ok(path);
                        }
                    }
                }
            }
        }
    }

    Err(AppError::AppArmor(format!("Profile file not found for: {}", profile_name)))
}

pub async fn apply_rules(profile_name: &str, rules: &[String]) -> Result<(), AppError> {
    validate_profile_name(profile_name)?;

    let profile_path = find_profile_path(profile_name)?;
    let content = std::fs::read_to_string(&profile_path)
        .map_err(|e| AppError::AppArmor(format!("Failed to read profile: {}", e)))?;

    let new_content = insert_rules_into_content(&content, rules)?;

    // Write to a secure temp file
    let temp_file = tempfile::Builder::new()
        .prefix("anvil-apparmor-")
        .tempfile()
        .map_err(|e| AppError::AppArmor(format!("Failed to create temp file: {}", e)))?;

    std::fs::write(temp_file.path(), &new_content)?;

    let temp_str = temp_file.path().to_string_lossy().to_string();
    let profile_str = profile_path.to_string_lossy().to_string();

    let output = run_apparmor_helper(&["copy-and-reload", &temp_str, &profile_str]).await?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(AppError::AppArmor(format!("Failed to apply rules: {}", stderr)));
    }

    Ok(())
}

/// Insert rules into a profile's content, returning the modified content.
fn insert_rules_into_content(content: &str, rules: &[String]) -> Result<String, AppError> {
    let mut lines: Vec<String> = content.lines().map(|l| l.to_string()).collect();
    let close_idx = lines.iter().rposition(|l| l.trim() == "}");

    if let Some(idx) = close_idx {
        lines.insert(idx, String::new());
        lines.insert(idx + 1, "  # Rules added by Anvil".to_string());
        for (i, rule) in rules.iter().enumerate() {
            lines.insert(idx + 2 + i, rule.clone());
        }
    } else {
        return Err(AppError::AppArmor(
            "Could not find closing brace in profile".into(),
        ));
    }

    Ok(lines.join("\n") + "\n")
}

/// Apply rules to multiple profiles in a single pkexec invocation.
///
/// 1. For each request, reads the profile and inserts the rules
/// 2. Writes each modified profile to a secure temp file
/// 3. Invokes the helper with batch-copy-and-reload and source/dest pairs
/// 4. Temp files are cleaned up automatically by tempfile crate
pub async fn apply_rules_batch(requests: &[ApplyPermissionsRequest]) -> Result<(), AppError> {
    if requests.is_empty() {
        return Ok(());
    }

    // Validate all profile names upfront
    for request in requests {
        validate_profile_name(&request.profile)?;
    }

    // Prepare modified profiles and temp files
    let mut temp_files: Vec<tempfile::NamedTempFile> = Vec::new();
    let mut helper_args: Vec<String> = vec!["batch-copy-and-reload".to_string()];

    for request in requests {
        let profile_path = find_profile_path(&request.profile)?;

        let content = std::fs::read_to_string(&profile_path).map_err(|e| {
            AppError::AppArmor(format!("Failed to read profile {}: {}", request.profile, e))
        })?;

        let new_content = insert_rules_into_content(&content, &request.rules)?;

        let temp_file = tempfile::Builder::new()
            .prefix("anvil-apparmor-batch-")
            .tempfile()
            .map_err(|e| AppError::AppArmor(format!("Failed to create temp file: {}", e)))?;

        std::fs::write(temp_file.path(), &new_content)?;

        helper_args.push(temp_file.path().to_string_lossy().to_string());
        helper_args.push(profile_path.to_string_lossy().to_string());

        // Keep temp_file alive so it isn't deleted before the helper runs
        temp_files.push(temp_file);
    }

    let arg_refs: Vec<&str> = helper_args.iter().map(|s| s.as_str()).collect();
    let output = run_apparmor_helper(&arg_refs).await?;

    // temp_files are dropped here, cleaning up automatically

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(AppError::AppArmor(format!("Batch apply failed: {}", stderr)));
    }

    Ok(())
}

/// Rewrite a profile's rules section, replacing all existing rules with the given ones.
///
/// Preserves the profile header (everything up to and including `{`),
/// include directives, comments, and the closing `}`. Replaces all rule lines
/// between `{` and `}` with the provided rules.
pub async fn rewrite_profile_rules(profile_name: &str, rules: &[String]) -> Result<(), AppError> {
    validate_profile_name(profile_name)?;

    let profile_path = find_profile_path(profile_name)?;
    let content = std::fs::read_to_string(&profile_path)
        .map_err(|e| AppError::AppArmor(format!("Failed to read profile: {}", e)))?;

    let lines: Vec<&str> = content.lines().collect();

    // Find the opening { and closing }
    let open_idx = lines.iter().position(|l| l.trim() == "{" || l.trim().ends_with('{'));
    let close_idx = lines.iter().rposition(|l| l.trim() == "}");

    let (open_idx, close_idx) = match (open_idx, close_idx) {
        (Some(o), Some(c)) if c > o => (o, c),
        _ => {
            return Err(AppError::AppArmor(
                "Could not find profile block boundaries".into(),
            ));
        }
    };

    // Build new content:
    // 1. Everything up to and including the opening brace
    // 2. Include directives and comments from the original rules section
    // 3. The new consolidated rules
    // 4. The closing brace and anything after
    let mut new_lines: Vec<String> = Vec::new();

    // Header (up to and including {)
    for line in &lines[..=open_idx] {
        new_lines.push(line.to_string());
    }

    // Preserve include directives and standalone comments from original body
    for line in &lines[open_idx + 1..close_idx] {
        let trimmed = line.trim();
        if trimmed.starts_with("include ")
            || trimmed.starts_with("#include ")
            || (trimmed.starts_with('#')
                && !trimmed.starts_with("# Rules added by Anvil")
                && !trimmed.starts_with("# Rules added by Machine Setup")
                && !trimmed.starts_with("# Rules consolidated by Anvil")
                && !trimmed.starts_with("# Rules consolidated by Machine Setup"))
        {
            new_lines.push(line.to_string());
        }
    }

    // Add new rules
    new_lines.push(String::new());
    new_lines.push("  # Rules consolidated by Anvil".to_string());
    for rule in rules {
        new_lines.push(rule.clone());
    }

    // Closing brace and anything after
    for line in &lines[close_idx..] {
        new_lines.push(line.to_string());
    }

    let new_content = new_lines.join("\n") + "\n";

    // Write to a secure temp file
    let temp_file = tempfile::Builder::new()
        .prefix("anvil-apparmor-rewrite-")
        .tempfile()
        .map_err(|e| AppError::AppArmor(format!("Failed to create temp file: {}", e)))?;

    std::fs::write(temp_file.path(), &new_content)?;

    let temp_str = temp_file.path().to_string_lossy().to_string();
    let profile_str = profile_path.to_string_lossy().to_string();

    let output = run_apparmor_helper(&["copy-and-reload", &temp_str, &profile_str]).await?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(AppError::AppArmor(format!(
            "Failed to rewrite profile: {}",
            stderr
        )));
    }

    Ok(())
}
