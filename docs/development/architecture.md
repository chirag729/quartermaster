# Architecture

Anvil is a Tauri 2 desktop application for provisioning and managing local and remote Linux machines. The frontend is built with React 18 and TypeScript, the backend with Rust and Tokio, and they communicate via Tauri's IPC invoke/event system.

## Technology Stack

| Layer | Technology |
|-------|-----------|
| Desktop framework | Tauri 2 |
| Frontend | React 18, TypeScript, Vite 6 |
| Styling | Tailwind CSS 4 |
| State management | Zustand 5 |
| Routing | React Router 7 (HashRouter) |
| Animations | Motion 12 |
| Icons | Lucide React |
| Backend runtime | Tokio (async) |
| Serialization | Serde / serde_json |
| SSH | ssh CLI (OpenSSH), russh 0.46 |
| Testing | Vitest + RTL (frontend), cargo test (backend) |
| Packaging | .deb, .appimage |

## Directory Structure

```
src/
├── components/
│   ├── apparmor/          # AppArmor feature
│   ├── blueprints/        # Blueprint cards and grid
│   ├── dashboard/         # Dashboard grid and task cards (legacy naming)
│   ├── fleet/             # Fleet/node components
│   ├── layout/            # AppShell, Sidebar, CommandPalette, TopBar
│   ├── modules/           # Task config panel (legacy naming)
│   ├── ssh/               # SSH config, key management
│   └── ui/                # Atomic UI primitives
├── hooks/                 # useTasks, useAppArmor, useTauriEvent, useTheme
├── pages/                 # FleetPage, NodeDetailPage, BlueprintsPage, etc.
├── services/              # tauriCommands.ts
├── stores/                # taskStore, fleetStore, blueprintStore, appArmorStore, themeStore, toastStore
├── types/                 # task.ts, node.ts, blueprint.ts, apparmor.ts, config.ts, events.ts

src-tauri/src/
├── apparmor/              # Log parsing, rule generation, monitoring
├── blueprints/            # Blueprint model, manager, defaults
├── commands/              # Tauri command handlers (tasks, fleet, blueprints, ssh, apparmor, config, system)
├── config/                # ConfigManager
├── executor/              # CommandExecutor trait, LocalExecutor, SshExecutor
├── fleet/                 # Node model, FleetManager
├── polkit/                # PolicyKit authorization
├── tasks/                 # SetupTask trait, registry, implementations
├── state.rs               # AppState struct
├── error.rs               # AppError enum
└── lib.rs                 # Tauri builder + command registration
```

## Application State

The backend's shared state is defined in `src-tauri/src/state.rs`:

```rust
pub struct AppState {
    pub registry: Arc<TaskRegistry>,
    pub config: Arc<Mutex<ConfigManager>>,
    pub monitor_running: Arc<Mutex<bool>>,
    pub fleet_manager: Arc<Mutex<FleetManager>>,
    pub blueprint_manager: Arc<Mutex<BlueprintManager>>,
}
```

State is initialized in `lib.rs::run()` and injected into all Tauri commands via `manage()`.

- **registry** -- Read-only after initialization. Contains all registered `SetupTask` implementations.
- **config** -- Thread-safe access to persistent configuration (`~/.config/anvil/config.json`).
- **monitor_running** -- Boolean flag controlling the AppArmor audit log monitor.
- **fleet_manager** -- Manages the set of fleet nodes (local and remote machines).
- **blueprint_manager** -- Manages blueprint definitions (built-in and user-created).

## Key Architectural Patterns

### 1. CommandExecutor abstraction

The `CommandExecutor` trait abstracts command execution so that tasks run identically on the local machine or over SSH:

```rust
pub trait CommandExecutor: Send + Sync {
    async fn run_command(&self, cmd: &str, args: &[&str]) -> Result<CommandOutput, AppError>;
    async fn file_exists(&self, path: &str) -> Result<bool, AppError>;
    async fn read_file(&self, path: &str) -> Result<String, AppError>;
    async fn write_file(&self, path: &str, content: &str) -> Result<(), AppError>;
    async fn create_dir_all(&self, path: &str) -> Result<(), AppError>;
    fn home_dir(&self) -> String;
    fn is_local(&self) -> bool;
}
```

