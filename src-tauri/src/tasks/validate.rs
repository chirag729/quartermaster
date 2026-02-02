use std::collections::HashSet;
use std::fmt;

use super::yaml_schema::TaskDefinition;

#[derive(Debug)]
pub struct ValidationError {
    pub field: String,
    pub message: String,
}

impl fmt::Display for ValidationError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}: {}", self.field, self.message)
    }
}

/// Validate a TaskDefinition, returning a list of errors (empty = valid).
pub fn validate_task(def: &TaskDefinition) -> Vec<ValidationError> {
    let mut errors = Vec::new();

    if def.id.is_empty() {
        errors.push(ValidationError {
            field: "id".into(),
            message: "must not be empty".into(),
        });
    }
    if def.name.is_empty() {
        errors.push(ValidationError {
            field: "name".into(),
            message: "must not be empty".into(),
        });
    }
    if def.description.is_empty() {
        errors.push(ValidationError {
            field: "description".into(),
            message: "must not be empty".into(),
        });
    }
    if def.icon.is_empty() {
        errors.push(ValidationError {
            field: "icon".into(),
            message: "must not be empty".into(),
        });
    }
    if def.category.is_empty() {
        errors.push(ValidationError {
            field: "category".into(),
            message: "must not be empty".into(),
        });
    }

    // Validate privilege
    match def.privilege.as_str() {
        "user" | "admin" => {}
        other => {
            errors.push(ValidationError {
                field: "privilege".into(),
                message: format!("must be \"user\" or \"admin\", got \"{}\"", other),
            });
        }
    }

    // Validate target
    match def.target.as_str() {
        "local_only" | "remote_only" | "any" => {}
        other => {
            errors.push(ValidationError {
                field: "target".into(),
                message: format!(
                    "must be \"local_only\", \"remote_only\", or \"any\", got \"{}\"",
                    other
                ),
            });
        }
    }

    // Validate config fields
    let mut config_keys: HashSet<&str> = HashSet::new();
    for (i, field) in def.config.iter().enumerate() {
        let prefix = format!("config[{}]", i);

        if field.key.is_empty() {
            errors.push(ValidationError {
                field: format!("{}.key", prefix),
                message: "must not be empty".into(),
            });
        }
        if !config_keys.insert(&field.key) {
            errors.push(ValidationError {
                field: format!("{}.key", prefix),
                message: format!("duplicate config key \"{}\"", field.key),
            });
        }

        match field.field_type.as_str() {
            "text" | "path" | "select" => {}
            other => {
                errors.push(ValidationError {
                    field: format!("{}.type", prefix),
                    message: format!(
                        "must be \"text\", \"path\", or \"select\", got \"{}\"",
                        other
                    ),
                });
            }
        }

        if field.field_type == "select" {
            match &field.options {
                Some(opts) if opts.is_empty() => {
                    errors.push(ValidationError {
                        field: format!("{}.options", prefix),
                        message: "select type must have non-empty options".into(),
                    });
                }
                None => {
                    errors.push(ValidationError {
                        field: format!("{}.options", prefix),
                        message: "select type must have options".into(),
                    });
                }
                _ => {}
            }
        }
    }

    // Validate steps
    if def.steps.is_empty() {
        errors.push(ValidationError {
            field: "steps".into(),
            message: "must have at least one step".into(),
        });
    }
    for (i, step) in def.steps.iter().enumerate() {
        let prefix = format!("steps[{}]", i);
        if step.name.is_empty() {
            errors.push(ValidationError {
                field: format!("{}.name", prefix),
                message: "must not be empty".into(),
            });
        }
        if step.progress > 100 {
            errors.push(ValidationError {
                field: format!("{}.progress", prefix),
                message: format!("must be 0-100, got {}", step.progress),
            });
        }
        if step.run.trim().is_empty() {
            errors.push(ValidationError {
                field: format!("{}.run", prefix),
                message: "must not be empty".into(),
            });
        }

        // Validate template variables in run
        validate_template_vars(&step.run, &config_keys, &format!("{}.run", prefix), &mut errors);
    }

    // Validate template variables in detect
    validate_template_vars(&def.detect, &config_keys, "detect", &mut errors);

    errors
}

