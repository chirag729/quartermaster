import { useStore } from "zustand";
import {
  fleetStore,
  taskStore,
  blueprintStore,
  appArmorStore,
  type FleetState,
  type TaskState,
  type BlueprintState,
  type AppArmorState,
} from "@quartermaster/core";

// Fleet store
export function useFleetStore(): FleetState;
export function useFleetStore<T>(selector: (state: FleetState) => T): T;
export function useFleetStore<T>(selector?: (state: FleetState) => T) {
  return useStore(fleetStore, selector as (state: FleetState) => T);
}

// Task store
export function useTaskStore(): TaskState;
export function useTaskStore<T>(selector: (state: TaskState) => T): T;
export function useTaskStore<T>(selector?: (state: TaskState) => T) {
  return useStore(taskStore, selector as (state: TaskState) => T);
}

// Blueprint store
export function useBlueprintStore(): BlueprintState;
export function useBlueprintStore<T>(selector: (state: BlueprintState) => T): T;
export function useBlueprintStore<T>(selector?: (state: BlueprintState) => T) {
  return useStore(blueprintStore, selector as (state: BlueprintState) => T);
}

// AppArmor store
export function useAppArmorStore(): AppArmorState;
export function useAppArmorStore<T>(selector: (state: AppArmorState) => T): T;
export function useAppArmorStore<T>(selector?: (state: AppArmorState) => T) {
  return useStore(appArmorStore, selector as (state: AppArmorState) => T);
}
