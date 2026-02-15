import { createStore } from "zustand/vanilla";
import type { Node, NodeStatus } from "../types/node";

export interface FleetState {
  nodes: Node[];
  loading: boolean;
  selectedNodeId: string | null;
  selectedNodeIds: string[];
  setNodes: (nodes: Node[]) => void;
  setLoading: (loading: boolean) => void;
  setSelectedNodeId: (id: string | null) => void;
  updateNodeStatus: (nodeId: string, status: NodeStatus) => void;
  toggleNodeSelection: (nodeId: string) => void;
  selectAllNodes: (nodeIds: string[]) => void;
  clearSelection: () => void;
}

export const fleetStore = createStore<FleetState>()((set) => ({
  nodes: [],
  loading: true,
  selectedNodeId: null,
  selectedNodeIds: [],
  setNodes: (nodes) => set({ nodes }),
  setLoading: (loading) => set({ loading }),
  setSelectedNodeId: (id) => set({ selectedNodeId: id }),
  updateNodeStatus: (nodeId, status) =>
    set((state) => ({
      nodes: state.nodes.map((n) =>
        n.id === nodeId ? { ...n, status } : n,
      ),
    })),
  toggleNodeSelection: (nodeId) =>
    set((state) => ({
      selectedNodeIds: state.selectedNodeIds.includes(nodeId)
        ? state.selectedNodeIds.filter((id) => id !== nodeId)
        : [...state.selectedNodeIds, nodeId],
    })),
  selectAllNodes: (nodeIds) => set({ selectedNodeIds: nodeIds }),
  clearSelection: () => set({ selectedNodeIds: [] }),
}));
