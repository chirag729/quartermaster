# Codex Code Review — Quartermaster

**Date**: 2026-02-07
**Engine**: Codex-backed subagents (10 review areas)
**Branch**: `initial-commit`

---

## Summary

| Severity | Count |
|----------|-------|
| HIGH     | 12    |
| MEDIUM   | 30    |
| LOW      | 20    |

---

## HIGH Findings

### H-1. Path traversal in blueprint package import
- **Area**: Security & Executors
- **File**: `src-tauri/src/blueprints/package.rs:195`
- **Issue**: When importing a `.qmbp` zip archive, file names extracted from the zip are used to construct output paths without sanitizing `..` or absolute path components.
- **Impact**: A malicious `.qmbp` file could write arbitrary files outside the blueprints directory.

### H-2. TOCTOU race condition in privileged write_file
- **Area**: Security & Executors
- **File**: `src-tauri/src/executor/privileged.rs:50-77`
- **Issue**: `write_file` writes content to a temp file, then invokes `pkexec cp` to copy it to the target. Between temp-file creation and the privileged copy, the temp file can be swapped via symlink.
- **Impact**: Attacker with local access could escalate a user-writable temp file into a privileged write to any system path.

### H-3. Inadequate argument escaping in create_dir_all
- **Area**: Security & Executors
- **File**: `src-tauri/src/executor/privileged.rs:79-92`
- **Issue**: The `create_dir_all` function passes the directory path to a shell command via `pkexec mkdir -p` without proper escaping. Paths containing shell metacharacters (`;`, `$()`, backticks) could inject commands.
- **Impact**: Command injection running as root via a crafted directory path.

### H-4. Executor lifetime violation in apply_blueprint
- **Area**: Blueprint System / Command Handlers
- **File**: `src-tauri/src/commands/blueprints.rs:386-389`
- **Issue**: `apply_blueprint` creates a `PrivilegedLocalExecutor` as a temporary reference that may not live long enough for async task execution. The executor reference is used across an `.await` boundary.
- **Impact**: Undefined behavior or use-after-free in privileged task execution paths.

### H-5. Lock acquisition ordering risk in apply_blueprint_bulk
- **Area**: Command Handlers
- **File**: `src-tauri/src/commands/blueprints.rs:~803`
- **Issue**: `apply_blueprint_bulk` acquires the blueprint manager lock and the fleet manager lock. If another command acquires them in reverse order, a deadlock occurs.
- **Impact**: Application freeze under concurrent bulk operations.

### H-6. Credential flow missing from SSH commands
- **Area**: Fleet & SSH
- **File**: `src-tauri/src/commands/ssh.rs`
- **Issue**: `test_ssh_connection` and related SSH commands do not retrieve passwords from the vault when the node uses password authentication. The vault credential flow is not integrated.
- **Impact**: SSH password authentication silently fails; users cannot test connections for password-authenticated nodes.

### H-7. Duplicate task IDs accepted without error
- **Area**: Task System
- **File**: `src-tauri/src/tasks/registry.rs:19-21`
- **Issue**: `TaskRegistry::register` pushes tasks into `self.tasks` without checking for duplicate `id()` values. If a user task reuses an existing ID, the registry contains multiple tasks with the same ID.
- **Impact**: `get()` returns the first match while `tasks()` exposes both, creating ambiguous execution and inconsistent dependency resolution.

### H-8. AppArmor monitor error swallowing on event emission
- **Area**: AppArmor System
- **File**: `src-tauri/src/apparmor/monitor.rs`
- **Issue**: The monitor uses `let _ = app.emit(...)` when emitting denial events. If the event channel is full or the app handle is invalid, the denial event is silently dropped.
- **Impact**: Users miss real-time AppArmor denial notifications with no indication of data loss.

### H-9. Log parser silently drops malformed denial events
- **Area**: AppArmor System
- **File**: `src-tauri/src/apparmor/log_parser.rs`
- **Issue**: Malformed audit log lines that partially match the denial pattern but lack required fields are silently skipped without logging a warning.
- **Impact**: Denial events from newer kernel versions or non-standard AppArmor configurations are invisible.

