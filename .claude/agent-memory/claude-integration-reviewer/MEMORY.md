# Integration Reviewer Memory

## Key Patterns Verified
- Tauri 2 IPC: camelCase (TS) -> snake_case (Rust) auto-conversion works for `invoke()` params
- Event payloads: Backend `app.emit()` JSON must match frontend `useTauriEvent<T>` type param
- Tagged enums: Rust `#[serde(tag = "type")]` serializes to `{ type: "variant", ...fields }` -- TS uses optional fields
- `#[serde(skip_serializing_if)]` fields should be `?` optional in TypeScript

## Known Issues Found (2026-02-07)
1. **TaskInfo type drift**: Rust has V2 fields (version, variables, download, desktop, apparmor) absent from TS type
2. **AppConfig type drift**: TS `AppConfig` has 3 fields, Rust `ConfigData` has 11+ fields
3. **Event payload inconsistency**: `task-progress` from `execute_task` lacks `node_id`; `uninstall_task`/`apply_blueprint` include it
4. **NodeStatusChangedEvent**: Defined in `events.ts` but never emitted by backend
5. **Blueprint docs missing**: No docs for dry-run, inheritance, versioning, import/export, bulk ops, activity log, uninstall, vault, variables
6. **FleetStore untested**: `toggleNodeSelection`, `selectAllNodes`, `clearSelection` have no tests
7. **BlueprintStore untested**: `importBlueprint`, `exportBlueprint` have no tests

## Review Execution Notes
- Backend has 32 modules with `#[cfg(test)]` (good coverage)
- Frontend has 6 test files covering 6 stores (no component/hook tests)
- Mock `invoke` returns `[]` universally -- only works because blueprint tests mock at service layer
- `matchMedia.ts` mock file listed in task doesn't exist separately -- mock is inline in `setup.ts`
