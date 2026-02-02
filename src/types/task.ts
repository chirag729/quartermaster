export type TaskStatus = "not_started" | "in_progress" | "completed" | "failed" | "skipped" | "unknown";
export type PrivilegeLevel = "user" | "admin";
export type ExecutionTarget = "local_only" | "remote_only" | "any";

export interface ConfigField {
  key: string;
  label: string;
  field_type: "text" | "toggle" | "select" | "path";
  default_value: string;
  options?: string[];
  required: boolean;
}

export interface TaskInfo {
  id: string;
  name: string;
  description: string;
  icon: string;
  category: string;
  tags: string[];
  privilege_level: PrivilegeLevel;
  execution_target: ExecutionTarget;
  depends_on: string[];
  config_schema: ConfigField[];
  status: TaskStatus;
  error_message?: string;
  _progress?: number;
  _progressMessage?: string;
}

export interface TaskProgress {
  task_id: string;
  progress: number;
  message: string;
}