### H-10. Monitor doesn't detect resource exhaustion or EOF
- **Area**: AppArmor System
- **File**: `src-tauri/src/apparmor/monitor.rs`
- **Issue**: The file-tailing monitor loop does not detect when the audit log file is truncated, rotated, or when the file handle reaches an error state.
- **Impact**: After log rotation, the monitor silently stops receiving new events while appearing to run normally.

### H-11. Missing type definition for bulk-blueprint-progress event
- **Area**: Frontend Stores & IPC
- **File**: `src/types/events.ts`
- **Issue**: The backend emits `bulk-blueprint-progress` events during bulk apply operations, but no corresponding TypeScript interface exists in `events.ts`. The frontend listener uses an inline type.
- **Impact**: Event payload drift between backend and frontend goes undetected by the type system.

### H-12. Generic invoke mock returns empty array for all commands
- **Area**: Cross-Cutting & Tests
- **File**: `src/__mocks__/tauri.ts:1-3`
- **Issue**: The mock `invoke()` function returns `[]` for ALL command invocations regardless of the command name. Tests pass but don't validate that correct data shapes flow through the system.
- **Impact**: Real API returning unexpected data structures would not be caught by tests. Store tests work around this by mocking the service layer separately.

---

## MEDIUM Findings

### M-1. Potential deadlock from nested lock acquisition
- **Area**: Backend Core & State
- **File**: `src-tauri/src/state.rs`
- **Issue**: Multiple `Arc<Mutex<T>>` fields in `AppState` could be locked in different orders across commands. No documented lock ordering exists.
- **Impact**: Deadlock under concurrent operations.

### M-2. Error swallowing in activity_log.rs
- **Area**: Backend Core & State
- **File**: `src-tauri/src/activity_log.rs`
- **Issue**: `let _ = self.save()` discards save failures silently.
- **Impact**: Activity log entries may be lost without user notification.

### M-3. Error swallowing in notifications.rs
- **Area**: Backend Core & State
- **File**: `src-tauri/src/notifications.rs`
- **Issue**: Notification send failures are silently ignored.
- **Impact**: Users may not receive completion notifications for long-running operations.

### M-4. Error swallowing in download/mod.rs
- **Area**: Backend Core & State
- **File**: `src-tauri/src/download/mod.rs`
- **Issue**: Download-related error results are discarded with `let _ =`.
- **Impact**: Failed downloads may not be reported to the user.

### M-5. Progress callback error suppression
- **Area**: Task System
- **File**: `src-tauri/src/commands/tasks.rs`
- **Issue**: `let _ = app.emit("task-progress", ...)` suppresses event emission failures.
- **Impact**: Frontend progress bars may stall without any error indication.

### M-6. Cyclic dependencies silently dropped
- **Area**: Task System
- **File**: `src-tauri/src/tasks/registry.rs:101-119`
- **Issue**: `resolve_dependencies` performs Kahn topological sort but never checks whether all required nodes were emitted. Cycles result in incomplete results rather than errors.
- **Impact**: Blueprint task ordering may silently omit required tasks when cycles exist.

### M-7. Runtime failures misreported as "NotStarted"
- **Area**: Task System
- **File**: `src-tauri/src/tasks/script_task.rs:185-188`
- **Issue**: `detect_state` maps all non-zero/Err outcomes to `TaskStatus::NotStarted` instead of an error state.
- **Impact**: SSH failures and permission errors display as "not installed" rather than indicating an error.

### M-8. Validation allows template variables runtime doesn't provide
- **Area**: Task System
- **File**: `src-tauri/src/tasks/validate.rs:19`
- **Issue**: `BUILTIN_VARS` includes `install_path` as always-available, but `build_context` never injects it unless also defined as a config key.
- **Impact**: Tasks pass validation but execute with unresolved `{{install_path}}` placeholders.

### M-9. Missing parameter validation in ssh.rs (key_type)
- **Area**: Command Handlers
- **File**: `src-tauri/src/commands/ssh.rs`
- **Issue**: `key_type` parameter is passed through to key generation without validating against allowed values (ed25519, rsa, ecdsa, ed25519-sk).
- **Impact**: Invalid key types produce confusing errors from ssh-keygen.

### M-10. Potential shell injection in system.rs
- **Area**: Command Handlers
- **File**: `src-tauri/src/commands/system.rs`
- **Issue**: System commands may pass user-controlled input to shell execution without sanitization.
- **Impact**: Potential command injection if user-controlled values reach shell execution.

