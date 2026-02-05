# API Reference

This document lists all Tauri IPC commands and backend-emitted events. Commands are invoked from the frontend via `invoke()` from `@tauri-apps/api/core`. All command wrappers are defined in `src/services/tauriCommands.ts`.

## Task Commands

Defined in `src-tauri/src/commands/tasks.rs`.

### list_tasks

```
list_tasks() -> Vec<TaskInfo>
```

Lists all registered tasks with their current detected states. For each task in the registry, loads the task's config from `ConfigManager`, runs `detect_state()` via a `LocalExecutor`, and returns the result as a `TaskInfo`.

**Returns:** Array of `TaskInfo` objects containing task metadata, config schema, and detected status.

### detect_all_states

```
detect_all_states() -> Vec<TaskInfo>
```

Re-detects the state of all registered tasks. Functionally identical to `list_tasks` -- delegates to the same implementation.

**Returns:** Array of `TaskInfo` objects with fresh status detection.

### execute_task

```
execute_task(task_id: String, node_id: Option<String>) -> ()
```

Executes a single task. If `node_id` is provided, looks up the target node and selects the appropriate executor (`LocalExecutor` for local nodes, `SshExecutor` for remote nodes). If `node_id` is omitted, defaults to local execution. Emits `task-progress` events during execution and a `task-state-changed` event on completion. Adds the task ID to `completed_tasks` in the config.

**Parameters:**
- `task_id` -- The ID of the task to execute.
- `node_id` -- Optional UUID of the target node. If omitted, executes locally.

**Emits:** `task-progress`, `task-state-changed`

**Errors:** `AppError::Task` if the task ID is not found or execution fails. `AppError::Fleet` if the node ID is not found. `AppError::Ssh` if a remote node lacks SSH configuration.

---

## AppArmor Commands

Defined in `src-tauri/src/commands/apparmor.rs`.

### get_denial_logs

```
get_denial_logs() -> DenialLogsResponse
```

Parses the system audit log for AppArmor denial events and generates rule suggestions.

**Returns:** `DenialLogsResponse` containing:
- `denials: Vec<DenialEvent>` -- Parsed denial events.
- `suggestions: Vec<PermissionSuggestion>` -- Generated rule suggestions with risk levels.

### get_profiles

```
get_profiles() -> Vec<ProfileInfo>
```

Lists all AppArmor profiles on the system by running `aa-status --json`.

**Returns:** Array of `ProfileInfo` objects with name, mode (enforce/complain/unconfined), and PID count.

### get_profile_detail

```
get_profile_detail(profile_name: String) -> ProfileDetail
```

Reads the full detail of an AppArmor profile from `/etc/apparmor.d/`.

**Parameters:**
- `profile_name` -- The name of the AppArmor profile.

**Returns:** `ProfileDetail` containing name, mode, raw file content, and parsed rules.

### apply_permission_rules

```
apply_permission_rules(request: ApplyPermissionsRequest) -> ()
```

Applies permission rules to an AppArmor profile. Uses PolicyKit (`pkexec`) for privilege escalation.

**Parameters:**
- `request` -- An `ApplyPermissionsRequest` containing:
  - `profile: String` -- Target profile name.
  - `rules: Vec<String>` -- Rules to apply.

### start_log_monitor

```
start_log_monitor() -> ()
```

Starts real-time monitoring of the audit log for AppArmor denial events. Uses the `notify` crate to watch the file, falling back to `journalctl --follow`. Emits `apparmor-denial` events as new denials are detected.

**Emits:** `apparmor-denial`

### stop_log_monitor

```
stop_log_monitor() -> ()
```

Stops the real-time audit log monitor by setting the `monitor_running` flag to `false`.

### consolidate_rules

```
consolidate_rules(suggestions: Vec<PermissionSuggestion>) -> Vec<ConsolidationResult>
```

Consolidates a set of permission suggestions into optimized rules by merging overlapping or redundant entries.

**Parameters:**
- `suggestions` -- Array of `PermissionSuggestion` objects to consolidate.

**Returns:** Array of `ConsolidationResult` objects, each containing a profile name and consolidated rules.

### apply_permission_rules_batch

```
apply_permission_rules_batch(requests: Vec<ApplyPermissionsRequest>) -> ()
```

Applies permission rules to multiple AppArmor profiles in a single operation.

