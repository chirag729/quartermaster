use serde::{Deserialize, Serialize};

use crate::commands::ssh::SshKeyInfo;
use crate::error::AppError;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct YubiKeyInfo {
    pub serial: String,
    pub firmware: String,
    pub model: String,
    pub fido2_supported: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Fido2Credential {
    pub credential_id: String,
    pub rp_id: String,
    pub user_name: Option<String>,
    pub resident: bool,
}

// ---------------------------------------------------------------------------
// Parsing helpers (public for testing)
// ---------------------------------------------------------------------------

/// Parse the output of `ykman info` into a YubiKeyInfo.
///
/// Expected format:
/// ```text
/// Device type: YubiKey 5 NFC
/// Serial number: 12345678
/// Firmware version: 5.4.3
/// ...
/// ```
pub fn parse_ykman_info(output: &str) -> Option<YubiKeyInfo> {
    let mut model = String::new();
    let mut serial = String::new();
    let mut firmware = String::new();
    let mut fido2_supported = false;

    for line in output.lines() {
        let line = line.trim();
        if let Some(value) = line.strip_prefix("Device type:") {
            model = value.trim().to_string();
        } else if let Some(value) = line.strip_prefix("Serial number:") {
            serial = value.trim().to_string();
        } else if let Some(value) = line.strip_prefix("Firmware version:") {
            firmware = value.trim().to_string();
        } else if line.contains("FIDO2") {
            // Lines like "FIDO2" or "FIDO2             Enabled" indicate FIDO2 support
            if line.contains("Enabled") || line == "FIDO2" {
                fido2_supported = true;
            }
        }
    }

    if serial.is_empty() && model.is_empty() && firmware.is_empty() {
        return None;
    }

    Some(YubiKeyInfo {
        serial,
        firmware,
        model,
        fido2_supported,
    })
}

/// Parse the output of `ykman fido credentials list` into a list of Fido2Credentials.
///
/// Expected format varies, but typically looks like:
/// ```text
/// ssh: 0000000000000000000000000000000000000000000000000000000000000000 openssh user@host
/// relying_party_id: credential_id username
/// ```
///
/// The output can also come in the more verbose format:
/// ```text
/// Credential ID:  0000000000000000000000000000000000000000000000000000000000000000
/// RP ID:          ssh:
/// User name:      user@host
/// ```
pub fn parse_fido2_credentials(output: &str) -> Vec<Fido2Credential> {
    let mut credentials = Vec::new();

    // Try verbose multi-line format first (ykman >= 5.x)
    if output.contains("Credential ID:") {
        let mut current_cred_id = String::new();
        let mut current_rp_id = String::new();
        let mut current_user_name: Option<String> = None;

        for line in output.lines() {
            let line = line.trim();
            if let Some(value) = line.strip_prefix("Credential ID:") {
                // If we already have a credential being built, push it
                if !current_cred_id.is_empty() {
                    credentials.push(Fido2Credential {
                        credential_id: current_cred_id.clone(),
                        rp_id: current_rp_id.clone(),
                        user_name: current_user_name.take(),
                        resident: true,
                    });
                }
                current_cred_id = value.trim().to_string();
                current_rp_id.clear();
                current_user_name = None;
            } else if let Some(value) = line.strip_prefix("RP ID:") {
                current_rp_id = value.trim().to_string();
            } else if let Some(value) = line.strip_prefix("User name:") {
                let name = value.trim().to_string();
                if !name.is_empty() {
                    current_user_name = Some(name);
                }
            }
        }

        // Push the last credential if present
        if !current_cred_id.is_empty() {
            credentials.push(Fido2Credential {
                credential_id: current_cred_id,
                rp_id: current_rp_id,
                user_name: current_user_name,
                resident: true,
            });
        }
    } else {
        // Single-line format: "rp_id credential_id [user_name]"
        for line in output.lines() {
            let line = line.trim();
            if line.is_empty() {
                continue;
            }
            let parts: Vec<&str> = line.splitn(3, ' ').collect();
            if parts.len() >= 2 {
                let rp_id = parts[0].to_string();
                let credential_id = parts[1].to_string();
                let user_name = parts.get(2).map(|s| s.trim().to_string()).filter(|s| !s.is_empty());

                credentials.push(Fido2Credential {
                    credential_id,
                    rp_id,
                    user_name,
                    resident: true,
                });
            }
        }
    }

    credentials
}

/// Sanitize a key name to contain only safe filesystem characters.
///
/// Allows alphanumeric, hyphens, underscores, and dots. All other characters
/// are replaced with underscores. Leading dots and hyphens are stripped to
/// prevent hidden files or option-like names.
pub fn sanitize_key_name(name: &str) -> String {
    let sanitized: String = name
        .chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() || c == '-' || c == '_' || c == '.' {
                c
            } else {
                '_'
            }
        })
        .collect();

    // Strip leading dots and hyphens
    let trimmed = sanitized.trim_start_matches(|c: char| c == '.' || c == '-');

    if trimmed.is_empty() {
        "fido2_key".to_string()
    } else {
        trimmed.to_string()
    }
}