### M-11. Implicit error swallowing in list_ssh_keys
- **Area**: Command Handlers
- **File**: `src-tauri/src/commands/ssh.rs`
- **Issue**: Errors from SSH key listing operations are silently converted to empty results.
- **Impact**: SSH key enumeration failures appear as "no keys found."

### M-12. Missing state validation in fleet.rs (update_node)
- **Area**: Command Handlers
- **File**: `src-tauri/src/commands/fleet.rs`
- **Issue**: `update_node` does not validate that the updated node data is consistent (e.g., SSH port in valid range, hostname not empty).
- **Impact**: Invalid node state can be persisted and cause failures later during SSH connections.

### M-13. No validation of config overrides in blueprints.rs
- **Area**: Command Handlers
- **File**: `src-tauri/src/commands/blueprints.rs`
- **Issue**: Config overrides provided during blueprint apply are not validated against the task's expected config schema.
- **Impact**: Invalid config values pass through and cause runtime failures during task execution.

### M-14. ExecutionTarget constraint bypass timing
- **Area**: Command Handlers
- **File**: `src-tauri/src/commands/blueprints.rs`
- **Issue**: In `apply_blueprint_bulk`, the ExecutionTarget check may occur after some setup work, wasting resources on tasks that will ultimately be rejected.
- **Impact**: Unnecessary processing and confusing partial-progress events for tasks that cannot run on a target.

### M-15. Race condition in node status polling
- **Area**: Command Handlers
- **File**: `src-tauri/src/fleet/status_poller.rs`
- **Issue**: Status poll results can arrive after a node has been removed, updating state for non-existent nodes.
- **Impact**: Stale status entries may accumulate in memory.

### M-16. Missing SSH key validation in deploy_ssh_key
- **Area**: Command Handlers
- **File**: `src-tauri/src/commands/ssh.rs`
- **Issue**: The public key content deployed to a remote node is not validated for format correctness before deployment.
- **Impact**: Malformed keys in `authorized_keys` could lock out SSH access.

### M-17. SSH error messages leak usernames
- **Area**: Security & Executors
- **File**: `src-tauri/src/executor/ssh.rs`
- **Issue**: SSH connection error messages include the username in the error string returned to the frontend.
- **Impact**: Username enumeration via error message inspection.

### M-18. SSH password in environment variable
- **Area**: Security & Executors
- **File**: `src-tauri/src/executor/ssh.rs`
- **Issue**: SSH passwords may be passed via environment variables during connection, visible in `/proc/<pid>/environ`.
- **Impact**: Local privilege escalation could expose SSH passwords.

### M-19. Unvalidated task/node/blueprint IDs in path construction
- **Area**: Security & Executors
- **File**: Multiple files under `src-tauri/src/`
- **Issue**: User-supplied IDs are used directly in file path construction without validating they don't contain path separators or special characters.
- **Impact**: Path traversal via crafted IDs when constructing config/log file paths.

### M-20. DryRunExecutor doesn't intercept read operations
- **Area**: Security & Executors
- **File**: `src-tauri/src/executor/dry_run.rs`
- **Issue**: `DryRunExecutor` intercepts mutations but passes through all read commands, which may have side effects (e.g., commands that read and modify state).
- **Impact**: Dry-run mode may not be fully side-effect-free.

### M-21. SSH deploy key lacks credential management
- **Area**: Fleet & SSH
- **File**: `src-tauri/src/commands/ssh.rs`
- **Issue**: The key deployment flow does not integrate with the vault for password retrieval when deploying to password-authenticated nodes.
- **Impact**: Key deployment fails silently for password-auth nodes.

### M-22. Node hostname uniqueness not enforced
- **Area**: Fleet & SSH
- **File**: `src-tauri/src/fleet/manager.rs`
- **Issue**: Multiple nodes can be created with the same hostname/IP and port combination without warning.
- **Impact**: Ambiguous node selection and potential for duplicate operations.

### M-23. Open terminal doesn't validate auth feasibility
- **Area**: Fleet & SSH
- **File**: `src-tauri/src/fleet/terminal.rs`
- **Issue**: Terminal launch does not verify that the required SSH authentication method (key, password, agent) is available before spawning the terminal process.
- **Impact**: Terminal opens and immediately fails with a cryptic error.

