# Frontend Stores & IPC Review Findings

## Critical Issues Identified

### 1. Missing/Incomplete Event Type Definitions
- `bulk-blueprint-progress` event emitted by backend (blueprints.rs:714) but type only defined inline in BulkBlueprintDialog.tsx
- `task-progress` event incomplete: backend emits with optional `node_id` during blueprint apply, but TaskProgressEvent doesn't include it
- `blueprint-apply-progress` and `blueprint-apply-complete` types exist but unused
- `node-status-changed` type defined but no backend emission found

### 2. Event Type Organization
- Event types scattered across component files instead of centralized in src/types/events.ts
- Creates maintenance burden and makes contracts non-discoverable

## IPC Contract Status
- All 82 backend commands have frontend wrappers
- Parameter naming: camelCase frontend auto-converts to snake_case (Tauri handles this)
- Return types align across boundary

## Patterns & Best Practices
- Error handling: All IPC calls properly catch and display toasts
- React hooks: No stale closure bugs; useEffect cleanup proper
- Store pattern: Mostly correct, but assignBlueprint/unassignBlueprint don't update store state

## Frontend UI Components Review Results (Latest)

### Critical Findings (8 Issues)
1. **HIGH**: ExecutionOutputPanel stays subscribed to task-output while hidden (NodeDetailPage:816, TaskExecutionDialog:299, ExecutionOutputPanel:41)
2. **MEDIUM**: blueprint-task-warning listeners unscoped - show toasts from unrelated operations (NodeDetailPage:74, BlueprintDetailPage:155)
3. **MEDIUM**: Event contract drift - TaskProgressEvent/TaskStateChangedEvent missing node_id field
4. **LOW**: Inline partial event types instead of centralized definitions (blueprint-task-warning, etc.)
5. **MEDIUM**: Silent error swallowing in preloads - NodeDetailPage:127, BlueprintDetailPage:247, BulkBlueprintDialog:93
6. **MEDIUM**: CommandPalette crashes on empty results (modulo by zero) - CommandPalette:120
7. **LOW**: Destructive actions without confirmation (ActivityFeed:80, SettingsPage:242)
8. **LOW**: Bulk progress displays impossible "N+1 of N" - BulkBlueprintDialog:215

### Root Causes
- Dialog components mount even when closed (useState controls visibility, not mounting)
- Dialog.tsx doesn't unmount on close - only calls .close()
- Event listeners not filtered by context (blueprint_id, node_id)
- Empty state not guarded in array operations (% flatFiltered.length)
- Error .catch(() => {}) pattern swallows failures silently

## Recommendations
1. Fix ExecutionOutputPanel to unmount when dialog closes
2. Consolidate event types to src/types/events.ts with full contracts
3. Add node_id/blueprint_id to event types and filter listeners by context
4. Replace silent .catch(() => {}) with actual error handling/toasts
5. Guard empty array access in CommandPalette
6. Add confirmation dialogs for destructive actions

## Architecture Plan Review (2025-02-15)

### HIGH-Severity Gaps in Service Layer Plan
1. **Zustand Vanilla Store Hook Duality Footgun**: Pages must import wrapper hooks from `packages/ui/hooks/useStore.ts`, not vanilla stores directly. Services use `.getState()`. Plan doesn't explicitly document this dual API.
2. **ToastStore UI Concern in Core**: Plan moves toastStore to core, but it uses React setTimeout for 5s auto-dismiss. Services shouldn't own UI timers. Keep toastStore in packages/ui, inject onToast callback into services.
3. **Event Type Duplication Not Audited**: Pages define inline event types for useTauriEvent. After migration, these must be centralized in core/types/events.ts. Plan's grep strategy only finds duplicate listeners, not duplicate type definitions.
4. **Service Init Order and Race Conditions**: Plan uses Promise.allSettled (parallel), but TaskService may emit events before listeners are attached, or BlueprintService may read fleetStore before Fleet init completes. Need sequential init with documented order.
5. **Blueprint Assign/Unassign Store Updates Missing**: Current code calls api.assignBlueprint but doesn't update fleetStore or blueprintStore. Bug migrates to service if not fixed first.

### MEDIUM-Severity Gaps
6. **Event Listener Cleanup Not Documented**: Services need destroy() methods that stop polling intervals and unsubscribe from events. Must be idempotent.
7. **Vite HMR Strategy Incomplete**: Plan excludes core from pre-bundling, but doesn't address symlink watchers on Windows or stale code issues if someone builds core to dist/.
8. **Test Mock Strategy Unclear**: Core package tests need their own vitest config with Tauri mocks. Plan doesn't specify mocking strategy for service unit tests.
9. **Cross-Package Import Cycles Not Prevented**: No guardrails documented to prevent accidental core → ui dependencies.
10. **Phase Ordering Suboptimal**: AppArmorService is independent and could be parallel to FleetService, but plan serializes it.
11. **Memory Leaks**: destroyServices() doesn't clear store state. Re-init without cleanup causes data duplication.
12. **Blueprint Detail Page Preloading**: Users navigating directly to /blueprints/abc123 before blueprintService.init() completes will see empty state. Plan doesn't address this.
13. **Component-Scoped Events Decision Framework**: Plan exempts task-output and bulk-blueprint-progress from services but doesn't explain the decision rule clearly.

### Recommendation Summary
- **Critical before Phase 0**: Fix assignBlueprint mutations, move toastStore to ui, document zustand duality, define Service interface with init/destroy
- **Before Phase 1**: Document sequential init order, add servicesReady state, create core test mocks
- **During migration**: Audit inline event types, add import cycle detection, test service idempotence
- **After migration**: Clear stores in destroyServices(), document decision frameworks

See `/home/chirag/Development/Projects/quartermaster/codex-review-arch-frontend.md` for full details and code examples.
