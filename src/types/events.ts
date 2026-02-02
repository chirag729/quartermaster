export interface TaskProgressEvent {
  task_id: string;
  progress: number;
  message: string;
}

export interface AppArmorDenialEvent {
  denial: import("./apparmor").DenialEvent;
}

export interface TaskStateChangedEvent {
  task_id: string;
  status: import("./task").TaskStatus;
}

export interface NodeStatusChangedEvent {
  node_id: string;
  status: string;
  last_seen: string | null;
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
  results: Record<string, string>;
}
