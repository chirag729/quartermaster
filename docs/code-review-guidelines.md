# Code Review Guidelines

These guidelines define **multiple complementary review strategies** that must be used together. No single strategy catches everything — each targets a different class of bug.

## Strategy 1: Contract Enforcement Tracing

**What it catches:** Declared constraints that are never enforced at runtime.

For every enum, type, or field that represents a constraint or invariant:

1. **Find the declaration** — e.g. `ExecutionTarget::LocalOnly`, `PrivilegeLevel::Admin`, `enabled: bool`
2. **Trace every call site** that should check it — follow the data from definition to the function that acts on it
3. **Verify the check exists** — if a task declares `LocalOnly`, does `execute_task` actually reject it when the target is a remote node?

Questions to ask:
- "This field exists. Where is it read? Is the read result used to gate behavior?"
- "If I set this to a restrictive value, does anything actually block the invalid case?"
- "Is there a code path that bypasses the check entirely?"

Common violations:
- Enums with variants that are never matched against in control flow
- Validation that runs at creation time but not at execution time
- Fields serialized to the frontend for display but never enforced on the backend

## Strategy 2: Cross-Boundary Payload Verification

**What it catches:** Frontend/backend event shape mismatches, IPC type drift.

For every event emitted by the backend (`app.emit()`):

1. **Find the emit site** — note the event name and the JSON payload shape
2. **Find the frontend listener** — `useTauriEvent<T>(event_name, ...)` or `listen()`
3. **Compare the type parameter `T`** against the actual emitted payload field-by-field
4. **Check the type definition** in `src/types/events.ts` — does it match both sides?

For every `invoke()` call:

1. **Compare the TypeScript parameter names** against the Rust `#[tauri::command]` function signature
2. **Remember**: Tauri 2 auto-converts camelCase (JS) to snake_case (Rust) — this is NOT a mismatch
3. **Compare return types** — the TypeScript `Promise<T>` must match the Rust `Result<T, AppError>` inner type

## Strategy 3: Error Propagation Audit

**What it catches:** Silently swallowed errors that hide failures from users.

Search for these patterns and evaluate each one:

- `let _ = expr;` where `expr` returns `Result` — is the error intentionally ignored or accidentally lost?
- `.ok()` / `.ok().flatten()` — is this converting a meaningful error into `None`?
- `.unwrap_or_default()` — is the default actually safe, or does it mask a problem?
- `if let Ok(x) = expr` — what happens in the `Err` case? Is it silently dropped?

Decision framework:
- **Intentional ignore is OK** for: event emissions (`app.emit`), cleanup operations, best-effort logging
- **Must propagate** for: config persistence, state mutations, security operations (vault, auth)
- **Must at least log** for: background operations where propagation isn't possible

## Strategy 4: Data Flow Completeness

**What it catches:** Features that are partially wired — backend exists but frontend doesn't use it, or vice versa.

For each major feature, trace the full path:

1. **Backend capability** — does the Rust code support it?
2. **Command exposure** — is there a `#[tauri::command]` that exposes it?
3. **IPC wrapper** — is there a function in `tauriCommands.ts`?
4. **Frontend usage** — does a component/page actually call it?
5. **User-facing UI** — can the user trigger it?

Also check the reverse: if the frontend assumes a capability (calls an IPC function, listens for an event), does the backend actually provide it?

## Strategy 5: State Consistency Under Failure

**What it catches:** Partial state updates that leave the system inconsistent when something fails mid-operation.

For multi-step operations (blueprint apply, bulk operations, task execution):

1. **Identify the steps** — what happens in sequence?
2. **For each step, ask**: "If this step fails, is the state from previous steps still consistent?"
3. **Check cleanup** — does the error path undo partial work, or leave it?
4. **Check reporting** — does the user see the partial success, or just a generic error?

Common violations:
- Config saved after step 3 of 5, then step 4 fails — config now reflects partial state
- Loop processes 10 items, item 7 fails, items 1-6 are committed but 8-10 are skipped without notice

## Strategy 6: Security Boundary Review

**What it catches:** Privilege escalation, credential leaks, injection vectors.

For each input that crosses a trust boundary:

1. **User input → shell command**: Is it sanitized? Could it inject?
2. **User input → file path**: Can it escape the expected directory (path traversal)?
3. **Credential handling**: Are secrets logged? Serialized? Sent to the frontend?
4. **Privilege operations**: Does the code verify authorization before acting?

For this project specifically:
- Vault passwords must never appear in logs, events, or error messages
- All privileged operations must go through PolicyKit (`pkexec`)
- SSH keys: private keys must never be read or transmitted; only public keys
- Desktop entry fields (especially `Exec`) must be escaped per freedesktop spec

## Review Execution Checklist

When conducting a code review, run **all six strategies**. Do not rely on a single pass.

1. [ ] **Contract tracing**: List all constraint types/enums → verify enforcement at every action site
2. [ ] **Payload verification**: List all `app.emit()` calls → match against frontend listeners and types
3. [ ] **Error audit**: Search for `let _ =`, `.ok()`, `.unwrap_or_default()` → classify each as intentional or bug
4. [ ] **Data flow**: For each feature, trace backend → command → IPC → frontend → UI
5. [ ] **Failure consistency**: For each multi-step operation, simulate failure at each step
6. [ ] **Security boundaries**: For each trust boundary crossing, verify sanitization and authorization

## Anti-Patterns to Avoid During Review

- **Module-only review**: Reviewing files in isolation misses cross-module contract violations. Always trace across boundaries.
- **Existence-as-enforcement**: Seeing that a constraint type exists (e.g., `ExecutionTarget`) does NOT mean it's enforced. Verify the check, not just the declaration.
- **False positive over-filtering**: When consolidating findings, verify each dismissal with concrete evidence. "It's probably handled somewhere" is not evidence.
- **Local-only bug focus**: Bugs like wrong types or missing error handling are easy to spot. Contract violations and payload mismatches require cross-file tracing and are more commonly missed.
- **Single-call-site fix**: When fixing a contract violation, grep for ALL call sites. For example, `task.execute()` is called from `execute_task`, `apply_blueprint`, AND `apply_blueprint_bulk`. Fixing only the first is an incomplete fix.
- **Trusting tool results without cross-checking**: If a search tool says a file doesn't exist but a reviewer cites it with specific line numbers, read the file directly. Tool failures are more likely than reviewer hallucinations.
- **Docs-don't-exist dismissal**: This project has real documentation in `docs/user-guide/` and `docs/development/`. Always verify cited documentation paths by reading them.

## Fix Verification Protocol

After applying fixes, follow this protocol before declaring them complete:

1. **Re-read every modified file** at the changed lines to confirm edits are present on disk.
2. **Run the compiler** (`cargo check`, `npx tsc --noEmit`) — compilation success does not prove correctness, but failure proves incorrectness.
3. **Run tests** (`cargo test`, `npm run test`) — passing tests confirm no regressions.
4. **Grep for all call sites** of the fixed function/pattern to ensure the fix covers every path, not just the first one found.
5. **Compare against the original finding** — re-read the exact file:line the reviewer cited to confirm the issue no longer exists at that location.