Two implementations exist:

- **LocalExecutor** -- Uses `tokio::process::Command` and native filesystem operations.
- **SshExecutor** -- Uses the `ssh` CLI binary to dispatch operations as remote shell commands. Arguments are shell-escaped for safety; `write_file` pipes content via stdin to avoid injection.

When a blueprint is applied, the system checks the target node's `NodeKind`:
- `Local` selects `LocalExecutor`
- `Remote` selects `SshExecutor` with the node's `SshConfig`

### 2. Blueprint config layering

Configuration for tasks follows a merge strategy:

1. **Global task defaults** -- Stored in `~/.config/anvil/config.json` under `task_configs`.
2. **Blueprint config overrides** -- Each `BlueprintTaskEntry` can specify `config_overrides`.

At execution time, the base config is loaded first, then blueprint overrides are merged on top. This allows the same task to behave differently depending on which blueprint is being applied.

### 3. Event-driven progress

Long-running backend operations emit events rather than blocking the IPC call:

| Event | Payload | Purpose |
|-------|---------|---------|
| `task-progress` | `{ node_id, task_id, progress, message }` | Progress updates during task execution |
| `task-state-changed` | `{ node_id, task_id, status }` | Task status transitions |
| `blueprint-apply-progress` | `{ node_id, blueprint_id, completed, total, current_task_id }` | Blueprint application progress |
| `blueprint-apply-complete` | `{ node_id, blueprint_id }` | Blueprint application finished |
| `node-status-changed` | `{ node_id, status }` | Node connectivity changes |
| `apparmor-denial` | `{ denial: DenialEvent }` | Real-time AppArmor denial log entries |

The frontend subscribes via `listen()` from `@tauri-apps/api/event`, wrapped in the `useTauriEvent` hook for automatic cleanup.

### 4. PolicyKit for privilege escalation

Operations requiring root privileges (AppArmor profile modification, PolicyKit policy installation) go through `pkexec` rather than running the entire application as root. The `polkit/auth.rs` module provides:

- `execute_privileged(command, args)` -- Runs commands via `pkexec`
- `is_policy_installed()` -- Checks for the policy file at `/usr/share/polkit-1/actions/com.anvil.policy`
- `is_helper_installed()` -- Checks for the helper script at `/usr/lib/anvil/anvil-apparmor-helper`

AppArmor operations use a dedicated helper script (`anvil-apparmor-helper`) registered with the `com.anvil.apparmor-manage` PolicyKit action. The `org.freedesktop.policykit.exec.path` annotation links the helper to the action's `auth_admin_keep` policy, enabling credential caching (~5 minutes). This means the user is prompted for their password once, and subsequent AppArmor operations within the cache window require no further prompts.

### 5. Independent Zustand stores

The frontend uses six independent Zustand stores, each managing a single domain:

| Store | Key State | Purpose |
|-------|-----------|---------|
| `taskStore` | tasks, loading, executing | Task list and execution state |
| `fleetStore` | nodes, selectedNode | Fleet node management |
| `blueprintStore` | blueprints, applying | Blueprint definitions and application state |
| `appArmorStore` | denials, suggestions, profiles, monitoring | AppArmor data and UI state |
| `themeStore` | theme, resolvedTheme | Light/dark/system preference |
| `toastStore` | toasts | Toast notification queue (5s auto-dismiss) |

Stores are independent and do not reference each other. Cross-domain coordination happens in hooks and page components.

## Routing

React Router 7 with `HashRouter` (hash-based URLs for Tauri WebView compatibility).

| Route | Page | Purpose |
|-------|------|---------|
| `/` | Redirect | Redirects to `/fleet` |
| `/fleet` | `FleetPage` | Fleet overview, node grid |
| `/fleet/:nodeId` | `NodeDetailPage` | Individual node detail and task execution |
| `/tasks` | `TasksPage` | Task list and management |
| `/blueprints` | `BlueprintsPage` | Blueprint grid |
| `/blueprints/:id` | `BlueprintDetailPage` | Blueprint detail and editing |
| `/apparmor` | `AppArmorPage` | AppArmor profiles, denial logs, rule suggestions |
| `/settings` | `SettingsPage` | Theme and preferences |