### M-24. Profile name validation inconsistency
- **Area**: AppArmor System
- **File**: `src-tauri/src/apparmor/profile_manager.rs`
- **Issue**: Profile names are validated differently in different code paths (creation vs. loading).
- **Impact**: Profiles created with certain characters may fail to load, or vice versa.

### M-25. Config save error swallowed after profile install
- **Area**: AppArmor System
- **File**: `src-tauri/src/apparmor/profile_manager.rs`
- **Issue**: After installing an AppArmor profile, the config save uses `let _ =` to discard errors.
- **Impact**: Profile installation appears successful but configuration changes are lost.

### M-26. Rule generator creates invalid syntax for embedded newlines
- **Area**: AppArmor System
- **File**: `src-tauri/src/apparmor/rule_generator.rs`
- **Issue**: Generated rules don't handle paths containing newline characters, producing invalid AppArmor syntax.
- **Impact**: Profile loading fails with syntax errors if denial paths contain unusual characters.

### M-27. Monitor file handle leak on early exit
- **Area**: AppArmor System
- **File**: `src-tauri/src/apparmor/monitor.rs`
- **Issue**: If the monitor is stopped while processing a batch of lines, the file handle may not be properly closed.
- **Impact**: File descriptor leak, eventually exhausting available handles.

### M-28. Template render doesn't escape braces
- **Area**: AppArmor System
- **File**: `src-tauri/src/apparmor/template_manager.rs`
- **Issue**: AppArmor profiles use `{` and `}` for glob patterns, but the template renderer also uses `{{` and `}}`. Literal braces in AppArmor syntax must be escaped but the renderer doesn't handle this.
- **Impact**: Templates with AppArmor glob patterns (e.g., `{bin,sbin}`) may be incorrectly interpreted as template variables.

### M-29. Consolidator doesn't validate resulting AppArmor syntax
- **Area**: AppArmor System
- **File**: `src-tauri/src/apparmor/rule_consolidator.rs`
- **Issue**: After consolidating multiple rules, the result is not validated for correct AppArmor syntax.
- **Impact**: Consolidated rules may produce invalid profiles that fail to load.

### M-30. Event listener not cleaned up in BulkBlueprintDialog
- **Area**: Frontend UI Components
- **File**: `src/components/blueprints/BulkBlueprintDialog.tsx`
- **Issue**: Event listeners registered for bulk progress tracking may not be properly cleaned up when the dialog is closed mid-operation.
- **Impact**: Memory leak and stale state updates after dialog dismissal.

---

## LOW Findings

### L-1. Config serde defaults inconsistency
- **Area**: Backend Core & State
- **File**: `src-tauri/src/config/mod.rs`
- **Issue**: Some config fields use `#[serde(default)]` while similar fields use `#[serde(default = "fn_name")]` without a clear pattern for when each approach is preferred.

### L-2. Legacy config path duplication
- **Area**: Backend Core & State
- **File**: `src-tauri/src/config/manager.rs`
- **Issue**: Migration code for legacy config paths is duplicated across config loading and saving.

### L-3. Error serialization performance
- **Area**: Backend Core & State
- **File**: `src-tauri/src/error.rs`
- **Issue**: `AppError` serialization for IPC uses `.to_string()` which allocates unnecessarily for some variants.

### L-4. Uninstall and version-detect scripts bypass template validation
- **Area**: Task System
- **File**: `src-tauri/src/tasks/validate.rs:166-168`
- **Issue**: Validator checks template variables in `steps[].run` and `detect`, but not in `uninstall[].run` or `version_detect`.
- **Impact**: Invalid placeholders in uninstall commands are accepted at load time and fail only at runtime.

### L-5. Missing bounds check in YubiKey serial parsing
- **Area**: Command Handlers
- **File**: `src-tauri/src/commands/yubikey.rs`
- **Issue**: YubiKey serial number parsing does not validate the numeric range.

### L-6. Inconsistent error handling in apparmor.rs commands
- **Area**: Command Handlers
- **File**: `src-tauri/src/commands/apparmor.rs`
- **Issue**: Some AppArmor commands use `map_err` while others use `?` with different error context patterns.

### L-7. Unused variable in tasks.rs
- **Area**: Command Handlers
- **File**: `src-tauri/src/commands/tasks.rs`
- **Issue**: Minor unused variable that should be prefixed with `_`.

