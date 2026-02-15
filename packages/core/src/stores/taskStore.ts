import { createStore } from "zustand/vanilla";
import type { TaskInfo, TaskStatus } from "../types/task";

export interface TaskState {
  tasks: TaskInfo[];
  loading: boolean;
  executing: string | null;
  uninstalling: string | null;
  setTasks: (tasks: TaskInfo[]) => void;
  setLoading: (loading: boolean) => void;
  setExecuting: (taskId: string | null) => void;
  setUninstalling: (taskId: string | null) => void;
  updateTaskStatus: (taskId: string, status: TaskStatus, error?: string) => void;
  updateTaskProgress: (taskId: string, progress: number, message: string) => void;
}

export const taskStore = createStore<TaskState>()((set) => ({
  tasks: [],
  loading: false,
  executing: null,
  uninstalling: null,
  setTasks: (tasks) => set({ tasks }),
  setLoading: (loading) => set({ loading }),
  setExecuting: (taskId) => set({ executing: taskId }),
  setUninstalling: (taskId) => set({ uninstalling: taskId }),
  updateTaskStatus: (taskId, status, error) =>
    set((state) => ({
      tasks: state.tasks.map((t) =>
        t.id === taskId ? { ...t, status, error_message: error } : t,
      ),
    })),
  updateTaskProgress: (taskId, progress, message) =>
    set((state) => ({
      tasks: state.tasks.map((t) =>
        t.id === taskId ? { ...t, _progress: progress, _progressMessage: message } : t,
      ),
    })),
}));
