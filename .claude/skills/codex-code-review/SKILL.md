---
description: "Comprehensive code review using OpenAI Codex CLI subagents as the review engine. Use when the user asks for a code review with Codex."
argument-hint: "optional scope: backend, frontend, security, or leave empty for all"
---

# Codex Code Review

Comprehensive code review using OpenAI Codex CLI (`codex`) via specialized subagents.

## Arguments

- `$ARGUMENTS` — optional scope filter. If provided (e.g., `backend`, `frontend`, `security`), only run review areas matching that keyword. If empty, run all review areas.

## Workflow

### Phase 1: Explore the Codebase

Launch an **Explore** subagent (quick) to identify any new files, recent changes, or structural shifts since the last review. Use the output to confirm the review area file lists below are still accurate. If new files have been added, assign them to the appropriate review area.

### Phase 2: Create a TODO List

Create a TaskCreate entry for **each review area** below (or the filtered subset if `$ARGUMENTS` was provided). Each task should use the review area name as the subject and include the area's scope and focus in the description. Set the `activeForm` to "Reviewing [area name] with Codex".

### Phase 3: Execute Parallel Codex Reviews

For each TODO task, launch the appropriate **named subagent** using the Task tool. Map review areas to subagents as follows:

| Review Area | Subagent (`subagent_type`) |
|---|---|
| Backend Core & State | `codex-backend-reviewer` |
| Task System | `codex-backend-reviewer` |
| Command Handlers | `codex-backend-reviewer` |
| Security & Executors | `codex-security-reviewer` |
| Fleet & SSH | `codex-backend-reviewer` |
| Blueprint System | `codex-backend-reviewer` |
| AppArmor System | `codex-backend-reviewer` |
| Frontend Stores & IPC | `codex-frontend-reviewer` |
| Frontend UI Components | `codex-frontend-reviewer` |
| Cross-Cutting & Tests | `codex-integration-reviewer` |

For each subagent launch, pass a prompt with this template:

```
Review Area: [AREA NAME]
Slug: [AREA SLUG]
Files to review:
- [file1]
- [file2]
- ...

Focus points:
- [focus1]
- [focus2]
- ...
```

Launch up to **4 subagents in parallel**. When one batch completes, launch the next. After each subagent finishes, mark its TODO task as completed.

### Phase 4: Consolidate Findings

After all reviews complete:

1. Read all `codex-review-*.md` files from the project root (the subagents create these).
2. Also collect any findings returned directly in the subagent responses.
3. Consolidate into a single `codex-code-review.md` with findings grouped by severity (HIGH first, then MEDIUM, then LOW).
4. Deduplicate any findings that appear in multiple review areas.
5. Add a "Validation" section recording current test counts (`cargo test`, `npm run test`) and build status (`cargo check`, `npx tsc --noEmit`).
6. Delete the individual `codex-review-*.md` files.

### Phase 5: Present Results

Show the user a summary of findings by severity count, then ask if they want to proceed with fixes.

---

## Review Areas

### 1. Backend Core & State

- **Slug**: `backend-core`
- **Subagent**: `codex-backend-reviewer`
- **Scope**: `src-tauri/src/state.rs`, `src-tauri/src/config/manager.rs`, `src-tauri/src/config/mod.rs`, `src-tauri/src/error.rs`, `src-tauri/src/dirs.rs`, `src-tauri/src/lib.rs`, `src-tauri/src/main.rs`, `src-tauri/src/activity_log.rs`, `src-tauri/src/notifications.rs`, `src-tauri/src/variables/mod.rs`
- **Focus**:
  - State management patterns (`Arc<Mutex<T>>`) — deadlock potential from nested lock acquisition
  - Config serialization/deserialization — backward compatibility of `#[serde(default)]`
  - Error type coverage — are all `AppError` variants used and properly mapped?
  - `let _ =` on `Result` — is the error intentionally or accidentally swallowed?
  - Command registration in `lib.rs` — are all commands registered? Any missing?
- **Keywords**: `backend`, `core`, `state`, `config`

### 2. Task System

- **Slug**: `task-system`
- **Subagent**: `codex-backend-reviewer`
- **Scope**: `src-tauri/src/tasks/mod.rs`, `src-tauri/src/tasks/registry.rs`, `src-tauri/src/tasks/script_task.rs`, `src-tauri/src/tasks/embedded.rs`, `src-tauri/src/tasks/install_state.rs`, `src-tauri/src/tasks/execution_log.rs`, `src-tauri/src/tasks/validate.rs`, `src-tauri/src/tasks/yaml_schema.rs`
- **Focus**:
  - `SetupTask` trait compliance — do all implementations satisfy the contract?
  - Task registry — are all tasks registered? Dependency resolution correctness?
  - Install state tracking — drift detection, version comparison logic
  - Script task execution — shell command construction, error handling
  - YAML schema validation — are invalid task definitions caught?
