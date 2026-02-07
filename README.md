# Quartermaster

A desktop application for provisioning and managing Linux development environments.

## Features

- **Fleet management** - manage local and remote Linux machines from a unified dashboard
- **Blueprint system** - reusable setup configurations (task groups with config overrides)
- **Task library** - automated setup tasks (Flutter SDK, Android SDK, IntelliJ IDEA, Claude Code, Git SSH, etc.)
- **SSH management** - agent, key file, certificate, password, and FIDO2 resident key authentication
- **YubiKey/FIDO2 integration** - detect keys, manage credentials, generate SSH keys
- **AppArmor management** - profile viewing, log monitoring, rule generation
- **Encrypted vault** - AES-256-GCM encrypted secret storage
- **Shared variables** - template variables for config portability
- **Status monitoring** - real-time node health polling
- **Dark/light theme** - system-aware appearance

## Tech Stack

| Layer    | Technologies                                          |
|----------|-------------------------------------------------------|
| Frontend | React 18, TypeScript, Tailwind CSS 4, Zustand 5      |
| Backend  | Rust, Tauri 2, Tokio async runtime, russh 0.46 (SSH)  |
| Build    | Vite 6, Cargo, tauri-cli                              |

## Prerequisites

- **Node.js** 18+
- **Rust** 1.75+ (install via [rustup](https://rustup.rs/))
- **System dependencies** (Ubuntu/Debian):

```bash
sudo apt install libwebkit2gtk-4.1-dev libgtk-3-dev libappindicator3-dev \
  librsvg2-dev patchelf libssl-dev libayatana-appindicator3-dev
```

## Quick Start

```bash
git clone <repo-url>
cd quartermaster
npm install
npm run dev
```

This starts the Vite dev server on port 1420 and launches the Tauri window with hot reload.

## Build

```bash
npm run build          # TypeScript check + Vite production build
npm run tauri build    # Full desktop app (.deb, .appimage)
```

Build artifacts are output to `src-tauri/target/release/bundle/`.

## Testing

```bash
npm run test                    # Frontend tests (Vitest)
npm run test:watch              # Frontend tests in watch mode
cd src-tauri && cargo test      # Backend tests (272 tests)
```

## Architecture

- **Frontend** (`src/`): React + TypeScript + Tailwind CSS, organized by feature domain. State management via independent Zustand stores. Routing with React Router 7 (HashRouter).
- **Backend** (`src-tauri/src/`): Rust + Tauri 2 + Tokio async runtime. SSH connections via russh. Encryption via aes-gcm and argon2.
- **IPC**: Frontend calls backend via Tauri `invoke()` (`@tauri-apps/api/core`). Backend emits progress events via `app.emit()`.
- **State**: Zustand stores on the frontend (one per domain), `Arc<Mutex<T>>` managed state on the backend.

## Project Structure

```
quartermaster/
  src/                          # Frontend (React + TypeScript)
    components/
      apparmor/                 # AppArmor profile management UI
      blueprints/               # Blueprint editor and list
      dashboard/                # Main dashboard
      fleet/                    # Fleet/node management
      layout/                   # App shell, sidebar, header
      modules/                  # Task module components
      ssh/                      # SSH connection and key management
      ui/                       # Atomic UI primitives (buttons, cards, etc.)
    hooks/                      # Custom React hooks (useTasks, useAppArmor, useTauriEvent, useTheme)
    pages/                      # Route page components
    services/tauriCommands.ts   # All Tauri IPC invoke wrappers
    stores/                     # Zustand stores (task, fleet, blueprint, apparmor, theme, toast)
    types/                      # TypeScript type definitions
  src-tauri/                    # Backend (Rust + Tauri 2)
    src/
      apparmor/                 # Log parsing, rule generation, profile management
      blueprints/               # Blueprint model, manager, defaults
      commands/                 # Tauri command handlers (IPC entry points)
      config/                   # ConfigManager - persistent config (~/.config/quartermaster/)
      executor/                 # CommandExecutor trait, LocalExecutor, SshExecutor
      fleet/                    # Node model + FleetManager
      polkit/                   # PolicyKit authorization and privilege escalation
      tasks/                    # SetupTask trait + task implementations
      variables/                # Shared template variables
      vault/                    # Encrypted secret storage (AES-256-GCM)
```

## Configuration

Application data is stored at `~/.config/quartermaster/`:

- `config.json` - application settings
- `nodes/` - saved node definitions
- `blueprints/` - saved blueprint definitions

## License

MIT
