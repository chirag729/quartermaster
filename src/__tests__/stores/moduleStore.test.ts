import { describe, it, expect, beforeEach } from "vitest";
import { useTaskStore } from "../../stores/taskStore";
import type { TaskInfo } from "../../types/task";

const mockTask: TaskInfo = {
  id: "test-task",
  name: "Test Task",
  description: "A test task",
  icon: "Box",
  category: "Testing",
  tags: [],
  privilege_level: "user",
  execution_target: "any",
  depends_on: [],
  config_schema: [],
  status: "not_started",
  supports_uninstall: false,
};

describe("taskStore", () => {
  beforeEach(() => {
    useTaskStore.setState({
      tasks: [],
      loading: true,
      executing: null,
      uninstalling: null,
    });
  });

  it("starts with empty tasks and loading true", () => {
    const state = useTaskStore.getState();
    expect(state.tasks).toHaveLength(0);
    expect(state.loading).toBe(true);
  });

  it("setTasks replaces tasks list", () => {
    useTaskStore.getState().setTasks([mockTask]);
    expect(useTaskStore.getState().tasks).toHaveLength(1);
    expect(useTaskStore.getState().tasks[0].id).toBe("test-task");
  });

  it("setLoading updates loading state", () => {
    useTaskStore.getState().setLoading(false);
    expect(useTaskStore.getState().loading).toBe(false);
  });

  it("setExecuting tracks which task is running", () => {
    useTaskStore.getState().setExecuting("test-task");
    expect(useTaskStore.getState().executing).toBe("test-task");
    useTaskStore.getState().setExecuting(null);
    expect(useTaskStore.getState().executing).toBeNull();
  });

  it("updateTaskStatus changes a specific task status", () => {
    useTaskStore.getState().setTasks([mockTask]);
    useTaskStore.getState().updateTaskStatus("test-task", "completed");
    const task = useTaskStore.getState().tasks[0];
    expect(task.status).toBe("completed");
  });

  it("updateTaskStatus with error message", () => {
    useTaskStore.getState().setTasks([mockTask]);
    useTaskStore.getState().updateTaskStatus("test-task", "failed", "Something broke");
    const task = useTaskStore.getState().tasks[0];
    expect(task.status).toBe("failed");
    expect(task.error_message).toBe("Something broke");
  });

  it("updateTaskProgress sets progress values", () => {
    useTaskStore.getState().setTasks([mockTask]);
    useTaskStore.getState().updateTaskProgress("test-task", 0.5, "Downloading...");
    const task = useTaskStore.getState().tasks[0];
    expect(task._progress).toBe(0.5);
    expect(task._progressMessage).toBe("Downloading...");
  });

  it("starts with uninstalling as null", () => {
    expect(useTaskStore.getState().uninstalling).toBeNull();
  });

  it("setUninstalling tracks which task is being uninstalled", () => {
    useTaskStore.getState().setUninstalling("test-task");
    expect(useTaskStore.getState().uninstalling).toBe("test-task");
    useTaskStore.getState().setUninstalling(null);
    expect(useTaskStore.getState().uninstalling).toBeNull();
  });

  it("setUninstalling is independent from executing", () => {
    useTaskStore.getState().setExecuting("task-a");
    useTaskStore.getState().setUninstalling("task-b");
    expect(useTaskStore.getState().executing).toBe("task-a");
    expect(useTaskStore.getState().uninstalling).toBe("task-b");
  });

  it("updateTaskStatus to not_started after uninstall", () => {
    const installedTask: TaskInfo = {
      ...mockTask,
      status: "completed",
      supports_uninstall: true,
    };
    useTaskStore.getState().setTasks([installedTask]);
    useTaskStore.getState().updateTaskStatus("test-task", "not_started");
    const task = useTaskStore.getState().tasks[0];
    expect(task.status).toBe("not_started");
  });

  it("uninstalling state resets properly on completion", () => {
    useTaskStore.getState().setUninstalling("test-task");
    expect(useTaskStore.getState().uninstalling).toBe("test-task");

    // Simulate uninstall completion
    useTaskStore.getState().setUninstalling(null);
    useTaskStore.getState().updateTaskStatus("test-task", "not_started");

    expect(useTaskStore.getState().uninstalling).toBeNull();
  });

  it("supports_uninstall flag is preserved through store operations", () => {
    const uninstallableTask: TaskInfo = {
      ...mockTask,
      id: "uninstallable",
      supports_uninstall: true,
      uninstall_steps: [{ name: "Remove files", progress: 100 }],
    };
    const nonUninstallableTask: TaskInfo = {
      ...mockTask,
      id: "not-uninstallable",
      supports_uninstall: false,
    };

    useTaskStore.getState().setTasks([uninstallableTask, nonUninstallableTask]);
    const tasks = useTaskStore.getState().tasks;

    expect(tasks.find(t => t.id === "uninstallable")?.supports_uninstall).toBe(true);
    expect(tasks.find(t => t.id === "uninstallable")?.uninstall_steps).toHaveLength(1);
    expect(tasks.find(t => t.id === "not-uninstallable")?.supports_uninstall).toBe(false);
  });
});
