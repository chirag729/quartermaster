import { describe, it, expect, vi } from "vitest";

describe("IPC provider", () => {
  it("throws when getIPCClient is called before setIPCClient", async () => {
    // Reset modules to get a fresh ipcClient = null
    vi.resetModules();
    const { getIPCClient } = await import("../../ipc/provider");

    expect(() => getIPCClient()).toThrow(
      "IPC client not initialized. Call setIPCClient() first.",
    );
  });

  it("setIPCClient and getIPCClient round-trip", async () => {
    vi.resetModules();
    const { setIPCClient, getIPCClient } = await import("../../ipc/provider");
    const { createMockIPC } = await import("../helpers/mockIPC");

    const mock = createMockIPC();
    setIPCClient(mock);
    expect(getIPCClient()).toBe(mock);
  });

  it("setIPCClient replaces a previously set client", async () => {
    vi.resetModules();
    const { setIPCClient, getIPCClient } = await import("../../ipc/provider");
    const { createMockIPC } = await import("../helpers/mockIPC");

    const first = createMockIPC();
    const second = createMockIPC();
    setIPCClient(first);
    expect(getIPCClient()).toBe(first);
    setIPCClient(second);
    expect(getIPCClient()).toBe(second);
    expect(getIPCClient()).not.toBe(first);
  });
});