/// Check if `ssh-keygen` help output mentions ed25519-sk support.
pub fn parse_ssh_keygen_supports_sk(help_output: &str) -> bool {
    help_output.contains("ed25519-sk") || help_output.contains("ecdsa-sk")
}

// ---------------------------------------------------------------------------
// Tauri commands
// ---------------------------------------------------------------------------

/// Detect connected YubiKeys using `ykman`.
///
/// Returns an empty vec (not an error) if `ykman` is not installed.
#[tauri::command]
pub async fn detect_yubikeys() -> Result<Vec<YubiKeyInfo>, AppError> {
    // First check if ykman is available
    let ykman_check = tokio::process::Command::new("which")
        .arg("ykman")
        .output()
        .await;

    match ykman_check {
        Ok(output) if !output.status.success() => return Ok(Vec::new()),
        Err(_) => return Ok(Vec::new()),
        _ => {}
    }

    // Get the list of serial numbers
    let serials_output = tokio::process::Command::new("ykman")
        .args(["list", "--serials"])
        .output()
        .await
        .map_err(|e| AppError::YubiKey(format!("Failed to run ykman list: {}", e)))?;

    if !serials_output.status.success() {
        // ykman list failing likely means no keys connected
        return Ok(Vec::new());
    }

    let serials_text = String::from_utf8_lossy(&serials_output.stdout);
    let serials: Vec<&str> = serials_text
        .lines()
        .map(|l| l.trim())
        .filter(|l| !l.is_empty())
        .collect();

    if serials.is_empty() {
        return Ok(Vec::new());
    }

    let mut keys = Vec::new();

    for serial in serials {
        let info_output = tokio::process::Command::new("ykman")
            .args(["--device", serial, "info"])
            .output()
            .await
            .map_err(|e| {
                AppError::YubiKey(format!(
                    "Failed to get info for YubiKey {}: {}",
                    serial, e
                ))
            })?;

        if info_output.status.success() {
            let info_text = String::from_utf8_lossy(&info_output.stdout);
            if let Some(mut info) = parse_ykman_info(&info_text) {
                // Ensure the serial is set (in case the info output didn't include it)
                if info.serial.is_empty() {
                    info.serial = serial.to_string();
                }
                keys.push(info);
            } else {
                // Minimal info if parsing failed
                keys.push(YubiKeyInfo {
                    serial: serial.to_string(),
                    firmware: String::new(),
                    model: "Unknown YubiKey".to_string(),
                    fido2_supported: false,
                });
            }
        }
    }

    Ok(keys)
}

/// List FIDO2 resident credentials stored on a YubiKey.
#[tauri::command]
pub async fn list_fido2_credentials(
    serial: Option<String>,
) -> Result<Vec<Fido2Credential>, AppError> {
    let mut cmd = tokio::process::Command::new("ykman");

    if let Some(ref s) = serial {
        cmd.args(["--device", s]);
    }

    cmd.args(["fido", "credentials", "list"]);

    let output = cmd
        .output()
        .await
        .map_err(|e| AppError::YubiKey(format!("Failed to run ykman fido credentials list: {}", e)))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(AppError::YubiKey(format!(
            "Failed to list FIDO2 credentials: {}",
            stderr
        )));
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    Ok(parse_fido2_credentials(&stdout))
}

