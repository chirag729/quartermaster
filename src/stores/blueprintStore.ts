import { create } from "zustand";
import type { Blueprint } from "../types/blueprint";
import * as api from "../services/tauriCommands";

interface BlueprintState {
  blueprints: Blueprint[];
  loading: boolean;
  selectedBlueprintId: string | null;
  setBlueprints: (blueprints: Blueprint[]) => void;
  setLoading: (loading: boolean) => void;
  setSelectedBlueprintId: (id: string | null) => void;
  addBlueprint: (blueprint: Blueprint) => void;
  updateBlueprint: (blueprint: Blueprint) => void;
  removeBlueprint: (id: string) => void;
  cloneBlueprint: (id: string, newName: string) => Promise<Blueprint>;
  createBlankBlueprint: (name: string, description: string) => Promise<Blueprint>;
  deleteBlueprint: (id: string) => Promise<void>;
  assignBlueprint: (nodeId: string, blueprintId: string) => Promise<void>;
  unassignBlueprint: (nodeId: string) => Promise<void>;
  importBlueprint: (path: string) => Promise<Blueprint>;
  exportBlueprint: (blueprintId: string, outputDir: string) => Promise<string>;
}

export const useBlueprintStore = create<BlueprintState>((set) => ({
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
  cloneBlueprint: async (id, newName) => {
    const cloned = await api.cloneBlueprint(id, newName);
    set((state) => ({
      blueprints: [...state.blueprints, cloned],
    }));
    return cloned;
  },
  createBlankBlueprint: async (name, description) => {
    const bp = await api.createBlankBlueprint(name, description);
    set((state) => ({
      blueprints: [...state.blueprints, bp],
    }));
    return bp;
  },
  deleteBlueprint: async (id) => {
    await api.deleteBlueprint(id);
    set((state) => ({
      blueprints: state.blueprints.filter((b) => b.id !== id),
    }));
  },
  assignBlueprint: async (nodeId, blueprintId) => {
    await api.assignBlueprint(nodeId, blueprintId);
  },
  unassignBlueprint: async (nodeId) => {
    await api.unassignBlueprint(nodeId);
  },
  importBlueprint: async (path) => {
    const bp = await api.importBlueprintPackage(path);
    set((state) => ({
      blueprints: [...state.blueprints, bp],
    }));
    return bp;
  },
  exportBlueprint: async (blueprintId, outputDir) => {
    return api.exportBlueprintPackage(blueprintId, outputDir);
  },
}));
