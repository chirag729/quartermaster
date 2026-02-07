import { describe, it, expect, beforeEach, vi } from "vitest";
import { useBlueprintStore } from "../../stores/blueprintStore";
import type { Blueprint } from "../../types/blueprint";

// Mock the tauriCommands module so async store methods resolve with controlled data.
// The actual invoke mock (via vitest alias) returns [] which is not a valid Blueprint,
// so we mock at the service layer instead.
vi.mock("../../services/tauriCommands", () => ({
  cloneBlueprint: vi.fn(),
  createBlankBlueprint: vi.fn(),
  deleteBlueprint: vi.fn(),
  assignBlueprint: vi.fn(),
  unassignBlueprint: vi.fn(),
}));

import * as api from "../../services/tauriCommands";

const makeBlueprint = (overrides: Partial<Blueprint> = {}): Blueprint => ({
  id: "bp-1",
  name: "Default Blueprint",
  description: "A test blueprint",
  icon: "Box",
  is_builtin: false,
  version: "1.0.0",
  task_entries: [],
  created_at: "2026-01-01T00:00:00Z",
  updated_at: "2026-01-01T00:00:00Z",
  ...overrides,
});

describe("blueprintStore", () => {
  beforeEach(() => {
    useBlueprintStore.setState({
      blueprints: [],
      loading: true,
      selectedBlueprintId: null,
    });
    vi.clearAllMocks();
  });

  // --- Initial state ---

  it("starts with empty blueprints, loading true, and no selection", () => {
    const state = useBlueprintStore.getState();
    expect(state.blueprints).toHaveLength(0);
    expect(state.loading).toBe(true);
    expect(state.selectedBlueprintId).toBeNull();
  });

  // --- setBlueprints ---

  it("setBlueprints replaces the blueprints list", () => {
    const bps = [makeBlueprint(), makeBlueprint({ id: "bp-2", name: "Second" })];
    useBlueprintStore.getState().setBlueprints(bps);
    expect(useBlueprintStore.getState().blueprints).toHaveLength(2);
  });

  it("setBlueprints with an empty array clears all blueprints", () => {
    useBlueprintStore.getState().setBlueprints([makeBlueprint()]);
    useBlueprintStore.getState().setBlueprints([]);
    expect(useBlueprintStore.getState().blueprints).toHaveLength(0);
  });

  // --- setLoading ---

  it("setLoading updates loading state", () => {
    useBlueprintStore.getState().setLoading(false);
    expect(useBlueprintStore.getState().loading).toBe(false);
  });

  // --- setSelectedBlueprintId ---

  it("setSelectedBlueprintId selects a blueprint", () => {
    useBlueprintStore.getState().setSelectedBlueprintId("bp-1");
    expect(useBlueprintStore.getState().selectedBlueprintId).toBe("bp-1");
  });

  it("setSelectedBlueprintId with null clears selection", () => {
    useBlueprintStore.getState().setSelectedBlueprintId("bp-1");
    useBlueprintStore.getState().setSelectedBlueprintId(null);
    expect(useBlueprintStore.getState().selectedBlueprintId).toBeNull();
  });

  // --- addBlueprint ---

  it("addBlueprint appends a blueprint to the list", () => {
    useBlueprintStore.getState().addBlueprint(makeBlueprint({ id: "bp-1" }));
    useBlueprintStore.getState().addBlueprint(makeBlueprint({ id: "bp-2" }));
    expect(useBlueprintStore.getState().blueprints).toHaveLength(2);
    expect(useBlueprintStore.getState().blueprints[1].id).toBe("bp-2");
  });

  it("addBlueprint to an empty list creates a single-element list", () => {
    useBlueprintStore.getState().addBlueprint(makeBlueprint());
    expect(useBlueprintStore.getState().blueprints).toHaveLength(1);
  });

  // --- updateBlueprint ---

  it("updateBlueprint replaces a blueprint by id", () => {
    useBlueprintStore.getState().setBlueprints([makeBlueprint({ id: "bp-1", name: "Old" })]);
    useBlueprintStore.getState().updateBlueprint(makeBlueprint({ id: "bp-1", name: "Updated" }));
    expect(useBlueprintStore.getState().blueprints[0].name).toBe("Updated");
  });

  it("updateBlueprint does not affect other blueprints", () => {
    useBlueprintStore.getState().setBlueprints([
      makeBlueprint({ id: "bp-1", name: "First" }),
      makeBlueprint({ id: "bp-2", name: "Second" }),
    ]);
    useBlueprintStore.getState().updateBlueprint(makeBlueprint({ id: "bp-1", name: "Changed" }));

    const bps = useBlueprintStore.getState().blueprints;
    expect(bps[0].name).toBe("Changed");
    expect(bps[1].name).toBe("Second");
  });

  it("updateBlueprint with a non-existent id leaves list unchanged", () => {
    useBlueprintStore.getState().setBlueprints([makeBlueprint({ id: "bp-1" })]);
    useBlueprintStore.getState().updateBlueprint(makeBlueprint({ id: "non-existent" }));

    const bps = useBlueprintStore.getState().blueprints;
    expect(bps).toHaveLength(1);
    expect(bps[0].id).toBe("bp-1");
  });

  // --- removeBlueprint ---

  it("removeBlueprint removes a blueprint by id", () => {
    useBlueprintStore.getState().setBlueprints([
      makeBlueprint({ id: "bp-1" }),
      makeBlueprint({ id: "bp-2" }),
    ]);
    useBlueprintStore.getState().removeBlueprint("bp-1");

    const bps = useBlueprintStore.getState().blueprints;
    expect(bps).toHaveLength(1);
    expect(bps[0].id).toBe("bp-2");
  });

  it("removeBlueprint with a non-existent id leaves list unchanged", () => {
    useBlueprintStore.getState().setBlueprints([makeBlueprint({ id: "bp-1" })]);
    useBlueprintStore.getState().removeBlueprint("non-existent");
    expect(useBlueprintStore.getState().blueprints).toHaveLength(1);
  });

  it("removeBlueprint on an empty list does nothing", () => {
    useBlueprintStore.getState().removeBlueprint("bp-1");
    expect(useBlueprintStore.getState().blueprints).toHaveLength(0);
  });

  // --- cloneBlueprint (async) ---

  it("cloneBlueprint calls API and adds the cloned blueprint to the store", async () => {
    const cloned = makeBlueprint({ id: "bp-clone", name: "Cloned" });
    vi.mocked(api.cloneBlueprint).mockResolvedValue(cloned);

    useBlueprintStore.getState().setBlueprints([makeBlueprint({ id: "bp-1" })]);
    const result = await useBlueprintStore.getState().cloneBlueprint("bp-1", "Cloned");

    expect(api.cloneBlueprint).toHaveBeenCalledWith("bp-1", "Cloned");
    expect(result).toEqual(cloned);
    expect(useBlueprintStore.getState().blueprints).toHaveLength(2);
    expect(useBlueprintStore.getState().blueprints[1].id).toBe("bp-clone");
  });

  it("cloneBlueprint propagates API errors", async () => {
    vi.mocked(api.cloneBlueprint).mockRejectedValue(new Error("Clone failed"));

    await expect(
      useBlueprintStore.getState().cloneBlueprint("bp-1", "Cloned"),
    ).rejects.toThrow("Clone failed");
  });

  // --- createBlankBlueprint (async) ---

  it("createBlankBlueprint calls API and adds the new blueprint to the store", async () => {
    const created = makeBlueprint({ id: "bp-new", name: "Blank", description: "Empty" });
    vi.mocked(api.createBlankBlueprint).mockResolvedValue(created);

    const result = await useBlueprintStore.getState().createBlankBlueprint("Blank", "Empty");

    expect(api.createBlankBlueprint).toHaveBeenCalledWith("Blank", "Empty");
    expect(result).toEqual(created);
    expect(useBlueprintStore.getState().blueprints).toHaveLength(1);
    expect(useBlueprintStore.getState().blueprints[0].id).toBe("bp-new");
  });

  it("createBlankBlueprint propagates API errors", async () => {
    vi.mocked(api.createBlankBlueprint).mockRejectedValue(new Error("Create failed"));

    await expect(
      useBlueprintStore.getState().createBlankBlueprint("Test", "Desc"),
    ).rejects.toThrow("Create failed");
  });

  // --- deleteBlueprint (async) ---

  it("deleteBlueprint calls API and removes the blueprint from the store", async () => {
    vi.mocked(api.deleteBlueprint).mockResolvedValue(undefined);

    useBlueprintStore.getState().setBlueprints([
      makeBlueprint({ id: "bp-1" }),
      makeBlueprint({ id: "bp-2" }),
    ]);
    await useBlueprintStore.getState().deleteBlueprint("bp-1");

    expect(api.deleteBlueprint).toHaveBeenCalledWith("bp-1");
    expect(useBlueprintStore.getState().blueprints).toHaveLength(1);
    expect(useBlueprintStore.getState().blueprints[0].id).toBe("bp-2");
  });

  it("deleteBlueprint propagates API errors and does not remove from store", async () => {
    vi.mocked(api.deleteBlueprint).mockRejectedValue(new Error("Delete failed"));

    useBlueprintStore.getState().setBlueprints([makeBlueprint({ id: "bp-1" })]);

    await expect(
      useBlueprintStore.getState().deleteBlueprint("bp-1"),
    ).rejects.toThrow("Delete failed");

    // Blueprint should still be in the store since the API call failed
    expect(useBlueprintStore.getState().blueprints).toHaveLength(1);
  });

  // --- assignBlueprint (async) ---

  it("assignBlueprint calls the API with correct arguments", async () => {
    vi.mocked(api.assignBlueprint).mockResolvedValue(undefined);

    await useBlueprintStore.getState().assignBlueprint("node-1", "bp-1");

    expect(api.assignBlueprint).toHaveBeenCalledWith("node-1", "bp-1");
  });

  it("assignBlueprint propagates API errors", async () => {
    vi.mocked(api.assignBlueprint).mockRejectedValue(new Error("Assign failed"));

    await expect(
      useBlueprintStore.getState().assignBlueprint("node-1", "bp-1"),
    ).rejects.toThrow("Assign failed");
  });

  // --- unassignBlueprint (async) ---

  it("unassignBlueprint calls the API with correct arguments", async () => {
    vi.mocked(api.unassignBlueprint).mockResolvedValue(undefined);

    await useBlueprintStore.getState().unassignBlueprint("node-1");

    expect(api.unassignBlueprint).toHaveBeenCalledWith("node-1");
  });

  it("unassignBlueprint propagates API errors", async () => {
    vi.mocked(api.unassignBlueprint).mockRejectedValue(new Error("Unassign failed"));

    await expect(
      useBlueprintStore.getState().unassignBlueprint("node-1"),
    ).rejects.toThrow("Unassign failed");
  });
});
