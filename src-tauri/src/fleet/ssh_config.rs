use std::path::PathBuf;

use serde::{Deserialize, Serialize};

/// A single host entry parsed from an SSH config file.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SshHostEntry {
    pub host_alias: String,
    pub hostname: String,
    pub port: u16,
    pub username: Option<String>,
    pub identity_file: Option<String>,
    pub proxy_jump: Option<String>,
}

/// Parses SSH config file content into a list of host entries.
///
/// Follows standard OpenSSH config conventions:
/// - `Host` starts a new entry block
/// - Wildcard hosts (`*`, `?`) are skipped
/// - Keys are case-insensitive
/// - If no `HostName` is specified, the `Host` alias is used as the hostname
/// - `Port` defaults to 22
pub fn parse_ssh_config(content: &str) -> Vec<SshHostEntry> {
    let mut entries: Vec<SshHostEntry> = Vec::new();
    let mut current: Option<SshHostEntry> = None;

    for line in content.lines() {
        let line = line.trim();

        // Skip empty lines and comments
        if line.is_empty() || line.starts_with('#') {
            continue;
        }

        // Split into key and value. SSH config uses whitespace or `=` as delimiter.
        let (key, value) = match line.split_once(|c: char| c.is_ascii_whitespace() || c == '=') {
            Some((k, v)) => (k.trim(), v.trim()),
            None => continue,
        };

        let key_lower = key.to_lowercase();

        if key_lower == "host" {
            // Flush the previous entry if any
            if let Some(entry) = current.take() {
                entries.push(entry);
            }

            // Skip wildcard patterns
            if value.contains('*') || value.contains('?') {
                continue;
            }

            current = Some(SshHostEntry {
                host_alias: value.to_string(),
                hostname: value.to_string(), // default: alias used as hostname
                port: 22,
                username: None,
                identity_file: None,
                proxy_jump: None,
            });
        } else if let Some(ref mut entry) = current {
            match key_lower.as_str() {
                "hostname" => {
                    entry.hostname = value.to_string();
                }
                "port" => {
                    if let Ok(p) = value.parse::<u16>() {
                        entry.port = p;
                    }
                }
                "user" => {
                    entry.username = Some(value.to_string());
                }
                "identityfile" => {
                    entry.identity_file = Some(value.to_string());
                }
                "proxyjump" => {
                    entry.proxy_jump = Some(value.to_string());
                }
                _ => {
                    // Ignore unknown directives
                }
            }
        }
    }

    // Flush the last entry
    if let Some(entry) = current.take() {
        entries.push(entry);
    }

    entries
}

