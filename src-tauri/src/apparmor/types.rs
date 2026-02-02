use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RiskLevel {
    Low,
    Medium,
    High,
    Critical,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ProfileMode {
    Enforce,
    Complain,
    Unconfined,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DenialEvent {
    pub id: String,
    pub timestamp: String,
    pub profile: String,
    pub operation: String,
    pub name: String,
    pub requested_mask: String,
    pub denied_mask: String,
    pub pid: u32,
    pub comm: String,
    pub raw_log: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PermissionSuggestion {
    pub id: String,
    pub denial_ids: Vec<String>,
    pub profile: String,
    pub description: String,
    pub rule_text: String,
    pub risk_level: RiskLevel,
    pub explanation: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProfileInfo {
    pub name: String,
    pub mode: ProfileMode,
    pub pid_count: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProfileDetail {
    pub name: String,
    pub mode: ProfileMode,
    pub raw_content: String,
    pub rules: Vec<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ApplyPermissionsRequest {
    pub profile: String,
    pub rules: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConsolidatedRule {
    pub rule_text: String,
    pub original_rules: Vec<String>,
    pub is_glob: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConsolidationResult {
    pub profile: String,
    pub rules: Vec<ConsolidatedRule>,
}
