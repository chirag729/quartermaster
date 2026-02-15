import { describe, it, expect, beforeEach, afterEach, vi } from "vitest";
import { taskStore } from "../../stores/taskStore";
import { installMockIPC, type MockIPC } from "../helpers/mockIPC";
import type { TaskInfo } from "../../types/task";

import { taskService } from "../../services/taskService";

const makeTask = (overrides: Partial<TaskInfo> = {}): TaskInfo => ({
  id: "task-1",
  name: "Test Task",
  description: "A test task",
  icon: "terminal",
  category: "dev",
  tags: ["test"],
  privilege_level: "user",
  execution_target: "any",
  depends_on: [],
  config_schema: [],
  status: "not_started",
  supports_uninstall: true,
  ...overrides,
});

describe("taskService", () => {
  let mockIPC: MockIPC;

  beforeEach(() => {
    mockIPC = installMockIPC();
    taskStore.setState({
      tasks: [],
      loading: true,
      executing: null,
      uninstalling: null,
    });
  });

  afterEach(() => {
    taskService.destroy();
  });

  // --- loadTasks ---

  it("loadTasks sets tasks from IPC and toggles loading", async () => {
    const tasks = [makeTask(), makeTask({ id: "task-2" })];
    mockIPC.invoke.mockResolvedValueOnce(tasks);

    await taskService.loadTasks();

    expect(mockIPC.invoke).toHaveBeenCalledWith("detect_all_states");
    expect(taskStore.getState().tasks).toHaveLength(2);
    expect(taskStore.getState().loading).toBe(false);
  });

  it("loadTasks sets loading false even on error", async () => {
    mockIPC.invoke.mockRejectedValueOnce(new Error("fail"));

    await expect(taskService.loadTasks()).rejects.toThrow("fail");
    expect(taskStore.getState().loading).toBe(false);
  });

  // --- executeTask ---

  it("executeTask sets executing, calls IPC, and updates status to completed", async () => {
    taskStore.getState().setTasks([makeTask({ id: "task-1", status: "not_started" })]);
    mockIPC.invoke.mockResolvedValueOnce(undefined);

    await taskService.executeTask("task-1");

    expect(mockIPC.invoke).toHaveBeenCalledWith("execute_task", {
      taskId: "task-1",
      nodeId: undefined,
      blueprintId: undefined,
      configOverrides: undefined,
    });
    expect(taskStore.getState().tasks[0].status).toBe("completed");
    expect(taskStore.getState().executing).toBeNull();
  });

  it("executeTask sets status to failed on error and re-throws", async () => {
    taskStore.getState().setTasks([makeTask({ id: "task-1" })]);
    mockIPC.invoke.mockRejectedValueOnce(new Error("script failed"));

    await expect(taskService.executeTask("task-1")).rejects.toThrow("script failed");
    expect(taskStore.getState().tasks[0].status).toBe("failed");
    expect(taskStore.getState().tasks[0].error_message).toBe("script failed");
    expect(taskStore.getState().executing).toBeNull();
  });

  it("executeTask clears executing in finally block", async () => {
    taskStore.getState().setTasks([makeTask({ id: "task-1" })]);
    mockIPC.invoke.mockRejectedValueOnce(new Error("oops"));

    try {
      await taskService.executeTask("task-1");
    } catch {
      // expected
    }

    expect(taskStore.getState().executing).toBeNull();
  });

  it("executeTask passes optional nodeId, blueprintId, and configOverrides", async () => {
    taskStore.getState().setTasks([makeTask({ id: "task-1" })]);
    mockIPC.invoke.mockResolvedValueOnce(undefined);

    await taskService.executeTask("task-1", "node-1", "bp-1", { key: "val" });

    expect(mockIPC.invoke).toHaveBeenCalledWith("execute_task", {
      taskId: "task-1",
      nodeId: "node-1",
      blueprintId: "bp-1",
      configOverrides: { key: "val" },
    });
  });

  // --- uninstallTask ---

  it("uninstallTask invokes IPC and reloads tasks", async () => {
    const tasks = [makeTask({ id: "task-1", status: "completed" })];
    mockIPC.invoke
      .mockResolvedValueOnce(undefined) // uninstall_task
      .mockResolvedValueOnce(tasks); // detect_all_states (from loadTasks)

    await taskService.uninstallTask("task-1");

    expect(mockIPC.invoke).toHaveBeenCalledWith("uninstall_task", { taskId: "task-1" });
    expect(taskStore.getState().uninstalling).toBeNull();
  });

  it("uninstallTask clears uninstalling even on error", async () => {
    mockIPC.invoke.mockRejectedValueOnce(new Error("fail"));

    await expect(taskService.uninstallTask("task-1")).rejects.toThrow("fail");
    expect(taskStore.getState().uninstalling).toBeNull();
  });

  // --- init event subscriptions ---

  it("init subscribes to task-progress and task-state-changed events", async () => {
    await taskService.init();

    expect(mockIPC.listen).toHaveBeenCalledWith("task-progress", expect.any(Function));
    expect(mockIPC.listen).toHaveBeenCalledWith("task-state-changed", expect.any(Function));
  });

  it("task-progress event handler updates task progress in store", async () => {
    // Pre-populate store (init() no longer calls loadTasks)
    taskStore.getState().setTasks([makeTask({ id: "task-1" })]);

    // Capture the handler for task-progress
    let progressHandler: ((payload: unknown) => void) | undefined;
    mockIPC.listen.mockImplementation(async (event: string, handler: (payload: unknown) => void) => {
      if (event === "task-progress") progressHandler = handler;
      return () => {};
    });

    await taskService.init();

    // Simulate an event
    progressHandler!({ task_id: "task-1", progress: 50, message: "Halfway" });

    const task = taskStore.getState().tasks[0];
    expect(task._progress).toBe(50);
    expect(task._progressMessage).toBe("Halfway");
  });

  it("task-state-changed event handler updates task status in store", async () => {
    // Pre-populate store (init() no longer calls loadTasks)
    taskStore.getState().setTasks([makeTask({ id: "task-1", status: "not_started" })]);

    let stateHandler: ((payload: unknown) => void) | undefined;
    mockIPC.listen.mockImplementation(async (event: string, handler: (payload: unknown) => void) => {
      if (event === "task-state-changed") stateHandler = handler;
      return () => {};
    });

    await taskService.init();

    stateHandler!({ task_id: "task-1", status: "completed" });

    expect(taskStore.getState().tasks[0].status).toBe("completed");
  });

  // --- destroy ---

  it("destroy calls unlisten functions", async () => {
    const unlistenProgress = vi.fn();
    const unlistenState = vi.fn();
    mockIPC.invoke.mockResolvedValueOnce([]); // detect_all_states
    mockIPC.listen
      .mockResolvedValueOnce(unlistenProgress)
      .mockResolvedValueOnce(unlistenState);

    await taskService.init();
    taskService.destroy();

    expect(unlistenProgress).toHaveBeenCalled();
    expect(unlistenState).toHaveBeenCalled();
  });
});
