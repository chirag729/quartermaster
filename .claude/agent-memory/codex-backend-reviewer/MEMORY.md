# Codex Backend Reviewer Memory

## Blueprint System Review (2026-02-07)

### Key Patterns Verified
- **Inheritance resolution**: `resolve_task_entries()` correctly walks ancestry chain, detects circular refs via `visited` HashSet (manager.rs:274-315)
- **Package safety**: Path traversal guard on line 195 (package.rs) checks `contains("..")` before extraction
- **Executor selection**: Tasks checked for `ExecutionTarget::LocalOnly` and `RemoteOnly` before execution (commands/blueprints.rs:359-369)
- **Privilege escalation**: `PrivilegeLevel::Admin` tasks use `PrivilegedLocalExecutor` (line 386-389)
- **Dry-run accuracy**: `DryRunExecutor` wraps real executor, clears actions between detect_state and execute (line 616-628)

### Critical Issues Found
1. **Executor lifetime error** (HIGH): apply_blueprint line 389, apply_blueprint_bulk line 803 — `PrivilegedLocalExecutor::new()` created in loop, returned as `&dyn CommandExecutor`. Executor is stack-allocated, reference outlives scope. This is an undefined behavior bug.
2. **Unguarded Result drops** (MEDIUM): package.rs lines 49, 127, 306-309, 318 — `fs::copy()`, `fs::write()` return `Result` but errors are silently dropped with `let _ =` or by forgetting to assign. Silent corruption during import/export.
3. **Config.save() error swallowing** (MEDIUM): apply_blueprint line 457, apply_blueprint_bulk line 859 — properly checked, but other codepaths may silently fail.
4. **SSH password retrieval silent failure** (MEDIUM): apply_blueprint line 334, line 554 — vault.get(key) called without strict error handling; None is accepted, potentially leading to SSH with no auth.
5. **Missing privilege level check in dry_run**: dry_run_blueprint (line 512) does not check `PrivilegeLevel::Admin` constraints, only `ExecutionTarget`. Dry-run can succeed when real run would fail.

### Safe Patterns
- Circular inheritance detected properly via visited set
- Blueprint validation enforces non-empty id/name/description/icon
- Package extraction validates manifest.yaml presence and zip integrity
- Task enable/disable flag honored during apply
- Task ordering preserved and re-sorted after dependency resolution

### Session Notes (Blueprint System)
- Manual code review (no codex tool used per user instructions)
- Focused on CONTRACT ENFORCEMENT, ERROR PROPAGATION, and STATE CONSISTENCY strategies
- Found executor lifetime bug that would crash at runtime or cause undefined behavior
- Silent error drops during package import are data-loss risk

## Backend Core & State Review (2026-02-09)

### Scope
Reviewed with codex CLI (applied all 6 strategies):
- src-tauri/src/state.rs
- src-tauri/src/config/manager.rs
- src-tauri/src/config/mod.rs
- src-tauri/src/error.rs
- src-tauri/src/dirs.rs
- src-tauri/src/lib.rs
- src-tauri/src/main.rs
- src-tauri/src/activity_log.rs
- src-tauri/src/notifications.rs
- src-tauri/src/variables/mod.rs

### Key Finding
No actionable bugs found in core/state infrastructure. Recent patch additions (uninstall_blueprint commands, notification helpers) are correctly wired and follow existing patterns.

## Architecture Plan Review (2026-02-15)

### Scope
Reviewed monorepo + service layer architecture plan from backend perspective:
- docs/development/service-layer-plan.md (frontend-focused plan)
- src-tauri/src/lib.rs, state.rs, commands/blueprints.rs
- src/types/events.ts, src/services/tauriCommands.ts
- package.json, vite.config.ts, tauri.conf.json

### Key Findings
1. **Backend requires NO code changes** for the restructure to work (HIGH confidence)
2. **Event subscription timing gap**: Services initialized at App.tsx mount need to safely subscribe to events before backend emits. Plan doesn't specify timing contract. Recommend: Add backend command `init_frontend_session()` for race-free delivery OR document timing assumptions explicitly.
3. **Event type contract mismatch**: Backend emits `blueprint-task-warning` and `bulk-blueprint-progress` events that lack frontend type definitions in `src/types/events.ts`. Should add these before moving subscriptions to service layer.
4. **CLI abstraction missing**: Plan mentions future CLI sharing `packages/core`, but services call `listen()` from `@tauri-apps/api/event` which requires Tauri webview context. Need event bus abstraction before CLI is built.
5. **AppArmor monitor cleanup**: Background task in apparmor/monitor.rs:107-110, 160-163 might outlive app if not properly stopped; ensure `appArmorService.stopMonitor()` is called in `destroyServices()`.

### Safe Findings
- Command registration in lib.rs is independent of monorepo structure
- AppState access pattern works correctly with services
- Tauri app.emit() compatible with singleton service listeners
- Event emission in blueprints.rs uses correct patterns (app.emit with typed events)
- Workspace structure won't affect Rust compilation

### Recommendations for Plan
1. Before Phase 0: Define how services subscribe to events (listen() requires Tauri context)
2. Phase 0: Add missing event types to `src/types/events.ts`
3. Phase 4: Ensure `appArmorService.stopMonitor()` cleanup is called
4. Future (before CLI): Design event bus abstraction to work in both Tauri and CLI contexts
5. Phase 5: Add test verifying no backend events are emitted without frontend type definitions
