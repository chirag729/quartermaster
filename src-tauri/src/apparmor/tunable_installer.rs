use std::collections::HashMap;

use serde::{Deserialize, Serialize};

use crate::error::AppError;
use crate::executor::CommandExecutor;

const TUNABLES_PATH: &str = "/etc/apparmor.d/tunables/quartermaster";
const HEADER_COMMENT: &str = "# Quartermaster managed tunables - DO NOT EDIT MANUALLY";

/// A set of tunable definitions to install.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TunableSet {
    pub entries: HashMap<String, String>,
}

impl TunableSet {
    /// Create an empty TunableSet.
    pub fn new() -> Self {
        Self {
            entries: HashMap::new(),
        }
    }
}

impl Default for TunableSet {
    fn default() -> Self {
        Self::new()
    }
}

/// Generate the content of the tunables file.
///
/// Produces a header comment followed by `@{KEY}=VALUE` lines sorted
/// alphabetically by key.
pub fn generate_tunables_content(tunables: &TunableSet) -> String {
    let mut lines = vec![HEADER_COMMENT.to_string()];

    let mut keys: Vec<&String> = tunables.entries.keys().collect();
    keys.sort();

    for key in keys {
        let value = &tunables.entries[key];
        lines.push(format!("@{{{}}}={}", key, value));
    }

    // Ensure trailing newline
    lines.push(String::new());
    lines.join("\n")
}

/// Parse tunables content from a string.
///
/// Extracts `@{KEY}=VALUE` lines, ignoring comments and blank lines.
/// Returns a TunableSet with the parsed entries.
pub fn parse_tunables_content(content: &str) -> TunableSet {
    let mut entries = HashMap::new();

    for line in content.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() || trimmed.starts_with('#') {
            continue;
        }
        // Match @{KEY}=VALUE pattern
        if let Some(rest) = trimmed.strip_prefix("@{") {
            if let Some(eq_pos) = rest.find('}') {
                let key = &rest[..eq_pos];
                let after_brace = &rest[eq_pos + 1..];
                if let Some(value) = after_brace.strip_prefix('=') {
                    entries.insert(key.to_string(), value.to_string());
                }
            }
        }
    }

    TunableSet { entries }
}

/// Install the tunables file to `/etc/apparmor.d/tunables/quartermaster`.
///
/// Since writing to `/etc/apparmor.d/tunables/` requires root privileges,
/// this function:
/// 1. Generates the tunables content
/// 2. Writes it to a temp file at `~/.cache/quartermaster/tunables.tmp`
/// 3. Uses `pkexec` via the executor to copy it to the system location
/// 4. Cleans up the temp file
pub async fn install_tunables(
    tunables: &TunableSet,
    exec: &dyn CommandExecutor,
) -> Result<(), AppError> {
    let content = generate_tunables_content(tunables);

    // Write content to a temp file
    let cache_dir = format!("{}/.cache/quartermaster", exec.home_dir());
    exec.create_dir_all(&cache_dir).await?;

    let tmp_path = format!("{}/tunables.tmp", cache_dir);
    exec.write_file(&tmp_path, &content).await?;

    // Use pkexec to copy the temp file to the system tunables directory
    let script = format!(
        "cp -- '{}' '{}' && chmod 644 '{}'",
        tmp_path.replace('\'', "'\\''"),
        TUNABLES_PATH.replace('\'', "'\\''"),
        TUNABLES_PATH.replace('\'', "'\\''"),
    );

    let output = exec.run_command("pkexec", &["sh", "-c", &script]).await?;

    // Clean up the temp file (best effort)
    let _ = exec.run_command("rm", &["-f", &tmp_path]).await;

    if output.status != 0 {
        return Err(AppError::AppArmor(format!(
            "Failed to install tunables: {}",
            output.stderr.trim()
        )));
    }

    Ok(())
}

/// Read the current tunables from `/etc/apparmor.d/tunables/quartermaster`.
///
/// Returns an empty TunableSet if the file does not exist.
pub async fn read_current_tunables(
    exec: &dyn CommandExecutor,
) -> Result<TunableSet, AppError> {
    let exists = exec.file_exists(TUNABLES_PATH).await?;
    if !exists {
        return Ok(TunableSet::new());
    }

    let content = exec.read_file(TUNABLES_PATH).await?;
    Ok(parse_tunables_content(&content))
}

/// Merge new tunables into the current set.
///
/// For keys present in both sets, the value from `new` takes precedence.
pub fn merge_tunables(current: &TunableSet, new: &TunableSet) -> TunableSet {
    let mut merged = current.entries.clone();
    for (key, value) in &new.entries {
        merged.insert(key.clone(), value.clone());
    }
    TunableSet { entries: merged }
}

