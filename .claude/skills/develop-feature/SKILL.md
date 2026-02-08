---
description: "Develop a feature end-to-end with professional engineering quality. Use when implementing any non-trivial feature, enhancement, or fix."
argument-hint: "description of the feature to implement"
---

# Develop Feature

Implements a feature end-to-end across the Rust backend and React frontend, producing professional-grade code with comprehensive tests and full verification.

## Arguments

- `$ARGUMENTS` — description of the feature to implement. Can be a brief phrase (e.g., "task detail page") or a detailed specification. If a plan file or issue URL is provided, use that as the source of truth.

## Quality Standard

All code produced by this skill must meet the following bar:

- **Production-ready**: No TODO comments, no placeholder logic, no hardcoded shortcuts. Every code path handles errors, edge cases, and empty states.
- **Idiomatic**: Rust code follows Clippy lints and the project's `Arc<Mutex<T>>` state patterns. TypeScript uses strict mode, proper typing (no `any`), and the project's Zustand/React conventions.
- **Secure by default**: User input is sanitized before shell use. Credentials are never logged or serialized to the frontend. Privileged operations go through PolicyKit. Path inputs are validated against traversal.
- **Tested**: Every important behavior has a corresponding test. Backend logic gets `#[test]` functions. Frontend stores and complex components get Vitest tests. Test helpers are updated when shared structs change.
- **Consistent**: New code matches existing patterns exactly. Study adjacent files before writing. Don't invent new conventions when the codebase already has one.

## Workflow

### Phase 1: Understand

Before writing any code, build a complete mental model of what needs to change.

1. **Parse the requirement**: Read `$ARGUMENTS` carefully. If it references a plan file, issue, or PR, fetch and read it in full.
2. **Explore the affected area**: Use the Explore agent or direct Read/Grep to understand every file that will be touched. Read related files (adjacent modules, types, tests) to understand existing patterns.
3. **Identify the full scope**: Map out ALL files that need changes. This includes:
   - Backend structs, traits, and implementations
   - Tauri command handlers and their registration in `src-tauri/src/lib.rs`
   - IPC wrappers in `src/services/tauriCommands.ts`
   - Frontend types in `src/types/`
   - Frontend stores, hooks, components, and pages
   - Routes in `src/App.tsx`
   - Test files and test helpers
   - Documentation in `docs/user-guide/` if user-visible behavior changes

**Critical rule**: Do NOT start writing code until you understand the existing patterns in every file you plan to modify. Read first, always.

### Phase 2: Plan with a TODO List

Create a TODO list using TaskCreate with one task per logical unit of work. This is mandatory for all but single-file trivial changes.

Guidelines for task granularity:
- **One task per file or tightly-coupled file group** (e.g., "Add StepInfo struct and trait methods to tasks/mod.rs" is one task)
- **Separate tasks for backend vs frontend** — they can often be done in parallel
- **A dedicated task for tests** when tests are substantial (new test file, many test cases)
- **A final verification task** that runs `cargo test`, `npm run test`, and `npm run build`

Set dependencies between tasks where order matters (e.g., backend types must exist before frontend types that reference them).

Mark each task `in_progress` when you start it and `completed` when done. Never skip the TODO list — it enforces thoroughness and prevents forgetting steps.

### Phase 3: Implement

Work through the TODO list methodically, one task at a time. For each task:

#### 3a. Backend Changes (Rust)

When modifying backend code, follow these rules:

- **Structs**: New optional fields on serialized structs (TaskInfo, Blueprint, Node) MUST use `#[serde(default)]` for backward compatibility. Update ALL constructors including `to_info()` and test helpers (`make_blueprint()`, `minimal_valid_blueprint()`, `minimal_valid_task()`, etc.).
- **Traits**: New methods on `SetupTask` or `CommandExecutor` MUST have default implementations to avoid breaking existing implementors.
- **Commands**: New `#[tauri::command]` functions must be registered in the `invoke_handler` in `src-tauri/src/lib.rs`. Every command must validate its inputs and propagate errors via `Result<T, AppError>`.
- **Error handling**: Never use `let _ =` on `Result` for config saves, vault operations, or state mutations. Use `?` or explicit error handling. `let _ =` is acceptable only for event emissions and best-effort logging.
- **Security**: Shell arguments from user input MUST be escaped. Use `shell_escape_value()` pattern from ScriptTask. File paths from user input must be validated against path traversal.
- **Concurrency**: When acquiring multiple `Mutex` locks, always acquire them in the same order to prevent deadlocks. Prefer holding locks for the minimum duration necessary.

#### 3b. Frontend Changes (TypeScript/React)

When modifying frontend code, follow these rules:

