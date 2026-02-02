# Development Setup

## Prerequisites

### System dependencies

Anvil is a Tauri 2 application. On Ubuntu/Debian, install the following system packages:

```bash
sudo apt update
sudo apt install -y \
  libwebkitgtk-6.0-dev \
  libgtk-4-dev \
  libadwaita-1-dev \
  build-essential \
  curl \
  wget \
  file \
  libssl-dev \
  libayatana-appindicator3-dev \
  librsvg2-dev \
  javascriptcoregtk-6.0 \
  libsoup-3.0-dev
```

### Node.js

Node.js 18 or later is required for the frontend toolchain. Install via your preferred method (nvm, package manager, etc.).

### Rust toolchain

Install Rust via rustup:

```bash
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
```

Ensure `cargo` and `rustc` are on your PATH after installation.

## Clone and install

```bash
git clone <repository-url>
cd anvil
npm install
```

This installs all frontend dependencies (React, Vite, Tailwind CSS, Zustand, etc.) and the Tauri CLI.

## Development

Start the development server:

```bash
npm run dev
```

This command:

1. Starts the Vite dev server on **http://localhost:1420** (strict port, required by Tauri).
2. Launches the Tauri development window, which loads the Vite dev server URL.
3. Enables hot module replacement (HMR) over WebSocket for instant frontend updates.

The Rust backend recompiles automatically when source files in `src-tauri/` change.

### Path alias

The project uses a path alias so that imports from the `src/` directory can use `@/` as a prefix:

```
@/* --> ./src/*
```

For example: `import { Button } from '@/components/ui/Button'`

This alias is configured in both `vite.config.ts` and `tsconfig.json`.

## Testing

### Frontend tests

Frontend tests use Vitest with a jsdom environment:

```bash
npm run test          # Run all frontend tests once
npm run test:watch    # Run in watch mode (re-runs on file change)
```

Tauri APIs are mocked in `src/__mocks__/` so tests can run without the Tauri runtime.

### Backend tests

Rust tests use the standard `cargo test` runner:

```bash
cd src-tauri && cargo test
```

Backend tests use the `tempfile` crate for filesystem isolation, so they do not write to your real config directories.

## Building

### TypeScript check + Vite production build

```bash
npm run build
```

This runs the TypeScript compiler for type checking, then builds an optimized production bundle via Vite into `dist/`.

### Full desktop app build

```bash
npm run tauri build
```

This compiles the Rust backend in release mode, bundles the Vite production output, and produces distributable packages:

- **.deb** -- Debian package, includes a postinst script that installs the PolicyKit policy and AppArmor helper script.
- **.appimage** -- Self-contained portable binary for Linux.

Output artifacts are placed in `src-tauri/target/release/bundle/`.

## Tauri dev URL

The Tauri dev configuration expects the frontend to be served at:

```
http://localhost:1420
```

This port is configured as strict in `vite.config.ts`. If port 1420 is already in use, the dev server will fail rather than pick a different port, because Tauri must know the exact URL to connect to.
