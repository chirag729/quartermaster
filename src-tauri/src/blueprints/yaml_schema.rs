use std::collections::HashMap;

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct BlueprintDefinition {
    pub id: String,
    pub name: String,
    pub description: String,
    pub icon: String,
    #[serde(default)]
    pub builtin: bool,
    pub tasks: Vec<BlueprintTaskDef>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct BlueprintTaskDef {
    pub id: String,
    pub enabled: bool,
    #[serde(default)]
    pub config: Option<HashMap<String, String>>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_blueprint() {
        let yaml = r#"
id: dev-workstation
name: Development Workstation
description: Full development setup
icon: Monitor
builtin: true
tasks:
  - id: create-dev-folder
    enabled: true
    config:
      path: "~/Development"
  - id: flutter-sdk
    enabled: true
  - id: claude-code
    enabled: false
"#;
        let def: BlueprintDefinition = serde_yaml::from_str(yaml).unwrap();
        assert_eq!(def.id, "dev-workstation");
        assert!(def.builtin);
        assert_eq!(def.tasks.len(), 3);
        assert_eq!(
            def.tasks[0].config.as_ref().unwrap().get("path").unwrap(),
            "~/Development"
        );
        assert!(def.tasks[1].config.is_none());
        assert!(!def.tasks[2].enabled);
    }

    #[test]
    fn parse_empty_tasks() {
        let yaml = r#"
id: minimal
name: Minimal
description: Nothing
icon: Server
tasks: []
"#;
        let def: BlueprintDefinition = serde_yaml::from_str(yaml).unwrap();
        assert!(def.tasks.is_empty());
        assert!(!def.builtin);
    }
}