**Parameters:**
- `requests` -- Array of `ApplyPermissionsRequest` objects, one per profile.

### consolidate_profile_rules

```
consolidate_profile_rules(profile_name: String) -> ConsolidationResult
```

Reads a profile's existing rules and consolidates them into optimized form.

**Parameters:**
- `profile_name` -- The name of the AppArmor profile.

**Returns:** A single `ConsolidationResult` with the profile name and consolidated rules.

### rewrite_profile_rules

```
rewrite_profile_rules(profile_name: String, rules: Vec<String>) -> ()
```

Rewrites an AppArmor profile's rules entirely, replacing the existing rule set.

**Parameters:**
- `profile_name` -- The name of the AppArmor profile.
- `rules` -- The new set of rules to write.

---

## Config Commands

Defined in `src-tauri/src/commands/config.rs`.

### get_config

```
get_config() -> ConfigData
```

Returns the current application configuration.

**Returns:** `ConfigData` object containing theme preference, task configs, and completed tasks list.

### set_config

```
set_config(config: ConfigData) -> ConfigData
```

Saves the provided configuration to disk and returns the saved state.

**Parameters:**
- `config` -- The `ConfigData` to persist.

**Returns:** The saved `ConfigData` (round-tripped through the manager).

---

## System Commands

Defined in `src-tauri/src/commands/system.rs`.

### get_system_info

```
get_system_info() -> SystemInfo
```

Returns information about the local system.

**Returns:** `SystemInfo` containing:
- `os: String` -- Operating system name (e.g., `"linux"`).
- `username: String` -- Current user.
- `hostname: String` -- Machine hostname.

### check_polkit_auth

```
check_polkit_auth(action_id: String) -> bool
```

Checks whether a PolicyKit action is registered on the system by running `pkaction --verbose --action-id`.

**Parameters:**
- `action_id` -- The PolicyKit action ID to check.

**Returns:** `true` if the action exists, `false` otherwise.

### is_polkit_policy_installed

```
is_polkit_policy_installed() -> bool
```

Checks whether Quartermaster's PolicyKit policy file is installed at `/usr/share/polkit-1/actions/com.quartermaster.policy`.

**Returns:** `true` if the policy file exists.

### is_apparmor_helper_installed

```
is_apparmor_helper_installed() -> bool
```

Checks whether Quartermaster's AppArmor helper script is installed at `/usr/lib/quartermaster/quartermaster-apparmor-helper`.

**Returns:** `true` if the helper script exists.

### install_polkit_policy

```
install_polkit_policy() -> ()
```

Installs Quartermaster's PolicyKit policy file and AppArmor helper script from the bundled resources:

1. Copies `com.quartermaster.policy` to `/usr/share/polkit-1/actions/` using `pkexec cp`.
2. If the bundled `quartermaster-apparmor-helper` script exists, installs it to `/usr/lib/quartermaster/` with root ownership and mode 755 via a single `pkexec bash -c` call.

The helper script is registered with the `com.quartermaster.apparmor-manage` PolicyKit action via `org.freedesktop.policykit.exec.path`, enabling `auth_admin_keep` credential caching for AppArmor operations.

**Errors:** `AppError::Other` if the bundled policy file is not found, or `AppError::Polkit` if the privileged commands fail.

---

## Fleet Commands

Defined in `src-tauri/src/commands/fleet.rs`.

### list_nodes

```
list_nodes() -> Vec<Node>
```

Returns all fleet nodes (both local and remote).

**Returns:** Array of `Node` objects.

### get_node

```
get_node(node_id: String) -> Node
```

Returns a single node by ID.

**Parameters:**
- `node_id` -- The UUID of the node.

**Errors:** `AppError::Fleet` if the node is not found.

### add_node

```
add_node(name: String, hostname: String, kind: NodeKind, tags: Vec<String>, ssh_config: Option<SshConfig>) -> Node
```

Creates a new fleet node. Generates a UUID, sets initial status (Online for local nodes, Unknown for remote), and persists to disk.

**Parameters:**
- `name` -- Display name for the node.
- `hostname` -- Hostname or IP address.
- `kind` -- `NodeKind::Local` or `NodeKind::Remote`.
- `tags` -- Array of string tags.
- `ssh_config` -- Optional SSH configuration (required for remote nodes).

**Returns:** The created `Node` with generated ID and timestamps.

