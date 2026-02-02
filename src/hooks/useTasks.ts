import { useEffect, useCallback } from "react";
import { useTaskStore } from "../stores/taskStore";
import { useToastStore } from "../stores/toastStore";
import { useTauriEvent } from "./useTauriEvent";
import * as api from "../services/tauriCommands";
import type { TaskProgressEvent, TaskStateChangedEvent } from "../types/events";

export function useTasks() {
  const { tasks, loading, executing, setTasks, setLoading, setExecuting, updateTaskStatus, updateTaskProgress } = useTaskStore();
  const { addToast } = useToastStore();

  useEffect(() => {
    loadTasks();
  }, []);

  const loadTasks = async () => {
    setLoading(true);
    try {
      const result = await api.detectAllStates();
      setTasks(result);
    } catch (err) {
      addToast({ type: "error", title: "Failed to load tasks", message: String(err) });
    } finally {
      setLoading(false);
    }
  };

  const executeTask = async (taskId: string) => {
    setExecuting(taskId);
    updateTaskStatus(taskId, "in_progress");
    try {
      await api.executeTask(taskId);
      updateTaskStatus(taskId, "completed");
      addToast({ type: "success", title: "Task completed", message: `${taskId} finished successfully.` });
    } catch (err) {
      updateTaskStatus(taskId, "failed", String(err));
      addToast({ type: "error", title: "Task failed", message: String(err) });
    } finally {
      setExecuting(null);
    }
  };

  const onProgress = useCallback((payload: TaskProgressEvent) => {
    updateTaskProgress(payload.task_id, payload.progress, payload.message);
  }, [updateTaskProgress]);

  const onStateChanged = useCallback((payload: TaskStateChangedEvent) => {
    updateTaskStatus(payload.task_id, payload.status);
  }, [updateTaskStatus]);

  useTauriEvent("task-progress", onProgress);
  useTauriEvent("task-state-changed", onStateChanged);

  return { tasks, loading, executing, executeTask, refreshTasks: loadTasks };
}
