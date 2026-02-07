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

### Session Notes
- Manual code review (no codex tool used per user instructions)
- Focused on CONTRACT ENFORCEMENT, ERROR PROPAGATION, and STATE CONSISTENCY strategies
- Found executor lifetime bug that would crash at runtime or cause undefined behavior
- Silent error drops during package import are data-loss risk
