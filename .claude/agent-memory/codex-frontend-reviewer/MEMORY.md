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
