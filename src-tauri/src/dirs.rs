//! Canonical XDG-compliant directory paths for the Quartermaster application.
//!
//! All paths follow the XDG Base Directory Specification:
//!
//! | Purpose        | Base                              | Path                                          |
//! |----------------|-----------------------------------|-----------------------------------------------|
//! | Config         | `$XDG_CONFIG_HOME`                | `~/.config/quartermaster/`                    |
//! | Data           | `$XDG_DATA_HOME`                  | `~/.local/share/quartermaster/`               |
//! | State          | `$XDG_STATE_HOME`                 | `~/.local/state/quartermaster/`               |
//! | Cache          | `$XDG_CACHE_HOME`                 | `~/.cache/quartermaster/`                     |
//!
//! Subdirectories:
//!
//! | Function               | Path                                                    |
//! |------------------------|---------------------------------------------------------|
//! | `downloads_dir()`      | `~/.cache/quartermaster/downloads/`                     |
//! | `logs_dir()`           | `~/.local/state/quartermaster/logs/`                    |
//! | `blueprints_dir()`     | `~/.config/quartermaster/blueprints/`                   |
//! | `user_tasks_dir()`     | `~/.config/quartermaster/tasks/`                        |
//! | `installed_profiles_dir()` | `~/.local/share/quartermaster/apparmor/profiles/`   |
//! | `tunables_dir()`       | `~/.local/share/quartermaster/apparmor/tunables/`       |
//!
//! File paths (parent directory is created, but not the file itself):
//!
//! | Function               | Path                                                    |
//! |------------------------|---------------------------------------------------------|
//! | `vault_path()`         | `~/.config/quartermaster/vault.enc`                     |
//! | `config_file_path()`   | `~/.config/quartermaster/config.json`                   |
//!
//! All directory-returning functions call `create_dir_all` to ensure the directory exists.
//! All functions are infallible; directory creation errors are silently ignored (best-effort).

use std::path::PathBuf;

const APP_NAME: &str = "quartermaster";

/// Ensures the given directory exists (best-effort) and returns the path unchanged.
/// Errors from `create_dir_all` are silently ignored — the caller will encounter
/// filesystem errors later if the directory truly cannot be created.
fn ensure_dir(path: PathBuf) -> PathBuf {
    let _ = std::fs::create_dir_all(&path);
    path
}

/// Returns the XDG config base directory, falling back to `~/.config` if
/// `dirs::config_dir()` is unavailable.
fn xdg_config_base() -> PathBuf {
    dirs::config_dir().unwrap_or_else(|| {
        dirs::home_dir()
            .unwrap_or_else(|| {
                eprintln!("Warning: could not determine home directory, falling back to /tmp for config storage");
                PathBuf::from("/tmp")
            })
            .join(".config")
    })
}

/// Returns the XDG data base directory, falling back to `~/.local/share` if
/// `dirs::data_dir()` is unavailable.
fn xdg_data_base() -> PathBuf {
    dirs::data_dir().unwrap_or_else(|| {
        dirs::home_dir()
            .unwrap_or_else(|| {
                eprintln!("Warning: could not determine home directory, falling back to /tmp for data storage");
                PathBuf::from("/tmp")
            })
            .join(".local")
            .join("share")
    })
}

/// Returns the XDG state base directory. Uses `dirs::state_dir()` if available,
/// otherwise constructs `~/.local/state` manually.
fn xdg_state_base() -> PathBuf {
    dirs::state_dir().unwrap_or_else(|| {
        dirs::home_dir()
            .unwrap_or_else(|| {
                eprintln!("Warning: could not determine home directory, falling back to /tmp for state storage");
                PathBuf::from("/tmp")
            })
            .join(".local")
            .join("state")
    })
}

/// Returns the XDG cache base directory, falling back to `~/.cache` if
/// `dirs::cache_dir()` is unavailable.
fn xdg_cache_base() -> PathBuf {
    dirs::cache_dir().unwrap_or_else(|| {
        dirs::home_dir()
            .unwrap_or_else(|| {
                eprintln!("Warning: could not determine home directory, falling back to /tmp for cache storage");
                PathBuf::from("/tmp")
            })
            .join(".cache")
    })
}