- **Types**: New types go in the appropriate file under `src/types/`. All fields from backend must be represented. Optional fields use `?`. Never use `any`.
- **IPC wrappers**: New backend commands need typed wrappers in `src/services/tauriCommands.ts`. Parameter names in the `invoke()` call must use camelCase (Tauri auto-converts to snake_case).
- **Stores**: Follow the existing Zustand 5 pattern — one store per domain, independent stores, actions as top-level functions in the store.
- **Components**: Organize by feature domain under `src/components/`. Atomic UI primitives go in `src/components/ui/`. Always handle loading, error, and empty states. Support both light and dark themes using the existing CSS custom property / `dark:` class pattern.
- **Pages**: Use the same layout patterns as existing pages (see `BlueprintDetailPage.tsx` and `NodeDetailPage.tsx` for detail page patterns, `TaskLibraryPage.tsx` for list page patterns). Always include back navigation for detail pages.
- **Events**: Every `useTauriEvent<T>` type parameter must exactly match the backend `app.emit()` payload shape. Cross-reference with `src/types/events.ts`.
- **Cleanup**: `useEffect` hooks that set up listeners or timers MUST return cleanup functions.

#### 3c. Writing Tests

Tests are not optional. Write them as you implement, not as an afterthought.

**Backend tests** (`#[test]` or `#[tokio::test]`):
- Unit tests go in `#[cfg(test)] mod tests` at the bottom of each file
- Use `tempfile` crate for filesystem isolation — never write to real config directories
- Test helpers: use existing `make_definition()`, `make_blueprint()`, `minimal_valid_blueprint()`, `minimal_valid_task()` patterns
- Test the happy path AND at least one error/edge case per function
- For new struct fields, test that they serialize/deserialize correctly and appear in `to_info()` output
- Integration tests go in `src-tauri/tests/` when testing cross-module behavior

**Frontend tests** (Vitest):
- Store tests go in `src/__tests__/stores/`
- Use the existing mock setup from `src/__mocks__/` — Tauri `invoke` is mocked
- Test state transitions, not implementation details
- Update existing test helpers (`mockTask`, `makeBlueprint`) when adding fields to shared types

#### 3d. Verify Each Step

After every file edit:
1. **Read back the changed lines** to confirm the edit landed correctly on disk
2. If the change is a Rust file, run `cargo check` from `src-tauri/` to catch compile errors early (but be aware of file lock contention if running parallel agents)
3. If the change is a TypeScript file, watch for import errors

Do NOT move to the next task until the current one compiles cleanly.

### Phase 4: Full Verification

After all implementation tasks are complete, run the full verification suite:

```bash
cd src-tauri && cargo test          # All backend tests pass
cd .. && npm run test               # All frontend tests pass
npm run build                       # TypeScript compiles, Vite builds
```

If any check fails:
1. Read the error carefully
2. Fix the root cause (don't suppress warnings or skip tests)
3. Re-run the failing check
4. Re-run the full suite to catch regressions

**All three checks must pass before the feature is considered complete.**

### Phase 5: Self-Review

Before declaring the feature done, do a brief self-review:

1. **Contract tracing**: For any new enums, types, or constraint fields — verify they are enforced at every call site, not just declared.
2. **Cross-boundary check**: For any new `app.emit()` events or `invoke()` calls — verify the payload/parameter shapes match between backend and frontend.
3. **Error propagation**: Scan your changes for `let _ =` on `Result` — justify each one.
4. **Completeness**: Re-read the original requirement. Does the implementation cover all requested behavior? Are there edge cases not addressed?
5. **No dead code**: Remove unused imports, variables, and functions introduced during development.

## Repository-Specific Pitfalls

These are hard-won lessons from this codebase. Violating any of these will likely cause bugs:

- **Glob tool is unreliable**: Never trust a negative Glob result as proof a file doesn't exist. Always verify with `Read` or `ls`.
- **Shared struct constructors**: When adding fields to `TaskInfo`, `Blueprint`, `Node`, or `TaskDefinition`, you MUST update every constructor and test helper. Grep for the struct name to find them all.
- **`to_info()` must include new fields**: If you add a field to `TaskInfo` and a corresponding method to `SetupTask`, you must also add it to the `to_info()` method on the trait.
- **lib.rs command registration**: Every new `#[tauri::command]` must be added to the `invoke_handler` macro in `src-tauri/src/lib.rs`. Missing this will cause a runtime "command not found" error with no compile-time warning.
- **Frontend type alignment**: TypeScript types in `src/types/` must mirror the Rust serialized types field-for-field. Rust uses `snake_case`; TypeScript should too (Tauri preserves field names as-is in serialized structs).
- **`cargo test` runs from `src-tauri/`**: There is no `Cargo.toml` at the project root.
- **Test helpers to update**: `make_blueprint()` in `manager.rs`, `minimal_valid_blueprint()` in `validate.rs`, `minimal_valid_task()` in `tasks/validate.rs`, `mockTask` in `moduleStore.test.ts`, `makeBlueprint` in `blueprintStore.test.ts`.
- **All call sites, not just the first**: When implementing a feature that touches `execute_task`, also check `apply_blueprint` and `apply_blueprint_bulk` — they share the same execution path.
- **Verify edits landed**: After editing a file, always `Read` the changed lines back. Context compaction can cause you to lose track of what's actually on disk.
