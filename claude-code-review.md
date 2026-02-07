# Claude Code Review Report

**Date**: 2026-02-07
**Reviewer**: Claude Opus 4.6 (10 parallel subagent reviews)
**Codebase**: Quartermaster (Tauri 2 / React + Rust)

## Validation Status

| Check | Result |
|-------|--------|
| `cargo test` | 301 passed, 0 failed |
| `npm run test` | 82 passed (6 test files) |
| `npx tsc --noEmit` | 0 errors |
| `cargo check` | OK |

---

## Findings Summary

| Severity | Count |
|----------|-------|
| HIGH | 14 |
| MEDIUM | 26 |
| LOW | 22 |
| **Total** | **62** |

---

## HIGH Severity Findings

### H1. Command Injection via Template Variable Expansion in `sh -c`
- **Files**: `src-tauri/src/tasks/script_task.rs:58-65, 204-205`
- **Areas**: Task System, Security & Executors
- **Issue**: `expand_template()` performs raw string replacement of `{{variable}}` placeholders with user-provided config values, then passes the result to `sh -c`. No shell escaping is applied. A malicious config value (e.g., from a blueprint's `config_overrides` or a `.qmbp` import) can inject arbitrary shell commands.
- **Impact**: Arbitrary command execution as the current user. For tasks with `privilege: admin`, this runs under `pkexec`, giving **root-level execution**.
- **Fix**: Shell-escape all config values in `build_context()` using `shell_escape::escape()`, or pass values as environment variables instead of interpolating into shell commands.

### H2. Predictable Temp File in Privileged `write_file` (TOCTOU)
- **File**: `src-tauri/src/executor/privileged.rs:50-77`
- **Area**: Security & Executors
- **Issue**: `write_file` uses a static temp path `~/.cache/quartermaster/privileged_write.tmp` for every privileged write. Between writing the temp file (as unprivileged user) and `pkexec cp` (as root), an attacker can replace the temp file content via symlink or direct write.
- **Impact**: Privilege escalation -- attacker-controlled content written to any root-owned file path.
- **Fix**: Use `tempfile::NamedTempFile` for random unique names with mode 0600, or pass content via stdin to `pkexec tee`.

### H3. Path Traversal in `generate_ssh_key` via Unsanitized `comment` Parameter
- **File**: `src-tauri/src/commands/ssh.rs:89-91`
- **Areas**: Command Handlers, Fleet & SSH
- **Issue**: The `comment` parameter only has spaces replaced -- `../` sequences pass through. A comment of `../../.bashrc` writes to `~/.bashrc`. The FIDO2 equivalent (`generate_fido2_ssh_key`) correctly uses `sanitize_key_name()`.
- **Impact**: Write SSH key files to arbitrary user-accessible filesystem locations.
- **Fix**: Apply `sanitize_key_name()` (from `yubikey.rs`) to both `comment` and `key_type` parameters.

### H4. Path Traversal in AppArmor `validate_profile_name` Allows Absolute Paths
- **File**: `src-tauri/src/apparmor/profile_manager.rs:8-19`
- **Area**: AppArmor System
- **Issue**: `validate_profile_name` allows `/` in names but doesn't reject names starting with `/`. `PathBuf::join("/etc/shadow")` discards the base path entirely. Profile operations like `get_profile_detail` and `apply_rules` (via pkexec) could read/write arbitrary files.
- **Impact**: Arbitrary file read/write with root privileges via pkexec.
- **Fix**: Add `name.starts_with('/')` to the rejection check, or canonicalize the result and verify it starts with `/etc/apparmor.d`.

### H5. Incomplete Path Traversal Protection in `.qmbp` Import (Absolute Paths)
- **File**: `src-tauri/src/blueprints/package.rs:195`
- **Area**: Blueprint System
- **Issue**: `unpack_blueprint` checks for `..` but NOT for absolute paths starting with `/`. On Linux, `Path::join("/etc/cron.d/malicious")` discards the base path entirely.
- **Impact**: Malicious `.qmbp` file writes to arbitrary filesystem locations.
- **Fix**: Add `entry_name.starts_with('/')` check, or canonicalize `out_path` and verify it starts with `dest_dir`.

### H6. `uninstall_task` Missing `ExecutionTarget` Enforcement
- **Files**: `src-tauri/src/commands/tasks.rs:289-418`
- **Areas**: Backend Core, Task System, Command Handlers, Fleet & SSH
- **Issue**: `execute_task`, `apply_blueprint`, and `apply_blueprint_bulk` all enforce `ExecutionTarget::LocalOnly`/`RemoteOnly` constraints. `uninstall_task` does not -- a `LocalOnly` task can be uninstalled on a remote node.
- **Impact**: Uninstall commands targeting wrong environments, potentially destructive on remote machines.
- **Fix**: Add the same `execution_target()` match block to `uninstall_task` after the `supports_uninstall()` check.

### H7. Config Mutex Held Across `.await` Points in `check_task_updates`
- **File**: `src-tauri/src/commands/tasks.rs:466-493`
- **Areas**: Backend Core, Command Handlers
- **Issue**: The config lock is acquired at line 466 and held while iterating all tasks calling `detect_installed_version().await` -- each involving shell/SSH commands. All other config-dependent commands are blocked for the entire duration.
- **Impact**: Application-wide UI freeze during update checks, especially for remote nodes with SSH latency.
- **Fix**: Clone `config.data.task_configs` and drop the lock before the loop, matching the pattern in `list_tasks`.

### H8. Config Mutex Held Across `.await` in `sync_installed_profiles`
- **File**: `src-tauri/src/commands/apparmor.rs:203-238`
- **Areas**: Backend Core, AppArmor System
- **Issue**: Mutable config lock held while calling `sync_profile().await` in a loop. Each sync invokes `pkexec` and waits for it (potentially including user password prompt).
- **Impact**: All config-dependent operations blocked for the entire sync duration.
- **Fix**: Collect needed data under the lock, drop it, perform async operations, then re-acquire to save results.

### H9. IPC Parameter Mismatch: `vault_create` / `vault_unlock`
- **Files**: `src/services/tauriCommands.ts:247-253`, `src-tauri/src/commands/vault.rs:19-40`
- **Areas**: Frontend Stores & IPC, Command Handlers
- **Issue**: Frontend sends `{ password }`, backend expects `masterPassword` (camelCase auto-rename of `master_password`). Key mismatch causes runtime deserialization failure.
- **Impact**: Users cannot create or unlock the encrypted credential vault.
- **Fix**: Rename frontend parameter to `masterPassword`, or rename Rust parameter to `password`.

### H10. IPC Parameter Mismatch: `add_node` ssh_config
- **Files**: `src/services/tauriCommands.ts:161-171`, `src-tauri/src/commands/fleet.rs:28-34`
- **Areas**: Frontend Stores & IPC, Command Handlers
- **Issue**: Frontend sends `ssh_config` (snake_case), backend expects `sshConfig` (camelCase). Since it's `Option`, it silently defaults to `None`.
- **Impact**: SSH configuration silently lost when adding remote nodes. Nodes appear added but cannot connect.
- **Fix**: Rename interface property to `sshConfig`.

### H11. IPC Parameter Mismatch: `create_blueprint` task_entries
- **Files**: `src/services/tauriCommands.ts:318-327`, `src-tauri/src/commands/blueprints.rs:62-67`
- **Areas**: Frontend Stores & IPC, Blueprint System
- **Issue**: Frontend sends `task_entries` (snake_case), backend expects `taskEntries` (camelCase). Task entries are silently dropped.
- **Impact**: Blueprints created via `createBlueprint` have no tasks.
- **Fix**: Rename to `taskEntries` in `CreateBlueprintParams`.

### H12. `update_blueprint` Missing `is_builtin` Guard
- **File**: `src-tauri/src/commands/blueprints.rs:85-153`
- **Areas**: Backend Core, Blueprint System
- **Issue**: `delete_blueprint` checks `is_builtin`, but `update_blueprint` does not. Built-in blueprints can be mutated.
- **Impact**: Corruption of default blueprint templates.
- **Fix**: Add `if updated.is_builtin { return Err(...) }` at the start.

### H13. `apply_blueprint_bulk` Does Not Update `applied_blueprint_version`
- **File**: `src-tauri/src/commands/blueprints.rs:666-921`
- **Areas**: Command Handlers, Blueprint System, Fleet & SSH
- **Issue**: `apply_blueprint` updates `node.applied_blueprint_version` after success. `apply_blueprint_bulk` does not, for any node.
- **Impact**: Version drift detection broken for all bulk-applied nodes.
- **Fix**: After each successful node, update `applied_blueprint_version`, mirroring `apply_blueprint`.

### H14. `update_node` Has No Validation -- Allows `NodeKind` Mutation
- **File**: `src-tauri/src/commands/fleet.rs:82-86`
- **Area**: Fleet & SSH
- **Issue**: `update_node` accepts a full `Node` struct with zero validation. Allows changing `kind` from Local to Remote (or vice versa), bypassing the single-local-node invariant. Also allows changing `id`, creating orphaned files.
- **Impact**: Fleet state corruption, broken invariants.
- **Fix**: Validate that `kind` and `id` have not changed, or re-enforce the local-node uniqueness constraint.

---

## MEDIUM Severity Findings

### M1. Template Variable Validation Missing for Uninstall/Version/Download Fields
- **File**: `src-tauri/src/tasks/validate.rs:134-216`
- **Issue**: `validate_template_vars` is called for `steps[].run` and `detect`, but not for `uninstall[].run`, `version_detect`, or `download.url`/`download.dest`.
- **Fix**: Add validation calls for all template-expanded fields.

### M2. Dependency Cycle Silently Drops Tasks
- **File**: `src-tauri/src/tasks/registry.rs:86-120`
- **Issue**: Kahn's algorithm silently excludes cycle members from the sorted output with no error.
- **Fix**: Compare `sorted.len()` against `required.len()`; error if they differ.

### M3. Duplicate Task IDs Allowed with Inconsistent Lookup
- **File**: `src-tauri/src/tasks/registry.rs:19-29`
- **Issue**: `register()` pushes without duplicate check. `get()` returns first match (built-in), `all()` returns both.
- **Fix**: Check for duplicates on registration and either skip, replace, or warn.

### M4. `apply_blueprint` Aborts Mid-Sequence with No Partial-Failure Event
- **File**: `src-tauri/src/commands/blueprints.rs:443`
- **Issue**: `?` operator causes immediate return on any task failure. No `blueprint-apply-failed` event is emitted.
- **Fix**: Wrap in `match`, emit failure event with partial progress details.

### M5. `ExecutionLog` Module Is Dead Code
- **File**: `src-tauri/src/tasks/execution_log.rs:1-180`
- **Issue**: Fully implemented with 7 tests but never called from any command handler. The documented feature does not work.
- **Fix**: Integrate into `execute_task`/`uninstall_task`, or remove dead code.

### M6. Wrong Escaping Layer in Privileged `create_dir_all`
- **File**: `src-tauri/src/executor/privileged.rs:79-92`
- **Issue**: Shell single-quote escaping applied, but `Command::args()` doesn't go through a shell. Escaping characters become literal.
- **Fix**: Remove the shell escaping; pass `path` directly to `Command::args()`.

### M7. SSH `cmd` Parameter Not Shell-Escaped
- **File**: `src-tauri/src/executor/ssh.rs:150-156`
- **Issue**: Only `args` are shell-escaped; `cmd` is interpolated verbatim into the remote command string. Currently safe (all callers use static strings) but latent vulnerability.
- **Fix**: Apply `shell_escape` to `cmd` as well.

### M8. SSH Password in `Debug`-Derived `AuthMode` Enum
- **File**: `src-tauri/src/executor/ssh.rs:14-24`
- **Issue**: `#[derive(Debug)]` on `AuthMode::Password(String)` exposes plaintext password in debug output.
- **Fix**: Implement custom `Debug` with redaction: `Password(****)`.

### M9. SSH Password Exposed in `/proc/<pid>/environ`
- **File**: `src-tauri/src/executor/ssh.rs:98-101`
- **Issue**: `SSHPASS` environment variable readable from `/proc`. Inherent `sshpass` limitation.
- **Fix**: Document as known limitation. Consider native SSH library auth.

### M10. `update_blueprint` TOCTOU: Double Lock with Gap
- **File**: `src-tauri/src/commands/blueprints.rs:129-152`
- **Issue**: Reads blueprint under first lock, drops, re-acquires to write. Version bump uses stale data.
- **Fix**: Hold a single mutable lock for both read and write.

### M11. `delete_blueprint` Non-Atomic: Fleet Unassignment + Deletion
- **File**: `src-tauri/src/commands/blueprints.rs:187-222`
- **Issue**: Three separate lock acquisitions with gaps. Nodes unassigned before blueprint confirmed deletable.
- **Fix**: Hold `blueprint_manager` across the entire operation.

### M12. `manager.update_blueprint()` Saves to Disk Before Checking Existence
- **File**: `src-tauri/src/blueprints/manager.rs:170-181`
- **Issue**: Writes YAML file first, then checks in-memory existence. Creates orphaned files on error.
- **Fix**: Check existence first, then save.

### M13. `to_definition()` Silently Loses Non-String Config Values
- **File**: `src-tauri/src/blueprints/manager.rs:384-385`
- **Issue**: `v.as_str().unwrap_or_default()` converts non-string JSON values (numbers, booleans) to empty string.
- **Fix**: Use `v.to_string()` for all JSON types.

### M14. Version Auto-Bump Ignores `enabled` Field Changes
- **File**: `src-tauri/src/commands/blueprints.rs:128-146`
- **Issue**: Compares task IDs and `config_overrides` but not `enabled`. Toggling a task doesn't bump version.
- **Fix**: Add `enabled` comparison alongside `config_overrides`.

### M15. `export_blueprint` Bundles ALL Tasks and AppArmor Files
- **File**: `src-tauri/src/blueprints/package.rs:408-430`
- **Issue**: Copies entire `tasks/` and `apparmor/` directories, not just blueprint-referenced files.
- **Fix**: Filter by `task_entries[].task_id` list.

### M16. `remove_node` Does Not Cascade-Clean `installed_tasks` or `node_variable_overrides`
- **File**: `src-tauri/src/commands/fleet.rs:89-101`
- **Issue**: Node removed but orphaned entries remain in config for deleted node's installed tasks and variable overrides.
- **Fix**: Clean up `installed_tasks` entries ending with `@{node_id}` and remove `node_variable_overrides[node_id]`.

### M17. `remove_node` Does Not Prevent Deletion of Local Node
- **File**: `src-tauri/src/commands/fleet.rs:89-101`
- **Issue**: No guard against deleting the auto-created local node. Mid-session deletion breaks task execution with no `node_id`.
- **Fix**: Check `node.kind != NodeKind::Local` before allowing removal.

### M18. SSH Config Parser Ignores `Include` Directives
- **File**: `src-tauri/src/fleet/ssh_config.rs:82-84`
- **Issue**: `Include` (standard since OpenSSH 7.3) treated as unknown directive. Hosts in included config files are missed.
- **Fix**: Implement `Include` directive processing with glob expansion.

### M19. SSH Config Parser Mishandles Multi-Alias `Host` Lines
- **File**: `src-tauri/src/fleet/ssh_config.rs:50-62`
- **Issue**: `Host alias1 alias2` treated as a single alias string. Wildcard check rejects entire line if any alias has wildcards.
- **Fix**: Split on whitespace and create separate entries per alias.

### M20. `SshConfig.fingerprint` Field Declared But Never Verified
- **File**: `src-tauri/src/fleet/mod.rs:49`
- **Issue**: Field exists but no code populates or checks it. Combined with `StrictHostKeyChecking=accept-new`, first-connection MITM is undetected.
- **Fix**: Implement fingerprint verification, or remove the field and document the accept-new limitation.

### M21. AppConfig TypeScript Type Missing 7+ Backend Fields
- **File**: `src/types/config.ts:1-5`
- **Issue**: Frontend `AppConfig` has 3 fields; backend `ConfigData` has 11+. Calling `setConfig` would wipe `installed_tasks`, `shared_variables`, `node_variable_overrides`, etc.
- **Fix**: Add all fields to `AppConfig`, or change backend `set_config` to partial merge.

### M22. AppArmor Monitor Can Be Started Multiple Times (Resource Leak)
- **File**: `src-tauri/src/apparmor/monitor.rs:10-97`
- **Issue**: No check if already running. Multiple spawned tasks emit duplicate events.
- **Fix**: Check `*running` before spawning; return early or error if already true.

### M23. Blocking I/O on Tokio Runtime Thread in AppArmor Monitor
- **File**: `src-tauri/src/apparmor/monitor.rs:58`
- **Issue**: `std::sync::mpsc::recv_timeout` and synchronous file I/O inside `tokio::spawn` blocks a worker thread.
- **Fix**: Use `tokio::sync::mpsc` with `tokio::select!` and `tokio::task::spawn_blocking` for file I/O.

### M24. `parse_audit_log` Reads Entire Audit Log Into Memory Synchronously
- **File**: `src-tauri/src/apparmor/log_parser.rs:52`
- **Issue**: `std::fs::read_to_string` on audit logs that can be hundreds of MB. Blocks Tokio thread and spikes memory.
- **Fix**: Use `tokio::task::spawn_blocking` and read only the tail of the file.

### M25. Rule Generator Creates Invalid AppArmor Syntax for Paths with Spaces
- **File**: `src-tauri/src/apparmor/rule_generator.rs:51`
- **Issue**: Paths with spaces are not quoted in generated rules, producing invalid AppArmor syntax.
- **Fix**: Quote paths containing spaces: `format!("  \"{}\" {},", name, mask)`.

### M26. `sync_installed_profiles` Swallows Errors via `.unwrap_or(false)`
- **File**: `src-tauri/src/commands/apparmor.rs:221-228`
- **Issue**: `config.save()` errors inside `sync_profile` are silently discarded. State becomes inconsistent.
- **Fix**: Log the error and include it in the `SyncResultInfo` response.

---

## LOW Severity Findings

### L1. `build_context` Silently Ignores Non-String JSON Config Values
- **File**: `src-tauri/src/tasks/script_task.rs:38-41`
- **Issue**: `.as_str()` returns `None` for numbers/booleans, falling back to default.

### L2. Tilde Expansion Replaces ALL Tildes, Not Just Leading `~/`
- **File**: `src-tauri/src/tasks/script_task.rs:45-48`
- **Issue**: `value.replace('~', home)` replaces every `~` occurrence.

### L3. Activity Log Written to Wrong Directory
- **File**: `src-tauri/src/lib.rs:42`
- **Issue**: Uses `dirs::config_dir()` (external crate) instead of `crate::dirs::config_dir()`. **Note**: One reviewer found this uses the crate's own `dirs` module and may be correct -- verify which `dirs` is in scope at line 42.

### L4. Path Duplication in `tasks/registry.rs`
- **File**: `src-tauri/src/tasks/registry.rs:123-126`
- **Issue**: Local `user_tasks_dir()` duplicates `crate::dirs::user_tasks_dir()` without `ensure_dir()`.

### L5. `/tmp` Fallback for Config Storage
- **File**: `src-tauri/src/dirs.rs:48-52`
- **Issue**: Falls back to `/tmp` if home directory detection fails, making config world-readable.

### L6. TOCTOU Between `is_policy_installed()` and `pkexec`
- **File**: `src-tauri/src/polkit/auth.rs:18-20`
- **Issue**: Policy file check is a UX guard, not security boundary. `pkexec` enforces real auth. Low risk.

### L7. `StrictHostKeyChecking=accept-new` Auto-Trusts First Connection
- **File**: `src-tauri/src/executor/ssh.rs:135`
- **Issue**: MITM vulnerability on first SSH connection to new hosts.

### L8. Vault Key Not Zeroized Using Constant-Time Operations
- **File**: `src-tauri/src/vault/mod.rs:161-168`
- **Issue**: `key.fill(0)` may be optimized away by compiler. Use `zeroize` crate.

### L9. Non-Atomic Vault File Writes
- **File**: `src-tauri/src/vault/mod.rs:204-208`
- **Issue**: `std::fs::write()` can leave corrupted vault on crash. Use write-to-temp + rename.

### L10. `dry_run_blueprint` Uses Wrong Executor for Admin Tasks
- **File**: `src-tauri/src/commands/blueprints.rs:544-545`
- **Issue**: Always uses `LocalExecutor`, never `PrivilegedLocalExecutor` for admin tasks in dry-run.

### L11. Blueprint Validation Missing: Version Format, Duplicate Task IDs
- **File**: `src-tauri/src/blueprints/validate.rs:18-56`
- **Issue**: No semver format check, no duplicate task ID detection.

### L12. Template Variable Validation Empty Block for Unknown References
- **File**: `src-tauri/src/apparmor/template_schema.rs:131-152`
- **Issue**: Unknown `{{var}}` references silently remain as literal text in rendered profiles.

### L13. `consolidate_masks` Defaults Empty Masks to "r" Permission
- **File**: `src-tauri/src/apparmor/rule_generator.rs:113`
- **Issue**: May grant incorrect permissions when `denied_mask` is empty.

### L14. Log Parser Regex Requires Fixed Field Order
- **File**: `src-tauri/src/apparmor/log_parser.rs:8-12`
- **Issue**: Different kernel/AppArmor versions may emit fields in different orders.

### L15. Inconsistent `task-progress` Event Payload (`node_id` present/absent)
- **Files**: `src-tauri/src/commands/tasks.rs:237` vs `:368`
- **Issue**: `execute_task` omits `node_id`; `uninstall_task` and blueprint commands include it.

### L16. `NodeStatusChangedEvent` Defined But Never Emitted
- **File**: `src/types/events.ts:16-20`
- **Issue**: Dead type. Node status uses polling, not events.

### L17. Unused `getConfig`/`setConfig` IPC Wrappers
- **File**: `src/services/tauriCommands.ts:121-127`
- **Issue**: Never called. `setConfig` would be destructive due to M21.

### L18. `useAppArmor` Hook Subscribes to Entire Store
- **File**: `src/hooks/useAppArmor.ts:10`
- **Issue**: No selector -- all components using this hook re-render on any store change.

### L19. Node Removal Has No Confirmation Dialog
- **File**: `src/pages/FleetPage.tsx:135-143`
- **Issue**: Single-click permanent deletion with no undo.

### L20. Multiple `ToastContainer` Instances Across Pages
- **File**: All 7 page components
- **Issue**: Should be rendered once at the app shell level.

### L21. `import * as TaskIcons` Prevents Tree-Shaking
- **Files**: `src/pages/BlueprintDetailPage.tsx:18`, `src/components/dashboard/ModuleCard.tsx:6`
- **Issue**: Namespace import of entire lucide-react library increases bundle size.

### L22. Documentation Gaps: 10 Implemented Features Undocumented
- **Files**: `docs/user-guide/*.md`
- **Issue**: Dry-run, inheritance, versioning, import/export, bulk apply, uninstall, activity log, vault, variables, and update detection have no user documentation.

---

## Recommended Priority

**Immediate fixes (security)**:
1. H1 -- Command injection in template expansion
2. H2 -- Privileged write TOCTOU
3. H3 -- SSH key path traversal
4. H4 -- AppArmor profile name path traversal
5. H5 -- `.qmbp` import path traversal

**High priority (functionality)**:
6. H9, H10, H11 -- IPC parameter mismatches (vault, node, blueprint)
7. H6 -- `uninstall_task` missing ExecutionTarget
8. H7, H8 -- Mutex held across await points
9. H13 -- Bulk apply missing version update
10. H14 -- `update_node` no validation

**Medium priority (correctness)**:
11. M2 -- Silent cycle drop in dependency resolution
12. M4 -- Blueprint apply partial failure handling
13. M6 -- Wrong escaping in privileged create_dir_all
14. M16, M17 -- Node removal cascade cleanup
15. M21 -- AppConfig type completeness
