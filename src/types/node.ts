export type NodeKind = "local" | "remote";
export type NodeStatus = "online" | "offline" | "connecting" | "error" | "unknown";
export type SshAuthMethod =
  | { type: "key_file"; private_key_path: string }
  | { type: "certificate"; certificate_path: string; private_key_path: string }
  | { type: "fido2"; key_handle: string }
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
  created_at: string;
}
