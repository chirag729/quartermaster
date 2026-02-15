export type RiskLevel = "low" | "medium" | "high" | "critical";
export type ProfileMode = "enforce" | "complain" | "unconfined";

export interface DenialEvent {
  id: string;
  timestamp: string;
  profile: string;
  operation: string;
  name: string;
  requested_mask: string;
  denied_mask: string;
  pid: number;
  comm: string;
  raw_log: string;
}

export interface PermissionSuggestion {
  id: string;
  denial_ids: string[];
  profile: string;
  description: string;
  rule_text: string;
  risk_level: RiskLevel;
  explanation: string;
}

export interface ProfileInfo {
  name: string;
  mode: ProfileMode;
  pid_count: number;
}

export interface ProfileDetail {
  name: string;
  mode: ProfileMode;
  raw_content: string;
  rules: string[];
}

export interface ApplyPermissionsRequest {
  profile: string;
  rules: string[];
}

export interface ConsolidatedRule {
  rule_text: string;
  original_rules: string[];
  is_glob: boolean;
}

export interface ConsolidationResult {
  profile: string;
  rules: ConsolidatedRule[];
}

// Profile template types

export type ProfileTemplateStatus = "not_installed" | "installed" | "stale" | "task_not_completed";

export interface ProfileVariable {
  key: string;
  label: string;
  var_type: string;
  default: string;
  options?: string[];
}

export interface ProfileTemplateInfo {
  id: string;
  name: string;
  description: string;
  task_id: string;
  profile_name: string;
  mode: string;
  variables: ProfileVariable[];
  subscribes_to: string[];
  status: ProfileTemplateStatus;
}

export interface ResolvedConfigField {
  key: string;
  label: string;
  field_type: string;
  value: string;
  default: string;
  options?: string[];
  source: "task" | "profile";
}

export interface ProfileTemplateConfig {
  template_id: string;
  fields: ResolvedConfigField[];
}

export interface SyncResultInfo {
  profile_id: string;
  profile_name: string;
  updated: boolean;
}
