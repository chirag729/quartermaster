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

export interface StepInfo {
  name: string;
  progress: number;
}

export interface DownloadInfo {
  url: string;
  extract: string;
  checksum_sha256?: string;
}

export interface DesktopInfo {
  name: string;
  exec: string;
  categories: string[];
}

export interface AppArmorInfo {
  profile: string;
  abstractions: string[];
}

export interface FragmentInfo {
  tags: string[];
  content: string;
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
  supports_uninstall: boolean;
  installed_version?: string;
  update_available?: boolean;
  // V2 fields
  version?: string;
  variables?: string[];
  download?: DownloadInfo;
  desktop?: DesktopInfo;
  apparmor?: AppArmorInfo;
  steps?: StepInfo[];
  uninstall_steps?: StepInfo[];
  fragments?: FragmentInfo[];
  // Frontend-only transient fields
  _progress?: number;
  _progressMessage?: string;
}

export interface TaskProgress {
  task_id: string;
  progress: number;
  message: string;
}

/** Extended task info for a specific node, including drift detection. */
export interface TaskStateInfo extends TaskInfo {
  config_drifted: boolean;
  version_changed: boolean;
  installed_at?: string;
  installed_version?: string;
}

/** Installation record for a task on a specific node. */
export interface InstalledTaskState {
  task_id: string;
  node_id: string;
  installed_at: string;
  version?: string;
  blueprint_id?: string;
  config_hash: string;
}
