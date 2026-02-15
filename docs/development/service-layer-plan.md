# Monorepo + Service Layer Architecture Plan

## Context

Node status polling lives inside `FleetPage.tsx` — navigate away and polling stops. This revealed a deeper problem: there is no service layer. Polling, event listeners, data fetching, and business logic are scattered across page components and hooks. Additionally, a future CLI version of the app needs to share services, stores, and types with the UI. This plan restructures the frontend into a monorepo with a React-free `core` package (services, stores, types) and a `ui` package (React pages, components, hooks), then introduces a service layer that runs for the app's lifetime.

The architecture follows established patterns: Spacedrive (Tauri + client library), GitButler (Tauri + shared services package), and Hopp (Tauri + Zustand + module-level services). Zustand's `createStore` from `zustand/vanilla` enables React-free stores that work in both UI and future CLI contexts.

---

## Alternatives Considered

### Q: Why a TypeScript core instead of Rust?

The CLI needs feature parity with the GUI: rich data models (nodes, blueprints, tasks), state management (polling, events), and shared business logic (task validation, blueprint versioning). Duplicating this in Rust would slow development. A TypeScript core lets both UI and CLI share it while keeping the Rust backend as the privileged execution layer.

**Considered but not chosen**:
1. **Pure Rust backend + TypeScript frontend**: High Rust learning curve, limits TypeScript developer contributions.
2. **HTTP RPC server**: More complex deployment, not suitable for a local desktop app.
3. **Embedded Rust library in Node.js (NAPI)**: Increases build complexity, limits portability.

### Q: Why an IPC abstraction instead of importing `@tauri-apps/api` directly?

A CLI app cannot run in a Tauri webview — there's no `invoke()` or `listen()`. By programming services against an `IPCClient` interface, the Tauri UI injects its adapter and a future CLI injects its own (e.g., HTTP, stdin/stdout, or direct Rust FFI).

---

## Architecture Principles

1. **Services use `IPCClient` interface**, never import `@tauri-apps/api` directly
2. **Core has no React imports** — verified by grep, enforced by tsconfig (no jsx, no DOM lib)
3. **Core has no UI concerns** — toast/notification is an injected callback, not a store dependency
4. **Services own data lifecycle**: init loading, polling, event subscriptions, mutations
5. **Pages are readers**: read from stores via hooks, call service methods for mutations
6. **Component-scoped events stay in components**: dialog-specific events (e.g., `task-output`) don't move to services
7. **Vault/credential operations stay in UI package**: never shared with CLI (different security model)

---

## Target Directory Structure

```
quartermaster/
├── package.json                    # Workspace root (npm workspaces)
├── tsconfig.base.json              # Shared TS config
├── src-tauri/                      # Rust backend (unchanged)
│   ├── tauri.conf.json             # Points to ../packages/ui/dist
│   └── ...
├── packages/
│   ├── core/                       # NO React imports — shared with future CLI
│   │   ├── package.json
│   │   ├── tsconfig.json
│   │   ├── vitest.config.ts        # Node environment (not jsdom)
│   │   └── src/
│   │       ├── index.ts            # Barrel export
│   │       ├── ipc/
│   │       │   ├── types.ts        # IPCClient interface
│   │       │   └── provider.ts     # getIPCClient() / setIPCClient()
│   │       ├── services/
│   │       │   ├── serviceInit.ts
│   │       │   ├── fleetService.ts
│   │       │   ├── taskService.ts
│   │       │   ├── blueprintService.ts
│   │       │   └── appArmorService.ts
│   │       ├── stores/
│   │       │   ├── fleetStore.ts       # zustand/vanilla
│   │       │   ├── taskStore.ts
│   │       │   ├── blueprintStore.ts
│   │       │   └── appArmorStore.ts
│   │       ├── types/
│   │       │   ├── node.ts, task.ts, blueprint.ts, apparmor.ts, config.ts, events.ts
│   │       └── lib/
│   │           ├── formatError.ts
│   │           └── taskDependencies.ts
│   │
│   └── ui/                         # React app
│       ├── package.json
│       ├── tsconfig.json
│       ├── vite.config.ts
│       ├── vitest.config.ts
│       ├── index.html
│       └── src/
│           ├── main.tsx
│           ├── App.tsx
│           ├── ipc/
│           │   └── tauriAdapter.ts # TauriIPCClient implements IPCClient
│           ├── services/
│           │   └── tauriCommands.ts # Vault + UI-specific IPC wrappers
│           ├── stores/
│           │   ├── toastStore.ts    # UI-only (uses setTimeout)
│           │   └── themeStore.ts    # UI-only (uses window.matchMedia)
│           ├── hooks/
│           │   ├── useStore.ts      # Typed wrappers around vanilla stores
│           │   ├── useTauriEvent.ts
│           │   ├── useDebounce.ts
│           │   └── useTheme.ts
│           ├── pages/
│           ├── components/
│           ├── __mocks__/           # Tauri mocks for tests
│           └── __tests__/
```

