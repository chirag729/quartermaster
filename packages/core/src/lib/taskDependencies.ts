import type { TaskInfo, TaskStateInfo } from "../types/task";
import type { BlueprintTaskEntry } from "../types/blueprint";

/**
 * Compute the ordered list of task IDs that must be installed to install `targetTaskId`.
 * Walks `depends_on` recursively, collecting any dependency that is not currently installed
 * and is an enabled entry in the blueprint. Returns them in dependency order (deps first, target last).
 */
export function getInstallChain(
  targetTaskId: string,
  taskInfoMap: Record<string, TaskInfo>,
  taskStateMap: Map<string, TaskStateInfo>,
  pendingEntries: BlueprintTaskEntry[],
): string[] {
  const enabledIds = new Set(
    pendingEntries.filter((e) => e.enabled).map((e) => e.task_id),
  );

  const chain: string[] = [];
  const visited = new Set<string>();

  function walk(taskId: string) {
    if (visited.has(taskId)) return; // prevent infinite loops on circular deps
    visited.add(taskId);

    const info = taskInfoMap[taskId];
    if (!info) return;

    // Walk dependencies first
    for (const depId of info.depends_on ?? []) {
      if (!enabledIds.has(depId)) continue;
      const depState = taskStateMap.get(depId);
      const isInstalled = depState?.status === "completed" && !depState.config_drifted && !depState.version_changed;
      if (!isInstalled) {
        walk(depId);
      }
    }

    // Add this task if not already in chain
    if (!chain.includes(taskId)) {
      chain.push(taskId);
    }
  }

  walk(targetTaskId);
  return chain;
}

/**
 * Compute the ordered list of task IDs that must be uninstalled to uninstall `targetTaskId`.
 * Finds reverse dependencies (which enabled blueprint tasks depend on this one),
 * recursively collects installed dependents. Returns them in reverse-dependency order
 * (dependents first, target last).
 */
export function getUninstallChain(
  targetTaskId: string,
  taskInfoMap: Record<string, TaskInfo>,
  taskStateMap: Map<string, TaskStateInfo>,
  pendingEntries: BlueprintTaskEntry[],
): string[] {
  const enabledIds = new Set(
    pendingEntries.filter((e) => e.enabled).map((e) => e.task_id),
  );

  // Build reverse dependency map: taskId -> list of tasks that depend on it
  const reverseDeps = new Map<string, string[]>();
  for (const id of enabledIds) {
    const info = taskInfoMap[id];
    if (!info) continue;
    for (const depId of info.depends_on ?? []) {
      if (!reverseDeps.has(depId)) {
        reverseDeps.set(depId, []);
      }
      reverseDeps.get(depId)!.push(id);
    }
  }

  const chain: string[] = [];
  const visited = new Set<string>();

  function walk(taskId: string) {
    if (visited.has(taskId)) return; // prevent infinite loops
    visited.add(taskId);

    // Walk reverse dependencies first (dependents that need to be uninstalled before this one)
    const dependents = reverseDeps.get(taskId) ?? [];
    for (const depId of dependents) {
      if (!enabledIds.has(depId)) continue;
      const depState = taskStateMap.get(depId);
      const isInstalled = depState?.status === "completed";
      if (isInstalled) {
        walk(depId);
      }
    }

    if (!chain.includes(taskId)) {
      chain.push(taskId);
    }
  }

  walk(targetTaskId);
  return chain;
}