/// Reload all AppArmor profiles.
///
/// Runs `pkexec apparmor_parser -r /etc/apparmor.d/` to reload all profiles
/// after tunable changes.
pub async fn reload_apparmor(exec: &dyn CommandExecutor) -> Result<(), AppError> {
    let output = exec
        .run_command("pkexec", &["apparmor_parser", "-r", "/etc/apparmor.d/"])
        .await?;

    if output.status != 0 {
        return Err(AppError::AppArmor(format!(
            "Failed to reload AppArmor profiles: {}",
            output.stderr.trim()
        )));
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn generate_tunables_content_empty_set() {
        let tunables = TunableSet::new();
        let content = generate_tunables_content(&tunables);
        assert_eq!(content, "# Quartermaster managed tunables - DO NOT EDIT MANUALLY\n");
    }

    #[test]
    fn generate_tunables_content_multiple_entries_sorted() {
        let mut tunables = TunableSet::new();
        tunables
            .entries
            .insert("QM_SDK".to_string(), "/home/user/.local/share/sdk".to_string());
        tunables
            .entries
            .insert("QM_PROJECTS".to_string(), "/home/user/Development/Projects".to_string());
        tunables
            .entries
            .insert("QM_HOME".to_string(), "/home/user".to_string());

        let content = generate_tunables_content(&tunables);
        let lines: Vec<&str> = content.lines().collect();

        assert_eq!(lines[0], "# Quartermaster managed tunables - DO NOT EDIT MANUALLY");
        assert_eq!(lines[1], "@{QM_HOME}=/home/user");
        assert_eq!(lines[2], "@{QM_PROJECTS}=/home/user/Development/Projects");
        assert_eq!(lines[3], "@{QM_SDK}=/home/user/.local/share/sdk");
        assert_eq!(lines.len(), 3 + 1); // header + 3 entries (trailing newline excluded by lines())
    }

    #[test]
    fn parse_tunables_content_valid() {
        let content = "\
# Quartermaster managed tunables - DO NOT EDIT MANUALLY
@{QM_PROJECTS}=/home/user/Development/Projects
@{QM_SDK}=/home/user/.local/share/sdk
";
        let tunables = parse_tunables_content(content);

        assert_eq!(tunables.entries.len(), 2);
        assert_eq!(
            tunables.entries.get("QM_PROJECTS").unwrap(),
            "/home/user/Development/Projects"
        );
        assert_eq!(
            tunables.entries.get("QM_SDK").unwrap(),
            "/home/user/.local/share/sdk"
        );
    }

    #[test]
    fn parse_tunables_content_ignores_comments_and_blanks() {
        let content = "\
# Header comment
# Another comment

@{MY_VAR}=/some/path

# Trailing comment
";
        let tunables = parse_tunables_content(content);
        assert_eq!(tunables.entries.len(), 1);
        assert_eq!(tunables.entries.get("MY_VAR").unwrap(), "/some/path");
    }

    #[test]
    fn merge_tunables_combines_two_sets() {
        let mut current = TunableSet::new();
        current
            .entries
            .insert("QM_PROJECTS".to_string(), "/home/user/Projects".to_string());
        current
            .entries
            .insert("QM_HOME".to_string(), "/home/user".to_string());

        let mut new = TunableSet::new();
        new.entries
            .insert("QM_SDK".to_string(), "/opt/sdk".to_string());

        let merged = merge_tunables(&current, &new);

        assert_eq!(merged.entries.len(), 3);
        assert_eq!(merged.entries.get("QM_PROJECTS").unwrap(), "/home/user/Projects");
        assert_eq!(merged.entries.get("QM_HOME").unwrap(), "/home/user");
        assert_eq!(merged.entries.get("QM_SDK").unwrap(), "/opt/sdk");
    }

    #[test]
    fn merge_tunables_new_overrides_current() {
        let mut current = TunableSet::new();
        current
            .entries
            .insert("QM_PROJECTS".to_string(), "/old/path".to_string());
        current
            .entries
            .insert("QM_HOME".to_string(), "/home/user".to_string());

        let mut new = TunableSet::new();
        new.entries
            .insert("QM_PROJECTS".to_string(), "/new/path".to_string());

        let merged = merge_tunables(&current, &new);

        assert_eq!(merged.entries.len(), 2);
        assert_eq!(merged.entries.get("QM_PROJECTS").unwrap(), "/new/path");
        assert_eq!(merged.entries.get("QM_HOME").unwrap(), "/home/user");
    }

    #[test]
    fn tunable_set_serialization_roundtrip() {
        let mut original = TunableSet::new();
        original
            .entries
            .insert("QM_PROJECTS".to_string(), "/home/user/Projects".to_string());
        original
            .entries
            .insert("QM_SDK".to_string(), "/opt/sdk".to_string());

        let json = serde_json::to_string(&original).unwrap();
        let deserialized: TunableSet = serde_json::from_str(&json).unwrap();

        assert_eq!(deserialized.entries.len(), 2);
        assert_eq!(
            deserialized.entries.get("QM_PROJECTS").unwrap(),
            "/home/user/Projects"
        );
        assert_eq!(deserialized.entries.get("QM_SDK").unwrap(), "/opt/sdk");
    }

    #[test]
    fn generate_and_parse_roundtrip() {
        let mut original = TunableSet::new();
        original
            .entries
            .insert("QM_PROJECTS".to_string(), "/home/user/Projects".to_string());
        original
            .entries
            .insert("QM_SDK".to_string(), "/opt/sdk".to_string());
        original
            .entries
            .insert("QM_HOME".to_string(), "/home/user".to_string());

        let content = generate_tunables_content(&original);
        let parsed = parse_tunables_content(&content);

        assert_eq!(parsed.entries, original.entries);
    }

    #[test]
    fn parse_tunables_content_handles_equals_in_value() {
        let content = "@{QM_VAR}=/path/with=equals\n";
        let tunables = parse_tunables_content(content);
        assert_eq!(
            tunables.entries.get("QM_VAR").unwrap(),
            "/path/with=equals"
        );
    }
}