### Key differences from original plan

| Change | Reason |
|--------|--------|
| `ipc/types.ts` + `ipc/provider.ts` in core | IPC abstraction for CLI support (H1) |
| `tauriCommands.ts` stays in `packages/ui/` | Vault operations must not be shared (H2) |
| `toastStore.ts` stays in `packages/ui/` | UI concern — uses setTimeout (H5) |
| `themeStore.ts` stays in `packages/ui/` | UI concern — uses window.matchMedia |
| `vitest.config.ts` added to core | Core needs independent test infrastructure (M6) |
| `tauriAdapter.ts` added to UI | Implements IPCClient for Tauri context |

---

## IPC Abstraction Layer

This is the critical design decision that enables CLI support.

### `packages/core/src/ipc/types.ts`
```typescript
export interface IPCClient {
  invoke<T>(command: string, args?: Record<string, unknown>): Promise<T>;
  listen<T>(event: string, handler: (payload: T) => void): Promise<() => void>;
}
```

### `packages/core/src/ipc/provider.ts`
```typescript
let ipcClient: IPCClient | null = null;

export function setIPCClient(client: IPCClient): void {
  ipcClient = client;
}

export function getIPCClient(): IPCClient {
  if (!ipcClient) throw new Error("IPC client not initialized. Call setIPCClient() first.");
  return ipcClient;
}
```

### `packages/ui/src/ipc/tauriAdapter.ts`
```typescript
import { invoke } from "@tauri-apps/api/core";
import { listen as tauriListen } from "@tauri-apps/api/event";
import type { IPCClient } from "@quartermaster/core";

export const tauriIPCClient: IPCClient = {
  invoke: <T>(command: string, args?: Record<string, unknown>) =>
    invoke<T>(command, args),

  listen: <T>(event: string, handler: (payload: T) => void) =>
    tauriListen<T>(event, (e) => handler(e.payload)).then(unlisten => unlisten),
};
```

### Usage in services
```typescript
// packages/core/src/services/fleetService.ts
import { getIPCClient } from "../ipc/provider";

async function loadNodes() {
  const ipc = getIPCClient();
  const nodes = await ipc.invoke<Node[]>("list_nodes");
  fleetStore.getState().setNodes(nodes);
}
```

### Future CLI adapter (not built now, but the interface enables it)
```typescript
// packages/cli/src/ipc/cliAdapter.ts
export const cliIPCClient: IPCClient = {
  invoke: async (command, args) => {
    // HTTP call to a local Tauri sidecar, or direct Rust FFI
  },
  listen: async (event, handler) => {
    // Subscribe to stdout events from backend process
    return () => {}; // no-op unlisten
  },
};
```

---

## Service Interface

All services implement this interface for consistent lifecycle management:

```typescript
export interface Service {
  init(): Promise<void>;
  destroy(): Promise<void>;
}
```

Services own:
- **Polling intervals** (stored internally, cleared in `destroy()`)
- **Event unsubscribe functions** (stored internally, called in `destroy()`)
- **Store mutations** via `store.getState().setXxx()`

Services receive:
- IPC client via `getIPCClient()` (set once at app startup)
- Notification callback via `ServiceConfig` (injected at init)

```typescript
export interface ServiceConfig {
  onNotify: (type: "success" | "error" | "info", title: string, message: string) => void;
}
```

---

## Service Initialization

### `packages/core/src/services/serviceInit.ts`

```typescript
import type { IPCClient } from "../ipc/types";
import type { ServiceConfig } from "./types";
import { setIPCClient } from "../ipc/provider";

let initialized = false;

export async function initializeServices(
  ipcClient: IPCClient,
  config: ServiceConfig
): Promise<void> {
  if (initialized) return; // idempotent

  // 1. Register IPC client (must happen before any service calls)
  setIPCClient(ipcClient);

  // 2. Initialize services SEQUENTIALLY (order matters)
  await fleetService.init(config);    // populates fleetStore
  await taskService.init(config);     // populates taskStore, subscribes to task events
  await blueprintService.init(config); // populates blueprintStore, may read fleetStore
  await appArmorService.init(config); // populates appArmorStore, subscribes to denial events

  initialized = true;
}

export async function destroyServices(): Promise<void> {
  if (!initialized) return;

  // Destroy in reverse order
  await appArmorService.destroy();
  await blueprintService.destroy();
  await taskService.destroy();
  await fleetService.destroy();

  // Clear all store state
  fleetStore.setState({ nodes: [], loading: false /* ... initial state */ });
  taskStore.setState({ tasks: [], loading: false /* ... initial state */ });
  blueprintStore.setState({ blueprints: [], loading: false /* ... initial state */ });
  appArmorStore.setState({ denials: [], profiles: [] /* ... initial state */ });

  initialized = false;
}
```