/// Generate a FIDO2 SSH key using `ssh-keygen -t ed25519-sk`.
#[tauri::command]
pub async fn generate_fido2_ssh_key(
    key_name: String,
    comment: String,
    resident: bool,
    application: Option<String>,
) -> Result<SshKeyInfo, AppError> {
    let ssh_dir = dirs::home_dir()
        .ok_or_else(|| AppError::YubiKey("Could not determine home directory".to_string()))?
        .join(".ssh");

    std::fs::create_dir_all(&ssh_dir)
        .map_err(|e| AppError::YubiKey(format!("Failed to create .ssh directory: {}", e)))?;

    let safe_name = sanitize_key_name(&key_name);
    let key_path = ssh_dir.join(&safe_name);

    // Don't overwrite existing keys
    if key_path.exists() {
        return Err(AppError::YubiKey(format!(
            "Key already exists: {}",
            key_path.display()
        )));
    }

    let pub_path = ssh_dir.join(format!("{}.pub", safe_name));
    if pub_path.exists() {
        return Err(AppError::YubiKey(format!(
            "Public key already exists: {}",
            pub_path.display()
        )));
    }

    let mut args: Vec<String> = vec![
        "-t".to_string(),
        "ed25519-sk".to_string(),
    ];

    if resident {
        args.push("-O".to_string());
        args.push("resident".to_string());
        args.push("-O".to_string());

        let app = application
            .filter(|a| !a.is_empty())
            .unwrap_or_else(|| safe_name.clone());
        args.push(format!("application=ssh:{}", app));
    }

    args.push("-f".to_string());
    args.push(key_path.to_string_lossy().to_string());
    args.push("-C".to_string());
    args.push(comment.clone());
    args.push("-N".to_string());
    args.push(String::new()); // empty passphrase

    let output = tokio::process::Command::new("ssh-keygen")
        .args(&args)
        .output()
        .await
        .map_err(|e| AppError::YubiKey(format!("Failed to run ssh-keygen: {}", e)))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(AppError::YubiKey(format!(
            "ssh-keygen failed: {}",
            stderr
        )));
    }

    // Read the generated public key
    let pub_content = std::fs::read_to_string(&pub_path)
        .map_err(|e| AppError::YubiKey(format!("Failed to read generated public key: {}", e)))?;

    let key_type = pub_content
        .split_whitespace()
        .next()
        .unwrap_or("sk-ssh-ed25519@openssh.com")
        .to_string();

    Ok(SshKeyInfo {
        name: safe_name,
        path: pub_path.to_string_lossy().to_string(),
        key_type,
        is_fido2: true,
    })
}

/// Check whether the system supports FIDO2 SSH keys.
///
/// Verifies that `ssh-keygen` supports the `-t ed25519-sk` key type and that
/// `libfido2` is available on the system.
#[tauri::command]
pub async fn is_fido2_supported() -> Result<bool, AppError> {
    // Check ssh-keygen support for ed25519-sk
    let keygen_help = tokio::process::Command::new("ssh-keygen")
        .arg("-t")
        .arg("ed25519-sk")
        .arg("-f")
        .arg("/dev/null")
        .arg("-N")
        .arg("")
        .arg("-C")
        .arg("test")
        .stdin(std::process::Stdio::null())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .output()
        .await;

    let keygen_supports_sk = match keygen_help {
        Ok(output) => {
            let stderr = String::from_utf8_lossy(&output.stderr);
            // If it fails with "unknown key type" then it's not supported.
            // If it fails for other reasons (e.g. no device) that's fine, the
            // key type is still recognised.
            !stderr.contains("unknown key type")
        }
        Err(_) => false,
    };

    if !keygen_supports_sk {
        return Ok(false);
    }

    // Check for libfido2 - try ldconfig or dpkg
    let libfido2_available = check_libfido2_available().await;

    Ok(libfido2_available)
}

