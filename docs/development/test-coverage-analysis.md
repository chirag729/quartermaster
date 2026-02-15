# Test Coverage Analysis

> Generated 2026-02-15. Covers the monorepo restructure (`packages/core` + `packages/ui`) and backend.

---

## 1. Frontend Core (`packages/core/src/`)

| File | Tests | Priority | Notes |
|------|-------|----------|-------|
| `ipc/provider.ts` | 0 | **Critical** | Core IPC plumbing — 3 tests needed (throw when uninitialized, set/get round-trip, replace client) |
| `ipc/types.ts` | — | Skip | Type-only file, no runtime logic |
| `lib/formatError.ts` | 0 | **Critical** | Pure function, easy to test — 5 tests (TauriError with/without hint, Error, string, other) |
| `lib/taskDependencies.ts` | 14 (in UI) | Skip | Already tested in `packages/ui/src/__tests__/lib/taskDependencies.test.ts` |
| `services/fleetService.ts` | 0 | **Critical** | 12 tests: load, poll, start/stop polling, add/remove/update node, discover, init/destroy |
| `services/taskService.ts` | 0 | **Critical** | 11 tests: load, execute (success/fail/finally), uninstall, event handlers, destroy |
| `services/blueprintService.ts` | 0 | **Critical** | 12 tests: load, clone/create/delete, assign/unassign, import/export, events, destroy |
| `services/appArmorService.ts` | 0 | **Critical** | 13 tests: loadData, start/stop monitor, reviewSelected, consolidate, confirmApply, cancel, events |
| `services/serviceInit.ts` | 0 | **Critical** | 5 tests: init order, idempotent, destroy, destroy idempotent, reset for re-init. **Bug**: `initialized = true` set before awaits |
| `stores/fleetStore.ts` | 15 (in UI) | Important | Missing 5 tests: toggleNodeSelection (add/remove/preserve), selectAllNodes, clearSelection |
| `stores/taskStore.ts` | 16 (in UI) | Skip | Well-covered by `moduleStore.test.ts` |
| `stores/blueprintStore.ts` | 18 (in UI) | Skip | Well-covered by `blueprintStore.test.ts` |
| `stores/appArmorStore.ts` | 25 (in UI) | Skip | Well-covered by `appArmorStore.test.ts` |
| `types/*.ts` | — | Skip | Type-only files |
| `index.ts` | — | Skip | Re-export barrel file |

**Core total: 0 existing → ~61 new tests needed**

---

## 2. Frontend UI (`packages/ui/src/`)

| File | Tests | Priority | Notes |
|------|-------|----------|-------|
| `ipc/tauriAdapter.ts` | 0 | Skip | Thin Tauri wrapper — testing would just test Tauri's `invoke`/`listen` |
| `services/tauriCommands.ts` | 0 | Skip | Thin `invoke()` wrappers — would only test mock plumbing |
| `hooks/useStore.ts` | 0 | Skip | Single-line React `useStore` re-exports |
| `hooks/useTauriEvent.ts` | 0 | Skip | Thin React wrapper around `listen()` |
| `hooks/useTheme.ts` | 2 (in UI) | Skip | Already tested in `themeStore.test.ts` |
| `hooks/useDebounce.ts` | 0 | Skip | Standard debounce hook, low risk |
| `stores/toastStore.ts` | 9 (in UI) | Skip | Well-covered |
| `stores/themeStore.ts` | 2 (in UI) | Skip | Well-covered |
| `__tests__/stores/fleetStore.test.ts` | 15 | Important | Missing `selectedNodeIds: []` in beforeEach; missing 5 action tests |
| Components (`components/**`) | 0 | Skip | Component tests require extensive DOM mocking; lower ROI than service tests |
| Pages (`pages/**`) | 0 | Skip | Page-level integration tests are complex; better covered by E2E |

**UI total: 94 existing → 5 new tests (fleetStore actions)**

---

## 3. Backend (`src-tauri/src/`)

