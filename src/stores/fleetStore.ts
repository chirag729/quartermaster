import { create } from "zustand";
import type { Node, NodeStatus } from "../types/node";

interface FleetState {
  nodes: Node[];
  loading: boolean;
  selectedNodeId: string | null;
  setNodes: (nodes: Node[]) => void;
  setLoading: (loading: boolean) => void;
  setSelectedNodeId: (id: string | null) => void;
  updateNodeStatus: (nodeId: string, status: NodeStatus) => void;
}

export const useFleetStore = create<FleetState>((set) => ({
  nodes: [],
  loading: true,
  selectedNodeId: null,
  setNodes: (nodes) => set({ nodes }),
  setLoading: (loading) => set({ loading }),
  setSelectedNodeId: (id) => set({ selectedNodeId: id }),
  updateNodeStatus: (nodeId, status) =>
    set((state) => ({
      nodes: state.nodes.map((n) =>
        n.id === nodeId ? { ...n, status } : n,
      ),
    })),
}));