/// Reads `~/.ssh/config` and returns discovered SSH host entries.
/// Returns an empty vec if the file does not exist or cannot be read.
pub fn discover_ssh_hosts() -> Vec<SshHostEntry> {
    let ssh_config_path: PathBuf = match dirs::home_dir() {
        Some(home) => home.join(".ssh").join("config"),
        None => return Vec::new(),
    };

    let content = match std::fs::read_to_string(&ssh_config_path) {
        Ok(c) => c,
        Err(_) => return Vec::new(),
    };

    parse_ssh_config(&content)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_single_host_entry() {
        let config = "\
Host myserver
    HostName 10.0.0.5
    User chirag
    Port 2222
";
        let entries = parse_ssh_config(config);
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].host_alias, "myserver");
        assert_eq!(entries[0].hostname, "10.0.0.5");
        assert_eq!(entries[0].username.as_deref(), Some("chirag"));
        assert_eq!(entries[0].port, 2222);
    }

    #[test]
    fn parse_multiple_host_entries() {
        let config = "\
Host alpha
    HostName 192.168.1.1
    User alice

Host beta
    HostName 192.168.1.2
    User bob
";
        let entries = parse_ssh_config(config);
        assert_eq!(entries.len(), 2);
        assert_eq!(entries[0].host_alias, "alpha");
        assert_eq!(entries[0].hostname, "192.168.1.1");
        assert_eq!(entries[0].username.as_deref(), Some("alice"));
        assert_eq!(entries[1].host_alias, "beta");
        assert_eq!(entries[1].hostname, "192.168.1.2");
        assert_eq!(entries[1].username.as_deref(), Some("bob"));
    }

    #[test]
    fn skip_wildcard_hosts() {
        let config = "\
Host *
    ServerAliveInterval 60

Host dev-server
    HostName 10.0.0.1

Host staging-?
    HostName 10.0.0.2
";
        let entries = parse_ssh_config(config);
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].host_alias, "dev-server");
    }

    #[test]
    fn default_port_is_22() {
        let config = "\
Host noport
    HostName example.com
";
        let entries = parse_ssh_config(config);
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].port, 22);
    }

    #[test]
    fn case_insensitive_keys() {
        let config = "\
host myhost
    hostname 10.0.0.99
    user admin
    PORT 8022
    IDENTITYFILE ~/.ssh/my_key
    proxyJump bastion
";
        let entries = parse_ssh_config(config);
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].host_alias, "myhost");
        assert_eq!(entries[0].hostname, "10.0.0.99");
        assert_eq!(entries[0].username.as_deref(), Some("admin"));
        assert_eq!(entries[0].port, 8022);
        assert_eq!(entries[0].identity_file.as_deref(), Some("~/.ssh/my_key"));
        assert_eq!(entries[0].proxy_jump.as_deref(), Some("bastion"));
    }

    #[test]
    fn host_without_hostname_uses_alias() {
        let config = "\
Host myalias
    User deploy
";
        let entries = parse_ssh_config(config);
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].host_alias, "myalias");
        assert_eq!(entries[0].hostname, "myalias");
    }

    #[test]
    fn empty_config_returns_empty_vec() {
        let entries = parse_ssh_config("");
        assert!(entries.is_empty());
    }

    #[test]
    fn handle_comments_and_blank_lines() {
        let config = "\
# This is a comment
   # Indented comment

Host commented
    # Another comment
    HostName 172.16.0.1

    User ops
";
        let entries = parse_ssh_config(config);
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].host_alias, "commented");
        assert_eq!(entries[0].hostname, "172.16.0.1");
        assert_eq!(entries[0].username.as_deref(), Some("ops"));
    }

    #[test]
    fn entry_with_all_fields_populated() {
        let config = "\
Host fullentry
    HostName 10.0.0.5
    Port 2222
    User chirag
    IdentityFile ~/.ssh/id_ed25519
    ProxyJump bastion-host
";
        let entries = parse_ssh_config(config);
        assert_eq!(entries.len(), 1);

        let entry = &entries[0];
        assert_eq!(entry.host_alias, "fullentry");
        assert_eq!(entry.hostname, "10.0.0.5");
        assert_eq!(entry.port, 2222);
        assert_eq!(entry.username.as_deref(), Some("chirag"));
        assert_eq!(
            entry.identity_file.as_deref(),
            Some("~/.ssh/id_ed25519")
        );
        assert_eq!(entry.proxy_jump.as_deref(), Some("bastion-host"));
    }

    #[test]
    fn proxy_jump_parsing() {
        let config = "\
Host jump-target
    HostName 10.10.10.10
    ProxyJump bastion1,bastion2
";
        let entries = parse_ssh_config(config);
        assert_eq!(entries.len(), 1);
        assert_eq!(
            entries[0].proxy_jump.as_deref(),
            Some("bastion1,bastion2")
        );
    }

    #[test]
    fn handles_equals_sign_delimiter() {
        let config = "\
Host equalshost
    HostName=10.0.0.42
    User=admin
    Port=3022
";
        let entries = parse_ssh_config(config);
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].hostname, "10.0.0.42");
        assert_eq!(entries[0].username.as_deref(), Some("admin"));
        assert_eq!(entries[0].port, 3022);
    }

    #[test]
    fn multiple_hosts_with_wildcard_interspersed() {
        let config = "\
Host first
    HostName 1.1.1.1

Host *
    ServerAliveInterval 60
    ServerAliveCountMax 3

Host second
    HostName 2.2.2.2
    User deploy

Host *.example.com
    User wildcard
";
        let entries = parse_ssh_config(config);
        assert_eq!(entries.len(), 2);
        assert_eq!(entries[0].host_alias, "first");
        assert_eq!(entries[0].hostname, "1.1.1.1");
        assert_eq!(entries[1].host_alias, "second");
        assert_eq!(entries[1].hostname, "2.2.2.2");
        assert_eq!(entries[1].username.as_deref(), Some("deploy"));
    }
}
