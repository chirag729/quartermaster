import { describe, it, expect, beforeEach, vi } from "vitest";

// Mock all 4 service modules before importing serviceInit
vi.mock("../../services/fleetService", () => ({
  fleetService: {
    init: vi.fn().mockResolvedValue(undefined),
    destroy: vi.fn(),
  },
}));
vi.mock("../../services/taskService", () => ({
  taskService: {
    init: vi.fn().mockResolvedValue(undefined),
    destroy: vi.fn(),
  },
}));
vi.mock("../../services/blueprintService", () => ({
  blueprintService: {
    init: vi.fn().mockResolvedValue(undefined),
    destroy: vi.fn(),
  },
}));
vi.mock("../../services/appArmorService", () => ({
  appArmorService: {
    init: vi.fn().mockResolvedValue(undefined),
    destroy: vi.fn(),
  },
}));

import { initializeServices, destroyServices } from "../../services/serviceInit";
import { fleetService } from "../../services/fleetService";
import { taskService } from "../../services/taskService";
import { blueprintService } from "../../services/blueprintService";
import { appArmorService } from "../../services/appArmorService";

describe("serviceInit", () => {
  beforeEach(() => {
    vi.clearAllMocks();
    // Reset the module's initialized flag by calling destroy
    destroyServices();
  });

  it("initializeServices calls all 4 service inits in order", async () => {
    const order: string[] = [];
    vi.mocked(fleetService.init).mockImplementation(async () => { order.push("fleet"); });
    vi.mocked(taskService.init).mockImplementation(async () => { order.push("task"); });
    vi.mocked(blueprintService.init).mockImplementation(async () => { order.push("blueprint"); });
    vi.mocked(appArmorService.init).mockImplementation(async () => { order.push("apparmor"); });

    await initializeServices();

    expect(order).toEqual(["fleet", "task", "blueprint", "apparmor"]);
  });

  it("initializeServices is idempotent — second call is a no-op", async () => {
    await initializeServices();
    await initializeServices();

    expect(fleetService.init).toHaveBeenCalledTimes(1);
    expect(taskService.init).toHaveBeenCalledTimes(1);
  });

  it("destroyServices calls all 4 service destroy methods", async () => {
    await initializeServices();
    destroyServices();

    expect(fleetService.destroy).toHaveBeenCalled();
    expect(taskService.destroy).toHaveBeenCalled();
    expect(blueprintService.destroy).toHaveBeenCalled();
    expect(appArmorService.destroy).toHaveBeenCalled();
  });

  it("destroyServices is idempotent — second call is a no-op", async () => {
    await initializeServices();
    destroyServices();
    destroyServices();

    expect(fleetService.destroy).toHaveBeenCalledTimes(1);
  });

  it("destroyServices resets flag so re-init is possible", async () => {
    await initializeServices();
    destroyServices();
    vi.clearAllMocks();

    await initializeServices();

    expect(fleetService.init).toHaveBeenCalledTimes(1);
  });

  it("initializeServices resets flag on error so retry is possible", async () => {
    vi.mocked(blueprintService.init).mockRejectedValueOnce(new Error("bp init failed"));

    await expect(initializeServices()).rejects.toThrow("bp init failed");

    // Reset mock for retry
    vi.mocked(blueprintService.init).mockResolvedValueOnce(undefined);
    vi.clearAllMocks();

    // Retry should work because flag was reset
    await initializeServices();
    expect(fleetService.init).toHaveBeenCalledTimes(1);
  });
});
