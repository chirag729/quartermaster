import { getIPCClient } from "../ipc/provider";
import { taskStore } from "../stores/taskStore";
import type { TaskInfo, TaskStatus } from "../types/task";

let unlistenProgress: (() => void) | null = null;
let unlistenStateChanged: (() => void) | null = null;

async function loadTasks(): Promise<void> {
  const ipc = getIPCClient();
  const { setTasks, setLoading } = taskStore.getState();
  setLoading(true);
  try {
    const tasks = await ipc.invoke<TaskInfo[]>("detect_all_states");
    setTasks(tasks);
  } finally {
    setLoading(false);
  }
}

async function executeTask(
  taskId: string,
  nodeId?: string,
  blueprintId?: string,
  configOverrides?: Record<string, unknown>,
): Promise<void> {
  const ipc = getIPCClient();
  const { setExecuting, updateTaskStatus } = taskStore.getState();
  setExecuting(taskId);
  updateTaskStatus(taskId, "in_progress");
  try {
    await ipc.invoke<void>("execute_task", { taskId, nodeId, blueprintId, configOverrides });
    updateTaskStatus(taskId, "completed");
  } catch (err) {
    const message = err instanceof Error ? err.message : String(err);
    updateTaskStatus(taskId, "failed", message);
    throw err;
  } finally {
    taskStore.getState().setExecuting(null);
  }
}

async function uninstallTask(taskId: string): Promise<void> {
  const ipc = getIPCClient();
  taskStore.getState().setUninstalling(taskId);
  try {
    await ipc.invoke<void>("uninstall_task", { taskId });
    await loadTasks(); // Refresh states after uninstall
  } finally {
    taskStore.getState().setUninstalling(null);
  }
}

async function init(): Promise<void> {
  const ipc = getIPCClient();
  // Do NOT call loadTasks() here — it runs detect_all_states which iterates
  // the entire task registry (expensive with 100s+ tasks). The Task Library
  // page calls loadTasks() when mounted instead.

  // Subscribe to global task events
  unlistenProgress = await ipc.listen<{ task_id: string; progress: number; message: string }>(
    "task-progress",
    (payload) => {
      taskStore.getState().updateTaskProgress(payload.task_id, payload.progress, payload.message);
    },
  );

  unlistenStateChanged = await ipc.listen<{ task_id: string; status: TaskStatus }>(
    "task-state-changed",
    (payload) => {
      taskStore.getState().updateTaskStatus(payload.task_id, payload.status);
    },
  );
}

function destroy(): void {
  unlistenProgress?.();
  unlistenProgress = null;
  unlistenStateChanged?.();
  unlistenStateChanged = null;
}

export const taskService = {
  init,
  destroy,
  loadTasks,
  executeTask,
  uninstallTask,
};
