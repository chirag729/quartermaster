# Backend Reviewer Memory

## Review Patterns Found

### Task System (reviewed 2026-02-07)
- `ExecutionTarget` enforcement exists in `execute_task` and `apply_blueprint`/`apply_blueprint_bulk`, but is MISSING from `uninstall_task`
- `check_task_updates` holds config Mutex across `.await` (detect_installed_version calls) -- blocks all config access
- Template variable validation is incomplete: covers `steps[].run` and `detect`, but not `uninstall[].run`, `version_detect`, or `download.url`
- Shell injection risk in `git-ssh.yaml`: user text fields interpolated into shell commands without escaping
- `ExecutionLog` module exists but is never called from any command handler -- dead code
- `resolve_dependencies` (Kahn's algorithm) silently drops nodes in cycles instead of erroring
- `TaskRegistry::register()` allows duplicate task IDs -- last one wins in Vec, `get()` returns first match

## Lock Ordering Observations
- Standard order: config -> fleet_manager -> vault (observed in execute_task, list_tasks_for_node)
- `check_task_updates` acquires fleet_manager first, then config -- reversal from standard order
- `apply_blueprint` acquires blueprint_manager -> fleet_manager -> vault -> config (repeating config in loop)

## Backend Core & State (reviewed 2026-02-07)
- Activity log path in `lib.rs:42` uses `dirs::config_dir()` (external crate = `~/.config`) not `crate::dirs::config_dir()` (`~/.config/quartermaster`) -- log file goes to wrong location
- `update_blueprint` command (blueprints.rs:85) has no `is_builtin` check -- allows modifying built-in blueprints
- `sync_installed_profiles` (apparmor.rs:203) holds mutable config lock across `.await` loop
- `let _ =` on `app.emit()` and `ActivityLog::save()` are intentional (documented best-effort)
- `config.save()` properly propagated via `?` in all command handlers
- Variable resolution 4-layer precedence is correctly implemented and well-tested
- `update_blueprint` has TOCTOU: reads blueprint with one lock, drops, re-acquires for write

## Tauri 2 IPC Parameter Naming Convention
- `#[tauri::command]` converts Rust `snake_case` parameter names to `camelCase` for frontend IPC
- Example: Rust `task_id: String` expects frontend to send `taskId`
- **Struct fields** within parameters use serde defaults (Rust field names = snake_case) unless `#[serde(rename_all)]` is set
- This creates a split: command param names are camelCase, but fields inside struct params are snake_case
- Known mismatches: `vault_create`/`vault_unlock` (master_password vs password), `add_node` (ssh_config vs sshConfig), `create_blueprint` (task_entries vs taskEntries)

## Command Handler Review (2026-02-07)
- `generate_ssh_key` (ssh.rs) has path traversal: `comment` param not sanitized for `/` chars, unlike `generate_fido2_ssh_key` which uses `sanitize_key_name()`
- `apply_blueprint_bulk` does NOT update `applied_blueprint_version` on nodes (unlike `apply_blueprint`)
- `uninstall_task` missing `ExecutionTarget` enforcement (confirmed from prior review)

## Blueprint System (reviewed 2026-02-07)
- `update_blueprint` command (blueprints.rs:85) has no `is_builtin` check -- allows modifying built-in blueprints
- `update_blueprint` TOCTOU: reads blueprint with one lock, drops, re-acquires for write
- `manager.update_blueprint()` saves to disk BEFORE checking in-memory existence -- creates orphan YAML on error
- `apply_blueprint_bulk` does NOT update `applied_blueprint_version` on nodes (unlike `apply_blueprint`)
- `to_definition()` loses non-string config_override values: `v.as_str().unwrap_or_default()` converts numbers/bools to ""
- Version auto-bump ignores `enabled` field changes and `name`/`description`/`icon` changes
- `unpack_blueprint` path traversal check incomplete: checks `..` but not absolute paths (leading `/`)
- `export_blueprint` bundles ALL tasks/apparmor from config dir, not just those referenced by the blueprint
- `delete_blueprint` has TOCTOU between `is_builtin` check (first lock) and `remove_blueprint` (second lock)
- `create_blueprint` IPC param name mismatch: Rust `task_entries` becomes `taskEntries`, frontend sends `task_entries`

## AppArmor System (reviewed 2026-02-07)
- `validate_profile_name` allows `/` in names but not leading `/` check -- `PathBuf::join` with absolute path replaces base
- `sync_installed_profiles` (apparmor.rs:203) holds mutable config lock across `.await` loop -- blocks all config access
- `sync_installed_profiles` swallows sync_profile errors via `.unwrap_or(false)` including config.save() failures
- `start_monitoring` spawns a new task without checking if one is already running -- double-start leaks resources
- Monitor uses blocking `std::sync::mpsc::recv_timeout` and sync I/O inside `tokio::spawn` async block
- `parse_audit_log` reads entire audit.log into memory synchronously in an async function
- Log parser regex requires fixed field order; real audit logs may vary field order between kernel versions
- Generated rules use `name` field from audit log verbatim -- paths with spaces would create invalid AppArmor syntax
- Rule consolidator's `parse_file_rule` splits on last whitespace -- breaks on paths containing spaces
- Template rendering has no protection against unreferenced/unresolved `{{var}}` placeholders remaining in output

## Fleet & SSH (reviewed 2026-02-07)
- `remove_node` does NOT cascade-clean `installed_tasks` or `node_variable_overrides` from config
- `remove_node` does NOT prevent deletion of the auto-created local node
- `update_node` has NO validation guards: allows changing NodeKind, mutating local node arbitrarily
- `generate_ssh_key` path traversal via `comment` param (confirmed, same as Command Handler review)
- SSH config parser ignores `Include` directives -- misses hosts in modular configs
- SSH config parser treats multi-alias `Host` lines as single alias
- `StrictHostKeyChecking=accept-new` used everywhere; `SshConfig.fingerprint` field exists but is never checked
- `apply_blueprint_bulk` does NOT update `applied_blueprint_version` (confirmed cross-ref)
- `list_ssh_keys` uses `unwrap_or_default()` for home dir (unsafe empty path on edge case)
- Lock ordering in fleet commands: fleet_manager only, no multi-lock concerns within fleet.rs itself

## Key Files
- Task trait + types: `src-tauri/src/tasks/mod.rs`
- Registry + Kahn's algo: `src-tauri/src/tasks/registry.rs`
- Script execution: `src-tauri/src/tasks/script_task.rs`
- YAML validation: `src-tauri/src/tasks/validate.rs`
- Task commands: `src-tauri/src/commands/tasks.rs`
- Blueprint commands: `src-tauri/src/commands/blueprints.rs`
- State definition: `src-tauri/src/state.rs`
- Config manager: `src-tauri/src/config/manager.rs`
- Canonical dirs: `src-tauri/src/dirs.rs`
- Activity log: `src-tauri/src/activity_log.rs`
- Error types: `src-tauri/src/error.rs`
- App init: `src-tauri/src/lib.rs`