### Why sequential, not parallel

- FleetService must finish before BlueprintService (blueprints reference nodes)
- TaskService event listeners must be attached before any task-emitting commands run
- Sequential init is ~200ms total (4 API calls) — negligible vs. parallel

### `packages/ui/src/App.tsx` integration

```typescript
import { tauriIPCClient } from "./ipc/tauriAdapter";
import { initializeServices, destroyServices } from "@quartermaster/core";
import { useToastStore } from "./stores/toastStore";

function App() {
  const [servicesReady, setServicesReady] = useState(false);
  const [initError, setInitError] = useState<string | null>(null);
  const addToast = useToastStore((s) => s.addToast);

  useEffect(() => {
    initializeServices(tauriIPCClient, {
      onNotify: (type, title, message) => addToast({ type, title, message }),
    })
      .then(() => setServicesReady(true))
      .catch((err) => setInitError(err.message));

    return () => { destroyServices(); };
  }, []);

  if (initError) return <ErrorScreen message={initError} />;
  if (!servicesReady) return <LoadingScreen />;

  return <RouterProvider ... />;
}
```

Pages render only after `servicesReady` is true, so stores are always populated.

---

## Phase 0: Monorepo Setup

Phase 0 is split into 4 milestones with independent verification checkpoints. Each milestone gets its own commit so failures can be isolated.

### Phase 0a: Workspace Structure

**Steps**:
1. Create `packages/core/` and `packages/ui/` directories
2. Create root `package.json` with `workspaces` field
3. Create `tsconfig.base.json` at root
4. Create `packages/core/package.json`:
   ```json
   {
     "name": "@quartermaster/core",
     "private": true,
     "version": "0.1.0",
     "type": "module",
     "main": "./src/index.ts",
     "types": "./src/index.ts",
     "exports": { ".": "./src/index.ts", "./*": "./src/*" },
     "scripts": { "test": "vitest run" },
     "dependencies": { "zustand": "^5" },
     "devDependencies": { "typescript": "^5.6.3", "vitest": "^4.0.18" }
   }
   ```
   Note: NO `@tauri-apps/api` dependency — core uses the IPCClient interface.
5. Create `packages/ui/package.json`:
   ```json
   {
     "name": "@quartermaster/ui",
     "private": true,
     "version": "0.1.0",
     "type": "module",
     "scripts": { "dev": "vite", "build": "tsc && vite build", "test": "vitest run", "test:watch": "vitest" },
     "dependencies": {
       "@quartermaster/core": "*",
       "@tauri-apps/api": "^2",
       "@tauri-apps/plugin-shell": "^2",
       "clsx": "^2.1.1", "lucide-react": "^0.468", "motion": "^12",
       "react": "^18.3.1", "react-dom": "^18.3.1", "react-router-dom": "^7", "zustand": "^5"
     },
     "devDependencies": { "...all current devDependencies..." }
   }
   ```
6. Create `packages/core/tsconfig.json` — extends base, NO jsx, NO DOM lib
7. Create `packages/ui/tsconfig.json` — extends base, has jsx + DOM

**Verify 0a**:
```bash
npm install                          # Workspace installs without errors
npm list @quartermaster/core         # Shows symlink to packages/core
npm ls @tauri-apps/api               # Single instance (not duplicated)
```

### Phase 0b: IPC Abstraction + File Moves