- **Keywords**: `backend`, `tasks`

### 3. Command Handlers

- **Slug**: `command-handlers`
- **Subagent**: `codex-backend-reviewer`
- **Scope**: `src-tauri/src/commands/tasks.rs`, `src-tauri/src/commands/blueprints.rs`, `src-tauri/src/commands/fleet.rs`, `src-tauri/src/commands/ssh.rs`, `src-tauri/src/commands/apparmor.rs`, `src-tauri/src/commands/config.rs`, `src-tauri/src/commands/system.rs`, `src-tauri/src/commands/activity.rs`, `src-tauri/src/commands/vault.rs`, `src-tauri/src/commands/yubikey.rs`, `src-tauri/src/commands/variables.rs`, `src-tauri/src/commands/mod.rs`
- **Focus**:
  - Parameter validation — are invalid inputs rejected early?
  - Lock acquisition ordering — could two commands deadlock by acquiring locks in different order?
  - Error propagation — no `let _ = result` on critical operations (config.save, fleet.update)
  - Constraint enforcement — ExecutionTarget, PrivilegeLevel checked before execution in ALL paths
  - Event emission — payload shapes must match frontend listener expectations
- **Keywords**: `backend`, `commands`, `handlers`

### 4. Security & Executors

- **Slug**: `security-executors`
- **Subagent**: `codex-security-reviewer`
- **Scope**: `src-tauri/src/executor/mod.rs`, `src-tauri/src/executor/local.rs`, `src-tauri/src/executor/ssh.rs`, `src-tauri/src/executor/privileged.rs`, `src-tauri/src/executor/dry_run.rs`, `src-tauri/src/polkit/auth.rs`, `src-tauri/src/polkit/mod.rs`, `src-tauri/src/vault/mod.rs`
- **Focus**:
  - Command injection — are shell arguments properly escaped? Especially in `privileged.rs` write_file/create_dir_all
  - Privilege escalation — does `PrivilegedLocalExecutor` correctly wrap ALL mutation paths with pkexec?
  - SSH credential handling — passwords never logged, private keys never read/transmitted
  - Vault security — encryption, key derivation, secret lifecycle
  - Dry-run executor — does it truly intercept all mutations? Could a mutation leak through?
  - `is_local()` correctness — does every executor return the right value?
- **Keywords**: `backend`, `security`, `executor`, `polkit`, `vault`

### 5. Fleet & SSH

- **Slug**: `fleet-ssh`
- **Subagent**: `codex-backend-reviewer`
- **Scope**: `src-tauri/src/fleet/manager.rs`, `src-tauri/src/fleet/mod.rs`, `src-tauri/src/fleet/ssh_config.rs`, `src-tauri/src/fleet/status_poller.rs`, `src-tauri/src/fleet/terminal.rs`, `src-tauri/src/commands/ssh.rs`, `src-tauri/src/commands/fleet.rs`
- **Focus**:
  - Node CRUD — validation, uniqueness, cascade on delete
  - SSH config discovery — parsing correctness, edge cases
  - Status poller — does it handle unreachable nodes gracefully? Memory leaks from polling?
  - Terminal session management — lifecycle, cleanup on disconnect
  - Credential flow — vault key resolution for password auth
- **Keywords**: `backend`, `fleet`, `ssh`

### 6. Blueprint System

- **Slug**: `blueprint-system`
- **Subagent**: `codex-backend-reviewer`
- **Scope**: `src-tauri/src/blueprints/manager.rs`, `src-tauri/src/blueprints/mod.rs`, `src-tauri/src/blueprints/package.rs`, `src-tauri/src/blueprints/validate.rs`, `src-tauri/src/blueprints/yaml_schema.rs`, `src-tauri/src/commands/blueprints.rs`
- **Focus**:
  - Blueprint lifecycle — create, update (version bumping), delete, clone, import/export
  - Inheritance resolution — `extends` field, task entry merging, circular reference detection
  - Package format — zip integrity, path traversal in import, manifest validation
  - Apply/bulk-apply — executor selection per task (privilege level), failure handling per node
  - Dry-run — accuracy of intercepted vs. passed-through operations
- **Keywords**: `backend`, `blueprints`

