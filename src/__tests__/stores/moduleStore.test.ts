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
});
