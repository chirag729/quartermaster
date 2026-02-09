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
    parse_ssh_config_inner(content, None)
}

/// Inner parser that supports Include directive resolution.
/// `ssh_dir` is needed to resolve relative Include paths. If `None`, Include
/// directives are silently skipped (e.g. when parsing snippets in tests).
fn parse_ssh_config_inner(content: &str, ssh_dir: Option<&std::path::Path>) -> Vec<SshHostEntry> {
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

        if key_lower == "include" {
            // Resolve Include directives if we know the ssh directory
            if let Some(dir) = ssh_dir {
                let pattern = if value.starts_with('/') || value.starts_with('~') {
                    // Expand ~ to home dir
                    if let Some(rest) = value.strip_prefix("~/") {
                        if let Some(home) = dirs::home_dir() {
                            home.join(rest).to_string_lossy().to_string()
                        } else {
                            continue;
                        }
                    } else {
                        value.to_string()
                    }
                } else {
                    // Relative to ~/.ssh/
                    dir.join(value).to_string_lossy().to_string()
                };

                // Glob expand the pattern
                if let Ok(paths) = glob::glob(&pattern) {
                    for path in paths.flatten() {
                        if let Ok(included) = std::fs::read_to_string(&path) {
                            // Flush current before including
                            if let Some(entry) = current.take() {
                                entries.push(entry);
                            }
                            let included_entries = parse_ssh_config_inner(&included, ssh_dir);
                            entries.extend(included_entries);
                        }
                    }
                }
            }
            continue;
        }

        if key_lower == "host" {
            // Flush the previous entry if any
            if let Some(entry) = current.take() {
                entries.push(entry);
            }

            // For multi-alias Host lines, use the first non-wildcard alias
            let aliases: Vec<&str> = value.split_whitespace().collect();
            let alias = aliases
                .iter()
                .find(|a| !a.contains('*') && !a.contains('?'))
                .copied();

            // Skip if all aliases are wildcards
            let alias = match alias {
                Some(a) => a,
                None => continue,
            };

            current = Some(SshHostEntry {
                host_alias: alias.to_string(),
                hostname: alias.to_string(), // default: alias used as hostname
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
                    // Keep first IdentityFile occurrence (SSH uses first match)
                    if entry.identity_file.is_none() {
                        entry.identity_file = Some(value.to_string());
                    }
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
    let home = match dirs::home_dir() {
        Some(home) => home,
        None => return Vec::new(),
    };
    let ssh_dir = home.join(".ssh");
    let ssh_config_path = ssh_dir.join("config");

    let content = match std::fs::read_to_string(&ssh_config_path) {
        Ok(c) => c,
        Err(_) => return Vec::new(),
    };

    parse_ssh_config_inner(&content, Some(&ssh_dir))
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

    #[test]
    fn multi_alias_host_uses_first_non_wildcard() {
        let config = "\
Host myserver myalias *.internal
    HostName 10.0.0.1
    User admin
";
        let entries = parse_ssh_config(config);
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].host_alias, "myserver");
        assert_eq!(entries[0].hostname, "10.0.0.1");
    }

    #[test]
    fn first_identity_file_is_kept() {
        let config = "\
Host myserver
    HostName 10.0.0.1
    IdentityFile ~/.ssh/first_key
    IdentityFile ~/.ssh/second_key
";
        let entries = parse_ssh_config(config);
        assert_eq!(entries.len(), 1);
        assert_eq!(
            entries[0].identity_file.as_deref(),
            Some("~/.ssh/first_key")
        );
    }

    #[test]
    fn include_directive_resolves_files() {
        use std::fs;
        let dir = tempfile::tempdir().unwrap();

        // Write an included config fragment
        let included = "\
Host included-server
    HostName 10.0.0.99
    User included
";
        let included_path = dir.path().join("conf.d");
        fs::create_dir_all(&included_path).unwrap();
        fs::write(included_path.join("extra.conf"), included).unwrap();

        // Main config that includes the fragment
        let main_config = format!(
            "Include {}/conf.d/*\n\nHost main-server\n    HostName 10.0.0.1\n",
            dir.path().display()
        );

        let entries = parse_ssh_config_inner(&main_config, Some(dir.path()));
        assert_eq!(entries.len(), 2);
        assert_eq!(entries[0].host_alias, "included-server");
        assert_eq!(entries[1].host_alias, "main-server");
    }
}