fn validate_template_vars(
    script: &str,
    config_keys: &HashSet<&str>,
    field: &str,
    errors: &mut Vec<ValidationError>,
) {
    let mut start = 0;
    while let Some(pos) = script[start..].find("{{") {
        let abs_pos = start + pos;
        if let Some(end) = script[abs_pos + 2..].find("}}") {
            let var_name = script[abs_pos + 2..abs_pos + 2 + end].trim();
            if var_name != "home" && !config_keys.contains(var_name) {
                errors.push(ValidationError {
                    field: field.to_string(),
                    message: format!(
                        "template variable \"{{{{{}}}}}\" does not reference a config key or \"home\"",
                        var_name
                    ),
                });
            }
            start = abs_pos + 2 + end + 2;
        } else {
            break;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tasks::yaml_schema::{ConfigFieldDef, StepDef, TaskDefinition};

    fn minimal_valid_task() -> TaskDefinition {
        TaskDefinition {
            id: "test".into(),
            name: "Test".into(),
            description: "A test task".into(),
            icon: "Star".into(),
            category: "Testing".into(),
            tags: vec![],
            privilege: "user".into(),
            target: "any".into(),
            depends_on: vec![],
            config: vec![],
            detect: "test -f /tmp/x".into(),
            steps: vec![StepDef {
                name: "Do it".into(),
                progress: 100,
                run: "echo hello".into(),
            }],
        }
    }

    #[test]
    fn valid_task_passes() {
        let errors = validate_task(&minimal_valid_task());
        assert!(errors.is_empty(), "expected no errors, got: {:?}", errors);
    }

    #[test]
    fn empty_id_fails() {
        let mut task = minimal_valid_task();
        task.id = String::new();
        let errors = validate_task(&task);
        assert!(errors.iter().any(|e| e.field == "id"));
    }

    #[test]
    fn invalid_privilege_fails() {
        let mut task = minimal_valid_task();
        task.privilege = "root".into();
        let errors = validate_task(&task);
        assert!(errors.iter().any(|e| e.field == "privilege"));
    }

    #[test]
    fn invalid_target_fails() {
        let mut task = minimal_valid_task();
        task.target = "both".into();
        let errors = validate_task(&task);
        assert!(errors.iter().any(|e| e.field == "target"));
    }

    #[test]
    fn select_without_options_fails() {
        let mut task = minimal_valid_task();
        task.config.push(ConfigFieldDef {
            key: "choice".into(),
            label: "A Choice".into(),
            field_type: "select".into(),
            default: "a".into(),
            options: None,
            required: None,
        });
        let errors = validate_task(&task);
        assert!(errors.iter().any(|e| e.field.contains("options")));
    }

    #[test]
    fn empty_steps_fails() {
        let mut task = minimal_valid_task();
        task.steps.clear();
        let errors = validate_task(&task);
        assert!(errors.iter().any(|e| e.field == "steps"));
    }

    #[test]
    fn unknown_template_variable_fails() {
        let mut task = minimal_valid_task();
        task.detect = "test -f {{nonexistent}}".into();
        let errors = validate_task(&task);
        assert!(errors.iter().any(|e| e.message.contains("nonexistent")));
    }

    #[test]
    fn home_template_variable_passes() {
        let mut task = minimal_valid_task();
        task.detect = "test -f {{home}}/.bashrc".into();
        let errors = validate_task(&task);
        assert!(errors.is_empty(), "expected no errors, got: {:?}", errors);
    }

    #[test]
    fn config_key_template_variable_passes() {
        let mut task = minimal_valid_task();
        task.config.push(ConfigFieldDef {
            key: "path".into(),
            label: "Path".into(),
            field_type: "path".into(),
            default: "~/test".into(),
            options: None,
            required: Some(true),
        });
        task.detect = "test -d {{path}}".into();
        task.steps[0].run = "mkdir -p {{path}}".into();
        let errors = validate_task(&task);
        assert!(errors.is_empty(), "expected no errors, got: {:?}", errors);
    }

    #[test]
    fn duplicate_config_key_fails() {
        let mut task = minimal_valid_task();
        let field = ConfigFieldDef {
            key: "path".into(),
            label: "Path".into(),
            field_type: "path".into(),
            default: "~/test".into(),
            options: None,
            required: Some(true),
        };
        task.config.push(field.clone());
        task.config.push(field);
        let errors = validate_task(&task);
        assert!(errors.iter().any(|e| e.message.contains("duplicate")));
    }
}
