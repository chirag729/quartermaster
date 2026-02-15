export type NodeKind = "local" | "remote";
export type NodeStatus = "online" | "offline" | "connecting" | "error" | "unknown";
export type SshAuthMethod =
  | { type: "password"; vault_key?: string }
  | { type: "key_file"; private_key_path: string }
  | { type: "certificate"; certificate_path: string; private_key_path: string }
  | { type: "fido2_resident"; application?: string }
  | { type: "agent" };

export interface SshConfig {
  host: string;
  port: number;
  username: string;
  auth_method: SshAuthMethod;
  fingerprint?: string;
  proxy_jump?: string;
}

export interface Node {
  id: string;
  name: string;
  kind: NodeKind;
  hostname: string;
  os?: string;
  tags: string[];
  ssh_config?: SshConfig;
  status: NodeStatus;
  last_seen?: string;
  blueprint_id?: string;
  applied_blueprint_version?: string | null;
  created_at: string;
}

export interface SshHostEntry {
  host_alias: string;
  hostname: string;
  port: number;
  username?: string;
  identity_file?: string;
  proxy_jump?: string;
}

export interface YubiKeyInfo {
  serial: string;
  firmware: string;
  model: string;
  fido2_supported: boolean;
}

export interface Fido2Credential {
  credential_id: string;
  rp_id: string;
  user_name?: string;
  resident: boolean;
}
