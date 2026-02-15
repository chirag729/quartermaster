import type { IPCClient } from "./types";

let ipcClient: IPCClient | null = null;

export function setIPCClient(client: IPCClient): void {
  ipcClient = client;
}

export function getIPCClient(): IPCClient {
  if (!ipcClient)
    throw new Error(
      "IPC client not initialized. Call setIPCClient() first.",
    );
  return ipcClient;
}
