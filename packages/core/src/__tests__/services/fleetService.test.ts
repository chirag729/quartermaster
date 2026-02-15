import { describe, it, expect, beforeEach, afterEach, vi } from "vitest";
import { fleetStore } from "../../stores/fleetStore";
import { installMockIPC, type MockIPC } from "../helpers/mockIPC";
import type { Node } from "../../types/node";

// Must import after mocks are set up
import { fleetService } from "../../services/fleetService";

const makeNode = (overrides: Partial<Node> = {}): Node => ({
  id: "node-1",
  name: "Test Node",
  kind: "remote",
  hostname: "192.168.1.10",
  tags: ["dev"],
  status: "online",
  created_at: "2026-01-01T00:00:00Z",
  ...overrides,
});

describe("fleetService", () => {
  let mockIPC: MockIPC;

  beforeEach(() => {
    vi.useFakeTimers();
    mockIPC = installMockIPC();
    fleetStore.setState({
      nodes: [],
      loading: true,
      selectedNodeId: null,
      selectedNodeIds: [],
    });
  });

  afterEach(() => {
    fleetService.destroy();
    vi.useRealTimers();
  });

  // --- loadNodes ---

  it("loadNodes sets nodes from IPC and toggles loading", async () => {
    const nodes = [makeNode(), makeNode({ id: "node-2" })];
    mockIPC.invoke.mockResolvedValueOnce(nodes);

    await fleetService.loadNodes();

    expect(mockIPC.invoke).toHaveBeenCalledWith("list_nodes");
    expect(fleetStore.getState().nodes).toHaveLength(2);
    expect(fleetStore.getState().loading).toBe(false);
  });

  it("loadNodes sets loading false even on error", async () => {
    mockIPC.invoke.mockRejectedValueOnce(new Error("network error"));

    await expect(fleetService.loadNodes()).rejects.toThrow("network error");
    expect(fleetStore.getState().loading).toBe(false);
  });

  // --- pollStatuses ---

  it("pollStatuses updates node statuses from IPC results", async () => {
    fleetStore.getState().setNodes([
      makeNode({ id: "n1", status: "unknown" }),
      makeNode({ id: "n2", status: "unknown" }),
    ]);
    mockIPC.invoke.mockResolvedValueOnce([
      ["n1", true],
      ["n2", false],
    ]);

    await fleetService.pollStatuses();

    expect(mockIPC.invoke).toHaveBeenCalledWith("poll_all_node_statuses");
    expect(fleetStore.getState().nodes[0].status).toBe("online");
    expect(fleetStore.getState().nodes[1].status).toBe("offline");
  });

  it("pollStatuses silently catches errors", async () => {
    mockIPC.invoke.mockRejectedValueOnce(new Error("timeout"));
    // Should not throw
    await fleetService.pollStatuses();
  });

  // --- startPolling / stopPolling ---

  it("startPolling calls pollStatuses immediately", async () => {
    mockIPC.invoke.mockResolvedValue([]);
    fleetService.startPolling();

    // The immediate call happens synchronously, then awaits
    expect(mockIPC.invoke).toHaveBeenCalledWith("poll_all_node_statuses");

    fleetService.stopPolling();
  });

  it("startPolling is idempotent — calling twice does not create two timers", () => {
    mockIPC.invoke.mockResolvedValue([]);
    fleetService.startPolling();
    fleetService.startPolling();

    // Advance one interval
    vi.advanceTimersByTime(30_000);

    // pollStatuses should have been called: once immediately + once from interval = 2
    // NOT 3 (which would happen with two timers: 1 immediate + 1 immediate + 1 interval)
    const pollCalls = mockIPC.invoke.mock.calls.filter(
      (c) => c[0] === "poll_all_node_statuses",
    );
    expect(pollCalls.length).toBe(2);

    fleetService.stopPolling();
  });

  it("stopPolling clears the interval", () => {
    mockIPC.invoke.mockResolvedValue([]);
    fleetService.startPolling();
    fleetService.stopPolling();

    vi.advanceTimersByTime(60_000);

    // Only the immediate call from startPolling, no interval calls
    const pollCalls = mockIPC.invoke.mock.calls.filter(
      (c) => c[0] === "poll_all_node_statuses",
    );
    expect(pollCalls.length).toBe(1);
  });

  // --- addNode ---

  it("addNode invokes add_node and reloads nodes", async () => {
    const newNode = makeNode({ id: "new-1" });
    mockIPC.invoke
      .mockResolvedValueOnce(newNode) // add_node
      .mockResolvedValueOnce([newNode]); // list_nodes (from loadNodes)

    const result = await fleetService.addNode({
      name: "New Node",
      kind: "remote",
      hostname: "10.0.0.1",
      tags: [],
    });

    expect(result.id).toBe("new-1");
    expect(mockIPC.invoke).toHaveBeenCalledWith("add_node", expect.objectContaining({ name: "New Node" }));
    expect(fleetStore.getState().nodes).toHaveLength(1);
  });

  // --- removeNode ---

  it("removeNode invokes remove_node and reloads nodes", async () => {
    mockIPC.invoke
      .mockResolvedValueOnce(undefined) // remove_node
      .mockResolvedValueOnce([]); // list_nodes

    await fleetService.removeNode("node-1");

    expect(mockIPC.invoke).toHaveBeenCalledWith("remove_node", { nodeId: "node-1" });
    expect(fleetStore.getState().nodes).toHaveLength(0);
  });

  // --- updateNode ---

  it("updateNode invokes update_node and reloads nodes", async () => {
    const updated = makeNode({ id: "node-1", name: "Updated" });
    mockIPC.invoke
      .mockResolvedValueOnce(updated) // update_node
      .mockResolvedValueOnce([updated]); // list_nodes

    const result = await fleetService.updateNode(updated);

    expect(result.name).toBe("Updated");
    expect(mockIPC.invoke).toHaveBeenCalledWith("update_node", { node: updated });
  });

  // --- discoverSshHosts ---

  it("discoverSshHosts returns SSH host entries", async () => {
    const hosts = [{ host_alias: "myhost", hostname: "10.0.0.5", port: 22 }];
    mockIPC.invoke.mockResolvedValueOnce(hosts);

    const result = await fleetService.discoverSshHosts();

    expect(mockIPC.invoke).toHaveBeenCalledWith("discover_ssh_hosts_cmd");
    expect(result).toEqual(hosts);
  });

  // --- init / destroy lifecycle ---

  it("init loads nodes and starts polling", async () => {
    const nodes = [makeNode()];
    mockIPC.invoke
      .mockResolvedValueOnce(nodes) // list_nodes (from loadNodes)
      .mockResolvedValue([]); // poll_all_node_statuses

    await fleetService.init();

    expect(fleetStore.getState().nodes).toHaveLength(1);
    // Verify polling was started by checking for poll call
    expect(mockIPC.invoke).toHaveBeenCalledWith("poll_all_node_statuses");
  });
});
