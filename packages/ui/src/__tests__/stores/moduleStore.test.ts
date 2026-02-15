import { describe, it, expect, beforeEach } from "vitest";
import { taskStore } from "@quartermaster/core";
import type { TaskInfo } from "@quartermaster/core";

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
    taskStore.setState({
      tasks: [],
      loading: true,
      executing: null,
      uninstalling: null,
    });
  });

  it("starts with empty tasks and loading true", () => {
    const state = taskStore.getState();
    expect(state.tasks).toHaveLength(0);
    expect(state.loading).toBe(true);
  });

  it("setTasks replaces tasks list", () => {
    taskStore.getState().setTasks([mockTask]);
    expect(taskStore.getState().tasks).toHaveLength(1);
    expect(taskStore.getState().tasks[0].id).toBe("test-task");
  });

  it("setLoading updates loading state", () => {
    taskStore.getState().setLoading(false);
    expect(taskStore.getState().loading).toBe(false);
  });

  it("setExecuting tracks which task is running", () => {
    taskStore.getState().setExecuting("test-task");
    expect(taskStore.getState().executing).toBe("test-task");
    taskStore.getState().setExecuting(null);
    expect(taskStore.getState().executing).toBeNull();
  });

  it("updateTaskStatus changes a specific task status", () => {
    taskStore.getState().setTasks([mockTask]);
    taskStore.getState().updateTaskStatus("test-task", "completed");
    const task = taskStore.getState().tasks[0];
    expect(task.status).toBe("completed");
  });

  it("updateTaskStatus with error message", () => {
    taskStore.getState().setTasks([mockTask]);
    taskStore.getState().updateTaskStatus("test-task", "failed", "Something broke");
    const task = taskStore.getState().tasks[0];
    expect(task.status).toBe("failed");
    expect(task.error_message).toBe("Something broke");
  });

  it("updateTaskProgress sets progress values", () => {
    taskStore.getState().setTasks([mockTask]);
    taskStore.getState().updateTaskProgress("test-task", 0.5, "Downloading...");
    const task = taskStore.getState().tasks[0];
    expect(task._progress).toBe(0.5);
    expect(task._progressMessage).toBe("Downloading...");
  });

  it("starts with uninstalling as null", () => {
    expect(taskStore.getState().uninstalling).toBeNull();
  });

  it("setUninstalling tracks which task is being uninstalled", () => {
    taskStore.getState().setUninstalling("test-task");
    expect(taskStore.getState().uninstalling).toBe("test-task");
    taskStore.getState().setUninstalling(null);
    expect(taskStore.getState().uninstalling).toBeNull();
  });

  it("setUninstalling is independent from executing", () => {
    taskStore.getState().setExecuting("task-a");
    taskStore.getState().setUninstalling("task-b");
    expect(taskStore.getState().executing).toBe("task-a");
    expect(taskStore.getState().uninstalling).toBe("task-b");
  });

  it("updateTaskStatus to not_started after uninstall", () => {
    const installedTask: TaskInfo = {
      ...mockTask,
      status: "completed",
      supports_uninstall: true,
    };
    taskStore.getState().setTasks([installedTask]);
    taskStore.getState().updateTaskStatus("test-task", "not_started");
    const task = taskStore.getState().tasks[0];
    expect(task.status).toBe("not_started");
  });

  it("uninstalling state resets properly on completion", () => {
    taskStore.getState().setUninstalling("test-task");
    expect(taskStore.getState().uninstalling).toBe("test-task");

    // Simulate uninstall completion
    taskStore.getState().setUninstalling(null);
    taskStore.getState().updateTaskStatus("test-task", "not_started");

    expect(taskStore.getState().uninstalling).toBeNull();
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

    taskStore.getState().setTasks([uninstallableTask, nonUninstallableTask]);
    const tasks = taskStore.getState().tasks;

    expect(tasks.find(t => t.id === "uninstallable")?.supports_uninstall).toBe(true);
    expect(tasks.find(t => t.id === "uninstallable")?.uninstall_steps).toHaveLength(1);
    expect(tasks.find(t => t.id === "not-uninstallable")?.supports_uninstall).toBe(false);
  });
});