### update_node

```
update_node(node: Node) -> Node
```

Updates an existing node. Persists changes to disk.

**Parameters:**
- `node` -- The full `Node` object with updated fields.

**Returns:** The updated `Node`.

**Errors:** `AppError::Fleet` if the node ID is not found.

### remove_node

```
remove_node(node_id: String) -> ()
```

Removes a node from the fleet and deletes its JSON file from disk.

**Parameters:**
- `node_id` -- The UUID of the node to remove.

**Errors:** `AppError::Fleet` if the node is not found.

### get_node_status

```
get_node_status(node_id: String) -> NodeStatus
```

Returns the current status of a node.

**Parameters:**
- `node_id` -- The UUID of the node.

**Returns:** `NodeStatus` enum value: `Online`, `Offline`, `Connecting`, `Error`, or `Unknown`.

**Errors:** `AppError::Fleet` if the node is not found.

---

## Blueprint Commands

Defined in `src-tauri/src/commands/blueprints.rs`.

### list_blueprints

```
list_blueprints() -> Vec<Blueprint>
```

Returns all blueprints (both built-in and user-created).

**Returns:** Array of `Blueprint` objects.

### get_blueprint

```
get_blueprint(blueprint_id: String) -> Blueprint
```

Returns a single blueprint by ID.

**Parameters:**
- `blueprint_id` -- The UUID of the blueprint.

**Errors:** `AppError::Blueprint` if the blueprint is not found.

### create_blueprint

```
create_blueprint(name: String, description: String, icon: String, task_entries: Vec<BlueprintTaskEntry>) -> Blueprint
```

Creates a new user blueprint. Generates a UUID, sets `is_builtin: false`, and persists to disk.

**Parameters:**
- `name` -- Display name.
- `description` -- Short description.
- `icon` -- Lucide icon name.
- `task_entries` -- Array of `BlueprintTaskEntry` objects defining the tasks and their order.

**Returns:** The created `Blueprint` with generated ID and timestamps.

### update_blueprint

```
update_blueprint(blueprint: Blueprint) -> Blueprint
```

Updates an existing blueprint. Persists changes to disk.

**Parameters:**
- `blueprint` -- The full `Blueprint` object with updated fields.

**Returns:** The updated `Blueprint`.

**Errors:** `AppError::Blueprint` if the blueprint ID is not found.

### remove_blueprint

```
remove_blueprint(blueprint_id: String) -> ()
```

Removes a blueprint and deletes its JSON file from disk. Built-in blueprints cannot be removed.

**Parameters:**
- `blueprint_id` -- The UUID of the blueprint to remove.

**Errors:** `AppError::Blueprint` if the blueprint is not found or is a built-in blueprint.

### apply_blueprint

```
apply_blueprint(node_id: String, blueprint_id: String) -> ()
```

Applies a blueprint to a fleet node. This is the primary orchestration command that:

1. Loads the blueprint and verifies the target node exists.
2. Filters to enabled task entries and sorts by order.
3. Selects the appropriate executor (`LocalExecutor` for local nodes, `SshExecutor` for remote nodes).
4. For each task entry:
   a. Merges the global task config with blueprint config overrides.
   b. Emits `blueprint-apply-progress`.
   c. Runs `detect_state()` -- skips if already `Completed`.
   d. Runs `execute()` with a progress callback that emits `task-progress`.
   e. Emits `task-state-changed` on completion.
5. Emits `blueprint-apply-complete` when all tasks finish.

**Parameters:**
- `node_id` -- The UUID of the target node.
- `blueprint_id` -- The UUID of the blueprint to apply.

**Emits:** `blueprint-apply-progress`, `task-progress`, `task-state-changed`, `blueprint-apply-complete`

**Errors:** `AppError::Blueprint` if the blueprint is not found, `AppError::Fleet` if the node is not found, `AppError::Ssh` if a remote node lacks SSH configuration, or `AppError::Task` if any task execution fails.

---

## SSH Commands

Defined in `src-tauri/src/commands/ssh.rs`.

### test_ssh_connection

```
test_ssh_connection(host: String, port: u16, username: String) -> String
```

Tests SSH connectivity to a remote host by running `ssh <user>@<host> echo ok` with `BatchMode=yes` and a 10-second timeout. Returns a success message on connection or an error with details.

