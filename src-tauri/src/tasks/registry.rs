use std::collections::{HashMap, HashSet, VecDeque};

use super::embedded;
use super::script_task::ScriptTask;
use super::yaml_schema::TaskDefinition;
use super::validate::validate_task;
use super::SetupTask;

pub struct TaskRegistry {
    tasks: Vec<Box<dyn SetupTask>>,
}

impl TaskRegistry {
    pub fn new() -> Self {
        Self { tasks: Vec::new() }
    }

    pub fn register(&mut self, task: Box<dyn SetupTask>) {
        self.tasks.push(task);
    }

    /// Register a task, replacing any existing task with the same ID.
    /// Used for user-defined tasks that may override built-in ones.
    pub fn register_or_replace(&mut self, task: Box<dyn SetupTask>) {
        let id = task.id().to_string();
        if let Some(pos) = self.tasks.iter().position(|t| t.id() == id) {
            eprintln!("Warning: user task '{}' overrides built-in task", id);
            self.tasks[pos] = task;
        } else {
            self.tasks.push(task);
        }
    }

    pub fn tasks(&self) -> &[Box<dyn SetupTask>] {
        &self.tasks
    }

    pub fn get(&self, id: &str) -> Option<&dyn SetupTask> {
        self.tasks.iter().find(|t| t.id() == id).map(|t| t.as_ref())
    }

    pub fn all(&self) -> Vec<&dyn SetupTask> {
        self.tasks.iter().map(|t| t.as_ref()).collect()
    }

    /// Given a set of task IDs, expand to include all transitive dependencies
    /// and return them in topological order (dependencies first).
    ///
    /// Returns an error if a dependency cycle is detected.
    pub fn resolve_dependencies(&self, task_ids: &[String]) -> Result<Vec<String>, String> {
        // Build dependency graph from registry
        let mut deps_map: HashMap<&str, Vec<String>> = HashMap::new();
        for task in &self.tasks {
            deps_map.insert(task.id(), task.depends_on());
        }

        // Expand: collect all transitive deps via BFS
        let mut required: HashSet<String> = HashSet::new();
        let mut queue: VecDeque<String> = VecDeque::new();

        for id in task_ids {
            if !required.contains(id) {
                required.insert(id.clone());
                queue.push_back(id.clone());
            }
        }

        while let Some(current) = queue.pop_front() {
            if let Some(deps) = deps_map.get(current.as_str()) {
                for dep in deps {
                    if !required.contains(dep) {
                        required.insert(dep.clone());
                        queue.push_back(dep.clone());
                    }
                }
            }
        }

        // Topological sort using Kahn's algorithm
        let mut in_degree: HashMap<&str, usize> = HashMap::new();
        let mut adj: HashMap<&str, Vec<&str>> = HashMap::new();

        // Initialize only for tasks in our required set
        for id in &required {
            in_degree.entry(id.as_str()).or_insert(0);
            adj.entry(id.as_str()).or_insert_with(Vec::new);
        }

        for id in &required {
            if let Some(deps) = deps_map.get(id.as_str()) {
                for dep in deps {
                    if required.contains(dep) {
                        adj.entry(dep.as_str()).or_insert_with(Vec::new).push(id.as_str());
                        *in_degree.entry(id.as_str()).or_insert(0) += 1;
                    }
                }
            }
        }

        let mut sorted: Vec<String> = Vec::new();
        let mut zero_queue: VecDeque<&str> = VecDeque::new();

        // Start with zero in-degree nodes, sorted for determinism
        let mut zero_nodes: Vec<&str> = in_degree
            .iter()
            .filter(|(_, &deg)| deg == 0)
            .map(|(&id, _)| id)
            .collect();
        zero_nodes.sort();
        for node in zero_nodes {
            zero_queue.push_back(node);
        }

        while let Some(current) = zero_queue.pop_front() {
            sorted.push(current.to_string());
            if let Some(neighbors) = adj.get(current) {
                let mut next: Vec<&str> = Vec::new();
                for &neighbor in neighbors {
                    let deg = in_degree.get_mut(neighbor).unwrap();
                    *deg -= 1;
                    if *deg == 0 {
                        next.push(neighbor);
                    }
                }
                next.sort();
                for n in next {
                    zero_queue.push_back(n);
                }
            }
        }

        if sorted.len() < required.len() {
            // Some tasks couldn't be sorted — cycle detected
            let unsorted: Vec<String> = required
                .iter()
                .filter(|id| !sorted.contains(id))
                .cloned()
                .collect();
            return Err(format!(
                "Dependency cycle detected involving tasks: {}",
                unsorted.join(", ")
            ));
        }

        Ok(sorted)
    }
}

