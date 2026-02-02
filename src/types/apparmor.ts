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
