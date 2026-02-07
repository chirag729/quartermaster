use std::collections::HashSet;
use std::fmt;

use super::yaml_schema::BlueprintDefinition;

#[derive(Debug)]
pub struct BlueprintValidationError {
    pub field: String,
    pub message: String,
}

impl fmt::Display for BlueprintValidationError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}: {}", self.field, self.message)
    }
}

/// Validate a BlueprintDefinition, returning a list of errors (empty = valid).
pub fn validate_blueprint(def: &BlueprintDefinition) -> Vec<BlueprintValidationError> {
    let mut errors = Vec::new();

    if def.id.is_empty() {
        errors.push(BlueprintValidationError {
            field: "id".into(),
            message: "must not be empty".into(),
        });
    }
    if def.name.is_empty() {
        errors.push(BlueprintValidationError {
            field: "name".into(),
            message: "must not be empty".into(),
        });
    }
    if def.description.is_empty() {
        errors.push(BlueprintValidationError {
            field: "description".into(),
            message: "must not be empty".into(),
        });
    }
    if def.icon.is_empty() {
        errors.push(BlueprintValidationError {
            field: "icon".into(),
            message: "must not be empty".into(),
        });
    }

    let mut seen_task_ids: HashSet<&str> = HashSet::new();
    for (i, task) in def.tasks.iter().enumerate() {
        if task.id.is_empty() {
            errors.push(BlueprintValidationError {
                field: format!("tasks[{}].id", i),
                message: "must not be empty".into(),
            });
        } else if !seen_task_ids.insert(&task.id) {
            errors.push(BlueprintValidationError {
                field: format!("tasks[{}].id", i),
                message: format!("duplicate task id \"{}\"", task.id),
            });
        }
    }

    errors
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::blueprints::yaml_schema::{BlueprintDefinition, BlueprintTaskDef};

    fn minimal_valid_blueprint() -> BlueprintDefinition {
        BlueprintDefinition {
            id: "test".into(),
            name: "Test".into(),
            description: "A test blueprint".into(),
            icon: "Star".into(),
            builtin: false,
            version: "1.0.0".into(),
            extends: None,
            tasks: vec![BlueprintTaskDef {
                id: "some-task".into(),
                enabled: true,
                config: None,
            }],
        }
    }

    #[test]
    fn valid_blueprint_passes() {
        let errors = validate_blueprint(&minimal_valid_blueprint());
        assert!(errors.is_empty(), "expected no errors, got: {:?}", errors);
    }

    #[test]
    fn empty_id_fails() {
        let mut bp = minimal_valid_blueprint();
        bp.id = String::new();
        let errors = validate_blueprint(&bp);
        assert!(errors.iter().any(|e| e.field == "id"));
    }

    #[test]
    fn empty_task_id_fails() {
        let mut bp = minimal_valid_blueprint();
        bp.tasks[0].id = String::new();
        let errors = validate_blueprint(&bp);
        assert!(errors.iter().any(|e| e.field.contains("tasks[0].id")));
    }

    #[test]
    fn duplicate_task_id_fails() {
        let mut bp = minimal_valid_blueprint();
        bp.tasks.push(BlueprintTaskDef {
            id: "some-task".into(),
            enabled: true,
            config: None,
        });
        let errors = validate_blueprint(&bp);
        assert!(errors.iter().any(|e| e.message.contains("duplicate")));
    }

    #[test]
    fn empty_tasks_is_valid() {
        let mut bp = minimal_valid_blueprint();
        bp.tasks.clear();
        let errors = validate_blueprint(&bp);
        assert!(errors.is_empty());
    }
}
