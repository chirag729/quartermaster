import { describe, it, expect, beforeEach } from "vitest";
import { fleetStore } from "@quartermaster/core";
import type { Node, NodeStatus } from "@quartermaster/core";

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
    fleetStore.setState({
      nodes: [],
      loading: true,
      selectedNodeId: null,
      selectedNodeIds: [],
    });
  });

  // --- Initial state ---

  it("starts with empty nodes, loading true, and no selection", () => {
    const state = fleetStore.getState();
    expect(state.nodes).toHaveLength(0);
    expect(state.loading).toBe(true);
    expect(state.selectedNodeId).toBeNull();
  });

  // --- setNodes ---

  it("setNodes replaces the nodes list", () => {
    const nodes = [makeNode(), makeNode({ id: "node-2", name: "Second" })];
    fleetStore.getState().setNodes(nodes);
    expect(fleetStore.getState().nodes).toHaveLength(2);
    expect(fleetStore.getState().nodes[0].id).toBe("node-1");
    expect(fleetStore.getState().nodes[1].id).toBe("node-2");
  });

  it("setNodes with an empty array clears all nodes", () => {
    fleetStore.getState().setNodes([makeNode()]);
    expect(fleetStore.getState().nodes).toHaveLength(1);

    fleetStore.getState().setNodes([]);
    expect(fleetStore.getState().nodes).toHaveLength(0);
  });

  it("setNodes replaces previous nodes entirely", () => {
    fleetStore.getState().setNodes([makeNode({ id: "old" })]);
    fleetStore.getState().setNodes([makeNode({ id: "new" })]);

    const nodes = fleetStore.getState().nodes;
    expect(nodes).toHaveLength(1);
    expect(nodes[0].id).toBe("new");
  });

  // --- setLoading ---

  it("setLoading updates loading to false", () => {
    fleetStore.getState().setLoading(false);
    expect(fleetStore.getState().loading).toBe(false);
  });

  it("setLoading updates loading to true", () => {
    fleetStore.getState().setLoading(false);
    fleetStore.getState().setLoading(true);
    expect(fleetStore.getState().loading).toBe(true);
  });

  // --- setSelectedNodeId ---

  it("setSelectedNodeId selects a node", () => {
    fleetStore.getState().setSelectedNodeId("node-1");
    expect(fleetStore.getState().selectedNodeId).toBe("node-1");
  });

  it("setSelectedNodeId with null clears selection", () => {
    fleetStore.getState().setSelectedNodeId("node-1");
    fleetStore.getState().setSelectedNodeId(null);
    expect(fleetStore.getState().selectedNodeId).toBeNull();
  });

  it("setSelectedNodeId allows selecting a non-existent node id", () => {
    // The store does not validate IDs; it simply stores the value
    fleetStore.getState().setSelectedNodeId("does-not-exist");
    expect(fleetStore.getState().selectedNodeId).toBe("does-not-exist");
  });

  // --- updateNodeStatus ---

  it("updateNodeStatus updates the status of an existing node", () => {
    fleetStore.getState().setNodes([makeNode({ id: "node-1", status: "online" })]);
    fleetStore.getState().updateNodeStatus("node-1", "offline");
    expect(fleetStore.getState().nodes[0].status).toBe("offline");
  });

  it("updateNodeStatus does not affect other nodes", () => {
    fleetStore.getState().setNodes([
      makeNode({ id: "node-1", status: "online" }),
      makeNode({ id: "node-2", status: "online" }),
    ]);
    fleetStore.getState().updateNodeStatus("node-1", "error");

    const nodes = fleetStore.getState().nodes;
    expect(nodes[0].status).toBe("error");
    expect(nodes[1].status).toBe("online");
  });

  it("updateNodeStatus with a non-existent node id leaves nodes unchanged", () => {
    const original = [makeNode({ id: "node-1", status: "online" })];
    fleetStore.getState().setNodes(original);
    fleetStore.getState().updateNodeStatus("non-existent", "offline");

    const nodes = fleetStore.getState().nodes;
    expect(nodes).toHaveLength(1);
    expect(nodes[0].status).toBe("online");
  });

  it("updateNodeStatus on an empty nodes array does nothing", () => {
    fleetStore.getState().updateNodeStatus("node-1", "offline");
    expect(fleetStore.getState().nodes).toHaveLength(0);
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
    fleetStore.getState().setNodes([node]);
    fleetStore.getState().updateNodeStatus("node-1", "online");

    const updated = fleetStore.getState().nodes[0];
    expect(updated.status).toBe("online");
    expect(updated.name).toBe("My Node");
    expect(updated.kind).toBe("remote");
    expect(updated.hostname).toBe("example.com");
    expect(updated.os).toBe("Fedora 40");
    expect(updated.tags).toEqual(["prod", "gpu"]);
  });

  it("updateNodeStatus cycles through all status values", () => {
    fleetStore.getState().setNodes([makeNode({ id: "node-1" })]);
    const statuses: NodeStatus[] = ["online", "offline", "connecting", "error", "unknown"];

    for (const status of statuses) {
      fleetStore.getState().updateNodeStatus("node-1", status);
      expect(fleetStore.getState().nodes[0].status).toBe(status);
    }
  });

  // --- toggleNodeSelection ---

  it("toggleNodeSelection adds a node to the selection", () => {
    fleetStore.getState().toggleNodeSelection("node-1");
    expect(fleetStore.getState().selectedNodeIds).toEqual(["node-1"]);
  });

  it("toggleNodeSelection removes a node that is already selected", () => {
    fleetStore.getState().toggleNodeSelection("node-1");
    fleetStore.getState().toggleNodeSelection("node-1");
    expect(fleetStore.getState().selectedNodeIds).toEqual([]);
  });

  it("toggleNodeSelection preserves other selections", () => {
    fleetStore.getState().toggleNodeSelection("node-1");
    fleetStore.getState().toggleNodeSelection("node-2");
    expect(fleetStore.getState().selectedNodeIds).toEqual(["node-1", "node-2"]);

    fleetStore.getState().toggleNodeSelection("node-1");
    expect(fleetStore.getState().selectedNodeIds).toEqual(["node-2"]);
  });

  // --- selectAllNodes ---

  it("selectAllNodes replaces the selection with the given IDs", () => {
    fleetStore.getState().selectAllNodes(["node-1", "node-2", "node-3"]);
    expect(fleetStore.getState().selectedNodeIds).toEqual(["node-1", "node-2", "node-3"]);
  });

  // --- clearSelection ---

  it("clearSelection empties the selectedNodeIds array", () => {
    fleetStore.getState().selectAllNodes(["node-1", "node-2"]);
    fleetStore.getState().clearSelection();
    expect(fleetStore.getState().selectedNodeIds).toEqual([]);
  });
});
