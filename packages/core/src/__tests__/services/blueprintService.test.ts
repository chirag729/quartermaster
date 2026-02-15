import { describe, it, expect, beforeEach, afterEach, vi } from "vitest";
import { blueprintStore } from "../../stores/blueprintStore";
import { installMockIPC, type MockIPC } from "../helpers/mockIPC";
import type { Blueprint } from "../../types/blueprint";

import { blueprintService } from "../../services/blueprintService";

const makeBlueprint = (overrides: Partial<Blueprint> = {}): Blueprint => ({
  id: "bp-1",
  name: "Test Blueprint",
  description: "A test blueprint",
  icon: "package",
  is_builtin: false,
  version: "1.0.0",
  task_entries: [],
  created_at: "2026-01-01T00:00:00Z",
  updated_at: "2026-01-01T00:00:00Z",
  ...overrides,
});

describe("blueprintService", () => {
  let mockIPC: MockIPC;

  beforeEach(() => {
    mockIPC = installMockIPC();
    blueprintStore.setState({
      blueprints: [],
      loading: true,
      selectedBlueprintId: null,
    });
  });

  afterEach(() => {
    blueprintService.destroy();
  });

  // --- loadBlueprints ---

  it("loadBlueprints sets blueprints from IPC and toggles loading", async () => {
    const bps = [makeBlueprint(), makeBlueprint({ id: "bp-2" })];
    mockIPC.invoke.mockResolvedValueOnce(bps);

    await blueprintService.loadBlueprints();

    expect(mockIPC.invoke).toHaveBeenCalledWith("list_blueprints");
    expect(blueprintStore.getState().blueprints).toHaveLength(2);
    expect(blueprintStore.getState().loading).toBe(false);
  });

  it("loadBlueprints sets loading false even on error", async () => {
    mockIPC.invoke.mockRejectedValueOnce(new Error("fail"));

    await expect(blueprintService.loadBlueprints()).rejects.toThrow("fail");
    expect(blueprintStore.getState().loading).toBe(false);
  });

  // --- cloneBlueprint ---

  it("cloneBlueprint invokes IPC and reloads blueprints", async () => {
    const cloned = makeBlueprint({ id: "bp-clone" });
    mockIPC.invoke
      .mockResolvedValueOnce(cloned) // clone_blueprint
      .mockResolvedValueOnce([cloned]); // list_blueprints

    const result = await blueprintService.cloneBlueprint("bp-1", "Cloned BP");

    expect(mockIPC.invoke).toHaveBeenCalledWith("clone_blueprint", { id: "bp-1", newName: "Cloned BP" });
    expect(result.id).toBe("bp-clone");
  });

  // --- createBlankBlueprint ---

  it("createBlankBlueprint invokes IPC and reloads blueprints", async () => {
    const created = makeBlueprint({ id: "bp-new", name: "New BP" });
    mockIPC.invoke
      .mockResolvedValueOnce(created) // create_blank_blueprint
      .mockResolvedValueOnce([created]); // list_blueprints

    const result = await blueprintService.createBlankBlueprint("New BP", "desc", "icon");

    expect(mockIPC.invoke).toHaveBeenCalledWith("create_blank_blueprint", {
      name: "New BP",
      description: "desc",
      icon: "icon",
    });
    expect(result.name).toBe("New BP");
  });

  // --- deleteBlueprint ---

  it("deleteBlueprint invokes IPC and reloads blueprints", async () => {
    mockIPC.invoke
      .mockResolvedValueOnce(undefined) // delete_blueprint
      .mockResolvedValueOnce([]); // list_blueprints

    await blueprintService.deleteBlueprint("bp-1");

    expect(mockIPC.invoke).toHaveBeenCalledWith("delete_blueprint", { id: "bp-1" });
    expect(blueprintStore.getState().blueprints).toHaveLength(0);
  });

  // --- assignBlueprint / unassignBlueprint ---

  it("assignBlueprint invokes IPC without reloading blueprints", async () => {
    mockIPC.invoke.mockResolvedValueOnce(undefined);

    await blueprintService.assignBlueprint("node-1", "bp-1");

    expect(mockIPC.invoke).toHaveBeenCalledWith("assign_blueprint", { nodeId: "node-1", blueprintId: "bp-1" });
    // Should NOT have called list_blueprints
    expect(mockIPC.invoke).toHaveBeenCalledTimes(1);
  });

  it("unassignBlueprint invokes IPC without reloading blueprints", async () => {
    mockIPC.invoke.mockResolvedValueOnce(undefined);

    await blueprintService.unassignBlueprint("node-1");

    expect(mockIPC.invoke).toHaveBeenCalledWith("unassign_blueprint", { nodeId: "node-1" });
    expect(mockIPC.invoke).toHaveBeenCalledTimes(1);
  });

  // --- importBlueprintPackage ---

  it("importBlueprintPackage invokes IPC and reloads blueprints", async () => {
    const imported = makeBlueprint({ id: "bp-imported" });
    mockIPC.invoke
      .mockResolvedValueOnce(imported) // import_blueprint_package
      .mockResolvedValueOnce([imported]); // list_blueprints

    const result = await blueprintService.importBlueprintPackage("/tmp/bp.qmbp");

    expect(mockIPC.invoke).toHaveBeenCalledWith("import_blueprint_package", { path: "/tmp/bp.qmbp" });
    expect(result.id).toBe("bp-imported");
  });

  // --- exportBlueprintPackage ---

  it("exportBlueprintPackage invokes IPC and returns the path", async () => {
    mockIPC.invoke.mockResolvedValueOnce("/tmp/exported.qmbp");

    const result = await blueprintService.exportBlueprintPackage("bp-1", "/tmp");

    expect(mockIPC.invoke).toHaveBeenCalledWith("export_blueprint_package", {
      blueprintId: "bp-1",
      outputDir: "/tmp",
    });
    expect(result).toBe("/tmp/exported.qmbp");
  });

  // --- init event subscriptions ---

  it("init subscribes to 5 blueprint events", async () => {
    mockIPC.invoke.mockResolvedValueOnce([]); // list_blueprints

    await blueprintService.init();

    expect(mockIPC.listen).toHaveBeenCalledWith("blueprint-task-warning", expect.any(Function));
    expect(mockIPC.listen).toHaveBeenCalledWith("blueprint-apply-progress", expect.any(Function));
    expect(mockIPC.listen).toHaveBeenCalledWith("blueprint-apply-complete", expect.any(Function));
    expect(mockIPC.listen).toHaveBeenCalledWith("blueprint-uninstall-progress", expect.any(Function));
    expect(mockIPC.listen).toHaveBeenCalledWith("blueprint-uninstall-complete", expect.any(Function));
  });

  it("blueprint-apply-complete event handler triggers loadBlueprints", async () => {
    mockIPC.invoke.mockResolvedValue([]); // list_blueprints for all calls

    let completeHandler: ((payload: unknown) => void) | undefined;
    mockIPC.listen.mockImplementation(async (event: string, handler: (payload: unknown) => void) => {
      if (event === "blueprint-apply-complete") completeHandler = handler;
      return () => {};
    });

    await blueprintService.init();
    const callCountBeforeEvent = mockIPC.invoke.mock.calls.filter(
      (c) => c[0] === "list_blueprints",
    ).length;

    // Simulate the event
    completeHandler!({ node_id: "n1", blueprint_id: "bp-1" });

    // Wait for the async loadBlueprints call
    await vi.waitFor(() => {
      const callCountAfterEvent = mockIPC.invoke.mock.calls.filter(
        (c) => c[0] === "list_blueprints",
      ).length;
      expect(callCountAfterEvent).toBeGreaterThan(callCountBeforeEvent);
    });
  });

  // --- destroy ---

  it("destroy calls all 5 unlisten functions", async () => {
    const unlistens = [vi.fn(), vi.fn(), vi.fn(), vi.fn(), vi.fn()];
    mockIPC.invoke.mockResolvedValueOnce([]); // list_blueprints
    for (const fn of unlistens) {
      mockIPC.listen.mockResolvedValueOnce(fn);
    }

    await blueprintService.init();
    blueprintService.destroy();

    for (const fn of unlistens) {
      expect(fn).toHaveBeenCalled();
    }
  });
});