### L-8. Event payload shape mismatch risk
- **Area**: Command Handlers
- **File**: `src-tauri/src/commands/tasks.rs`
- **Issue**: `task-progress` event is emitted with/without `node_id` field depending on execution context. Frontend listener type (`TaskProgressEvent`) does not include `node_id`.

### L-9. Missing newline in SSH key deployment
- **Area**: Command Handlers
- **File**: `src-tauri/src/commands/ssh.rs`
- **Issue**: Deployed SSH key may not have a trailing newline in `authorized_keys`.

### L-10. Missing ID validation in export_blueprint_package
- **Area**: Security & Executors
- **File**: `src-tauri/src/blueprints/package.rs`
- **Issue**: Blueprint ID used in export path is not validated for path-unsafe characters.

### L-11. Overly permissive vault error details
- **Area**: Security & Executors
- **File**: `src-tauri/src/vault/mod.rs`
- **Issue**: Vault error messages include internal details that could aid an attacker in understanding the encryption implementation.

### L-12. SSH config parser discards multiple identity files
- **Area**: Fleet & SSH
- **File**: `src-tauri/src/fleet/ssh_config.rs`
- **Issue**: When an SSH config host specifies multiple `IdentityFile` directives, only the last one is retained.

### L-13. Terminal processes silently detached
- **Area**: Fleet & SSH
- **File**: `src-tauri/src/fleet/terminal.rs`
- **Issue**: Spawned terminal processes are detached without tracking. No way to know if the terminal successfully connected.

### L-14. SSH key generation uses predictable names
- **Area**: Fleet & SSH
- **File**: `src-tauri/src/commands/ssh.rs`
- **Issue**: Generated SSH key files use a predictable naming pattern based on the key type.

### L-15. Audit log parser truncates to 200 denials
- **Area**: AppArmor System
- **File**: `src-tauri/src/apparmor/log_parser.rs`
- **Issue**: The parser silently truncates results to 200 denial entries without notifying the user that more exist.

### L-16. Template variables are case-sensitive
- **Area**: AppArmor System
- **File**: `src-tauri/src/apparmor/template_manager.rs`
- **Issue**: Template variable substitution is case-sensitive (`{{HOME}}` != `{{home}}`), but this is not documented.

### L-17. useTauriEvent hook dependency array
- **Area**: Frontend Stores & IPC
- **File**: `src/hooks/useTauriEvent.ts`
- **Issue**: The hook uses a ref for the handler to avoid re-subscribing, but the event name is in the dependency array — changing the event name at runtime causes a re-subscribe without cleanup race potential.

### L-18. Blueprint store methods don't update state for remote operations
- **Area**: Frontend Stores & IPC
- **File**: `src/stores/blueprintStore.ts`
- **Issue**: Some store methods (assign/unassign) don't optimistically update local state, requiring a full refresh.

### L-19. Test coverage gaps — FleetStore missing multi-select tests
- **Area**: Cross-Cutting & Tests
- **File**: `src/__tests__/stores/fleetStore.test.ts`
- **Issue**: `toggleNodeSelection()`, `selectAllNodes()`, and `clearSelection()` are untested in the dedicated store test file.

### L-20. No cross-boundary event payload validation tests
- **Area**: Cross-Cutting & Tests
- **File**: `src/__tests__/`, `src/types/events.ts`
- **Issue**: No tests validate that backend `app.emit()` payloads match the TypeScript type definitions in `events.ts`. A backend change could silently break frontend listeners.

---

## Validation

| Check | Result |
|-------|--------|
| `cargo test` | 301 passed, 0 failed |
| `npm run test` | 82 passed (6 test files) |
| `cargo check` | Clean (no warnings) |
| `npx tsc --noEmit` | Clean (0 errors) |

---

## Notes

- Several findings overlap across review areas (e.g., executor lifetime issue found by both Blueprint System and Command Handlers reviewers) — deduplicated above.
- The `let _ = result` pattern appears in 6+ locations across the codebase and represents a systemic issue worth addressing with a lint rule.
- Lock ordering across `AppState` mutexes has no documented convention — this is the root cause of multiple deadlock-risk findings.
- The AppArmor monitor has the highest concentration of issues (5 findings), suggesting it may benefit from a focused refactor.