### 7. AppArmor System

- **Slug**: `apparmor`
- **Subagent**: `codex-backend-reviewer`
- **Scope**: `src-tauri/src/apparmor/mod.rs`, `src-tauri/src/apparmor/log_parser.rs`, `src-tauri/src/apparmor/monitor.rs`, `src-tauri/src/apparmor/profile_manager.rs`, `src-tauri/src/apparmor/rule_consolidator.rs`, `src-tauri/src/apparmor/rule_generator.rs`, `src-tauri/src/apparmor/template_manager.rs`, `src-tauri/src/apparmor/template_schema.rs`, `src-tauri/src/apparmor/tunable_installer.rs`, `src-tauri/src/apparmor/types.rs`, `src-tauri/src/commands/apparmor.rs`
- **Focus**:
  - Log parsing — edge cases in audit log format, malformed lines
  - Rule generation — are generated rules correct AppArmor syntax?
  - Profile management — loading, saving, reloading profiles safely
  - Monitor — real-time log tailing, resource cleanup on stop
  - Template system — schema validation, substitution correctness
- **Keywords**: `backend`, `apparmor`

### 8. Frontend Stores & IPC

- **Slug**: `frontend-stores`
- **Subagent**: `codex-frontend-reviewer`
- **Scope**: `src/stores/taskStore.ts`, `src/stores/fleetStore.ts`, `src/stores/blueprintStore.ts`, `src/stores/appArmorStore.ts`, `src/stores/themeStore.ts`, `src/stores/toastStore.ts`, `src/services/tauriCommands.ts`, `src/types/task.ts`, `src/types/node.ts`, `src/types/blueprint.ts`, `src/types/events.ts`, `src/types/apparmor.ts`, `src/types/config.ts`, `src/hooks/useTauriEvent.ts`, `src/hooks/useTasks.ts`, `src/hooks/useAppArmor.ts`
- **Focus**:
  - Store patterns — proper state updates, no stale closures
  - IPC contract alignment — every `invoke()` parameter name and return type must match the Rust command signature (accounting for camelCase→snake_case auto-conversion)
  - Event type definitions — every `useTauriEvent<T>` type param must match the backend `app.emit()` payload shape
  - Error handling — are IPC errors caught and surfaced to the user?
  - Missing IPC wrappers — are there backend commands with no frontend wrapper?
- **Keywords**: `frontend`, `stores`, `ipc`, `types`

### 9. Frontend UI Components

- **Slug**: `frontend-ui`
- **Subagent**: `codex-frontend-reviewer`
- **Scope**: `src/pages/DashboardPage.tsx`, `src/pages/FleetPage.tsx`, `src/pages/NodeDetailPage.tsx`, `src/pages/BlueprintsPage.tsx`, `src/pages/BlueprintDetailPage.tsx`, `src/pages/TaskLibraryPage.tsx`, `src/pages/AppArmorPage.tsx`, `src/pages/SettingsPage.tsx`, `src/components/blueprints/*.tsx`, `src/components/fleet/*.tsx`, `src/components/dashboard/*.tsx`, `src/components/apparmor/*.tsx`, `src/components/ssh/*.tsx`, `src/components/layout/*.tsx`, `src/components/modules/*.tsx`
- **Focus**:
  - Loading/error states — does every page handle loading and error correctly?
  - Event listener cleanup — are `useTauriEvent` and `useEffect` properly cleaned up on unmount?
  - Conditional rendering — are null/undefined cases handled? Empty state display?
  - User feedback — do destructive actions have confirmation? Are operations reported via toasts?
  - Dark mode — are all components styled for both light and dark themes?
- **Keywords**: `frontend`, `ui`, `components`, `pages`

### 10. Cross-Cutting Contracts & Tests

- **Slug**: `cross-cutting`
- **Subagent**: `codex-integration-reviewer`
- **Scope**: `src/__tests__/stores/*.test.ts`, `src/__mocks__/*.ts`, `src/__tests__/setup.ts`, `docs/user-guide/*.md`, `docs/code-review-guidelines.md`, `CLAUDE.md`
- **Focus**:
  - Test coverage gaps — which backend functions and frontend stores lack tests?
  - Mock accuracy — do test mocks reflect the actual API surface?
  - Documentation accuracy — do docs match the current implementation?
  - Backend-to-frontend contract — for each `app.emit()` event, does the frontend listener use the correct type?
  - For each feature documented in user guides, is it actually implemented end-to-end?
- **Keywords**: `tests`, `docs`, `contracts`, `cross-cutting`
