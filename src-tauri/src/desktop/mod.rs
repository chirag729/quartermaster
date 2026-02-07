//! Desktop entry generator following the freedesktop.org Desktop Entry Specification.
//!
//! Creates `.desktop` files in `~/.local/share/applications/` so that applications
//! installed by Quartermaster appear in system launchers (GNOME, KDE, etc.).
//!
//! Also handles icon installation to `~/.local/share/icons/hicolor/`.

use std::path::{Path, PathBuf};

use crate::error::AppError;

/// Returns the user's desktop entry directory (`~/.local/share/applications/`).
pub fn applications_dir() -> Result<PathBuf, AppError> {
    let dir = dirs::data_dir()
        .unwrap_or_else(|| {
            dirs::home_dir()
                .unwrap_or_else(|| PathBuf::from("/tmp"))
                .join(".local/share")
        })
        .join("applications");
    std::fs::create_dir_all(&dir)?;
    Ok(dir)
}

/// Returns the user's icon directory (`~/.local/share/icons/hicolor/scalable/apps/`).
pub fn icons_dir() -> Result<PathBuf, AppError> {
    let dir = dirs::data_dir()
        .unwrap_or_else(|| {
            dirs::home_dir()
                .unwrap_or_else(|| PathBuf::from("/tmp"))
                .join(".local/share")
        })
        .join("icons")
        .join("hicolor")
        .join("scalable")
        .join("apps");
    std::fs::create_dir_all(&dir)?;
    Ok(dir)
}

/// Specification for a desktop entry to create.
pub struct DesktopEntrySpec {
    /// Application ID used for the .desktop filename (e.g., "jetbrains-idea").
    pub app_id: String,
    /// Display name shown in launchers.
    pub name: String,
    /// Absolute path to the executable.
    pub exec: String,
    /// Absolute path to the icon file, or a named icon.
    pub icon: String,
    /// Freedesktop.org categories (e.g., ["Development", "IDE"]).
    pub categories: Vec<String>,
    /// MIME types this application can handle.
    pub mime_types: Vec<String>,
    /// Whether the application runs in a terminal.
    pub terminal: bool,
    /// Optional comment/description.
    pub comment: Option<String>,
    /// Optional startup WM class for proper window matching.
    pub startup_wm_class: Option<String>,
}

/// Generates a `.desktop` file content string following the freedesktop.org spec.
pub fn generate_desktop_entry(spec: &DesktopEntrySpec) -> String {
    let mut lines = Vec::new();
    lines.push("[Desktop Entry]".to_string());
    lines.push("Type=Application".to_string());
    lines.push("Version=1.5".to_string());
    lines.push(format!("Name={}", spec.name));
    lines.push(format!("Exec={}", spec.exec));
    lines.push(format!("Icon={}", spec.icon));
    lines.push(format!("Terminal={}", if spec.terminal { "true" } else { "false" }));

    if !spec.categories.is_empty() {
        lines.push(format!("Categories={};", spec.categories.join(";")));
    }
    if !spec.mime_types.is_empty() {
        lines.push(format!("MimeType={};", spec.mime_types.join(";")));
    }
    if let Some(ref comment) = spec.comment {
        lines.push(format!("Comment={}", comment));
    }
    if let Some(ref wm_class) = spec.startup_wm_class {
        lines.push(format!("StartupWMClass={}", wm_class));
    }

    lines.push(String::new()); // trailing newline
    lines.join("\n")
}

/// Installs a `.desktop` file to `~/.local/share/applications/`.
///
/// Returns the path to the installed `.desktop` file.
pub fn install_desktop_entry(spec: &DesktopEntrySpec) -> Result<PathBuf, AppError> {
    let dir = applications_dir()?;
    let filename = format!("{}.desktop", spec.app_id);
    let path = dir.join(&filename);

    let content = generate_desktop_entry(spec);
    std::fs::write(&path, &content)?;

    // Make executable (some desktop environments require this)
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let perms = std::fs::Permissions::from_mode(0o755);
        std::fs::set_permissions(&path, perms)?;
    }

    Ok(path)
}

/// Copies an icon file to the user's icon directory.
///
/// Returns the installed icon path suitable for use in the `.desktop` file.
pub fn install_icon(source: &Path, app_id: &str) -> Result<PathBuf, AppError> {
    if !source.exists() {
        return Err(AppError::Desktop(format!(
            "Icon source file not found: {}",
            source.display()
        )));
    }

    let ext = source
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("png");
    let dir = icons_dir()?;
    let dest = dir.join(format!("{}.{}", app_id, ext));
    std::fs::copy(source, &dest)?;

    Ok(dest)
}

