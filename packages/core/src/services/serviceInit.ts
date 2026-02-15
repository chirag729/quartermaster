import { fleetService } from "./fleetService";
import { taskService } from "./taskService";
import { blueprintService } from "./blueprintService";
import { appArmorService } from "./appArmorService";

let initialized = false;

/**
 * Initializes all app-level services sequentially.
 * Call once from the root UI component on mount.
 * Fleet → Task → Blueprint → AppArmor (later phases).
 */
export async function initializeServices(): Promise<void> {
  if (initialized) return;

  try {
    // Phase 1: Fleet (loads nodes, starts polling)
    await fleetService.init();

    // Phase 2: Task (loads tasks, subscribes to progress/state events)
    await taskService.init();

    // Phase 3: Blueprint (loads blueprints, subscribes to apply/uninstall events)
    await blueprintService.init();

    // Phase 4: AppArmor (loads denials/profiles, subscribes to denial events)
    await appArmorService.init();

    initialized = true;
  } catch (err) {
    // Reset so retry is possible after a failed init
    initialized = false;
    throw err;
  }
}

/**
 * Tears down all services. Call on unmount.
 */
export function destroyServices(): void {
  if (!initialized) return;
  initialized = false;

  fleetService.destroy();
  taskService.destroy();
  blueprintService.destroy();
  appArmorService.destroy();
}
