import { createStore } from "zustand/vanilla";
import type { Blueprint } from "../types/blueprint";

export interface BlueprintState {
  blueprints: Blueprint[];
  loading: boolean;
  selectedBlueprintId: string | null;
  setBlueprints: (blueprints: Blueprint[]) => void;
  setLoading: (loading: boolean) => void;
  setSelectedBlueprintId: (id: string | null) => void;
  addBlueprint: (blueprint: Blueprint) => void;
  updateBlueprint: (blueprint: Blueprint) => void;
  removeBlueprint: (id: string) => void;
}

export const blueprintStore = createStore<BlueprintState>()((set) => ({
  blueprints: [],
  loading: true,
  selectedBlueprintId: null,
  setBlueprints: (blueprints) => set({ blueprints }),
  setLoading: (loading) => set({ loading }),
  setSelectedBlueprintId: (id) => set({ selectedBlueprintId: id }),
  addBlueprint: (blueprint) =>
    set((state) => ({
      blueprints: [...state.blueprints, blueprint],
    })),
  updateBlueprint: (blueprint) =>
    set((state) => ({
      blueprints: state.blueprints.map((b) =>
        b.id === blueprint.id ? blueprint : b,
      ),
    })),
  removeBlueprint: (id) =>
    set((state) => ({
      blueprints: state.blueprints.filter((b) => b.id !== id),
    })),
}));
