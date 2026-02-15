export interface TaskProgressEvent {
  task_id: string;
  node_id?: string;
  progress: number;
  message: string;
}

export interface AppArmorDenialEvent {
  denial: import("./apparmor").DenialEvent;
}

export interface TaskStateChangedEvent {
  task_id: string;
  node_id?: string;
  status: import("./task").TaskStatus;
}

export interface BlueprintApplyProgressEvent {
  node_id: string;
  blueprint_id: string;
  completed: number;
  total: number;
  current_task_id: string;
}

export interface BlueprintApplyCompleteEvent {
  node_id: string;
  blueprint_id: string;
}

export interface TaskOutputEvent {
  node_id: string;
  task_id: string;
  step_index: number;
  step_total: number;
  step_name: string;
  command: string;
  stdout: string;
  stderr: string;
  exit_code: number;
  duration_ms: number;
}

export interface BlueprintTaskWarningEvent {
  node_id: string;
  blueprint_id: string;
  task_id: string;
  warning: string;
}

export interface BlueprintUninstallProgressEvent {
  node_id: string;
  blueprint_id: string;
  completed: number;
  total: number;
  current_task_id: string;
}

export interface BlueprintUninstallCompleteEvent {
  node_id: string;
  blueprint_id: string;
  uninstalled: number;
  skipped: number;
}

export interface BulkBlueprintProgressEvent {
  completed: number;
  total: number;
  current_node_id: string;
  current_node_name: string;
}
