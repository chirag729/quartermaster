# Contributing to Quartermaster

## Development Setup

### Prerequisites

- **Node.js** 18+
- **Rust** 1.75+ (install via [rustup](https://rustup.rs/))
- **System dependencies** (Ubuntu/Debian):

```bash
sudo apt install libwebkit2gtk-4.1-dev libgtk-3-dev libappindicator3-dev \
  librsvg2-dev patchelf libssl-dev libayatana-appindicator3-dev
```

### Step by Step

1. Clone the repository:
   ```bash
   git clone <repo-url>
   cd quartermaster
   ```

2. Install frontend dependencies:
   ```bash
   npm install
   ```

3. Start the development server:
   ```bash
   npm run dev
   ```
   This launches the Vite dev server (port 1420) and the Tauri window with hot reload.

## Architecture Overview

Quartermaster is a Tauri 2 desktop application with a clear frontend/backend split:

- **Frontend** (`src/`): React 18 + TypeScript + Tailwind CSS 4. Components are organized by feature domain (fleet, blueprints, apparmor, ssh, etc.). State is managed via independent Zustand stores -- one per domain.
- **Backend** (`src-tauri/src/`): Rust with Tokio async runtime. Handles system operations, SSH connections (via russh), file management, encryption, and PolicyKit privilege escalation.
- **IPC**: The frontend calls Rust functions via Tauri `invoke()`. Long-running backend operations emit progress events that the frontend subscribes to via the `useTauriEvent` hook.

### Key Patterns

- **CommandExecutor trait**: All system operations go through `CommandExecutor`, which has `LocalExecutor` and `SshExecutor` implementations. This allows the same task code to run locally or on remote machines.
- **SetupTask trait**: Each automated setup task (Flutter SDK, Android SDK, etc.) implements `SetupTask` and is registered in a central registry.
- **Error handling**: Errors are serialized as `AppError` variants from Rust to the frontend, where they are displayed as toast notifications.

## Development Workflow

### Running the App

```bash
npm run dev           # Start dev server + Tauri window
```

### Type Checking

TypeScript strict mode is enforced. All code must pass:

```bash
npx tsc --noEmit
```

### Rust Checks

```bash
cd src-tauri
cargo clippy          # Lint check
cargo test            # Run backend tests
```

### Frontend Tests

```bash
npm run test          # Run once (Vitest)
npm run test:watch    # Watch mode
```

Frontend tests use jsdom and mock Tauri APIs via `src/__mocks__/`. Store tests live in `src/__tests__/stores/`.

## Code Conventions

### Frontend

- React components are organized by feature domain in `src/components/` (e.g., `fleet/`, `blueprints/`, `apparmor/`, `ssh/`).
- Atomic UI primitives (buttons, cards, inputs) live in `src/components/ui/`.
- Each Zustand store manages a single domain and is independent of other stores.
- All Tauri IPC calls are wrapped in `src/services/tauriCommands.ts`.
- Path alias: `@/*` maps to `./src/*`.
- Dark mode is implemented via CSS custom properties and the `dark:` Tailwind prefix, toggled by a `dark` class on `<html>`.

### Backend

- Tasks implement the `SetupTask` trait and use `CommandExecutor` for all system operations.
- All privileged operations use PolicyKit (`pkexec`). Never run commands as root directly.
- Long-running operations emit progress events via `app.emit()`.
- Backend state is managed with `Arc<Mutex<T>>` and registered as Tauri managed state.
- Tests use the `tempfile` crate for filesystem isolation.

## Adding a New Task

1. Create `src-tauri/src/tasks/your_task.rs` implementing `SetupTask`.
2. Add `pub mod your_task;` to `src-tauri/src/tasks/mod.rs`.
3. Register it in `create_registry()` in `src-tauri/src/tasks/registry.rs`.
4. The frontend picks it up automatically via `list_tasks` / `detect_all_states`.

## Pull Request Process

### Branch Naming

Use descriptive branch names with a prefix:

- `feature/` - new features (e.g., `feature/docker-task`)
- `fix/` - bug fixes (e.g., `fix/ssh-timeout-handling`)
- `refactor/` - code refactoring
- `docs/` - documentation changes

### Commit Messages

Write clear, concise commit messages:

- Use the imperative mood ("Add feature" not "Added feature")
- Keep the subject line under 72 characters
- Reference issue numbers where applicable

### Review Checklist

Before submitting a PR, verify:

- [ ] `npx tsc --noEmit` passes with no errors
- [ ] `npm run test` passes all frontend tests
- [ ] `cd src-tauri && cargo test` passes all backend tests
- [ ] `cd src-tauri && cargo clippy` reports no warnings
- [ ] New tasks follow the `SetupTask` pattern and are registered in the registry
- [ ] Privileged operations use PolicyKit, not direct root access
- [ ] New UI components follow the existing feature-domain organization
- [ ] Dark mode is supported for any new UI elements
