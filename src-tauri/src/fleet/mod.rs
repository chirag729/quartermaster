pub mod manager;
pub mod ssh_config;
pub mod status_poller;
pub mod terminal;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Node {
    pub id: String,
    pub name: String,
    pub kind: NodeKind,
    pub hostname: String,
    pub os: Option<String>,
    pub tags: Vec<String>,
    pub ssh_config: Option<SshConfig>,
    pub status: NodeStatus,
    pub last_seen: Option<DateTime<Utc>>,
    pub blueprint_id: Option<String>,
    #[serde(default)]
    pub applied_blueprint_version: Option<String>,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum NodeKind {
    Local,
    Remote,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum NodeStatus {
    Online,
    Offline,
    Connecting,
    Error,
    Unknown,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SshConfig {
    pub host: String,
    pub port: u16,
    pub username: String,
    pub auth_method: SshAuthMethod,
    pub fingerprint: Option<String>,
    pub proxy_jump: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", tag = "type")]
pub enum SshAuthMethod {
    /// Password-based authentication. Password stored encrypted in the vault.
    Password {
        /// Key into the vault where the encrypted password is stored.
        vault_key: Option<String>,
    },
    /// Key file authentication (e.g., ~/.ssh/id_ed25519).
    KeyFile { private_key_path: String },
    /// Certificate-based authentication.
    Certificate {
        certificate_path: String,
        private_key_path: String,
    },
    /// FIDO2 resident key authentication (ed25519-sk on a YubiKey).
    Fido2Resident {
        /// Application string (e.g., "ssh:quartermaster-myserver").
        application: Option<String>,
    },
    /// SSH agent forwarding (uses SSH_AUTH_SOCK).
    Agent,
}
