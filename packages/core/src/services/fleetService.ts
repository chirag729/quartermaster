import { getIPCClient } from "../ipc/provider";
import { fleetStore } from "../stores/fleetStore";
import type { Node, NodeStatus, SshHostEntry } from "../types/node";

// Timer functions declared for environments without DOM lib
declare function setInterval(callback: () => void, ms: number): number;
declare function clearInterval(id: number): void;

const POLL_INTERVAL_MS = 30_000;

let pollTimer: number | null = null;

interface AddNodeParams {
  name: string;
  kind: "local" | "remote";
  hostname: string;
  tags: string[];
  sshConfig?: {
    host: string;
    port: number;
    username: string;
    auth_method: Record<string, unknown>;
    proxy_jump?: string;
  };
}

async function loadNodes(): Promise<void> {
  const ipc = getIPCClient();
  const { setNodes, setLoading } = fleetStore.getState();
  setLoading(true);
  try {
    const nodes = await ipc.invoke<Node[]>("list_nodes");
    setNodes(nodes);
  } finally {
    setLoading(false);
  }
}

async function pollStatuses(): Promise<void> {
  const ipc = getIPCClient();
  const { updateNodeStatus } = fleetStore.getState();
  try {
    const results = await ipc.invoke<[string, boolean][]>("poll_all_node_statuses");
    for (const [nodeId, isOnline] of results) {
      const status: NodeStatus = isOnline ? "online" : "offline";
      updateNodeStatus(nodeId, status);
    }
  } catch {
    // Silently fail polling — not worth surfacing every 30s
  }
}

function startPolling(): void {
  if (pollTimer) return;
  pollStatuses();
  pollTimer = setInterval(pollStatuses, POLL_INTERVAL_MS);
}

function stopPolling(): void {
  if (pollTimer) {
    clearInterval(pollTimer);
    pollTimer = null;
  }
}

async function addNode(params: AddNodeParams): Promise<Node> {
  const ipc = getIPCClient();
  const node = await ipc.invoke<Node>("add_node", params as unknown as Record<string, unknown>);
  await loadNodes();
  return node;
}

async function removeNode(nodeId: string): Promise<void> {
  const ipc = getIPCClient();
  await ipc.invoke<void>("remove_node", { nodeId });
  await loadNodes();
}

async function updateNode(node: Node): Promise<Node> {
  const ipc = getIPCClient();
  const updated = await ipc.invoke<Node>("update_node", { node });
  await loadNodes();
  return updated;
}

async function discoverSshHosts(): Promise<SshHostEntry[]> {
  const ipc = getIPCClient();
  return ipc.invoke<SshHostEntry[]>("discover_ssh_hosts_cmd");
}

async function openTerminal(nodeId: string): Promise<void> {
  const ipc = getIPCClient();
  await ipc.invoke<void>("open_node_terminal", { nodeId });
}

async function init(): Promise<void> {
  await loadNodes();
  startPolling();
}

function destroy(): void {
  stopPolling();
}

export const fleetService = {
  init,
  destroy,
  loadNodes,
  pollStatuses,
  startPolling,
  stopPolling,
  addNode,
  removeNode,
  updateNode,
  discoverSshHosts,
  openTerminal,
};