// ---------------------------------------------------------------------------
// Top-level XDG directories
// ---------------------------------------------------------------------------

/// User configuration directory: `~/.config/quartermaster/`
pub fn config_dir() -> PathBuf {
    ensure_dir(xdg_config_base().join(APP_NAME))
}

/// Persistent application data: `~/.local/share/quartermaster/`
pub fn data_dir() -> PathBuf {
    ensure_dir(xdg_data_base().join(APP_NAME))
}

/// Runtime state and logs: `~/.local/state/quartermaster/`
pub fn state_dir() -> PathBuf {
    ensure_dir(xdg_state_base().join(APP_NAME))
}

/// Cache, downloads, temporary files: `~/.cache/quartermaster/`
pub fn cache_dir() -> PathBuf {
    ensure_dir(xdg_cache_base().join(APP_NAME))
}

// ---------------------------------------------------------------------------
// Subdirectories
// ---------------------------------------------------------------------------

/// Downloaded artifacts cache: `~/.cache/quartermaster/downloads/`
pub fn downloads_dir() -> PathBuf {
    ensure_dir(cache_dir().join("downloads"))
}

/// Application logs: `~/.local/state/quartermaster/logs/`
pub fn logs_dir() -> PathBuf {
    ensure_dir(state_dir().join("logs"))
}

/// User blueprint definitions: `~/.config/quartermaster/blueprints/`
pub fn blueprints_dir() -> PathBuf {
    ensure_dir(config_dir().join("blueprints"))
}

/// User-defined task definitions: `~/.config/quartermaster/tasks/`
pub fn user_tasks_dir() -> PathBuf {
    ensure_dir(config_dir().join("tasks"))
}

/// Installed AppArmor profile copies: `~/.local/share/quartermaster/apparmor/profiles/`
pub fn installed_profiles_dir() -> PathBuf {
    ensure_dir(data_dir().join("apparmor").join("profiles"))
}

/// AppArmor tunables: `~/.local/share/quartermaster/apparmor/tunables/`
pub fn tunables_dir() -> PathBuf {
    ensure_dir(data_dir().join("apparmor").join("tunables"))
}

// ---------------------------------------------------------------------------
// File paths (parent directory created, file itself is NOT created)
// ---------------------------------------------------------------------------

/// Encrypted vault file: `~/.config/quartermaster/vault.enc`
///
/// The parent directory is created if it does not exist, but the file itself
/// is not created.
pub fn vault_path() -> PathBuf {
    let path = config_dir().join("vault.enc");
    // config_dir() already ensures the parent exists
    path
}