/// Check if libfido2 is available on the system.
async fn check_libfido2_available() -> bool {
    // Try ldconfig first
    let ldconfig_result = tokio::process::Command::new("ldconfig")
        .args(["-p"])
        .output()
        .await;

    if let Ok(output) = ldconfig_result {
        let stdout = String::from_utf8_lossy(&output.stdout);
        if stdout.contains("libfido2") {
            return true;
        }
    }

    // Try dpkg as fallback (Debian/Ubuntu)
    let dpkg_result = tokio::process::Command::new("dpkg")
        .args(["-l", "libfido2-dev"])
        .output()
        .await;

    if let Ok(output) = dpkg_result {
        if output.status.success() {
            let stdout = String::from_utf8_lossy(&output.stdout);
            if stdout.contains("ii") {
                return true;
            }
        }
    }

    // Try checking for libfido2-1 package
    let dpkg_result2 = tokio::process::Command::new("dpkg")
        .args(["-l", "libfido2-1"])
        .output()
        .await;

    if let Ok(output) = dpkg_result2 {
        if output.status.success() {
            let stdout = String::from_utf8_lossy(&output.stdout);
            if stdout.contains("ii") {
                return true;
            }
        }
    }

    // Try pkg-config as another fallback
    let pkgconfig_result = tokio::process::Command::new("pkg-config")
        .args(["--exists", "libfido2"])
        .output()
        .await;

    if let Ok(output) = pkgconfig_result {
        if output.status.success() {
            return true;
        }
    }

    false
}

// ---------------------------------------------------------------------------
// FIDO2 dependency management
// ---------------------------------------------------------------------------

/// Status of FIDO2 dependencies on the system.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Fido2DependencyStatus {
    pub libfido2_installed: bool,
    pub ykman_installed: bool,
    pub all_satisfied: bool,
    pub install_attempted: bool,
    pub install_output: Option<String>,
}

/// Check if `ykman` (yubikey-manager) is available on the system.
async fn check_ykman_available() -> bool {
    let result = tokio::process::Command::new("which")
        .arg("ykman")
        .output()
        .await;

    matches!(result, Ok(output) if output.status.success())
}

/// Build the list of apt packages that need to be installed.
///
/// Returns a vector of package name strings based on which dependencies are
/// currently missing. This is a pure function over the two boolean inputs,
/// making it easy to unit-test without spawning processes.
pub fn missing_packages(libfido2_installed: bool, ykman_installed: bool) -> Vec<&'static str> {
    let mut packages = Vec::new();
    if !libfido2_installed {
        packages.push("libfido2-1");
    }
    if !ykman_installed {
        packages.push("yubikey-manager");
    }
    packages
}

/// Check FIDO2 dependency status without attempting installation.
#[tauri::command]
pub async fn check_fido2_dependencies() -> Result<Fido2DependencyStatus, AppError> {
    let libfido2_installed = check_libfido2_available().await;
    let ykman_installed = check_ykman_available().await;
    let all_satisfied = libfido2_installed && ykman_installed;

    Ok(Fido2DependencyStatus {
        libfido2_installed,
        ykman_installed,
        all_satisfied,
        install_attempted: false,
        install_output: None,
    })
}

