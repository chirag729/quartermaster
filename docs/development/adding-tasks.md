# Adding Tasks

This guide covers how to add a new setup task to Anvil. Tasks are the fundamental unit of work -- each task performs a single provisioning action (installing a tool, creating a directory, configuring a service, etc.) and can run on local or remote machines via the `CommandExecutor` abstraction.

## Overview

1. Create `src-tauri/src/tasks/your_task.rs` implementing the `SetupTask` trait.
2. Add `pub mod your_task;` to `src-tauri/src/tasks/mod.rs`.
3. Register the task in `create_registry()` in `src-tauri/src/tasks/registry.rs`.
4. The frontend picks it up automatically via `list_tasks` / `detect_all_states`.

No frontend changes are required. The task list, detection, and execution all flow through the registry.

## The SetupTask Trait

Every task implements the `SetupTask` trait defined in `src-tauri/src/tasks/mod.rs`:

```rust
#[async_trait::async_trait]
pub trait SetupTask: Send + Sync {
    // --- Metadata ---
    fn id(&self) -> &str;
    fn name(&self) -> &str;
    fn description(&self) -> &str;
    fn icon(&self) -> &str;                      // Lucide icon name (e.g. "FolderOpen", "Terminal")
    fn category(&self) -> &str;                  // Grouping label (e.g. "Folders", "SDK", "IDE")

    // --- Optional metadata (have defaults) ---
    fn tags(&self) -> Vec<String> { vec![] }     // Categorization tags
    fn execution_target(&self) -> ExecutionTarget { ExecutionTarget::Any }
    fn depends_on(&self) -> Vec<String> { vec![] }

    // --- Configuration ---
    fn privilege_level(&self) -> PrivilegeLevel; // User or Admin
    fn config_schema(&self) -> Vec<ConfigField>; // Configurable fields shown in the UI

    // --- Core logic ---
    async fn detect_state(
        &self,
        config: &HashMap<String, Value>,
        exec: &dyn CommandExecutor,
    ) -> TaskStatus;

    async fn execute(
        &self,
        config: &HashMap<String, Value>,
        exec: &dyn CommandExecutor,
        on_progress: &ProgressCallback,
    ) -> Result<(), AppError>;
}
```

### Metadata methods

| Method | Purpose |
|--------|---------|
| `id()` | Unique identifier (kebab-case, e.g. `"install-docker"`). Used in configs and blueprints. |
| `name()` | Human-readable display name. |
| `description()` | Short description shown in the UI. |
| `icon()` | Lucide React icon name. The frontend renders this as a component. |
| `category()` | Grouping label for the task grid. |
| `tags()` | Optional tags for filtering and search. Defaults to empty. |
| `privilege_level()` | `PrivilegeLevel::User` or `PrivilegeLevel::Admin`. Admin tasks may trigger PolicyKit prompts. |
| `execution_target()` | `ExecutionTarget::LocalOnly`, `RemoteOnly`, or `Any`. Controls which node types this task supports. Defaults to `Any`. |
| `depends_on()` | List of task IDs that must be completed first. Defaults to empty. |
| `config_schema()` | Defines configurable fields. Return an empty `vec![]` if the task has no configuration. |

### ConfigField

Each configurable field is described by a `ConfigField`:

```rust
pub struct ConfigField {
    pub key: String,         // Config key used in the HashMap
    pub label: String,       // Display label in the UI
    pub field_type: String,  // "text", "select", "boolean", etc.
    pub default_value: String,
    pub options: Option<Vec<String>>,  // For "select" type
    pub required: bool,
}
```

### detect_state

This method checks whether the task has already been completed. It receives:

- `config` -- The merged task configuration (global defaults + blueprint overrides).
- `exec` -- A `CommandExecutor` for running checks on the target machine.

Return one of:
- `TaskStatus::Completed` -- The task's desired state is already present.
- `TaskStatus::NotStarted` -- The task has not been run.
- `TaskStatus::Failed` -- A previous attempt failed and left a detectable state.
- `TaskStatus::Unknown` -- Cannot determine the current state.

Use `exec.file_exists()`, `exec.run_command()`, etc. so the check works on both local and remote machines.

### execute

This method performs the actual work. It receives:

- `config` -- The merged task configuration.
- `exec` -- A `CommandExecutor` for running commands on the target machine.
- `on_progress` -- A callback to report progress. Call as `on_progress(fraction, message)` where fraction is 0.0 to 1.0.