/// Main configuration file: `~/.config/quartermaster/config.json`
///
/// The parent directory is created if it does not exist, but the file itself
/// is not created.
pub fn config_file_path() -> PathBuf {
    let path = config_dir().join("config.json");
    // config_dir() already ensures the parent exists
    path
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;

    /// Every path returned by the module must contain "quartermaster" somewhere.
    #[test]
    fn all_paths_contain_app_name() {
        let paths: Vec<PathBuf> = vec![
            config_dir(),
            data_dir(),
            state_dir(),
            cache_dir(),
            downloads_dir(),
            logs_dir(),
            blueprints_dir(),
            user_tasks_dir(),
            vault_path(),
            config_file_path(),
            installed_profiles_dir(),
            tunables_dir(),
        ];
        for p in &paths {
            assert!(
                p.to_string_lossy().contains("quartermaster"),
                "Path {:?} does not contain 'quartermaster'",
                p
            );
        }
    }

    /// Directory-returning functions must actually create the directory on disk.
    #[test]
    fn directory_functions_create_dirs() {
        // We call the functions and check that the paths exist as directories.
        // On a normal system these will succeed. In a sandboxed/read-only
        // environment the ensure_dir silently ignores errors, so we only
        // assert when the dir was actually created.
        let dirs: Vec<PathBuf> = vec![
            config_dir(),
            data_dir(),
            state_dir(),
            cache_dir(),
            downloads_dir(),
            logs_dir(),
            blueprints_dir(),
            user_tasks_dir(),
            installed_profiles_dir(),
            tunables_dir(),
        ];
        for d in &dirs {
            assert!(
                d.is_dir(),
                "Expected {:?} to be an existing directory",
                d
            );
        }
    }

    /// Check that each path ends with the expected suffix.
    #[test]
    fn paths_end_with_expected_suffix() {
        assert!(config_dir().ends_with("quartermaster"));
        assert!(data_dir().ends_with("quartermaster"));
        assert!(state_dir().ends_with("quartermaster"));
        assert!(cache_dir().ends_with("quartermaster"));
        assert!(downloads_dir().ends_with("quartermaster/downloads"));
        assert!(logs_dir().ends_with("quartermaster/logs"));
        assert!(blueprints_dir().ends_with("quartermaster/blueprints"));
        assert!(user_tasks_dir().ends_with("quartermaster/tasks"));
        assert!(installed_profiles_dir().ends_with("quartermaster/apparmor/profiles"));
        assert!(tunables_dir().ends_with("quartermaster/apparmor/tunables"));
        assert!(vault_path().ends_with("quartermaster/vault.enc"));
        assert!(config_file_path().ends_with("quartermaster/config.json"));
    }

    /// vault_path and config_file_path should return file paths, not directories.
    /// Specifically, they should have a file name with an extension and should
    /// NOT be existing directories.
    #[test]
    fn file_paths_are_not_directories() {
        let vault = vault_path();
        let config = config_file_path();

        // They should have file extensions
        assert_eq!(
            vault.extension().and_then(|e| e.to_str()),
            Some("enc"),
            "vault_path should have .enc extension"
        );
        assert_eq!(
            config.extension().and_then(|e| e.to_str()),
            Some("json"),
            "config_file_path should have .json extension"
        );

        // They should not exist as directories (the functions don't create the files)
        assert!(
            !vault.is_dir(),
            "vault_path should not be a directory"
        );
        assert!(
            !config.is_dir(),
            "config_file_path should not be a directory"
        );
    }

    /// File-path functions should ensure the parent directory exists.
    #[test]
    fn file_path_parents_exist() {
        let vault = vault_path();
        let config = config_file_path();

        assert!(
            vault.parent().map_or(false, |p| p.is_dir()),
            "Parent of vault_path should be an existing directory"
        );
        assert!(
            config.parent().map_or(false, |p| p.is_dir()),
            "Parent of config_file_path should be an existing directory"
        );
    }

    /// All directory paths should be absolute.
    #[test]
    fn all_paths_are_absolute() {
        let paths: Vec<PathBuf> = vec![
            config_dir(),
            data_dir(),
            state_dir(),
            cache_dir(),
            downloads_dir(),
            logs_dir(),
            blueprints_dir(),
            user_tasks_dir(),
            vault_path(),
            config_file_path(),
            installed_profiles_dir(),
            tunables_dir(),
        ];
        for p in &paths {
            assert!(
                p.is_absolute(),
                "Path {:?} should be absolute",
                p
            );
        }
    }

    /// The top-level dirs should sit under distinct XDG base directories.
    #[test]
    fn xdg_base_directories_are_distinct() {
        let config = config_dir();
        let data = data_dir();
        let state = state_dir();
        let cache = cache_dir();

        // All four should have different parent directories
        let parents: Vec<&Path> = vec![
            config.parent().unwrap(),
            data.parent().unwrap(),
            state.parent().unwrap(),
            cache.parent().unwrap(),
        ];

        // Check pairwise distinctness
        for i in 0..parents.len() {
            for j in (i + 1)..parents.len() {
                assert_ne!(
                    parents[i], parents[j],
                    "XDG base dirs should be distinct: {:?} vs {:?}",
                    parents[i], parents[j]
                );
            }
        }
    }
}
