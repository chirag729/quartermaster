import { describe, it, expect, beforeEach } from "vitest";
import { useFleetStore } from "../../stores/fleetStore";
import type { Node, NodeStatus } from "../../types/node";

const makeNode = (overrides: Partial<Node> = {}): Node => ({
  id: "node-1",
  name: "Test Node",
  kind: "remote",
  hostname: "192.168.1.10",
  os: "Ubuntu 24.04",
  tags: ["dev"],
  status: "online",
  created_at: "2026-01-01T00:00:00Z",
  ...overrides,
});

describe("fleetStore", () => {
  beforeEach(() => {
    useFleetStore.setState({
      nodes: [],
      loading: true,
      selectedNodeId: null,
    });
  });

  // --- Initial state ---

  it("starts with empty nodes, loading true, and no selection", () => {
    const state = useFleetStore.getState();
    expect(state.nodes).toHaveLength(0);
    expect(state.loading).toBe(true);
    expect(state.selectedNodeId).toBeNull();
  });

  // --- setNodes ---

  it("setNodes replaces the nodes list", () => {
    const nodes = [makeNode(), makeNode({ id: "node-2", name: "Second" })];
    useFleetStore.getState().setNodes(nodes);
    expect(useFleetStore.getState().nodes).toHaveLength(2);
    expect(useFleetStore.getState().nodes[0].id).toBe("node-1");
    expect(useFleetStore.getState().nodes[1].id).toBe("node-2");
  });

  it("setNodes with an empty array clears all nodes", () => {
    useFleetStore.getState().setNodes([makeNode()]);
    expect(useFleetStore.getState().nodes).toHaveLength(1);

    useFleetStore.getState().setNodes([]);
    expect(useFleetStore.getState().nodes).toHaveLength(0);
  });

  it("setNodes replaces previous nodes entirely", () => {
    useFleetStore.getState().setNodes([makeNode({ id: "old" })]);
    useFleetStore.getState().setNodes([makeNode({ id: "new" })]);

    const nodes = useFleetStore.getState().nodes;
    expect(nodes).toHaveLength(1);
    expect(nodes[0].id).toBe("new");
  });

  // --- setLoading ---

  it("setLoading updates loading to false", () => {
    useFleetStore.getState().setLoading(false);
    expect(useFleetStore.getState().loading).toBe(false);
  });

  it("setLoading updates loading to true", () => {
    useFleetStore.getState().setLoading(false);
    useFleetStore.getState().setLoading(true);
    expect(useFleetStore.getState().loading).toBe(true);
  });

  // --- setSelectedNodeId ---

  it("setSelectedNodeId selects a node", () => {
    useFleetStore.getState().setSelectedNodeId("node-1");
    expect(useFleetStore.getState().selectedNodeId).toBe("node-1");
  });

  it("setSelectedNodeId with null clears selection", () => {
    useFleetStore.getState().setSelectedNodeId("node-1");
    useFleetStore.getState().setSelectedNodeId(null);
    expect(useFleetStore.getState().selectedNodeId).toBeNull();
  });

  it("setSelectedNodeId allows selecting a non-existent node id", () => {
    // The store does not validate IDs; it simply stores the value
    useFleetStore.getState().setSelectedNodeId("does-not-exist");
    expect(useFleetStore.getState().selectedNodeId).toBe("does-not-exist");
  });

  // --- updateNodeStatus ---

  it("updateNodeStatus updates the status of an existing node", () => {
    useFleetStore.getState().setNodes([makeNode({ id: "node-1", status: "online" })]);
    useFleetStore.getState().updateNodeStatus("node-1", "offline");
    expect(useFleetStore.getState().nodes[0].status).toBe("offline");
  });

  it("updateNodeStatus does not affect other nodes", () => {
    useFleetStore.getState().setNodes([
      makeNode({ id: "node-1", status: "online" }),
      makeNode({ id: "node-2", status: "online" }),
    ]);
    useFleetStore.getState().updateNodeStatus("node-1", "error");

    const nodes = useFleetStore.getState().nodes;
    expect(nodes[0].status).toBe("error");
    expect(nodes[1].status).toBe("online");
  });

  it("updateNodeStatus with a non-existent node id leaves nodes unchanged", () => {
    const original = [makeNode({ id: "node-1", status: "online" })];
    useFleetStore.getState().setNodes(original);
    useFleetStore.getState().updateNodeStatus("non-existent", "offline");

    const nodes = useFleetStore.getState().nodes;
    expect(nodes).toHaveLength(1);
    expect(nodes[0].status).toBe("online");
  });

  it("updateNodeStatus on an empty nodes array does nothing", () => {
    useFleetStore.getState().updateNodeStatus("node-1", "offline");
    expect(useFleetStore.getState().nodes).toHaveLength(0);
  });

  it("updateNodeStatus preserves all other node properties", () => {
    const node = makeNode({
      id: "node-1",
      name: "My Node",
      kind: "remote",
      hostname: "example.com",
      os: "Fedora 40",
      tags: ["prod", "gpu"],
      status: "connecting",
    });
    useFleetStore.getState().setNodes([node]);
    useFleetStore.getState().updateNodeStatus("node-1", "online");

    const updated = useFleetStore.getState().nodes[0];
    expect(updated.status).toBe("online");
    expect(updated.name).toBe("My Node");
    expect(updated.kind).toBe("remote");
    expect(updated.hostname).toBe("example.com");
    expect(updated.os).toBe("Fedora 40");
    expect(updated.tags).toEqual(["prod", "gpu"]);
  });

  it("updateNodeStatus cycles through all status values", () => {
    useFleetStore.getState().setNodes([makeNode({ id: "node-1" })]);
    const statuses: NodeStatus[] = ["online", "offline", "connecting", "error", "unknown"];

    for (const status of statuses) {
      useFleetStore.getState().updateNodeStatus("node-1", status);
      expect(useFleetStore.getState().nodes[0].status).toBe(status);
    }
  });
});
