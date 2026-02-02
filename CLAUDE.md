# CLAUDE.md

## Project

Anvil - a Tauri 2 desktop app (React + Rust) for provisioning and managing local and remote Linux machines, automating development environment setup, and managing AppArmor profiles.

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
- **State**: Zustand stores (frontend), `Arc<Mutex<T>>` managed state (backend)

## Key Directories

| Path | Contains |
|------|----------|
| `src/components/` | React components organized by feature (apparmor/, blueprints/, dashboard/, fleet/, layout/, modules/, ssh/, ui/) |
| `src/stores/` | Zustand stores: taskStore, fleetStore, blueprintStore, appArmorStore, themeStore, toastStore |
| `src/hooks/` | Custom hooks: useTasks, useAppArmor, useTauriEvent, useTheme |
| `src/services/tauriCommands.ts` | All Tauri IPC invoke wrappers |
| `src/types/` | TypeScript type definitions (task, node, blueprint, apparmor, config, events) |
| `src-tauri/src/commands/` | Tauri command handlers (tasks, fleet, blueprints, ssh, apparmor, config, system) |
| `src-tauri/src/tasks/` | SetupTask trait + task implementations (create_folder, flutter_sdk, android_sdk, intellij, claude_code, git_ssh) |
| `src-tauri/src/executor/` | CommandExecutor trait, LocalExecutor, SshExecutor |
| `src-tauri/src/fleet/` | Node model + FleetManager |
| `src-tauri/src/blueprints/` | Blueprint model, manager, defaults |
| `src-tauri/src/apparmor/` | Log parsing, rule generation, profile management, real-time monitoring |
| `src-tauri/src/polkit/` | PolicyKit authorization and privilege escalation |
| `src-tauri/src/config/` | ConfigManager - persistent config at ~/.config/anvil/ |

## Conventions

- Frontend components are organized by feature domain, with atomic UI primitives in `src/components/ui/`
- Each Zustand store manages a single domain; stores are independent
- Backend tasks implement the `SetupTask` trait and are registered in `tasks/registry.rs`
- Tasks use `CommandExecutor` trait for all system operations, enabling local and remote execution
- All privileged operations use PolicyKit (`pkexec`), never direct root
- Long-running backend operations emit progress events; frontend subscribes via `useTauriEvent` hook
- Errors flow as serialized `AppError` variants from Rust to frontend, displayed as toasts
- Dark mode uses CSS custom properties + `dark` class on `<html>`
- Path alias: `@/*` maps to `./src/*`

## Adding a New Task

1. Create `src-tauri/src/tasks/your_task.rs` implementing `SetupTask`
2. Add `pub mod your_task;` to `src-tauri/src/tasks/mod.rs`
3. Register it in `create_registry()` in `src-tauri/src/tasks/registry.rs`
4. The frontend picks it up automatically via `list_tasks` / `detect_all_states`

## Testing

- **Frontend mocks**: Tauri APIs are mocked in `src/__mocks__/` (invoke, events, matchMedia)
- **Frontend tests**: Store tests in `src/__tests__/stores/`; run with `npm run test` (16 tests)
- **Backend tests**: Inline `#[test]` functions; uses `tempfile` crate for filesystem isolation (59 tests)

## Config

- Tauri config: `src-tauri/tauri.conf.json`
- Vite config: `vite.config.ts` (dev port 1420, path aliases)
- TypeScript: `tsconfig.json` (strict, ES2020 target)
- Vitest: `vitest.config.ts` (jsdom environment, setup file at `src/__tests__/setup.ts`)
- App data: `~/.config/anvil/` (config.json, nodes/, blueprints/)
