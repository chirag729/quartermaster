# CLAUDE.md

## Project

Quartermaster - a Tauri 2 desktop app (React + Rust) for provisioning and managing local and remote Linux machines, automating development environment setup, and managing AppArmor profiles.

## Quick Reference

```bash
npm run dev          # Start dev server + Tauri
npm run build        # TypeScript check + Vite production build
npm run tauri build  # Full desktop app build (.deb, .appimage)
npm run test         # Run frontend tests (Vitest)
npm run test:watch   # Frontend tests in watch mode
cd src-tauri && cargo test  # Run Rust unit tests
```

## Architecture

- **Frontend**: `src/` - React 18, TypeScript, Tailwind CSS 4, Zustand 5, React Router 7 (HashRouter)
- **Backend**: `src-tauri/src/` - Rust, Tauri 2, Tokio async runtime, russh 0.46 (SSH)
- **IPC**: Frontend calls backend via `invoke()` (`@tauri-apps/api/core`); backend emits events via `app.emit()`
- **State**: Zustand stores (frontend), `Arc<Mutex<T>>` managed state in `AppState` (backend)
- **Security**: PolicyKit for privilege escalation, encrypted vault (Argon2id + AES-256-GCM), AppArmor profile management, FIDO2/YubiKey SSH support

## Key Directories

| Path | Contains |
|------|----------|
| `src/components/` | React components by feature (apparmor/, blueprints/, dashboard/, fleet/, layout/, modules/, ssh/, ui/) |
| `src/pages/` | 8 page components: Dashboard, Fleet, NodeDetail, Blueprints, BlueprintDetail, AppArmor, TaskLibrary, Settings |
| `src/stores/` | Zustand stores: taskStore, fleetStore, blueprintStore, appArmorStore, themeStore, toastStore |
| `src/hooks/` | Custom hooks: useTasks, useAppArmor, useTauriEvent, useTheme, useDebounce |
| `src/services/tauriCommands.ts` | All Tauri IPC invoke wrappers (typed) |
| `src/types/` | TypeScript type definitions (task, node, blueprint, apparmor, config, events) |
| `src-tauri/src/commands/` | Tauri command handlers (tasks, fleet, blueprints, ssh, apparmor, config, system, variables, vault, yubikey, activity) |
| `src-tauri/src/tasks/` | SetupTask trait, ScriptTask (YAML-based), TaskRegistry with dependency resolution |
| `src-tauri/src/executor/` | CommandExecutor trait: LocalExecutor, SshExecutor, DryRunExecutor |
| `src-tauri/src/fleet/` | Node model, FleetManager, SSH config parser, status poller, terminal launcher |
| `src-tauri/src/blueprints/` | Blueprint model, manager, validation, YAML schema, `.qmbp` packaging |
| `src-tauri/src/apparmor/` | Log parsing, rule generation/consolidation, profile management, real-time monitoring, templates, tunable installer |
| `src-tauri/src/vault/` | Encrypted credential vault (Argon2id KDF, AES-256-GCM) |
| `src-tauri/src/variables/` | Variable resolution with 4-layer precedence (task defaults → blueprint → user → node overrides) |
| `src-tauri/src/polkit/` | PolicyKit authorization and privilege escalation |
| `src-tauri/src/config/` | ConfigManager - persistent config at ~/.config/quartermaster/ |
| `docs/AppArmor Profiles/` | AppArmor profiles (intellij, claude-code, codex-cli, flutter) + shared abstractions/ |

## Key Features

### Blueprint System
- **YAML-defined blueprints** with ordered task entries and config overrides
- **Inheritance**: Blueprints can `extends` a parent; `resolve_task_entries()` merges parent + child
- **Versioning**: Auto-bumps semver (minor for task changes, patch for config changes)
- **Import/Export**: `.qmbp` zip archive format via `package.rs`
- **Dry-run**: `DryRunExecutor` intercepts mutations, previews changes without execution
- **Bulk apply**: Apply blueprints to multiple nodes with per-node progress events

