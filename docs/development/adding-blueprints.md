# Adding Default Blueprints

Blueprints are ordered collections of tasks that can be applied to a node as a group. Anvil ships with built-in default blueprints that are created on first launch. This guide covers how to add new default blueprints.

## Overview

Default blueprints are defined in `src-tauri/src/blueprints/defaults.rs` in the `create_default_blueprints()` function. They are created when the `BlueprintManager` loads and finds an empty blueprints directory (i.e., on a fresh install).

## Blueprint Model

A `Blueprint` has the following fields (defined in `src-tauri/src/blueprints/mod.rs`):

```rust
pub struct Blueprint {
    pub id: String,                          // UUID, auto-generated
    pub name: String,                        // Display name
    pub description: String,                 // Short description
    pub icon: String,                        // Lucide icon name
    pub is_builtin: bool,                    // Built-in blueprints cannot be deleted
    pub task_entries: Vec<BlueprintTaskEntry>,// Ordered list of tasks
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}
```

Each task in the blueprint is represented by a `BlueprintTaskEntry`:

```rust
pub struct BlueprintTaskEntry {
    pub task_id: String,                         // Must match a registered task's id()
    pub enabled: bool,                           // Whether this entry is active
    pub config_overrides: HashMap<String, Value>, // Per-blueprint config overrides
    pub order: u32,                              // Execution order (0-based, ascending)
}
```

### BlueprintTaskEntry fields

| Field | Type | Purpose |
|-------|------|---------|
| `task_id` | `String` | The ID of a registered task (e.g. `"flutter-sdk"`). Must match a task's `id()` method. |
| `enabled` | `bool` | Whether this task is included when the blueprint is applied. Users can toggle this in the UI. |
| `config_overrides` | `HashMap<String, Value>` | Key-value pairs that override the global task config for this blueprint only. Use an empty `HashMap::new()` for no overrides. |
| `order` | `u32` | Execution order. Tasks are sorted by this field in ascending order before execution. Use sequential values starting from 0. |

## How to add a default blueprint

### 1. Edit defaults.rs

Open `src-tauri/src/blueprints/defaults.rs` and add a new `Blueprint` entry to the vector returned by `create_default_blueprints()`:

```rust
pub fn create_default_blueprints() -> Vec<Blueprint> {
    let now = Utc::now();

    vec![
        // ... existing blueprints ...

        Blueprint {
            id: Uuid::new_v4().to_string(),
            name: "Web Development".to_string(),
            description: "Node.js and web development toolchain".to_string(),
            icon: "Globe".to_string(),
            is_builtin: true,
            task_entries: vec![
                BlueprintTaskEntry {
                    task_id: "create-development-folder".to_string(),
                    enabled: true,
                    config_overrides: HashMap::new(),
                    order: 0,
                },
                BlueprintTaskEntry {
                    task_id: "git-ssh".to_string(),
                    enabled: true,
                    config_overrides: HashMap::new(),
                    order: 1,
                },
                // Add more task entries as needed...
            ],
            created_at: now,
            updated_at: now,
        },
    ]
}
```

### 2. Key rules

- **Set `is_builtin: true`.** Built-in blueprints are protected from deletion by the user. The `BlueprintManager.remove_blueprint()` method rejects attempts to delete blueprints with `is_builtin: true`.
- **Use `Uuid::new_v4().to_string()` for the ID.** Each blueprint gets a unique UUID generated at creation time.
- **Reference only registered task IDs.** The `task_id` field must match the `id()` of a task registered in `create_registry()`. If a task ID does not exist in the registry, it is silently skipped during blueprint application.
- **Keep order values sequential.** Use 0, 1, 2, etc. The `order` field determines execution sequence when the blueprint is applied.

### 3. When defaults appear

Default blueprints are only created when the blueprints directory (`~/.config/anvil/blueprints/`) is empty. This happens:

- On a **fresh install** (no prior config exists).
- If the user **manually deletes all blueprint JSON files** from the config directory.

If the user already has blueprints on disk, new defaults added to `create_default_blueprints()` will **not** appear automatically. This is by design -- it avoids overwriting user customizations.

### 4. Using config overrides

To customize a task's behavior within a blueprint, add entries to `config_overrides`. The keys must match the `key` field of the task's `config_schema()`:

```rust
BlueprintTaskEntry {
    task_id: "flutter-sdk".to_string(),
    enabled: true,
    config_overrides: HashMap::from([
        ("branch".to_string(), Value::String("beta".to_string())),
    ]),
    order: 3,
},
```

At execution time, these overrides are merged on top of the global task config. See the architecture doc for details on config layering.

### 5. Verify

Run the backend tests to ensure the new blueprint compiles and passes validation:

```bash
cd src-tauri && cargo test
```

The existing tests in `defaults.rs` check that:
- All default blueprints have unique IDs.
- All default blueprints have `is_builtin: true`.
- Task entry order values are sequential (0, 1, 2, ...).

Consider adding a test for your new blueprint that verifies its expected task count and icon.

## Existing default blueprints

| Name | Icon | Tasks | Purpose |
|------|------|-------|---------|
| Development Workstation | Monitor | 7 | Full dev environment: folders, git, Flutter, Android, IntelliJ, Claude Code |
| Mobile Development | Smartphone | 5 | Flutter and Android SDK setup for mobile development |
| Minimal Server | Server | 0 | Minimal server setup (placeholder, no tasks yet) |