**Parameters:**
- `host` -- Remote hostname or IP.
- `port` -- SSH port.
- `username` -- SSH username.

**Returns:** A success message string (e.g., `"Connected to deploy@192.168.1.100:22"`).

**Errors:** `AppError::Ssh` if the connection fails (wrong credentials, unreachable host, timeout, etc.).

### list_ssh_keys

```
list_ssh_keys() -> Vec<SshKeyInfo>
```

Scans `~/.ssh/` for public key files (`*.pub`) and returns information about each key.

**Returns:** Array of `SshKeyInfo` objects with `name`, `path`, `key_type`, and `is_fido2` fields.

### generate_ssh_key

```
generate_ssh_key(key_type: String, comment: String) -> SshKeyInfo
```

Generates a new SSH key pair using `ssh-keygen`.

**Parameters:**
- `key_type` -- Key algorithm: `"ed25519"` or `"ed25519-sk"` (FIDO2).
- `comment` -- Key comment (typically an email or identifier).

**Returns:** `SshKeyInfo` for the generated key.

**Errors:** `AppError::Ssh` if the key already exists or `ssh-keygen` fails.

### deploy_ssh_key

```
deploy_ssh_key(public_key_path: String, target_host: String, target_port: u16, target_username: String) -> ()
```

Deploys a public key to a remote machine's `~/.ssh/authorized_keys`.

**Parameters:**
- `public_key_path` -- Local path to the public key file.
- `target_host` -- Remote hostname or IP.
- `target_port` -- SSH port on the remote host.
- `target_username` -- Username on the remote host.

**Errors:** `AppError::Ssh` if the key file cannot be read or the SSH command fails.

---

## Events Emitted

The backend emits these events via `app.emit()`. The frontend subscribes via the `useTauriEvent` hook.

| Event | Payload | Emitted By |
|-------|---------|------------|
| `task-progress` | `{ task_id: String, progress: f32, message: String }` | `execute_task` |
| `task-progress` | `{ node_id: String, task_id: String, progress: f32, message: String }` | `apply_blueprint` |
| `task-state-changed` | `{ task_id: String, status: String }` | `execute_task` |
| `task-state-changed` | `{ node_id: String, task_id: String, status: String }` | `apply_blueprint` |
| `blueprint-apply-progress` | `{ node_id: String, blueprint_id: String, completed: usize, total: usize, current_task_id: String }` | `apply_blueprint` |
| `blueprint-apply-complete` | `{ node_id: String, blueprint_id: String }` | `apply_blueprint` |
| `node-status-changed` | `{ node_id: String, status: String }` | Fleet status monitoring |
| `apparmor-denial` | `{ denial: DenialEvent }` | `start_log_monitor` |

### Payload types

**DenialEvent:**
```json
{
  "id": "string",
  "timestamp": "string",
  "profile": "string",
  "operation": "string",
  "name": "string",
  "requested_mask": "string",
  "denied_mask": "string",
  "pid": 0,
  "comm": "string",
  "raw_log": "string"
}
```

**SshKeyInfo:**
```json
{
  "name": "string",
  "path": "string",
  "key_type": "string",
  "is_fido2": false
}
```

**TaskInfo:**
```json
{
  "id": "string",
  "name": "string",
  "description": "string",
  "icon": "string",
  "category": "string",
  "tags": ["string"],
  "privilege_level": "user | admin",
  "execution_target": "local_only | remote_only | any",
  "depends_on": ["string"],
  "config_schema": [],
  "status": "not_started | in_progress | completed | failed | skipped | unknown",
  "error_message": "string | null"
}
```

**Node:**
```json
{
  "id": "string",
  "name": "string",
  "kind": "local | remote",
  "hostname": "string",
  "os": "string | null",
  "tags": ["string"],
  "ssh_config": "SshConfig | null",
  "status": "online | offline | connecting | error | unknown",
  "last_seen": "datetime | null",
  "blueprint_id": "string | null",
  "created_at": "datetime"
}
```

**Blueprint:**
```json
{
  "id": "string",
  "name": "string",
  "description": "string",
  "icon": "string",
  "is_builtin": false,
  "task_entries": [
    {
      "task_id": "string",
      "enabled": true,
      "config_overrides": {},
      "order": 0
    }
  ],
  "created_at": "datetime",
  "updated_at": "datetime"
}
```