Page transitions use Motion's `AnimatePresence` with fade and vertical slide animations.

## Error Handling

All backend errors are represented as the `AppError` enum:

```rust
pub enum AppError {
    Io(io::Error),
    Serde(serde_json::Error),
    Command(String),
    Task(String),
    AppArmor(String),
    Polkit(String),
    Config(String),
    Ssh(String),
    Fleet(String),
    Blueprint(String),
    Connection(String),
    Other(String),
}
```

`AppError` implements `Serialize` so it can be returned as a JSON string to the frontend via Tauri's IPC. The frontend displays errors via toast notifications using the `toastStore`.

## Data Persistence

All persistent data is stored under `~/.config/anvil/`:

| Path | Contents |
|------|----------|
| `~/.config/anvil/config.json` | Application configuration (theme, task configs, completed tasks) |
| `~/.config/anvil/nodes/*.json` | Fleet node definitions (one file per node) |
| `~/.config/anvil/blueprints/*.json` | Blueprint definitions (one file per blueprint) |

## Frontend-Backend Communication

The frontend calls Rust functions via `invoke()` from `@tauri-apps/api/core`. All command wrappers live in `src/services/tauriCommands.ts`:

```
Frontend                            Backend
invoke("list_tasks")        -->     commands::tasks::list_tasks()
invoke("execute_task")      -->     commands::tasks::execute_task()
invoke("list_nodes")        -->     commands::fleet::list_nodes()
invoke("apply_blueprint")   -->     commands::blueprints::apply_blueprint()
invoke("get_denial_logs")   -->     commands::apparmor::get_denial_logs()
...
```

All Tauri commands are registered in `lib.rs` via `generate_handler![]`.

## AppArmor Subsystem

```
apparmor/
├── types.rs            # DenialEvent, PermissionSuggestion, ProfileInfo, etc.
├── log_parser.rs       # Regex-based audit.log parser
├── rule_generator.rs   # Generates AppArmor rules from denial patterns
├── rule_consolidator.rs# Consolidates overlapping rules
├── profile_manager.rs  # Lists/reads/modifies profiles via aa-status + pkexec
└── monitor.rs          # Real-time file watcher on audit.log (notify crate)
```

- **Log parsing** -- Reads `/var/log/audit/audit.log` (falls back to `journalctl`), extracts denial events with regex, keeps last 200 entries.
- **Rule generation** -- Analyzes denial patterns, generates rules with risk levels (Low, Medium, High, Critical).
- **Rule consolidation** -- Merges overlapping or redundant rules into consolidated glob-based rules.
- **Profile management** -- Lists profiles via `aa-status --json`, reads from `/etc/apparmor.d/`, applies rules via a privileged helper script (falls back to direct `pkexec` if the helper is not installed). Uses `tempfile` crate for secure temp file creation.
- **Monitoring** -- Uses the `notify` crate to watch the audit log file; falls back to `journalctl --follow`.

## Key Design Decisions

1. **HashRouter over BrowserRouter** -- Tauri loads the frontend from a file URL, so hash-based routing avoids server-side routing requirements.
2. **Zustand over Redux/Context** -- Lightweight stores with no boilerplate. Each domain gets its own independent store.
3. **Trait-based task registry** -- New tasks are added by implementing `SetupTask` and registering in `create_registry()`. No changes needed to the command layer.
4. **CommandExecutor abstraction** -- Enables the same task logic to run on local or remote machines without modification.
5. **PolicyKit for privilege escalation** -- Operations requiring root go through `pkexec` rather than running the whole app as root.
6. **Event-driven progress** -- Task execution is async; the backend emits progress events rather than blocking the IPC call.
7. **Blueprint config layering** -- Allows per-blueprint customization of task behavior without duplicating task definitions.
8. **CSS custom properties for theming** -- Semantic color tokens (surface, card, border, text) allow light/dark switching by toggling a single DOM class.
