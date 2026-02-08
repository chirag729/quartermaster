use std::collections::HashMap;

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct TaskDefinition {
    pub id: String,
    pub name: String,
    pub description: String,
    pub icon: String,
    pub category: String,
    #[serde(default)]
    pub tags: Vec<String>,
    #[serde(default = "default_privilege")]
    pub privilege: String,
    #[serde(default = "default_target")]
    pub target: String,
    #[serde(default)]
    pub depends_on: Vec<String>,
    #[serde(default)]
    pub config: Vec<ConfigFieldDef>,
    pub detect: String,
    pub steps: Vec<StepDef>,

    /// Optional uninstall steps (reverse of install).
    #[serde(default)]
    pub uninstall: Vec<StepDef>,

    // ── Schema v2 fields (all optional for backwards compatibility) ──

    /// Semantic version of the software this task installs.
    #[serde(default)]
    pub version: Option<String>,

    /// References to shared blueprint variables this task consumes.
    #[serde(default)]
    pub variables: Vec<String>,

    /// Download specification for fetching an archive or binary.
    #[serde(default)]
    pub download: Option<DownloadDef>,

    /// Desktop entry (.desktop file) generation spec.
    #[serde(default)]
    pub desktop: Option<DesktopDef>,

    /// AppArmor profile to install alongside this task.
    #[serde(default)]
    pub apparmor: Option<AppArmorDef>,

    /// Optional shell command to detect the installed version.
    /// The command should output just the version string (e.g., "3.24.0").
    #[serde(default)]
    pub version_detect: Option<String>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ConfigFieldDef {
    pub key: String,
    pub label: String,
    #[serde(rename = "type")]
    pub field_type: String,
    #[serde(default)]
    pub default: String,
    #[serde(default)]
    pub options: Option<Vec<String>>,
    #[serde(default)]
    pub required: Option<bool>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct StepDef {
    pub name: String,
    pub progress: u8,
    pub run: String,
}

/// Download specification for archives or binaries.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct DownloadDef {
    /// URL template (may contain {{variable}} placeholders).
    pub url: String,
    /// Optional URL for a checksum file (SHA-256).
    #[serde(default)]
    pub checksum_url: Option<String>,
    /// Optional inline SHA-256 checksum.
    #[serde(default)]
    pub checksum_sha256: Option<String>,
    /// Archive extraction format: tar.gz, zip, or none (raw binary).
    #[serde(default = "default_extract")]
    pub extract: String,
    /// Target directory for extraction (template-expanded).
    #[serde(default)]
    pub dest: Option<String>,
}

fn default_extract() -> String {
    "none".to_string()
}

/// Desktop entry generation specification (.desktop file).
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct DesktopDef {
    /// Display name in application launchers.
    pub name: String,
    /// Relative path to the icon file within the install directory.
    #[serde(default)]
    pub icon_source: Option<String>,
    /// Relative path to the executable within the install directory.
    pub exec: String,
    /// Freedesktop.org categories.
    #[serde(default)]
    pub categories: Vec<String>,
    /// MIME types this application can handle.
    #[serde(default)]
    pub mime_types: Vec<String>,
    /// Whether the application runs in a terminal.
    #[serde(default)]
    pub terminal: bool,
}

/// AppArmor profile reference bundled with a task.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct AppArmorDef {
    /// Profile name (references apparmor/profiles/<name> in the blueprint).
    pub profile: String,
    /// Abstractions this profile depends on (installed alongside).
    #[serde(default)]
    pub abstractions: Vec<String>,
    /// Custom tunables to set (key=value pairs for /etc/apparmor.d/tunables/quartermaster).
    #[serde(default)]
    pub tunables: HashMap<String, String>,
}

fn default_privilege() -> String {
    "user".to_string()
}

