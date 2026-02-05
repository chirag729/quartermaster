import { invoke } from "@tauri-apps/api/core";
import type { TaskInfo } from "../types/task";
import type { DenialEvent, PermissionSuggestion, ProfileInfo, ProfileDetail, ApplyPermissionsRequest, ConsolidationResult, ProfileTemplateInfo, ProfileTemplateConfig, SyncResultInfo } from "../types/apparmor";
import type { AppConfig } from "../types/config";
import type { Node, NodeKind, NodeStatus, SshConfig } from "../types/node";
import type { Blueprint, BlueprintTaskEntry } from "../types/blueprint";

// Task commands
export async function listTasks(): Promise<TaskInfo[]> {
  return invoke("list_tasks");
}

export async function detectAllStates(): Promise<TaskInfo[]> {
  return invoke("detect_all_states");
}

export async function executeTask(taskId: string, nodeId?: string): Promise<void> {
  return invoke("execute_task", { taskId, nodeId });
}

// AppArmor commands
export async function getDenialLogs(): Promise<{ denials: DenialEvent[]; suggestions: PermissionSuggestion[] }> {
  return invoke("get_denial_logs");
}

export async function getProfiles(): Promise<ProfileInfo[]> {
  return invoke("get_profiles");
}

export async function getProfileDetail(profileName: string): Promise<ProfileDetail> {
  return invoke("get_profile_detail", { profileName });
}

export async function applyPermissionRules(request: ApplyPermissionsRequest): Promise<void> {
  return invoke("apply_permission_rules", { request });
}

export async function consolidateRules(suggestions: PermissionSuggestion[]): Promise<ConsolidationResult[]> {
  return invoke("consolidate_rules", { suggestions });
}

export async function applyPermissionRulesBatch(requests: ApplyPermissionsRequest[]): Promise<void> {
  return invoke("apply_permission_rules_batch", { requests });
}

export async function consolidateProfileRules(profileName: string): Promise<ConsolidationResult> {
  return invoke("consolidate_profile_rules", { profileName });
}

export async function rewriteProfileRules(profileName: string, rules: string[]): Promise<void> {
  return invoke("rewrite_profile_rules", { profileName, rules });
}

export async function startLogMonitor(): Promise<void> {
  return invoke("start_log_monitor");
}

export async function stopLogMonitor(): Promise<void> {
  return invoke("stop_log_monitor");
}

// Profile template commands
export async function listProfileTemplates(): Promise<ProfileTemplateInfo[]> {
  return invoke("list_profile_templates");
}

export async function getProfileTemplate(templateId: string): Promise<ProfileTemplateInfo> {
  return invoke("get_profile_template", { templateId });
}

export async function previewProfile(templateId: string): Promise<string> {
  return invoke("preview_profile", { templateId });
}

export async function installProfileTemplate(templateId: string): Promise<void> {
  return invoke("install_profile_template", { templateId });
}

export async function uninstallProfileTemplate(templateId: string): Promise<void> {
  return invoke("uninstall_profile_template", { templateId });
}

export async function syncInstalledProfiles(): Promise<SyncResultInfo[]> {
  return invoke("sync_installed_profiles");
}

export async function getProfileTemplateConfig(templateId: string): Promise<ProfileTemplateConfig> {
  return invoke("get_profile_template_config", { templateId });
}

export async function setProfileTemplateConfig(templateId: string, configValues: Record<string, string>): Promise<void> {
  return invoke("set_profile_template_config", { templateId, configValues });
}

// Config commands
export async function getConfig(): Promise<AppConfig> {
  return invoke("get_config");
}

export async function setConfig(config: AppConfig): Promise<AppConfig> {
  return invoke("set_config", { config });
}

// System commands
export interface SystemInfo {
  os: string;
  username: string;
  hostname: string;
}

export async function getSystemInfo(): Promise<SystemInfo> {
  return invoke("get_system_info");
}

export async function checkPolkitAuth(actionId: string): Promise<boolean> {
  return invoke("check_polkit_auth", { actionId });
}

export async function isPolkitPolicyInstalled(): Promise<boolean> {
  return invoke("is_polkit_policy_installed");
}

export async function installPolkitPolicy(): Promise<void> {
  return invoke("install_polkit_policy");
}

// Fleet / Node commands
export async function listNodes(): Promise<Node[]> {
  return invoke("list_nodes");
}

export async function getNode(nodeId: string): Promise<Node> {
  return invoke("get_node", { nodeId });
}

export interface AddNodeParams {
  name: string;
  kind: NodeKind;
  hostname: string;
  tags: string[];
  ssh_config?: SshConfig;
}

export async function addNode(params: AddNodeParams): Promise<Node> {
  return invoke("add_node", { ...params });
}

export async function updateNode(node: Node): Promise<Node> {
  return invoke("update_node", { node });
}

export async function removeNode(nodeId: string): Promise<void> {
  return invoke("remove_node", { nodeId });
}

export async function getNodeStatus(nodeId: string): Promise<NodeStatus> {
  return invoke("get_node_status", { nodeId });
}

// SSH commands
export interface SshKeyInfo {
  name: string;
  path: string;
  key_type: string;
  is_fido2: boolean;
}

export async function testSshConnection(host: string, port: number, username: string): Promise<string> {
  return invoke("test_ssh_connection", { host, port, username });
}

export async function listSshKeys(): Promise<SshKeyInfo[]> {
  return invoke("list_ssh_keys");
}

export async function generateSshKey(keyType: string, comment: string): Promise<SshKeyInfo> {
  return invoke("generate_ssh_key", { keyType, comment });
}

export async function deploySshKey(publicKeyPath: string, targetHost: string, targetPort: number, targetUsername: string): Promise<void> {
  return invoke("deploy_ssh_key", { publicKeyPath, targetHost, targetPort, targetUsername });
}

// Blueprint commands
export async function listBlueprints(): Promise<Blueprint[]> {
  return invoke("list_blueprints");
}

export async function getBlueprint(blueprintId: string): Promise<Blueprint> {
  return invoke("get_blueprint", { blueprintId });
}

export interface CreateBlueprintParams {
  name: string;
  description: string;
  icon: string;
  task_entries: BlueprintTaskEntry[];
}

export async function createBlueprint(params: CreateBlueprintParams): Promise<Blueprint> {
  return invoke("create_blueprint", { ...params });
}

export async function updateBlueprint(blueprint: Blueprint): Promise<Blueprint> {
  return invoke("update_blueprint", { blueprint });
}

export async function removeBlueprint(blueprintId: string): Promise<void> {
  return invoke("remove_blueprint", { blueprintId });
}

export async function applyBlueprint(nodeId: string, blueprintId: string): Promise<void> {
  return invoke("apply_blueprint", { nodeId, blueprintId });
}

export async function cloneBlueprint(id: string, newName: string): Promise<Blueprint> {
  return invoke("clone_blueprint", { id, newName });
}

export async function createBlankBlueprint(name: string, description: string): Promise<Blueprint> {
  return invoke("create_blank_blueprint", { name, description });
}

export async function deleteBlueprint(id: string): Promise<void> {
  return invoke("delete_blueprint", { id });
}

export async function assignBlueprint(nodeId: string, blueprintId: string): Promise<void> {
  return invoke("assign_blueprint", { nodeId, blueprintId });
}

export async function unassignBlueprint(nodeId: string): Promise<void> {
  return invoke("unassign_blueprint", { nodeId });
}
