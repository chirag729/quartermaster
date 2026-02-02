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
depends_on: [create-sdk-folder]
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
        assert_eq!(def.depends_on, vec!["create-sdk-folder"]);
        assert_eq!(def.config.len(), 2);
        assert_eq!(def.config[0].field_type, "select");
        assert_eq!(def.config[0].options.as_ref().unwrap().len(), 3);
        assert_eq!(def.steps.len(), 2);
    }
}