### Task System
- **SetupTask trait**: `detect_state()`, `execute()`, `uninstall()`, `detect_installed_version()`
- **ScriptTask**: YAML-defined tasks with `steps`, `uninstall` steps, `version_detect` command
- **Registry**: Dependency resolution with topological sort (Kahn's algorithm)
- **Execution logging**: Per-step stdout/stderr capture in `~/.local/state/quartermaster/logs/`
- **Auto-update detection**: Compares defined vs installed versions across all tasks

### Fleet Management
- **Local + Remote nodes**: SSH execution with multiple auth methods (password, key file, certificate, FIDO2 resident, agent)
- **SSH config discovery**: Parses `~/.ssh/config` for host import
- **Status polling**: 30-second background TCP connectivity probes
- **Terminal integration**: Detects and launches 8 terminal emulators with SSH args

### Security
- **Encrypted vault**: Master-password-protected storage for SSH passwords and API keys
- **YubiKey/FIDO2**: Hardware key detection, resident credential management, `ed25519-sk` key generation
- **AppArmor**: Denial log monitoring, rule suggestion/consolidation, profile templates, tunable installation
- **PolicyKit**: All privileged operations use `pkexec`, never direct root

### Desktop Integration
- **Notifications**: `tauri-plugin-notification` for blueprint apply/bulk completion
- **Activity log**: 500-entry log at `~/.config/quartermaster/activity_log.json`
- **Collapsible sidebar**: Persistent collapsed state in localStorage, tooltip navigation
- **Command palette**: Keyboard-driven navigation (Cmd+K)

## Conventions

- Frontend components are organized by feature domain, with atomic UI primitives in `src/components/ui/`
- Each Zustand store manages a single domain; stores are independent
- Backend tasks implement the `SetupTask` trait and are registered in `tasks/registry.rs`
- Tasks use `CommandExecutor` trait for all system operations, enabling local and remote execution
- All privileged operations use PolicyKit (`pkexec`), never direct root
- Long-running backend operations emit progress events; frontend subscribes via `useTauriEvent` hook
- Errors flow as serialized `AppError` variants from Rust to frontend, displayed as toasts
- New optional struct fields use `#[serde(default)]` for backward-compatible deserialization
- Dark mode uses CSS custom properties + `dark` class on `<html>`
- Path alias: `@/*` maps to `./src/*`

## Adding a New Task

1. Create `src-tauri/src/tasks/your_task.rs` implementing `SetupTask`
2. Add `pub mod your_task;` to `src-tauri/src/tasks/mod.rs`
3. Register it in `create_registry()` in `src-tauri/src/tasks/registry.rs`
4. The frontend picks it up automatically via `list_tasks` / `detect_all_states`

Alternatively, create a YAML task definition in `~/.config/quartermaster/tasks/` — `ScriptTask` will load it automatically.

## Adding a New Tauri Command

1. Add the `#[tauri::command]` function in the appropriate `src-tauri/src/commands/*.rs` module
2. Register it in the `invoke_handler` in `src-tauri/src/lib.rs`
3. Add the typed IPC wrapper in `src/services/tauriCommands.ts`

## Testing

- **Frontend tests**: 82 tests across 6 store test files; run with `npm run test`
- **Frontend mocks**: Tauri APIs mocked in `src/__mocks__/` (invoke, events, matchMedia)
- **Backend tests**: 299 inline `#[test]` functions; uses `tempfile` crate for filesystem isolation
- **Test helpers**: `make_blueprint()` in manager.rs, `minimal_valid_blueprint()` in validate.rs, `minimal_valid_task()` in tasks/validate.rs, `mockTask` in moduleStore.test.ts, `makeBlueprint` in blueprintStore.test.ts

When adding fields to shared structs (Blueprint, TaskInfo, Node), update ALL constructors including test helpers.

## Code Review

Follow the review guidelines in [`docs/code-review-guidelines.md`](docs/code-review-guidelines.md). Key points:

- **Run all 6 strategies**, not just one. Single-pass reviews miss cross-cutting bugs.
- **Contract tracing is mandatory**: For every constraint type/enum, verify enforcement at every call site — not just that the type exists.
- **Cross-boundary payload verification**: Every `app.emit()` payload must be checked field-by-field against the frontend listener's type parameter and the definition in `src/types/events.ts`.
- **Error propagation audit**: Every `let _ =` on a `Result` must be justified. Config saves, vault operations, and state mutations must not swallow errors.
- **Existence is not enforcement**: Seeing `ExecutionTarget::LocalOnly` declared on a task does NOT mean `execute_task` checks it. Trace the call path.
- **Fix ALL call sites**: When fixing a contract violation, grep for every call site. `task.execute()` is called from `execute_task`, `apply_blueprint`, and `apply_blueprint_bulk` — fixing only one is incomplete.
- **Verify fixes landed**: After editing, re-read the changed lines to confirm edits are on disk. Follow the Fix Verification Protocol in the guidelines doc.
- **Never dismiss cited files as non-existent without reading them**: If a reviewer cites `docs/user-guide/tasks.md:127`, use `Read` on that path. Do not use `Glob` — it is unreliable in this workspace.

## Documentation

User-facing documentation lives in `docs/user-guide/` and must stay in sync with the implementation:

- `tasks.md` — task catalog with privilege/target/config details per task
- `blueprints.md` — blueprint authoring, task entries, assignment workflow
- `fleet-management.md`, `ssh-setup.md`, `apparmor.md`, `settings.md`, `getting-started.md`

Developer documentation lives in `docs/development/`.

When changing task behavior (privilege level, config schema, installation method), update `docs/user-guide/tasks.md` to match.

## Config

- Tauri config: `src-tauri/tauri.conf.json`
- Tauri capabilities: `src-tauri/capabilities/default.json` (permissions for shell, notification, etc.)
- Vite config: `vite.config.ts` (dev port 1420, path aliases)
- TypeScript: `tsconfig.json` (strict, ES2020 target)
- Vitest: `vitest.config.ts` (jsdom environment, setup file at `src/__tests__/setup.ts`)
- App data: `~/.config/quartermaster/` (config.json, nodes/, blueprints/, tasks/, vault.enc, activity_log.json)
- App state: `~/.local/state/quartermaster/logs/` (execution logs)
- App cache: `~/.cache/quartermaster/downloads/` (downloaded files)
