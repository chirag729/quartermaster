import { describe, it, expect, beforeEach } from "vitest";
import { blueprintStore } from "@quartermaster/core";
import type { Blueprint } from "@quartermaster/core";

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
    blueprintStore.setState({
      blueprints: [],
      loading: true,
      selectedBlueprintId: null,
    });
  });

  // --- Initial state ---

  it("starts with empty blueprints, loading true, and no selection", () => {
    const state = blueprintStore.getState();
    expect(state.blueprints).toHaveLength(0);
    expect(state.loading).toBe(true);
    expect(state.selectedBlueprintId).toBeNull();
  });

  // --- setBlueprints ---

  it("setBlueprints replaces the blueprints list", () => {
    const bps = [makeBlueprint(), makeBlueprint({ id: "bp-2", name: "Second" })];
    blueprintStore.getState().setBlueprints(bps);
    expect(blueprintStore.getState().blueprints).toHaveLength(2);
  });

  it("setBlueprints with an empty array clears all blueprints", () => {
    blueprintStore.getState().setBlueprints([makeBlueprint()]);
    blueprintStore.getState().setBlueprints([]);
    expect(blueprintStore.getState().blueprints).toHaveLength(0);
  });

  // --- setLoading ---

  it("setLoading updates loading state", () => {
    blueprintStore.getState().setLoading(false);
    expect(blueprintStore.getState().loading).toBe(false);
  });

  // --- setSelectedBlueprintId ---

  it("setSelectedBlueprintId selects a blueprint", () => {
    blueprintStore.getState().setSelectedBlueprintId("bp-1");
    expect(blueprintStore.getState().selectedBlueprintId).toBe("bp-1");
  });

  it("setSelectedBlueprintId with null clears selection", () => {
    blueprintStore.getState().setSelectedBlueprintId("bp-1");
    blueprintStore.getState().setSelectedBlueprintId(null);
    expect(blueprintStore.getState().selectedBlueprintId).toBeNull();
  });

  // --- addBlueprint ---

  it("addBlueprint appends a blueprint to the list", () => {
    blueprintStore.getState().addBlueprint(makeBlueprint({ id: "bp-1" }));
    blueprintStore.getState().addBlueprint(makeBlueprint({ id: "bp-2" }));
    expect(blueprintStore.getState().blueprints).toHaveLength(2);
    expect(blueprintStore.getState().blueprints[1].id).toBe("bp-2");
  });

  it("addBlueprint to an empty list creates a single-element list", () => {
    blueprintStore.getState().addBlueprint(makeBlueprint());
    expect(blueprintStore.getState().blueprints).toHaveLength(1);
  });

  // --- updateBlueprint ---

  it("updateBlueprint replaces a blueprint by id", () => {
    blueprintStore.getState().setBlueprints([makeBlueprint({ id: "bp-1", name: "Old" })]);
    blueprintStore.getState().updateBlueprint(makeBlueprint({ id: "bp-1", name: "Updated" }));
    expect(blueprintStore.getState().blueprints[0].name).toBe("Updated");
  });

  it("updateBlueprint does not affect other blueprints", () => {
    blueprintStore.getState().setBlueprints([
      makeBlueprint({ id: "bp-1", name: "First" }),
      makeBlueprint({ id: "bp-2", name: "Second" }),
    ]);
    blueprintStore.getState().updateBlueprint(makeBlueprint({ id: "bp-1", name: "Changed" }));

    const bps = blueprintStore.getState().blueprints;
    expect(bps[0].name).toBe("Changed");
    expect(bps[1].name).toBe("Second");
  });

  it("updateBlueprint with a non-existent id leaves list unchanged", () => {
    blueprintStore.getState().setBlueprints([makeBlueprint({ id: "bp-1" })]);
    blueprintStore.getState().updateBlueprint(makeBlueprint({ id: "non-existent" }));

    const bps = blueprintStore.getState().blueprints;
    expect(bps).toHaveLength(1);
    expect(bps[0].id).toBe("bp-1");
  });

  // --- removeBlueprint ---

  it("removeBlueprint removes a blueprint by id", () => {
    blueprintStore.getState().setBlueprints([
      makeBlueprint({ id: "bp-1" }),
      makeBlueprint({ id: "bp-2" }),
    ]);
    blueprintStore.getState().removeBlueprint("bp-1");

    const bps = blueprintStore.getState().blueprints;
    expect(bps).toHaveLength(1);
    expect(bps[0].id).toBe("bp-2");
  });

  it("removeBlueprint with a non-existent id leaves list unchanged", () => {
    blueprintStore.getState().setBlueprints([makeBlueprint({ id: "bp-1" })]);
    blueprintStore.getState().removeBlueprint("non-existent");
    expect(blueprintStore.getState().blueprints).toHaveLength(1);
  });

  it("removeBlueprint on an empty list does nothing", () => {
    blueprintStore.getState().removeBlueprint("bp-1");
    expect(blueprintStore.getState().blueprints).toHaveLength(0);
  });
});
