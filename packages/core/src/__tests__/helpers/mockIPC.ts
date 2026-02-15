import { vi } from "vitest";
import type { IPCClient } from "../../ipc/types";
import { setIPCClient } from "../../ipc/provider";

export interface MockIPC {
  invoke: IPCClient["invoke"] & ReturnType<typeof vi.fn>;
  listen: IPCClient["listen"] & ReturnType<typeof vi.fn>;
}

/**
 * Create a fresh mock IPC client with vi.fn() for invoke and listen.
 * By default, invoke resolves to undefined and listen returns a noop unlisten.
 */
export function createMockIPC(): MockIPC {
  return {
    invoke: vi.fn().mockResolvedValue(undefined) as MockIPC["invoke"],
    listen: vi.fn().mockResolvedValue(() => {}) as MockIPC["listen"],
  };
}

/**
 * Create a mock IPC client and install it as the global IPC provider.
 * Returns the mock so tests can configure it.
 */
export function installMockIPC(): MockIPC {
  const mock = createMockIPC();
  setIPCClient(mock as unknown as IPCClient);
  return mock;
}
