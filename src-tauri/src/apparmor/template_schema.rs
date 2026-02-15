use std::collections::HashSet;

use serde::{Deserialize, Serialize};

fn default_mode() -> String {
    "complain".to_string()
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ProfileTemplate {
    pub id: String,
    pub name: String,
    pub description: String,
    pub task_id: String,
    pub profile_name: String,
    #[serde(default = "default_mode")]
    pub mode: String,
    #[serde(default)]
    pub variables: Vec<ProfileVariable>,
    /// Fragment tags this profile subscribes to.
    /// When a completed task advertises fragments with matching tags, those fragments
    /// are automatically included in the profile via the {{fragments}} placeholder.
    #[serde(default)]
    pub subscribes_to: Vec<String>,
    pub content: String,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ProfileVariable {
    pub key: String,
    pub label: String,
    #[serde(rename = "type")]
    pub var_type: String,
    #[serde(default)]
    pub default: String,
    #[serde(default)]
    pub options: Option<Vec<String>>,
}

/// Validate a profile template, returning a list of error messages.
/// An empty list means the template is valid.
pub fn validate_profile_template(template: &ProfileTemplate) -> Vec<String> {
    let mut errors = Vec::new();

    // Required non-empty fields
    if template.id.is_empty() {
        errors.push("id must not be empty".into());
    }
    if template.name.is_empty() {
        errors.push("name must not be empty".into());
    }
    if template.description.is_empty() {
        errors.push("description must not be empty".into());
    }
    if template.task_id.is_empty() {
        errors.push("task_id must not be empty".into());
    }
    if template.profile_name.is_empty() {
        errors.push("profile_name must not be empty".into());
    }
    if template.content.is_empty() {
        errors.push("content must not be empty".into());
    }

    // Mode validation
    if template.mode != "enforce" && template.mode != "complain" {
        errors.push(format!(
            "mode must be 'enforce' or 'complain', got '{}'",
            template.mode
        ));
    }

    // Profile name validation (same rules as profile_manager)
    if !template.profile_name.is_empty() {
        if template.profile_name.contains("..") {
            errors.push("profile_name must not contain '..'".into());
        }
        if template.profile_name.starts_with('-') {
            errors.push("profile_name must not start with '-'".into());
        }
        if !template
            .profile_name
            .chars()
            .all(|c| c.is_alphanumeric() || "._/-".contains(c))
        {
            errors.push(
                "profile_name contains invalid characters (allowed: alphanumeric, ., _, /, -)"
                    .into(),
            );
        }
    }

    // subscribes_to validation: no empty tags
    for tag in &template.subscribes_to {
        if tag.is_empty() {
            errors.push("subscribes_to contains an empty tag".into());
        }
    }

    // Variable validation
    let mut seen_keys = HashSet::new();
    for var in &template.variables {
        if var.key.is_empty() {
            errors.push("variable key must not be empty".into());
            continue;
        }
        if !seen_keys.insert(&var.key) {
            errors.push(format!("duplicate variable key: '{}'", var.key));
        }
        match var.var_type.as_str() {
            "text" | "path" => {}
            "select" => {
                match &var.options {
                    Some(opts) if !opts.is_empty() => {}
                    _ => {
                        errors.push(format!(
                            "variable '{}' of type 'select' must have non-empty options",
                            var.key
                        ));
                    }
                }
            }
            other => {
                errors.push(format!(
                    "variable '{}' has invalid type '{}' (must be text, path, or select)",
                    var.key, other
                ));
            }
        }
    }

    // Extract {{var}} references from content and check for unused variables
    let mut referenced_keys: HashSet<&str> = HashSet::new();
    let mut i = 0;
    let content_bytes = template.content.as_bytes();
    while i + 3 < content_bytes.len() {
        if content_bytes[i] == b'{' && content_bytes[i + 1] == b'{' {
            if let Some(end) = template.content[i + 2..].find("}}") {
                let var_name = &template.content[i + 2..i + 2 + end];
                let var_name = var_name.trim();
                if !var_name.is_empty() {
                    referenced_keys.insert(var_name);
                }
                i = i + 2 + end + 2;
            } else {
                i += 1;
            }
        } else {
            i += 1;
        }
    }

    // Warn about declared variables never referenced in content
    for var in &template.variables {
        if !var.key.is_empty() && !referenced_keys.contains(var.key.as_str()) {
            errors.push(format!(
                "variable '{}' is declared but never referenced in template content",
                var.key
            ));
        }
    }

    errors
}

/// Extract all {{variable}} references from a template's content.
pub fn extract_content_variables(content: &str) -> Vec<String> {
    let mut vars = Vec::new();
    let mut i = 0;
    let bytes = content.as_bytes();
    while i + 3 < bytes.len() {
        if bytes[i] == b'{' && bytes[i + 1] == b'{' {
            if let Some(end) = content[i + 2..].find("}}") {
                let var_name = content[i + 2..i + 2 + end].trim().to_string();
                if !var_name.is_empty() && !vars.contains(&var_name) {
                    vars.push(var_name);
                }
                i = i + 2 + end + 2;
            } else {
                i += 1;
            }
        } else {
            i += 1;
        }
    }
    vars
}

// Response types for IPC

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ProfileTemplateStatus {
    NotInstalled,
    Installed,
    Stale,
    TaskNotCompleted,
}

#[derive(Debug, Clone, Serialize)]
pub struct ProfileTemplateInfo {
    pub id: String,
    pub name: String,
    pub description: String,
    pub task_id: String,
    pub profile_name: String,
    pub mode: String,
    pub variables: Vec<ProfileVariable>,
    pub subscribes_to: Vec<String>,
    pub status: ProfileTemplateStatus,
}

#[derive(Debug, Clone, Serialize)]
pub struct ProfileTemplateConfig {
    pub template_id: String,
    pub fields: Vec<ResolvedConfigField>,
}

#[derive(Debug, Clone, Serialize)]
pub struct ResolvedConfigField {
    pub key: String,
    pub label: String,
    pub field_type: String,
    pub value: String,
    pub default: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub options: Option<Vec<String>>,
    pub source: String, // "task" | "profile"
}

#[derive(Debug, Clone, Serialize)]
pub struct SyncResultInfo {
    pub profile_id: String,
    pub profile_name: String,
    pub updated: bool,
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_minimal_template() -> ProfileTemplate {
        ProfileTemplate {
            id: "test".into(),
            name: "Test Profile".into(),
            description: "A test profile".into(),
            task_id: "test-task".into(),
            profile_name: "quartermaster.test".into(),
            mode: "complain".into(),
            variables: vec![],
            subscribes_to: vec![],
            content: "profile test flags=({{mode}}) { }".into(),
        }
    }

    fn make_full_template() -> ProfileTemplate {
        ProfileTemplate {
            id: "flutter-sdk".into(),
            name: "Flutter SDK".into(),
            description: "Confine Flutter SDK".into(),
            task_id: "flutter-sdk".into(),
            profile_name: "quartermaster.flutter".into(),
            mode: "complain".into(),
            variables: vec![
                ProfileVariable {
                    key: "projects_path".into(),
                    label: "Projects Directory".into(),
                    var_type: "path".into(),
                    default: "~/Development/Projects".into(),
                    options: None,
                },
            ],
            subscribes_to: vec![],
            content: "profile flutter {{sdk_base_path}}/flutter flags=({{mode}}) {\n  {{home}}/.pub-cache/** rwk,\n  {{projects_path}}/** rwk,\n}".into(),
        }
    }

    #[test]
    fn validate_minimal_template_passes() {
        let template = make_minimal_template();
        let errors = validate_profile_template(&template);
        assert!(errors.is_empty(), "errors: {:?}", errors);
    }

    #[test]
    fn validate_full_template_passes() {
        let template = make_full_template();
        let errors = validate_profile_template(&template);
        assert!(errors.is_empty(), "errors: {:?}", errors);
    }

    #[test]
    fn validate_empty_id_fails() {
        let mut template = make_minimal_template();
        template.id = "".into();
        let errors = validate_profile_template(&template);
        assert!(errors.iter().any(|e| e.contains("id must not be empty")));
    }

    #[test]
    fn validate_empty_name_fails() {
        let mut template = make_minimal_template();
        template.name = "".into();
        let errors = validate_profile_template(&template);
        assert!(errors.iter().any(|e| e.contains("name must not be empty")));
    }

    #[test]
    fn validate_empty_content_fails() {
        let mut template = make_minimal_template();
        template.content = "".into();
        let errors = validate_profile_template(&template);
        assert!(errors
            .iter()
            .any(|e| e.contains("content must not be empty")));
    }

    #[test]
    fn validate_invalid_mode_fails() {
        let mut template = make_minimal_template();
        template.mode = "disabled".into();
        let errors = validate_profile_template(&template);
        assert!(errors
            .iter()
            .any(|e| e.contains("mode must be 'enforce' or 'complain'")));
    }

    #[test]
    fn validate_enforce_mode_passes() {
        let mut template = make_minimal_template();
        template.mode = "enforce".into();
        let errors = validate_profile_template(&template);
        assert!(errors.is_empty(), "errors: {:?}", errors);
    }

    #[test]
    fn validate_profile_name_with_traversal_fails() {
        let mut template = make_minimal_template();
        template.profile_name = "quartermaster..test".into();
        let errors = validate_profile_template(&template);
        assert!(errors
            .iter()
            .any(|e| e.contains("profile_name must not contain '..'")));
    }

    #[test]
    fn validate_profile_name_starting_with_dash_fails() {
        let mut template = make_minimal_template();
        template.profile_name = "-quartermaster.test".into();
        let errors = validate_profile_template(&template);
        assert!(errors
            .iter()
            .any(|e| e.contains("profile_name must not start with '-'")));
    }

    #[test]
    fn validate_profile_name_with_invalid_chars_fails() {
        let mut template = make_minimal_template();
        template.profile_name = "quartermaster test!".into();
        let errors = validate_profile_template(&template);
        assert!(errors
            .iter()
            .any(|e| e.contains("profile_name contains invalid characters")));
    }

    #[test]
    fn validate_variable_empty_key_fails() {
        let mut template = make_minimal_template();
        template.variables = vec![ProfileVariable {
            key: "".into(),
            label: "Test".into(),
            var_type: "text".into(),
            default: "".into(),
            options: None,
        }];
        let errors = validate_profile_template(&template);
        assert!(errors
            .iter()
            .any(|e| e.contains("variable key must not be empty")));
    }

    #[test]
    fn validate_variable_invalid_type_fails() {
        let mut template = make_minimal_template();
        template.variables = vec![ProfileVariable {
            key: "test_key".into(),
            label: "Test".into(),
            var_type: "number".into(),
            default: "".into(),
            options: None,
        }];
        let errors = validate_profile_template(&template);
        assert!(errors.iter().any(|e| e.contains("invalid type 'number'")));
    }

    #[test]
    fn validate_select_without_options_fails() {
        let mut template = make_minimal_template();
        template.variables = vec![ProfileVariable {
            key: "test_key".into(),
            label: "Test".into(),
            var_type: "select".into(),
            default: "".into(),
            options: None,
        }];
        let errors = validate_profile_template(&template);
        assert!(errors
            .iter()
            .any(|e| e.contains("must have non-empty options")));
    }

    #[test]
    fn validate_select_with_empty_options_fails() {
        let mut template = make_minimal_template();
        template.variables = vec![ProfileVariable {
            key: "test_key".into(),
            label: "Test".into(),
            var_type: "select".into(),
            default: "".into(),
            options: Some(vec![]),
        }];
        let errors = validate_profile_template(&template);
        assert!(errors
            .iter()
            .any(|e| e.contains("must have non-empty options")));
    }

    #[test]
    fn validate_select_with_options_passes() {
        let mut template = make_minimal_template();
        template.content = "profile test flags=({{mode}}) { {{test_key}} }".into();
        template.variables = vec![ProfileVariable {
            key: "test_key".into(),
            label: "Test".into(),
            var_type: "select".into(),
            default: "a".into(),
            options: Some(vec!["a".into(), "b".into()]),
        }];
        let errors = validate_profile_template(&template);
        assert!(errors.is_empty(), "errors: {:?}", errors);
    }

    #[test]
    fn validate_duplicate_variable_keys_fails() {
        let mut template = make_minimal_template();
        template.variables = vec![
            ProfileVariable {
                key: "dup".into(),
                label: "First".into(),
                var_type: "text".into(),
                default: "".into(),
                options: None,
            },
            ProfileVariable {
                key: "dup".into(),
                label: "Second".into(),
                var_type: "text".into(),
                default: "".into(),
                options: None,
            },
        ];
        let errors = validate_profile_template(&template);
        assert!(errors
            .iter()
            .any(|e| e.contains("duplicate variable key: 'dup'")));
    }

    #[test]
    fn extract_variables_from_content() {
        let content = "profile {{name}} flags=({{mode}}) {\n  {{home}}/.cache/** rwk,\n  {{projects_path}}/** rwk,\n}";
        let vars = extract_content_variables(content);
        assert_eq!(vars, vec!["name", "mode", "home", "projects_path"]);
    }

    #[test]
    fn extract_variables_no_duplicates() {
        let content = "{{mode}} and {{mode}} again";
        let vars = extract_content_variables(content);
        assert_eq!(vars, vec!["mode"]);
    }

    #[test]
    fn parse_minimal_yaml() {
        let yaml = r#"
id: test
name: Test
description: A test
task_id: test-task
profile_name: quartermaster.test
content: "profile test { }"
"#;
        let template: ProfileTemplate = serde_yaml::from_str(yaml).unwrap();
        assert_eq!(template.id, "test");
        assert_eq!(template.mode, "complain"); // default
        assert!(template.variables.is_empty());
    }

    #[test]
    fn parse_full_yaml() {
        let yaml = r#"
id: flutter-sdk
name: Flutter SDK
description: Confine Flutter SDK
task_id: flutter-sdk
profile_name: quartermaster.flutter
mode: enforce
variables:
  - key: projects_path
    label: Projects Directory
    type: path
    default: "~/Projects"
content: |
  profile flutter flags=({{mode}}) { }
"#;
        let template: ProfileTemplate = serde_yaml::from_str(yaml).unwrap();
        assert_eq!(template.id, "flutter-sdk");
        assert_eq!(template.mode, "enforce");
        assert_eq!(template.variables.len(), 1);
        assert_eq!(template.variables[0].key, "projects_path");
        assert_eq!(template.variables[0].var_type, "path");
    }

    #[test]
    fn parse_template_with_subscribes_to() {
        let yaml = r#"
id: intellij
name: IntelliJ
description: Confine IntelliJ
task_id: intellij-idea
profile_name: quartermaster.intellij
subscribes_to:
  - sdk
  - mobile-sdk
content: |
  profile intellij flags=({{mode}}) {
  {{fragments}}
  }
"#;
        let template: ProfileTemplate = serde_yaml::from_str(yaml).unwrap();
        assert_eq!(template.subscribes_to, vec!["sdk", "mobile-sdk"]);
    }

    #[test]
    fn validate_template_with_empty_subscription_tag_fails() {
        let mut template = make_minimal_template();
        template.subscribes_to = vec!["sdk".into(), "".into()];
        let errors = validate_profile_template(&template);
        assert!(
            errors.iter().any(|e| e.contains("subscribes_to contains an empty tag")),
            "Should reject empty tag: {:?}", errors
        );
    }

    #[test]
    fn validate_template_with_valid_subscription_tags_passes() {
        let mut template = make_minimal_template();
        template.subscribes_to = vec!["sdk".into(), "mobile-sdk".into()];
        let errors = validate_profile_template(&template);
        assert!(errors.is_empty(), "Valid subscription tags should pass: {:?}", errors);
    }
}