/// Install missing FIDO2 dependencies using PolicyKit for privilege escalation.
///
/// Checks which packages are missing, installs them via `pkexec apt install -y`,
/// then verifies installation succeeded by re-checking availability.
#[tauri::command]
pub async fn install_fido2_dependencies() -> Result<Fido2DependencyStatus, AppError> {
    // Initial check
    let libfido2_installed = check_libfido2_available().await;
    let ykman_installed = check_ykman_available().await;

    // Return early if everything is already installed
    if libfido2_installed && ykman_installed {
        return Ok(Fido2DependencyStatus {
            libfido2_installed: true,
            ykman_installed: true,
            all_satisfied: true,
            install_attempted: false,
            install_output: None,
        });
    }

    // Build list of missing packages
    let packages = missing_packages(libfido2_installed, ykman_installed);

    // Install via pkexec (PolicyKit privilege escalation)
    let mut cmd = tokio::process::Command::new("pkexec");
    cmd.arg("apt").arg("install").arg("-y");
    for pkg in &packages {
        cmd.arg(pkg);
    }

    let output = cmd
        .output()
        .await
        .map_err(|e| AppError::YubiKey(format!("Failed to run pkexec apt install: {}", e)))?;

    let stdout = String::from_utf8_lossy(&output.stdout).to_string();
    let stderr = String::from_utf8_lossy(&output.stderr).to_string();

    let install_output = if stderr.is_empty() {
        stdout.clone()
    } else {
        format!("{}\n{}", stdout, stderr)
    };

    if !output.status.success() {
        return Err(AppError::YubiKey(format!(
            "Package installation failed (packages: {}): {}",
            packages.join(", "),
            stderr
        )));
    }

    // Re-check after installation
    let libfido2_installed = check_libfido2_available().await;
    let ykman_installed = check_ykman_available().await;
    let all_satisfied = libfido2_installed && ykman_installed;

    if !all_satisfied {
        return Err(AppError::YubiKey(format!(
            "Installation completed but verification failed -- libfido2: {}, ykman: {}",
            if libfido2_installed { "ok" } else { "missing" },
            if ykman_installed { "ok" } else { "missing" },
        )));
    }

    Ok(Fido2DependencyStatus {
        libfido2_installed,
        ykman_installed,
        all_satisfied,
        install_attempted: true,
        install_output: Some(install_output),
    })
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_ykman_info_standard() {
        let output = "\
Device type: YubiKey 5 NFC
Serial number: 12345678
Firmware version: 5.4.3
Form factor: Keychain (USB-A)
Enabled USB interfaces: OTP, FIDO, CCID
NFC transport is enabled.

Applications    USB          NFC
OTP             Enabled      Enabled
FIDO U2F        Enabled      Enabled
FIDO2           Enabled      Enabled
OATH            Enabled      Enabled
PIV             Enabled      Enabled
OpenPGP         Enabled      Enabled
YubiHSM Auth    Not available    Not available
";

        let info = parse_ykman_info(output).expect("Should parse ykman info");
        assert_eq!(info.serial, "12345678");
        assert_eq!(info.firmware, "5.4.3");
        assert_eq!(info.model, "YubiKey 5 NFC");
        assert!(info.fido2_supported);
    }

    #[test]
    fn test_parse_ykman_info_no_fido2() {
        let output = "\
Device type: YubiKey NEO
Serial number: 87654321
Firmware version: 3.4.9
";

        let info = parse_ykman_info(output).expect("Should parse ykman info");
        assert_eq!(info.serial, "87654321");
        assert_eq!(info.firmware, "3.4.9");
        assert_eq!(info.model, "YubiKey NEO");
        assert!(!info.fido2_supported);
    }

    #[test]
    fn test_parse_ykman_info_empty() {
        let info = parse_ykman_info("");
        assert!(info.is_none());
    }

    #[test]
    fn test_parse_fido2_credentials_verbose_format() {
        let output = "\
Credential ID:  abcdef0123456789abcdef0123456789
RP ID:          ssh:
User name:      user@example.com

Credential ID:  fedcba9876543210fedcba9876543210
RP ID:          github.com
User name:      ghuser
";

        let creds = parse_fido2_credentials(output);
        assert_eq!(creds.len(), 2);

        assert_eq!(creds[0].credential_id, "abcdef0123456789abcdef0123456789");
        assert_eq!(creds[0].rp_id, "ssh:");
        assert_eq!(creds[0].user_name, Some("user@example.com".to_string()));
        assert!(creds[0].resident);

        assert_eq!(creds[1].credential_id, "fedcba9876543210fedcba9876543210");
        assert_eq!(creds[1].rp_id, "github.com");
        assert_eq!(creds[1].user_name, Some("ghuser".to_string()));
        assert!(creds[1].resident);
    }

    #[test]
    fn test_parse_fido2_credentials_single_line_format() {
        let output = "\
ssh: abcdef01234567890 user@host
github.com fedcba9876543210
";

        let creds = parse_fido2_credentials(output);
        assert_eq!(creds.len(), 2);

        assert_eq!(creds[0].rp_id, "ssh:");
        assert_eq!(creds[0].credential_id, "abcdef01234567890");
        assert_eq!(creds[0].user_name, Some("user@host".to_string()));

        assert_eq!(creds[1].rp_id, "github.com");
        assert_eq!(creds[1].credential_id, "fedcba9876543210");
        assert_eq!(creds[1].user_name, None);
    }

    #[test]
    fn test_parse_fido2_credentials_empty() {
        let creds = parse_fido2_credentials("");
        assert!(creds.is_empty());
    }

    #[test]
    fn test_sanitize_key_name_basic() {
        assert_eq!(sanitize_key_name("my-key"), "my-key");
        assert_eq!(sanitize_key_name("my_key_2"), "my_key_2");
        assert_eq!(sanitize_key_name("id_ed25519"), "id_ed25519");
    }

    #[test]
    fn test_sanitize_key_name_special_chars() {
        assert_eq!(sanitize_key_name("my key!@#$%"), "my_key_____");
        assert_eq!(sanitize_key_name("path/to/key"), "path_to_key");
        assert_eq!(sanitize_key_name("key with spaces"), "key_with_spaces");
    }

    #[test]
    fn test_sanitize_key_name_leading_dots_hyphens() {
        assert_eq!(sanitize_key_name("..hidden"), "hidden");
        assert_eq!(sanitize_key_name("--flag"), "flag");
        assert_eq!(sanitize_key_name(".-mixed"), "mixed");
    }

    #[test]
    fn test_sanitize_key_name_empty() {
        assert_eq!(sanitize_key_name(""), "fido2_key");
        assert_eq!(sanitize_key_name("..."), "fido2_key");
        assert_eq!(sanitize_key_name("---"), "fido2_key");
    }

    #[test]
    fn test_parse_ssh_keygen_supports_sk_true() {
        let help = "usage: ssh-keygen ... [-t dsa | ecdsa | ecdsa-sk | ed25519 | ed25519-sk | rsa]";
        assert!(parse_ssh_keygen_supports_sk(help));
    }

    #[test]
    fn test_parse_ssh_keygen_supports_sk_false() {
        let help = "usage: ssh-keygen ... [-t dsa | ecdsa | ed25519 | rsa]";
        assert!(!parse_ssh_keygen_supports_sk(help));
    }

    #[test]
    fn test_parse_ykman_info_fido2_standalone_line() {
        // Some ykman versions print just "FIDO2" as a standalone application line
        let output = "\
Device type: YubiKey 5Ci
Serial number: 99999999
Firmware version: 5.2.4
FIDO2
";
        let info = parse_ykman_info(output).expect("Should parse");
        assert!(info.fido2_supported);
    }

    // -----------------------------------------------------------------------
    // FIDO2 dependency management tests
    // -----------------------------------------------------------------------

    #[test]
    fn test_missing_packages_both_missing() {
        let pkgs = missing_packages(false, false);
        assert_eq!(pkgs, vec!["libfido2-1", "yubikey-manager"]);
    }

    #[test]
    fn test_missing_packages_only_libfido2_missing() {
        let pkgs = missing_packages(false, true);
        assert_eq!(pkgs, vec!["libfido2-1"]);
    }

    #[test]
    fn test_missing_packages_only_ykman_missing() {
        let pkgs = missing_packages(true, false);
        assert_eq!(pkgs, vec!["yubikey-manager"]);
    }

    #[test]
    fn test_missing_packages_none_missing() {
        let pkgs = missing_packages(true, true);
        assert!(pkgs.is_empty());
    }

    #[test]
    fn test_fido2_dependency_status_serialization() {
        let status = Fido2DependencyStatus {
            libfido2_installed: true,
            ykman_installed: false,
            all_satisfied: false,
            install_attempted: true,
            install_output: Some("installed libfido2-1".to_string()),
        };

        let json = serde_json::to_string(&status).expect("Should serialize");
        let deserialized: Fido2DependencyStatus =
            serde_json::from_str(&json).expect("Should deserialize");

        assert!(deserialized.libfido2_installed);
        assert!(!deserialized.ykman_installed);
        assert!(!deserialized.all_satisfied);
        assert!(deserialized.install_attempted);
        assert_eq!(
            deserialized.install_output,
            Some("installed libfido2-1".to_string())
        );
    }

    #[test]
    fn test_fido2_dependency_status_serialization_no_output() {
        let status = Fido2DependencyStatus {
            libfido2_installed: true,
            ykman_installed: true,
            all_satisfied: true,
            install_attempted: false,
            install_output: None,
        };

        let json = serde_json::to_string(&status).expect("Should serialize");
        let deserialized: Fido2DependencyStatus =
            serde_json::from_str(&json).expect("Should deserialize");

        assert!(deserialized.all_satisfied);
        assert!(!deserialized.install_attempted);
        assert!(deserialized.install_output.is_none());
    }
}