| Module | Inline Tests | Integration Tests | Priority | Notes |
|--------|-------------|-------------------|----------|-------|
| `apparmor/log_parser.rs` | 6 | 16 (apparmor_profiles.rs) | Skip | Well-covered |
| `apparmor/rule_consolidator.rs` | 12 | — | Skip | Well-covered |
| `apparmor/rule_generator.rs` | 11 | — | Skip | Well-covered |
| `apparmor/template_manager.rs` | 12 | — | Skip | Well-covered |
| `apparmor/template_schema.rs` | 20 | — | Skip | Well-covered |
| `apparmor/tunable_installer.rs` | 9 | — | Skip | Well-covered |
| `blueprints/manager.rs` | 17 | 8 (builtin_blueprints.rs) | Skip | Well-covered |
| `blueprints/package.rs` | 27 | — | Skip | Well-covered |
| `blueprints/validate.rs` | 8 | — | Skip | Well-covered |
| `blueprints/yaml_schema.rs` | 4 | — | Skip | Well-covered |
| `config/manager.rs` | 5 | 5 (config_persistence.rs) | Skip | Well-covered |
| `fleet/manager.rs` | 7 | 5 (fleet_lifecycle.rs) | Skip | Well-covered |
| `fleet/ssh_config.rs` | 15 | — | Skip | Well-covered |
| `fleet/terminal.rs` | 6 | — | Skip | Well-covered |
| `fleet/status_poller.rs` | 0 | — | Skip | Requires live TCP connections |
| `tasks/script_task.rs` | 18 | 8 (task_registry.rs) | Skip | Well-covered |
| `tasks/execution_log.rs` | 8 | — | Skip | Well-covered |
| `tasks/install_state.rs` | 10 | — | Skip | Well-covered |
| `tasks/validate.rs` | 10 | — | Skip | Well-covered |
| `tasks/yaml_schema.rs` | 3 | — | Skip | Well-covered |
| `executor/local.rs` | 0 | — | Skip | Requires shell execution |
| `executor/dry_run.rs` | 0 | — | Skip | Covered indirectly via cross_module.rs |
| `executor/privileged.rs` | 2 | — | Skip | Basic coverage |
| `variables/mod.rs` | 24 | 6 (cross_module.rs) | Skip | Well-covered |
| `vault/mod.rs` | 10 | 4 (vault_lifecycle.rs) | Skip | Well-covered |
| `dirs.rs` | 7 | — | Skip | Well-covered |
| `download/mod.rs` | 7 | — | Skip | Well-covered |
| `desktop/mod.rs` | 7 | — | Skip | Well-covered |
| `activity_log.rs` | 10 | — | Skip | Well-covered |
| `commands/tasks.rs` | 0 | — | Skip | Requires `tauri::AppHandle` (Tauri test harness) |
| `commands/fleet.rs` | 0 | — | Skip | Requires `tauri::AppHandle` |
| `commands/blueprints.rs` | 4 | — | Skip | Has basic tests |
| `commands/apparmor.rs` | 0 | — | Skip | Requires `tauri::AppHandle` |
| `commands/config.rs` | 0 | — | Skip | Requires `tauri::AppHandle` |
| `commands/ssh.rs` | 1 | — | Skip | Has basic test |
| `commands/system.rs` | 0 | — | Skip | Requires `tauri::AppHandle` |
| `commands/variables.rs` | 0 | — | Skip | Requires `tauri::AppHandle` |
| `commands/vault.rs` | 0 | — | Skip | Requires `tauri::AppHandle` |
| `commands/activity.rs` | 0 | — | Skip | Requires `tauri::AppHandle` |
| `commands/yubikey.rs` | 19 | — | Skip | Well-covered |
| `commands/yubikey_setup.rs` | 22 | — | Skip | Well-covered |

**Backend total: 321 inline + 61 integration = 382 tests. No new tests needed — coverage is strong.**

---

## 4. Priority Summary

| Priority | Area | New Tests |
|----------|------|-----------|
| Critical | Core IPC provider | 3 |
| Critical | Core formatError | 5 |
| Critical | Core fleetService | 12 |
| Critical | Core taskService | 11 |
| Critical | Core blueprintService | 12 |
| Critical | Core appArmorService | 13 |
| Critical | Core serviceInit | 5 |
| Important | UI fleetStore (gaps) | 5 |
| **Total** | | **~66** |

---

## 5. Known Issues

1. **serviceInit.ts bug**: `initialized = true` is set _before_ the async service inits complete. If any init throws, the flag remains true and retry is impossible. Fix: move `initialized = true` after all awaits, and reset in catch.

2. **fleetStore.test.ts beforeEach gap**: `selectedNodeIds: []` is missing from the reset, so tests of bulk selection actions could leak state between tests.
