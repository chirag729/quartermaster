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

## Recommendations for Next Review
1. Consolidate event types to src/types/events.ts
2. Make task-progress event type handle both single-task and blueprint-apply contexts
3. Verify whether node-status-changed backend emission should exist or type should be removed
4. Document event naming convention (kebab-case vs command snake_case)