Return `Ok(())` on success or `Err(AppError::Task(...))` on failure.

## Step-by-step

### 1. Create the task file

Create `src-tauri/src/tasks/your_task.rs`:

```rust
use std::collections::HashMap;
use serde_json::Value;

use super::{
    SetupTask, TaskStatus, PrivilegeLevel, ExecutionTarget,
    ConfigField, ProgressCallback,
};
use crate::error::AppError;
use crate::executor::CommandExecutor;

pub struct InstallRipgrepTask;

#[async_trait::async_trait]
impl SetupTask for InstallRipgrepTask {
    fn id(&self) -> &str { "install-ripgrep" }
    fn name(&self) -> &str { "Ripgrep" }
    fn description(&self) -> &str { "Install ripgrep (rg) for fast recursive search" }
    fn icon(&self) -> &str { "Search" }
    fn category(&self) -> &str { "Tools" }
    fn tags(&self) -> Vec<String> { vec!["search".to_string(), "cli".to_string()] }
    fn privilege_level(&self) -> PrivilegeLevel { PrivilegeLevel::User }
    fn execution_target(&self) -> ExecutionTarget { ExecutionTarget::Any }

    fn config_schema(&self) -> Vec<ConfigField> {
        // No configuration needed for this task
        vec![]
    }

    async fn detect_state(
        &self,
        _config: &HashMap<String, Value>,
        exec: &dyn CommandExecutor,
    ) -> TaskStatus {
        // Check if rg binary exists by running "which rg"
        match exec.run_command("which", &["rg"]).await {
            Ok(output) if output.status == 0 => TaskStatus::Completed,
            _ => TaskStatus::NotStarted,
        }
    }

    async fn execute(
        &self,
        _config: &HashMap<String, Value>,
        exec: &dyn CommandExecutor,
        on_progress: &ProgressCallback,
    ) -> Result<(), AppError> {
        on_progress(0.0, "Checking for ripgrep...".to_string());

        // Check if already installed
        if let Ok(output) = exec.run_command("which", &["rg"]).await {
            if output.status == 0 {
                on_progress(1.0, "Ripgrep is already installed".to_string());
                return Ok(());
            }
        }

        on_progress(0.3, "Downloading ripgrep...".to_string());

        // Install via apt (works on both local and remote via exec)
        let output = exec.run_command("sudo", &["apt", "install", "-y", "ripgrep"]).await
            .map_err(|e| AppError::Task(format!("Failed to install ripgrep: {}", e)))?;

        if output.status != 0 {
            return Err(AppError::Task(format!(
                "apt install failed: {}",
                output.stderr
            )));
        }

        on_progress(1.0, "Ripgrep installed successfully".to_string());
        Ok(())
    }
}
```

### 2. Register the module

Add the module declaration to `src-tauri/src/tasks/mod.rs`:

```rust
pub mod your_task;
```

For the example above:

```rust
pub mod install_ripgrep;
```

### 3. Register in the task registry

Add the task to `create_registry()` in `src-tauri/src/tasks/registry.rs`:

```rust
pub fn create_registry() -> TaskRegistry {
    let mut registry = TaskRegistry::new();

    // ... existing tasks ...

    registry.register(Box::new(install_ripgrep::InstallRipgrepTask));

    registry
}
```

### 4. Verify

Run the backend tests to check for compilation errors:

```bash
cd src-tauri && cargo test
```

Then start the dev server:

```bash
npm run dev
```

The new task should appear in the task list automatically. The frontend fetches all tasks via `list_tasks`, which iterates over the registry and calls `detect_state` on each task.

## Guidelines

- **Use the CommandExecutor for all operations.** Call `exec.run_command()`, `exec.file_exists()`, `exec.read_file()`, `exec.write_file()`, and `exec.create_dir_all()` instead of direct filesystem or process APIs. This ensures the task works on both local and remote machines.
- **Use `exec.home_dir()`** instead of hardcoding home directory paths. Tilde paths like `~/` will not expand automatically on remote executors.
- **Report progress.** Call `on_progress(fraction, message)` at meaningful intervals so the UI provides feedback.
- **Return clear errors.** Use `AppError::Task(message)` with a human-readable description of what went wrong.
- **Keep detect_state fast.** It runs on every task when the list loads. Avoid expensive operations -- prefer quick checks like `file_exists` or `which`.
- **Idempotency.** Tasks should be safe to run multiple times. Check whether work is already done before performing it.
