import { getIPCClient } from "../ipc/provider";
import { blueprintStore } from "../stores/blueprintStore";
import type { Blueprint } from "../types/blueprint";

let unlistenTaskWarning: (() => void) | null = null;
let unlistenApplyProgress: (() => void) | null = null;
let unlistenApplyComplete: (() => void) | null = null;
let unlistenUninstallProgress: (() => void) | null = null;
let unlistenUninstallComplete: (() => void) | null = null;

async function loadBlueprints(): Promise<void> {
  const ipc = getIPCClient();
  const { setBlueprints, setLoading } = blueprintStore.getState();
  setLoading(true);
  try {
    const blueprints = await ipc.invoke<Blueprint[]>("list_blueprints");
    setBlueprints(blueprints);
  } finally {
    setLoading(false);
  }
}

async function cloneBlueprint(id: string, newName: string): Promise<Blueprint> {
  const ipc = getIPCClient();
  const cloned = await ipc.invoke<Blueprint>("clone_blueprint", { id, newName });
  await loadBlueprints();
  return cloned;
}

async function createBlankBlueprint(name: string, description: string, icon?: string): Promise<Blueprint> {
  const ipc = getIPCClient();
  const bp = await ipc.invoke<Blueprint>("create_blank_blueprint", { name, description, icon });
  await loadBlueprints();
  return bp;
}

async function deleteBlueprint(id: string): Promise<void> {
  const ipc = getIPCClient();
  await ipc.invoke<void>("delete_blueprint", { id });
  await loadBlueprints();
}

async function assignBlueprint(nodeId: string, blueprintId: string): Promise<void> {
  const ipc = getIPCClient();
  await ipc.invoke<void>("assign_blueprint", { nodeId, blueprintId });
}

async function unassignBlueprint(nodeId: string): Promise<void> {
  const ipc = getIPCClient();
  await ipc.invoke<void>("unassign_blueprint", { nodeId });
}

async function importBlueprintPackage(path: string): Promise<Blueprint> {
  const ipc = getIPCClient();
  const bp = await ipc.invoke<Blueprint>("import_blueprint_package", { path });
  await loadBlueprints();
  return bp;
}

async function exportBlueprintPackage(blueprintId: string, outputDir: string): Promise<string> {
  const ipc = getIPCClient();
  return ipc.invoke<string>("export_blueprint_package", { blueprintId, outputDir });
}

async function init(): Promise<void> {
  const ipc = getIPCClient();
  await loadBlueprints();

  // Subscribe to blueprint events
  unlistenTaskWarning = await ipc.listen<{ node_id: string; blueprint_id: string; task_id: string; warning: string }>(
    "blueprint-task-warning",
    () => {
      // Task warnings are handled by page-level listeners for UI display.
      // Service subscription exists so a future CLI can also react.
    },
  );

  unlistenApplyProgress = await ipc.listen<{ node_id: string; blueprint_id: string; completed: number; total: number; current_task_id: string }>(
    "blueprint-apply-progress",
    () => {
      // Apply progress is consumed by page-level listeners (NodeDetailPage, BlueprintDetailPage).
    },
  );

  unlistenApplyComplete = await ipc.listen<{ node_id: string; blueprint_id: string }>(
    "blueprint-apply-complete",
    () => {
      // Trigger a refresh so the dashboard and blueprints page reflect the new state.
      loadBlueprints().catch(() => {});
    },
  );

  unlistenUninstallProgress = await ipc.listen<{ node_id: string; blueprint_id: string; completed: number; total: number; current_task_id: string }>(
    "blueprint-uninstall-progress",
    () => {
      // Consumed by page-level listeners.
    },
  );

  unlistenUninstallComplete = await ipc.listen<{ node_id: string; blueprint_id: string; uninstalled: number; skipped: number }>(
    "blueprint-uninstall-complete",
    () => {
      loadBlueprints().catch(() => {});
    },
  );
}

function destroy(): void {
  unlistenTaskWarning?.();
  unlistenTaskWarning = null;
  unlistenApplyProgress?.();
  unlistenApplyProgress = null;
  unlistenApplyComplete?.();
  unlistenApplyComplete = null;
  unlistenUninstallProgress?.();
  unlistenUninstallProgress = null;
  unlistenUninstallComplete?.();
  unlistenUninstallComplete = null;
}

export const blueprintService = {
  init,
  destroy,
  loadBlueprints,
  cloneBlueprint,
  createBlankBlueprint,
  deleteBlueprint,
  assignBlueprint,
  unassignBlueprint,
  importBlueprintPackage,
  exportBlueprintPackage,
};