**Steps**:
1. Create `packages/core/src/ipc/types.ts` — `IPCClient` interface
2. Create `packages/core/src/ipc/provider.ts` — `getIPCClient()` / `setIPCClient()`
3. Move files:

   | From | To |
   |------|-----|
   | `src/stores/fleetStore.ts` | `packages/core/src/stores/fleetStore.ts` |
   | `src/stores/taskStore.ts` | `packages/core/src/stores/taskStore.ts` |
   | `src/stores/blueprintStore.ts` | `packages/core/src/stores/blueprintStore.ts` |
   | `src/stores/appArmorStore.ts` | `packages/core/src/stores/appArmorStore.ts` |
   | `src/types/*.ts` | `packages/core/src/types/*.ts` |
   | `src/lib/*.ts` | `packages/core/src/lib/*.ts` |
   | `src/stores/toastStore.ts` | `packages/ui/src/stores/toastStore.ts` |
   | `src/stores/themeStore.ts` | `packages/ui/src/stores/themeStore.ts` |
   | `src/services/tauriCommands.ts` | `packages/ui/src/services/tauriCommands.ts` |
   | `src/pages/*.tsx` | `packages/ui/src/pages/*.tsx` |
   | `src/components/**/*` | `packages/ui/src/components/**/*` |
   | `src/hooks/*.ts` | `packages/ui/src/hooks/*.ts` |
   | `src/App.tsx`, `src/main.tsx` | `packages/ui/src/` |
   | `src/index.css` | `packages/ui/src/index.css` |
   | `src/__mocks__/*` | `packages/ui/src/__mocks__/` |
   | `src/__tests__/*` | `packages/ui/src/__tests__/` |
   | `index.html` | `packages/ui/index.html` |
   | `vite.config.ts` | `packages/ui/vite.config.ts` |
   | `vitest.config.ts` | `packages/ui/vitest.config.ts` |

4. Create `packages/ui/src/ipc/tauriAdapter.ts` — implements `IPCClient` using `@tauri-apps/api`
5. Create `packages/core/src/index.ts` — barrel exports for stores, types, lib, ipc

**Verify 0b**: Files are in the right place, no orphans in `src/`.

### Phase 0c: Convert Stores + Update Imports

**Steps**:
1. Convert 4 core stores from `create` to `createStore`:

   **Before** (React-dependent):
   ```typescript
   import { create } from "zustand";
   export const useFleetStore = create<FleetState>((set) => ({ ... }));
   ```

   **After** (React-free):
   ```typescript
   import { createStore } from "zustand/vanilla";
   export const fleetStore = createStore<FleetState>()((set) => ({ ... }));
   ```

