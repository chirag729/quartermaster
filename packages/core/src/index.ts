// IPC abstraction
export type { IPCClient } from "./ipc/types";
export { setIPCClient, getIPCClient } from "./ipc/provider";

// Stores (vanilla — use store.getState() in services, useStore(store) in React)
export { fleetStore } from "./stores/fleetStore";
export type { FleetState } from "./stores/fleetStore";
export { taskStore } from "./stores/taskStore";
export type { TaskState } from "./stores/taskStore";
export { blueprintStore } from "./stores/blueprintStore";
export type { BlueprintState } from "./stores/blueprintStore";
export { appArmorStore } from "./stores/appArmorStore";
export type { AppArmorState } from "./stores/appArmorStore";

// Types
export type * from "./types/node";
export type * from "./types/task";
export type * from "./types/blueprint";
export type * from "./types/apparmor";
export type * from "./types/config";
export type * from "./types/events";

// Services
export { fleetService } from "./services/fleetService";
export { taskService } from "./services/taskService";
export { blueprintService } from "./services/blueprintService";
export { appArmorService } from "./services/appArmorService";
export { initializeServices, destroyServices } from "./services/serviceInit";

// Lib
export { formatError } from "./lib/formatError";
export { getInstallChain, getUninstallChain } from "./lib/taskDependencies";
