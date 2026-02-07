import { invoke } from "@tauri-apps/api/core";
import type { TaskInfo, TaskStateInfo, InstalledTaskState } from "../types/task";
import type { DenialEvent, PermissionSuggestion, ProfileInfo, ProfileDetail, ApplyPermissionsRequest, ConsolidationResult, ProfileTemplateInfo, ProfileTemplateConfig, SyncResultInfo } from "../types/apparmor";
import type { AppConfig } from "../types/config";
import type { Node, NodeKind, NodeStatus, SshConfig, SshHostEntry, YubiKeyInfo, Fido2Credential } from "../types/node";
import type { Blueprint, BlueprintTaskEntry } from "../types/blueprint";

// Task commands
export async function listTasks(): Promise<TaskInfo[]> {
  return invoke("list_tasks");
}

export async function detectAllStates(): Promise<TaskInfo[]> {
  return invoke("detect_all_states");
}

export async function executeTask(taskId: string, nodeId?: string, blueprintId?: string): Promise<void> {
  return invoke("execute_task", { taskId, nodeId, blueprintId });
}

export async function listTasksForNode(nodeId: string): Promise<TaskStateInfo[]> {
  return invoke("list_tasks_for_node", { nodeId });
}

export async function uninstallTask(taskId: string, nodeId?: string): Promise<void> {
  return invoke("uninstall_task", { taskId, nodeId });
}

export async function getInstallationStates(nodeId: string): Promise<Record<string, InstalledTaskState>> {
  return invoke("get_installation_states", { nodeId });
}

// Task update checking
export interface TaskUpdateInfo {
  task_id: string;
  task_name: string;
  defined_version: string | null;
  installed_version: string | null;
  update_available: boolean;
}

export async function checkTaskUpdates(nodeId?: string): Promise<TaskUpdateInfo[]> {
  return invoke("check_task_updates", { nodeId: nodeId ?? null });
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

export async function pollAllNodeStatuses(): Promise<[string, boolean][]> {
  return invoke("poll_all_node_statuses");
}

export async function discoverSshHosts(): Promise<SshHostEntry[]> {
  return invoke("discover_ssh_hosts_cmd");
}

export async function openNodeTerminal(nodeId: string): Promise<void> {
  return invoke("open_node_terminal", { nodeId });
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

// YubiKey / FIDO2 commands
export async function detectYubikeys(): Promise<YubiKeyInfo[]> {
  return invoke("detect_yubikeys");
}

export async function listFido2Credentials(serial?: string): Promise<Fido2Credential[]> {
  return invoke("list_fido2_credentials", { serial });
}

export async function generateFido2SshKey(keyName: string, comment: string, resident: boolean, application?: string): Promise<SshKeyInfo> {
  return invoke("generate_fido2_ssh_key", { keyName, comment, resident, application });
}

export async function isFido2Supported(): Promise<boolean> {
  return invoke("is_fido2_supported");
}

// Vault commands
export async function vaultExists(): Promise<boolean> {
  return invoke("vault_exists");
}

export async function vaultIsUnlocked(): Promise<boolean> {
  return invoke("vault_is_unlocked");
}

export async function vaultCreate(password: string): Promise<void> {
  return invoke("vault_create", { password });
}

export async function vaultUnlock(password: string): Promise<void> {
  return invoke("vault_unlock", { password });
}

export async function vaultLock(): Promise<void> {
  return invoke("vault_lock");
}

export async function vaultGet(key: string): Promise<string | null> {
  return invoke("vault_get", { key });
}

export async function vaultSet(key: string, value: string): Promise<void> {
  return invoke("vault_set", { key, value });
}

export async function vaultRemove(key: string): Promise<boolean> {
  return invoke("vault_remove", { key });
}

export async function vaultListKeys(): Promise<string[]> {
  return invoke("vault_list_keys");
}

export async function vaultChangePassword(currentPassword: string, newPassword: string): Promise<void> {
  return invoke("vault_change_password", { currentPassword, newPassword });
}

// Variable commands
export async function getSharedVariables(): Promise<Record<string, string>> {
  return invoke("get_shared_variables");
}

export async function setSharedVariable(key: string, value: string): Promise<void> {
  return invoke("set_shared_variable", { key, value });
}

export async function removeSharedVariable(key: string): Promise<void> {
  return invoke("remove_shared_variable", { key });
}

export async function getNodeVariableOverrides(nodeId: string): Promise<Record<string, string>> {
  return invoke("get_node_variable_overrides", { nodeId });
}

export async function setNodeVariableOverride(nodeId: string, key: string, value: string): Promise<void> {
  return invoke("set_node_variable_override", { nodeId, key, value });
}

export async function removeNodeVariableOverride(nodeId: string, key: string): Promise<void> {
  return invoke("remove_node_variable_override", { nodeId, key });
}

// System commands (additional)
export async function isApparmorHelperInstalled(): Promise<boolean> {
  return invoke("is_apparmor_helper_installed");
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

export interface BulkApplyResult {
  node_id: string;
  node_name: string;
  success: boolean;
  error?: string | null;
}

export async function applyBlueprintBulk(nodeIds: string[], blueprintId: string): Promise<BulkApplyResult[]> {
  return invoke("apply_blueprint_bulk", { nodeIds, blueprintId });
}

// --- Dry-Run ---

export interface DryRunAction {
  type: "run_command" | "write_file" | "create_dir";
  command?: string;
  args?: string[];
  path?: string;
  content_length?: number;
}

export interface DryRunTaskResult {
  task_id: string;
  task_name: string;
  status: "would_run" | "already_completed" | "skipped";
  actions: DryRunAction[];
}

export interface DryRunResult {
  node_id: string;
  blueprint_id: string;
  task_actions: DryRunTaskResult[];
}

export async function dryRunBlueprint(nodeId: string, blueprintId: string): Promise<DryRunResult> {
  return invoke("dry_run_blueprint", { nodeId, blueprintId });
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

export async function importBlueprintPackage(path: string): Promise<Blueprint> {
  return invoke("import_blueprint_package", { path });
}

export async function exportBlueprintPackage(blueprintId: string, outputDir: string): Promise<string> {
  return invoke("export_blueprint_package", { blueprintId, outputDir });
}

// --- Activity Log ---

export interface ActivityEntry {
  id: string;
  timestamp: string;
  action: string;
  target: string;
  detail?: string | null;
  success: boolean;
}

export async function getActivityLog(count?: number): Promise<ActivityEntry[]> {
  return invoke<ActivityEntry[]>("get_activity_log", { count: count ?? null });
}

export async function clearActivityLog(): Promise<void> {
  return invoke<void>("clear_activity_log");
}