2. Clean up `blueprintStore.ts` — remove 8 async API methods, remove `import * as api`
   - Fix `assignBlueprint`/`unassignBlueprint` bug first (they don't update stores after API call)
   - These methods move to `blueprintService` in Phase 3

3. Create `packages/ui/src/hooks/useStore.ts` — React wrappers:
   ```typescript
   import { useStore } from "zustand";
   import { fleetStore, taskStore, blueprintStore, appArmorStore } from "@quartermaster/core";

   export function useFleetStore<T>(selector: (state: FleetState) => T): T {
     return useStore(fleetStore, selector);
   }
   // Overload for no-selector (returns full state):
   export function useFleetStore(): FleetState;
   export function useFleetStore<T>(selector?: (state: FleetState) => T) {
     return useStore(fleetStore, selector!);
   }
   // ...one per store
   ```

4. Update ALL imports in moved files:
   - UI files importing core: `@/stores/*` → `@quartermaster/core`
   - UI files importing core: `@/types/*` → `@quartermaster/core`
   - UI files importing core: `@/lib/*` → `@quartermaster/core`
   - UI files importing hooks: `@/hooks/*` → `@/hooks/*` (stays as `@` alias within ui)
   - UI internal: `@/pages/*`, `@/components/*` — stays as `@` alias within ui
   - Core internal: relative paths only (no aliases)
   - Store imports in pages: `import { useFleetStore } from "@/stores/fleetStore"` → `import { useFleetStore } from "@/hooks/useStore"`

5. Update test imports:
   ```typescript
   // OLD
   import { useFleetStore } from "../../stores/fleetStore";
   beforeEach(() => { useFleetStore.setState({ nodes: [] }); });

   // NEW
   import { fleetStore } from "@quartermaster/core";
   beforeEach(() => { fleetStore.setState({ nodes: [] }); });

   // Assertions unchanged:
   expect(fleetStore.getState().nodes).toHaveLength(1);
   ```

**API duality rule** (document this prominently):
- **Services** (in core): use `store.getState().setFoo()` — direct vanilla store access
- **Pages** (in ui): use `const { foo } = useFleetStore()` — React hook wrapper
- **Tests**: use `store.setState()` and `store.getState()` — direct vanilla store access

**Verify 0c**:
```bash
npx tsc --noEmit -p packages/core/   # 0 errors
npx tsc --noEmit -p packages/ui/     # 0 errors
grep -r "from \"react\"" packages/core/src/  # 0 matches
grep -r "from \"zustand\"" packages/core/src/  # Only zustand/vanilla
```

### Phase 0d: Build Configs + Test Infrastructure

**Steps**:
1. Update `packages/ui/vite.config.ts`:
   ```typescript
   resolve: {
     alias: {
       "@": path.resolve(__dirname, "./src"),
       "@quartermaster/core": path.resolve(__dirname, "../core/src"),
     },
   },
   optimizeDeps: {
     exclude: ["@quartermaster/core"],
   },
   ```

2. Update `src-tauri/tauri.conf.json`:
   ```json
   {
     "build": {
       "beforeDevCommand": "npm run dev -w @quartermaster/ui",
       "devUrl": "http://localhost:1420",
       "beforeBuildCommand": "npm run build -w @quartermaster/ui",
       "frontendDist": "../packages/ui/dist"
     }
   }
   ```

3. Update `packages/ui/vitest.config.ts`:
   - Update `@` alias path
   - Update Tauri mock paths
   - Update `setupFiles` path
   - Add `@quartermaster/core` alias to `../core/src`

4. Create `packages/core/vitest.config.ts`:
   ```typescript
   import { defineConfig } from "vitest/config";
   import path from "path";

   export default defineConfig({
     test: {
       environment: "node",  // NOT jsdom — core is platform-agnostic
       globals: true,
     },
   });
   ```

5. Create `packages/core/src/__mocks__/ipc.ts` for core tests:
   ```typescript
   import type { IPCClient } from "../ipc/types";

   export function createMockIPCClient(handlers: Record<string, Function> = {}): IPCClient {
     return {
       invoke: async (command, args) => {
         if (handlers[command]) return handlers[command](args);
         throw new Error(`Unmocked command: ${command}`);
       },
       listen: async () => () => {}, // no-op
     };
   }
   ```

6. Update root `package.json`:
   ```json
   {
     "scripts": {
       "dev": "npm run dev -w @quartermaster/ui",
       "build": "npm run build -w @quartermaster/ui",
       "test": "npm run test -w @quartermaster/core && npm run test -w @quartermaster/ui",
       "test:watch": "npm run test:watch -w @quartermaster/ui",
       "tauri": "tauri"
     }
   }
   ```

7. Add missing event types to `packages/core/src/types/events.ts`:
   ```typescript
   export interface BlueprintTaskWarningEvent {
     node_id: string;
     blueprint_id: string;
     task_id: string;
     warning: string;
   }

   export interface BulkBlueprintProgressEvent {
     completed: number;
     total: number;
     current_node_id: string;
   }
   ```

**Verify 0d** (full integration test):
```bash
npm run test                         # Both core and ui tests pass
npm run dev                          # Vite starts on :1420
npm run tauri dev                    # App launches, all features work
npm run tauri build                  # Full desktop app build succeeds
grep -r "react" packages/core/src/   # 0 matches
grep -r "@tauri-apps/api" packages/core/src/  # 0 matches (uses IPCClient)
```

### Phase 0 Commit Strategy
```
Phase 0a: git commit -m "monorepo: workspace structure and package configs"
Phase 0b: git commit -m "monorepo: IPC abstraction + move files to packages"
Phase 0c: git commit -m "monorepo: vanilla stores, import rewrites, test updates"
Phase 0d: git commit -m "monorepo: build configs, test infra, event types"
```

---

## Import Alias Reference

**packages/ui/** (React app):
- `@/*` → `./src/*` — UI-local files (pages, components, hooks)
- `@quartermaster/core` → `../core/src/*` — shared domain logic, stores, types

**packages/core/** (domain layer):
- No aliases. Use relative imports: `../stores/`, `./lib/`, etc.

**Example correct imports** (in packages/ui):
```typescript
// UI page imports UI components
import { DashboardWidget } from "@/components/dashboard/Widget";

// UI page imports shared store
import { useFleetStore } from "@/hooks/useStore";

// UI page imports shared types
import type { Node, NodeStatus } from "@quartermaster/core";

// WRONG: Import vanilla store directly in a React component
import { fleetStore } from "@quartermaster/core";  // Use useFleetStore() hook instead

// WRONG: Use relative paths for cross-package imports
import { fleetStore } from "../../core/src/stores/fleetStore";  // Use @quartermaster/core
```

---

## Phase 1: FleetService (fixes the polling bug)

### Create `packages/core/src/services/fleetService.ts`

```typescript
import type { Service, ServiceConfig } from "./types";
import { getIPCClient } from "../ipc/provider";
import { fleetStore } from "../stores/fleetStore";
import type { Node, NodeStatus } from "../types/node";

const POLL_INTERVAL_MS = 30_000;

let pollInterval: ReturnType<typeof setInterval> | null = null;
let config: ServiceConfig | null = null;

export const fleetService: Service & {
  loadNodes(): Promise<void>;
  pollStatuses(): Promise<void>;
  addNode(node: Omit<Node, "id">): Promise<Node>;
  removeNode(nodeId: string): Promise<void>;
  updateNode(nodeId: string, updates: Partial<Node>): Promise<void>;
  discoverSshHosts(): Promise<Node[]>;
  importSshHosts(hosts: string[]): Promise<void>;
} = {
  async init(cfg: ServiceConfig) {
    config = cfg;

    // Load initial data
    await this.loadNodes();

    // Start polling
    await this.pollStatuses(); // immediate first poll
    pollInterval = setInterval(() => this.pollStatuses(), POLL_INTERVAL_MS);
  },

  async destroy() {
    if (pollInterval) {
      clearInterval(pollInterval);
      pollInterval = null;
    }
    config = null;
  },

  async loadNodes() {
    const ipc = getIPCClient();
    fleetStore.getState().setLoading(true);
    try {
      const nodes = await ipc.invoke<Node[]>("list_nodes");
      fleetStore.getState().setNodes(nodes);
    } finally {
      fleetStore.getState().setLoading(false);
    }
  },

  async pollStatuses() {
    const ipc = getIPCClient();
    try {
      const results = await ipc.invoke<[string, boolean][]>("poll_all_node_statuses");
      for (const [nodeId, isOnline] of results) {
        const status: NodeStatus = isOnline ? "online" : "offline";
        fleetStore.getState().updateNodeStatus(nodeId, status);
      }
    } catch {
      // Silently fail polling — don't disrupt the app
    }
  },

  async addNode(node) {
    const ipc = getIPCClient();
    const created = await ipc.invoke<Node>("add_node", { node });
    await this.loadNodes(); // refresh list
    return created;
  },

  // ... other mutations follow same pattern
};
```

### Modify `packages/ui/src/App.tsx`

See "Service Initialization" section above for the full `App.tsx` integration.

### Refactor `FleetPage.tsx`

Remove: `loadNodes()`, `pollStatuses()`, `pollTimerRef`, both polling `useEffect` hooks.
Replace: mutations call `fleetService.addNode()`, etc. Read from `useFleetStore()`.

### Refactor `DashboardPage.tsx`

Remove: `loadNodes()`, `loadBlueprints()`, `useEffect` data loading.
Read from `useFleetStore()` — data already populated by service init.

### Verify Phase 1
- Navigate to Fleet → see nodes with live status
- Navigate to Dashboard → navigate back to Fleet → statuses still updating (polling never stopped)
- Close and reopen a node detail → data still fresh

---

## Phase 2: TaskService

### Create `packages/core/src/services/taskService.ts`

| Concern | Details |
|---------|---------|
| **Init** | `detectAllStates()` → `taskStore.getState().setTasks()` + subscribe to events |
| **Events** | `task-progress` → `updateTaskProgress()`, `task-state-changed` → `updateTaskStatus()` |
| **Mutations** | `loadTasks`, `executeTask`, `uninstallTask` |
| **Store** | `taskStore` |
| **Cleanup** | Unsubscribe from both event listeners |

### Delete `packages/ui/src/hooks/useTasks.ts`

### Refactor pages
- `TaskLibraryPage.tsx` — remove `loadTasks()` + `useEffect`, read from `useTaskStore()`
- `TaskDetailPage.tsx` — remove `useTauriEvent("task-progress")`, read from store, call `taskService.uninstallTask()`

---

## Phase 3: BlueprintService

### Create `packages/core/src/services/blueprintService.ts`

| Concern | Details |
|---------|---------|
| **Init** | `listBlueprints()` → `blueprintStore.getState().setBlueprints()` + subscribe to events |
| **Events** | `blueprint-task-warning`, `blueprint-apply-progress`, `blueprint-apply-complete`, `blueprint-uninstall-progress`, `blueprint-uninstall-complete` |
| **Mutations** | `cloneBlueprint`, `createBlankBlueprint`, `deleteBlueprint`, `assignBlueprint`, `unassignBlueprint`, `importBlueprint`, `exportBlueprint`, `applyBlueprint`, `uninstallBlueprint`, `loadBlueprints` |
| **Store** | `blueprintStore` (+ `fleetStore` for assign/unassign cross-store updates) |
| **Cleanup** | Unsubscribe from all 5 event listeners |

**Important**: `assignBlueprint` and `unassignBlueprint` must update BOTH `blueprintStore` and `fleetStore` after the API call. The current code has a bug where neither store is updated — fix this when extracting to the service.

### Refactor pages
- `BlueprintsPage.tsx` — remove `loadBlueprints()`, call `blueprintService.xxx()`
- `BlueprintDetailPage.tsx` — remove 5 `useTauriEvent` calls, read tasks from `useTaskStore()`
- `NodeDetailPage.tsx` — remove 3 `useTauriEvent` calls, remove `api.listBlueprints()` + `api.listTasks()`, read from stores
- `DashboardPage.tsx` — remove `loadBlueprints()` (data from service init)

### Component-scoped event exceptions (stay in components)
- `task-output` in `ExecutionOutputPanel` — dialog-scoped step log
- `bulk-blueprint-progress` in `BulkBlueprintDialog` — dialog-scoped

**Decision rule**: If the event is relevant to any page in the app (e.g., progress bar on Dashboard), it's service-owned. If it's only relevant while a specific dialog/modal is open, it's component-scoped. Component-scoped listeners must clean up on dialog close via the unlisten function.

---

## Phase 4: AppArmorService

### Create `packages/core/src/services/appArmorService.ts`

| Concern | Details |
|---------|---------|
| **Init** | `getDenialLogs()` + `getProfiles()` → `appArmorStore` + subscribe to events |
| **Events** | `apparmor-denial` → `addDenial()` |
| **Mutations** | `startMonitor`, `stopMonitor`, `reviewSelected`, `consolidateProfile`, `confirmApply`, `cancelReview`, `loadData` |
| **Store** | `appArmorStore` |
| **Cleanup** | Unsubscribe from `apparmor-denial` event, call `stopMonitor()` if active |

**Note**: `appArmorService.destroy()` MUST call `stopMonitor()`. The backend monitor spawns a background task that outlives the app if not stopped.

### Delete `packages/ui/src/hooks/useAppArmor.ts`

### Refactor `AppArmorPage.tsx` + `AppArmorDashboard.tsx`
Read from `useAppArmorStore()`, call `appArmorService.xxx()` for mutations.

### Parallelization note
AppArmorService is fully independent of Fleet/Task/Blueprint. If two developers are working simultaneously, one can build Fleet+Task while the other builds AppArmor.

---

## Phase 5: Cleanup, Tests & Documentation

### 5.1 Update store tests
- `blueprintStore.test.ts` — remove tests for deleted async API methods
- All store tests — update imports to use vanilla store directly

### 5.2 Add service tests in `packages/core/`
- `fleetService.test.ts` — mock IPC, verify init populates store, verify polling updates status
- `taskService.test.ts` — mock IPC, verify event subscriptions update store
- `blueprintService.test.ts` — mock IPC, verify assign updates both stores
- Use `createMockIPCClient()` from `packages/core/src/__mocks__/ipc.ts`

### 5.3 Verify no duplicate data loading
```bash
grep -r 'invoke.*list_nodes' packages/ui/src/pages/      # 0 matches (only in fleetService)
grep -r 'invoke.*list_blueprints' packages/ui/src/pages/  # 0 matches (only in blueprintService)
grep -r 'invoke.*detect_all' packages/ui/src/pages/       # 0 matches (only in taskService)
```

### 5.4 Verify no duplicate event subscriptions
For each event in the Event Subscription Map below, verify it appears exactly once:
```bash
grep -r '"task-progress"' packages/  # Only in taskService
grep -r '"blueprint-apply-progress"' packages/  # Only in blueprintService
grep -r '"apparmor-denial"' packages/  # Only in appArmorService
```

### 5.5 Verify no inline event types
```bash
grep -r 'useTauriEvent<{' packages/ui/src/  # 0 matches (all types from core/types/events.ts)
```

### 5.6 Update documentation

**CLAUDE.md** — update these sections:
- Key Directories table: add packages/core/ and packages/ui/
- Conventions: add service layer patterns, IPC abstraction, store duality rule
- Adding a New Tauri Command: add "Register IPC in IPCClient if needed by core services"
- Testing: update paths and describe core vs ui test setup

**docs/development/monorepo.md** (new file):
- Explain packages/core (domain logic) vs packages/ui (React)
- IPC abstraction layer and how services work
- Import alias reference
- How to test core services without React
- How to add a new service

**docs/code-review-guidelines.md** — add:
- Cross-package review: trace features from core service to UI page
- Verify IPC client is never imported directly in core (only IPCClient interface)

---

## Event Subscription Map (After)

| Event | Owner | Type |
|-------|-------|------|
| `task-progress` | `taskService` | Service-owned |
| `task-state-changed` | `taskService` | Service-owned |
| `task-output` | `ExecutionOutputPanel` | Component-scoped |
| `blueprint-task-warning` | `blueprintService` | Service-owned |
| `blueprint-apply-progress` | `blueprintService` | Service-owned |
| `blueprint-apply-complete` | `blueprintService` | Service-owned |
| `blueprint-uninstall-progress` | `blueprintService` | Service-owned |
| `blueprint-uninstall-complete` | `blueprintService` | Service-owned |
| `bulk-blueprint-progress` | `BulkBlueprintDialog` | Component-scoped |
| `apparmor-denial` | `appArmorService` | Service-owned |

---

## Security Boundaries

| Package | Contains | Can Access Secrets? |
|---------|----------|---------------------|
| `packages/core/` | Types, stores, services, IPC interface | NO — no vault operations |
| `packages/ui/` | React app, Tauri adapter, vault IPC wrappers, toast/theme stores | YES — through Tauri IPC |
| Future `packages/cli/` | CLI entry point, CLI IPC adapter | Different auth model (stdin, env vars) |

**Rules**:
- `packages/core/` must NEVER contain functions that accept secrets as parameters
- Vault operations (`vaultCreate`, `vaultUnlock`, `vaultGet`, `vaultSet`, `vaultChangePassword`) stay in `packages/ui/src/services/tauriCommands.ts`
- YubiKey operations stay in `packages/ui/` (hardware-specific, UI-driven)
- SSH key generation/deployment stays in `packages/ui/` (involves credential handling)

---

## Key Files (After)

| File | Role |
|------|------|
| `packages/core/src/ipc/types.ts` | IPCClient interface — the contract between core and host |
| `packages/core/src/ipc/provider.ts` | Global IPC client accessor |
| `packages/core/src/services/serviceInit.ts` | Boot coordinator — called once from App.tsx |
| `packages/core/src/stores/*.ts` | Vanilla Zustand stores — services write, UI reads |
| `packages/ui/src/ipc/tauriAdapter.ts` | Tauri-specific IPCClient implementation |
| `packages/ui/src/services/tauriCommands.ts` | Vault + UI-specific IPC wrappers (NOT shared) |
| `packages/ui/src/hooks/useStore.ts` | React wrappers around vanilla stores |
| `packages/ui/src/stores/toastStore.ts` | UI-only toast notification store |
| `packages/ui/src/App.tsx` | Calls `initializeServices()` on mount |

---

## Final Verification Checklist

1. **TypeScript**: `tsc --noEmit` in both core and ui — 0 errors
2. **React-free core**: `grep -r "react" packages/core/src/` — 0 matches
3. **Tauri-free core**: `grep -r "@tauri-apps" packages/core/src/` — 0 matches
4. **Tests**: `npm run test` — both core and ui suites pass
5. **Polling fix**: Add a node on Fleet, navigate to Node Detail → status shows online/offline, not unknown
6. **No duplicate listeners**: each service-owned event appears exactly once in codebase
7. **No duplicate data loads**: `listNodes`, `listBlueprints`, `detectAllStates` called only in services
8. **No inline event types**: `grep -r 'useTauriEvent<{' packages/ui/src/` — 0 matches
9. **Dev server**: `npm run dev` → Vite on :1420
10. **Tauri dev**: `npm run tauri dev` → app launches, all features work
11. **Tauri build**: `npm run tauri build` → .deb and .appimage generated

---

## Research References

- [Spacedrive](https://github.com/spacedriveapp/spacedrive) — Tauri app with dedicated `@sd/ts-client` library + React Query
- [GitButler](https://github.com/gitbutlerapp/gitbutler) — Tauri app with `@gitbutler/shared` services package + RTK Query
- [Hopp](https://www.gethopp.app/blog/tauri-window-state-sync) — Tauri + Zustand + module-level state sync
- [Zustand Architecture at Scale](https://brainhub.eu/library/zustand-architecture-patterns-at-scale) — Pure reducers + action separation
- [Zustand vanilla store docs](https://docs.pmnd.rs/zustand/guides/how-to-reset-state#creating-a-store-with-vanilla-store) — `createStore` from `zustand/vanilla`

---

## Review History

- **2026-02-15**: Initial plan created
- **2026-02-15**: Updated after Codex architecture review (4 reviewers: backend, frontend, cross-cutting, security). Changes:
  - Added IPC abstraction layer (IPCClient interface) for CLI support
  - Moved vault/credential operations out of core package
  - Moved toastStore/themeStore to UI package
  - Changed service init from Promise.allSettled to sequential
  - Added Service interface with init/destroy lifecycle
  - Split Phase 0 into 4 milestones with checkpoints
  - Added core test infrastructure (vitest config, mock IPC client)
  - Added security boundaries section
  - Added missing event type definitions
  - Added alternatives considered section
  - Added import alias reference
  - Added component-scoped vs service-owned event decision rule