fn load_and_validate_task(path: &std::path::Path) -> Result<ScriptTask, String> {
    let content = std::fs::read_to_string(path)
        .map_err(|e| format!("failed to read: {}", e))?;
    let def: TaskDefinition = serde_yaml::from_str(&content)
        .map_err(|e| format!("YAML parse error: {}", e))?;
    let errors = validate_task(&def);
    if !errors.is_empty() {
        let msgs: Vec<String> = errors.iter().map(|e| e.to_string()).collect();
        return Err(msgs.join("; "));
    }
    Ok(ScriptTask::new(def))
}

pub fn create_registry() -> TaskRegistry {
    let mut registry = TaskRegistry::new();

    // Load built-in tasks from embedded YAML
    for yaml_str in embedded::BUILTIN_TASK_YAMLS {
        let def: TaskDefinition =
            serde_yaml::from_str(yaml_str).expect("invalid built-in task YAML");
        registry.register(Box::new(ScriptTask::new(def)));
    }

    // Load user tasks from ~/.config/quartermaster/tasks/
    let user_dir = crate::dirs::user_tasks_dir();
    if let Ok(entries) = std::fs::read_dir(&user_dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path
                .extension()
                .map_or(false, |e| e == "yaml" || e == "yml")
            {
                match load_and_validate_task(&path) {
                    Ok(task) => registry.register_or_replace(Box::new(task)),
                    Err(e) => eprintln!(
                        "Skipping invalid task {}: {}",
                        path.display(),
                        e
                    ),
                }
            }
        }
    }

    registry
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn resolve_dependencies_no_deps_flutter() {
        let registry = create_registry();
        let result = registry.resolve_dependencies(&["flutter-sdk".to_string()]).unwrap();
        assert_eq!(result, vec!["flutter-sdk".to_string()]);
    }

    #[test]
    fn resolve_dependencies_multiple_independent() {
        let registry = create_registry();
        let result = registry.resolve_dependencies(&[
            "flutter-sdk".to_string(),
            "android-sdk".to_string(),
        ]).unwrap();
        assert!(result.contains(&"flutter-sdk".to_string()));
        assert!(result.contains(&"android-sdk".to_string()));
        assert_eq!(result.len(), 2);
    }

    #[test]
    fn resolve_dependencies_no_deps() {
        let registry = create_registry();
        let result = registry.resolve_dependencies(&["create-development-folder".to_string()]).unwrap();
        assert_eq!(result, vec!["create-development-folder".to_string()]);
    }

    #[test]
    fn resolve_dependencies_empty_input() {
        let registry = create_registry();
        let result = registry.resolve_dependencies(&[]).unwrap();
        assert!(result.is_empty());
    }

    #[test]
    fn registry_loads_all_builtin_tasks() {
        let registry = create_registry();
        let ids: Vec<&str> = registry.tasks().iter().map(|t| t.id()).collect();
        assert!(ids.contains(&"create-development-folder"));
        assert!(ids.contains(&"flutter-sdk"));
        assert!(ids.contains(&"android-sdk"));
        assert!(ids.contains(&"intellij-idea"));
        assert!(ids.contains(&"claude-code"));
        assert!(ids.contains(&"git-ssh"));
    }

    #[test]
    fn builtin_task_metadata_preserved() {
        let registry = create_registry();
        let flutter = registry.get("flutter-sdk").unwrap();
        assert_eq!(flutter.name(), "Flutter SDK");
        assert_eq!(flutter.category(), "SDKs");
        assert_eq!(flutter.tags(), vec!["mobile".to_string(), "flutter".to_string()]);
        assert!(flutter.depends_on().is_empty());
    }
}