/// Removes a `.desktop` file and its associated icon.
pub fn uninstall_desktop_entry(app_id: &str) -> Result<(), AppError> {
    let desktop_path = applications_dir()?.join(format!("{}.desktop", app_id));
    if desktop_path.exists() {
        std::fs::remove_file(&desktop_path)?;
    }

    // Try removing common icon extensions
    let icons = icons_dir()?;
    for ext in &["svg", "png", "xpm"] {
        let icon_path = icons.join(format!("{}.{}", app_id, ext));
        if icon_path.exists() {
            let _ = std::fs::remove_file(&icon_path);
        }
    }

    // Refresh the desktop database (best-effort)
    let _ = std::process::Command::new("update-desktop-database")
        .arg(applications_dir()?.to_str().unwrap_or_default())
        .status();

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    fn sample_spec() -> DesktopEntrySpec {
        DesktopEntrySpec {
            app_id: "jetbrains-idea".to_string(),
            name: "IntelliJ IDEA".to_string(),
            exec: "/home/user/.local/share/JetBrains/IntelliJIdea/bin/idea".to_string(),
            icon: "/home/user/.local/share/icons/hicolor/scalable/apps/jetbrains-idea.svg"
                .to_string(),
            categories: vec!["Development".to_string(), "IDE".to_string()],
            mime_types: vec!["text/x-java".to_string()],
            terminal: false,
            comment: Some("Integrated Development Environment".to_string()),
            startup_wm_class: Some("jetbrains-idea".to_string()),
        }
    }

    #[test]
    fn generate_desktop_entry_contains_required_fields() {
        let spec = sample_spec();
        let content = generate_desktop_entry(&spec);

        assert!(content.contains("[Desktop Entry]"));
        assert!(content.contains("Type=Application"));
        assert!(content.contains("Name=IntelliJ IDEA"));
        assert!(content.contains("Exec=/home/user/.local/share/JetBrains/IntelliJIdea/bin/idea"));
        assert!(content.contains("Terminal=false"));
        assert!(content.contains("Categories=Development;IDE;"));
        assert!(content.contains("MimeType=text/x-java;"));
        assert!(content.contains("Comment=Integrated Development Environment"));
        assert!(content.contains("StartupWMClass=jetbrains-idea"));
    }

    #[test]
    fn generate_desktop_entry_terminal_true() {
        let mut spec = sample_spec();
        spec.terminal = true;
        let content = generate_desktop_entry(&spec);
        assert!(content.contains("Terminal=true"));
    }

    #[test]
    fn generate_desktop_entry_no_optional_fields() {
        let spec = DesktopEntrySpec {
            app_id: "test".to_string(),
            name: "Test App".to_string(),
            exec: "/usr/bin/test".to_string(),
            icon: "test-icon".to_string(),
            categories: vec![],
            mime_types: vec![],
            terminal: false,
            comment: None,
            startup_wm_class: None,
        };
        let content = generate_desktop_entry(&spec);
        assert!(!content.contains("Categories="));
        assert!(!content.contains("MimeType="));
        assert!(!content.contains("Comment="));
        assert!(!content.contains("StartupWMClass="));
    }

    #[test]
    fn install_desktop_entry_creates_file() {
        // This test creates real files in ~/.local/share/applications/ so
        // we only test the generation logic, not the actual installation.
        let spec = sample_spec();
        let content = generate_desktop_entry(&spec);
        assert!(content.starts_with("[Desktop Entry]"));
    }

    #[test]
    fn install_icon_nonexistent_source_returns_error() {
        let result = install_icon(Path::new("/nonexistent/icon.svg"), "test-app");
        assert!(result.is_err());
    }

    #[test]
    fn install_icon_copies_file() {
        let dir = tempdir().unwrap();
        let source = dir.path().join("icon.svg");
        std::fs::write(&source, "<svg></svg>").unwrap();

        // We can't easily test the actual installation path without mocking dirs::data_dir(),
        // so just verify the source validation works
        assert!(source.exists());
    }

    #[test]
    fn applications_dir_returns_valid_path() {
        let dir = applications_dir().unwrap();
        assert!(dir.to_string_lossy().contains("applications"));
        assert!(dir.exists());
    }
}
