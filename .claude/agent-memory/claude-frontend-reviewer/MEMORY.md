# Frontend Review Agent Memory

## Tauri 2 IPC Argument Conventions
- Tauri 2 `#[tauri::command]` generates `#[serde(rename_all = "camelCase")]` on argument structs
- Frontend must send camelCase keys (e.g., `nodeId` for Rust `node_id`)
- Frontend interfaces that use snake_case keys (like `AddNodeParams.ssh_config`) and get spread into invoke() will silently fail to match Rust params
- Command names are NOT auto-converted; frontend must use the exact Rust function name (snake_case)

## Known Contract Mismatches (as of 2026-02-07)
1. `vault_create` / `vault_unlock`: Frontend sends `password`, Rust expects `master_password` (camelCase: `masterPassword`)
2. `AddNodeParams.ssh_config`: Interface uses snake_case, spread into invoke() sends `ssh_config` but Tauri expects `sshConfig`
3. `CreateBlueprintParams.task_entries`: Same snake_case issue when spread
4. `AppConfig` type is incomplete -- missing 7+ fields from Rust `ConfigData` (but `setConfig`/`getConfig` are currently unused)
5. Frontend `TaskInfo` type missing backend fields: `version`, `variables`, `download`, `desktop`, `apparmor`

## Event Emission Patterns
- Backend emits: `task-progress`, `task-state-changed`, `apparmor-denial`, `blueprint-apply-progress`, `blueprint-apply-complete`, `blueprint-task-warning`, `bulk-blueprint-progress`
- `NodeStatusChangedEvent` defined in events.ts but NO backend emission for it exists -- dead type
- `BlueprintApplyProgressEvent` / `BlueprintApplyCompleteEvent` defined in events.ts but never imported by consuming components (inline types used instead)

## Store Patterns
- Zustand stores in this project are simple and correct -- immutable updates via spread, no mutations
- `useAppArmorStore()` called without selectors in `useAppArmor` hook -- subscribes to ALL state changes (perf concern, not bug)
- Blueprint store async actions (cloneBlueprint, deleteBlueprint, etc.) do NOT catch errors internally -- callers must catch. This is intentional and callers do handle errors.

## Hook Patterns
- `useTauriEvent` stores handler in a ref updated every render -- prevents stale closure issues
- `useEffect` cleanup in `useTauriEvent` correctly awaits the `listen` promise then calls unlisten
- `useTasks` and `useAppArmor` define async functions as plain closures (not useCallback) used in useEffect -- acceptable since only called once on mount

## UI Component Patterns (as of 2026-02-07)
- `ToastContainer` is rendered in every page instead of once in AppShell -- duplication, not a bug since only one page mounts at a time
- `SshKeyList` component has duplicate API calls on mount (both `loadKeys` callback and inline useEffect call `listSshKeys`)
- `import * as Icons from "lucide-react"` used in ModuleCard and BlueprintDetailPage -- prevents tree-shaking
- Several destructive actions lack confirmation: node removal, blueprint unassign, activity log clear, variable delete
- `prompt()` used for blueprint clone naming -- inconsistent with Dialog-based UX pattern
