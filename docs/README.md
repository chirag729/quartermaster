# Quartermaster

Quartermaster is a Tauri 2 desktop application for provisioning and managing local and remote Linux machines. It provides a graphical interface for automating development environment setup, executing tasks across a fleet of nodes, and managing AppArmor security profiles.

## Core Concepts

| Concept | Name | Description |
|---------|------|-------------|
| Application | **Quartermaster** | The desktop app itself |
| Machine list | **Fleet** | The collection of all managed machines |
| Individual machine | **Node** | A single local or remote Linux machine |
| Machine profile | **Blueprint** | A reusable template defining which tasks to run on a node |
| Setup item | **Task** | A discrete unit of work (install Docker, create a folder, etc.) |

## Tech Stack

| Layer | Technologies |
|-------|-------------|
| Frontend | React 18, TypeScript, Tailwind CSS 4, Zustand 5, React Router 7 (HashRouter), Motion 12, Lucide React icons |
| Backend | Rust, Tauri 2, Tokio async runtime |
| Packaging | `.deb` and `.appimage` for Linux |

## Key Features

**Fleet Management** -- Add and manage local and remote Linux machines (nodes). The local machine is auto-detected. Each node displays online/offline status.

**Blueprint System** -- Reusable configuration templates that bundle tasks together. Ships with built-in blueprints: Development Workstation, Mobile Development, and Minimal Server. Custom blueprints can be created and saved.

**Task Execution** -- Seven built-in tasks: create folders, Flutter SDK, Android SDK, IntelliJ IDEA, Claude Code, Git SSH. Tasks detect existing state and skip work that is already completed.

**SSH Remote Execution** -- Execute tasks on remote machines over SSH. Supports key file, certificate, FIDO2/YubiKey, and agent authentication methods.

**AppArmor Management** -- Monitor denial logs in real-time, receive rule suggestions, apply rules with risk assessment, and manage security profiles.

**Command Palette** -- Press `Ctrl+K` for quick navigation to pages and actions.

**Dark/Light Theme** -- System-aware theming with manual override.

## Quick Start

```bash
# Install dependencies
npm install

# Development
npm run dev          # Start dev server + Tauri

# Testing
npm run test         # Frontend tests (Vitest)
cd src-tauri && cargo test  # Rust tests

# Build
npm run tauri build  # Full desktop app build (.deb, .appimage)
```

## Routes

```
/fleet              Fleet Overview (landing page)
/fleet/:nodeId      Node Detail
/tasks              Task Library
/blueprints         Blueprints
/blueprints/:id     Blueprint Detail / Editor
/apparmor           AppArmor Dashboard
/settings           Settings
```

## Configuration

All persistent data is stored at `~/.config/quartermaster/`:

| Path | Purpose |
|------|---------|
| `config.json` | App settings and task configurations |
| `nodes/` | Fleet node JSON files |
| `blueprints/` | Blueprint JSON files |

## Documentation

- **[User Guide](user-guide/)** -- Usage instructions, feature walkthroughs, and troubleshooting.
- **[Development](development/)** -- Architecture overview, adding new tasks and blueprints, API reference, and testing guide.