fn default_target() -> String {
    "any".to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_minimal_task() {
        let yaml = r#"
id: test-task
name: Test Task
description: A test
icon: Star
category: Testing
detect: "test -f /tmp/x"
steps:
  - name: Do thing
    progress: 100
    run: "echo hello"
"#;
        let def: TaskDefinition = serde_yaml::from_str(yaml).unwrap();
        assert_eq!(def.id, "test-task");
        assert_eq!(def.privilege, "user");
        assert_eq!(def.target, "any");
        assert!(def.tags.is_empty());
        assert!(def.depends_on.is_empty());
        assert!(def.config.is_empty());
        assert_eq!(def.steps.len(), 1);
        assert!(def.version.is_none());
        assert!(def.download.is_none());
        assert!(def.desktop.is_none());
        assert!(def.apparmor.is_none());
        assert!(def.variables.is_empty());
    }

    #[test]
    fn parse_full_task() {
        let yaml = r#"
id: flutter-sdk
name: Flutter SDK
description: Download and install Flutter SDK
icon: Smartphone
category: SDKs
tags: [mobile, flutter]
privilege: user
target: any
depends_on: [create-development-folder]
config:
  - key: channel
    label: Flutter Channel
    type: select
    default: stable
    options: [stable, beta, dev]
    required: false
  - key: sdk_base_path
    label: SDK base path
    type: path
    default: "~/SDK"
    required: false
detect: |
  test -f "{{sdk_base_path}}/flutter/bin/flutter"
steps:
  - name: Clone Flutter
    progress: 50
    run: |
      git clone https://github.com/flutter/flutter.git
  - name: Verify
    progress: 100
    run: |
      echo done
"#;
        let def: TaskDefinition = serde_yaml::from_str(yaml).unwrap();
        assert_eq!(def.id, "flutter-sdk");
        assert_eq!(def.tags, vec!["mobile", "flutter"]);
        assert_eq!(def.depends_on, vec!["create-development-folder"]);
        assert_eq!(def.config.len(), 2);
        assert_eq!(def.config[0].field_type, "select");
        assert_eq!(def.config[0].options.as_ref().unwrap().len(), 3);
        assert_eq!(def.steps.len(), 2);
    }

    #[test]
    fn parse_v2_task_with_download() {
        let yaml = r#"
id: intellij-idea
name: IntelliJ IDEA
description: Download and install IntelliJ IDEA
icon: Code
category: Applications
version: "2025.2"
variables: [dev_folder]
download:
  url: "https://download.jetbrains.com/idea/ideaIC-{{version}}.tar.gz"
  checksum_sha256: "abc123"
  extract: tar.gz
  dest: "{{install_path}}"
desktop:
  name: IntelliJ IDEA
  icon_source: bin/idea.svg
  exec: bin/idea
  categories: [Development, IDE]
apparmor:
  profile: intellij
  abstractions: [shell-environment, git-client]
  tunables:
    QM_PROJECTS: "{{dev_folder}}/Projects"
detect: "test -d /tmp/idea"
steps:
  - name: Extract
    progress: 100
    run: "echo done"
"#;
        let def: TaskDefinition = serde_yaml::from_str(yaml).unwrap();
        assert_eq!(def.version.as_deref(), Some("2025.2"));
        assert_eq!(def.variables, vec!["dev_folder"]);

        let dl = def.download.unwrap();
        assert!(dl.url.contains("ideaIC"));
        assert_eq!(dl.extract, "tar.gz");
        assert_eq!(dl.checksum_sha256.as_deref(), Some("abc123"));

        let desktop = def.desktop.unwrap();
        assert_eq!(desktop.name, "IntelliJ IDEA");
        assert_eq!(desktop.exec, "bin/idea");
        assert_eq!(desktop.categories, vec!["Development", "IDE"]);

        let aa = def.apparmor.unwrap();
        assert_eq!(aa.profile, "intellij");
        assert_eq!(aa.abstractions, vec!["shell-environment", "git-client"]);
        assert_eq!(aa.tunables.get("QM_PROJECTS").unwrap(), "{{dev_folder}}/Projects");
    }
}
