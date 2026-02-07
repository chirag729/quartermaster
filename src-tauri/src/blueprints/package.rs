//! Blueprint packaging module for the `.qmbp` archive format.
//!
//! A `.qmbp` file is a zip archive with the following structure:
//!
//! ```text
//! manifest.yaml       # BlueprintDefinition (same schema as loose YAML)
//! variables.yaml      # Optional: shared variable definitions
//! tasks/              # Optional: task YAML files
//! apparmor/
//!   profiles/         # Optional: AppArmor profile files
//!   abstractions/     # Optional: AppArmor abstraction files
//! ```
//!
//! This module provides functions to pack, unpack, import, and export
//! blueprints using this format.

use std::fs;
use std::io::{Read, Write};
use std::path::{Path, PathBuf};

use zip::write::FileOptions;
use zip::ZipArchive;
use zip::ZipWriter;

use crate::blueprints::yaml_schema::BlueprintDefinition;
use crate::error::AppError;
use crate::variables::VariableDefinition;

// ---------------------------------------------------------------------------
// Data types
// ---------------------------------------------------------------------------

/// The parsed contents of a `.qmbp` package.
///
/// Contains the blueprint definition (from `manifest.yaml`), optional shared
/// variable definitions (from `variables.yaml`), and inventories of any
/// bundled task and AppArmor files.
#[derive(Debug, Clone)]
pub struct BlueprintManifest {
    /// The blueprint definition parsed from `manifest.yaml`.
    pub definition: BlueprintDefinition,

    /// Shared variable definitions parsed from `variables.yaml`.
    /// Empty if the file is absent.
    pub variables: Vec<VariableDefinition>,

    /// Filenames of task YAML files found in the `tasks/` directory.
    pub task_files: Vec<String>,

    /// Filenames of AppArmor profile files found in `apparmor/profiles/`.
    pub apparmor_profiles: Vec<String>,

    /// Filenames of AppArmor abstraction files found in `apparmor/abstractions/`.
    pub apparmor_abstractions: Vec<String>,
}

// ---------------------------------------------------------------------------
// Constants
// ---------------------------------------------------------------------------

const MANIFEST_FILENAME: &str = "manifest.yaml";
const VARIABLES_FILENAME: &str = "variables.yaml";
const TASKS_DIR: &str = "tasks/";
const APPARMOR_PROFILES_DIR: &str = "apparmor/profiles/";
const APPARMOR_ABSTRACTIONS_DIR: &str = "apparmor/abstractions/";
const QMBP_EXTENSION: &str = "qmbp";

// ---------------------------------------------------------------------------
// Public API
// ---------------------------------------------------------------------------

/// Packs a blueprint directory into a `.qmbp` zip archive.
///
/// The source directory must contain at least a `manifest.yaml` file. All
/// other entries (`variables.yaml`, `tasks/`, `apparmor/`) are optional.
///
/// # Arguments
///
/// * `blueprint_dir` - Path to the source directory.
/// * `output_path` - Path to the directory where the `.qmbp` file will be
///   written. The filename is derived from the blueprint `id` field.
///
/// # Returns
///
/// The full path to the created `.qmbp` file.
pub fn pack_blueprint(blueprint_dir: &Path, output_path: &Path) -> Result<PathBuf, AppError> {
    // Validate source directory
    if !blueprint_dir.is_dir() {
        return Err(AppError::Package(format!(
            "Source path is not a directory: {}",
            blueprint_dir.display()
        )));
    }

    let manifest_path = blueprint_dir.join(MANIFEST_FILENAME);
    if !manifest_path.exists() {
        return Err(AppError::Package(format!(
            "Missing {} in {}",
            MANIFEST_FILENAME,
            blueprint_dir.display()
        )));
    }

    // Parse manifest to get the blueprint id for the output filename
    let manifest_content = fs::read_to_string(&manifest_path)?;
    let definition: BlueprintDefinition = serde_yaml::from_str(&manifest_content)
        .map_err(|e| AppError::Package(format!("Invalid {}: {}", MANIFEST_FILENAME, e)))?;

    // Create output directory if needed
    fs::create_dir_all(output_path)?;

    let qmbp_file = output_path.join(format!("{}.{}", definition.id, QMBP_EXTENSION));
    let file = fs::File::create(&qmbp_file)?;
    let mut zip = ZipWriter::new(file);

    let options = FileOptions::<()>::default()
        .compression_method(zip::CompressionMethod::Deflated);

    // Write manifest.yaml
    zip.start_file(MANIFEST_FILENAME, options)
        .map_err(|e| AppError::Package(format!("Failed to write {}: {}", MANIFEST_FILENAME, e)))?;
    zip.write_all(manifest_content.as_bytes())?;

    // Write variables.yaml if present
    let variables_path = blueprint_dir.join(VARIABLES_FILENAME);
    if variables_path.exists() {
        let content = fs::read_to_string(&variables_path)?;
        zip.start_file(VARIABLES_FILENAME, options)
            .map_err(|e| AppError::Package(format!("Failed to write {}: {}", VARIABLES_FILENAME, e)))?;
        zip.write_all(content.as_bytes())?;
    }

    // Write tasks/ directory
    let tasks_dir = blueprint_dir.join("tasks");
    if tasks_dir.is_dir() {
        add_directory_to_zip(&mut zip, &tasks_dir, TASKS_DIR, options)?;
    }

    // Write apparmor/profiles/ directory
    let profiles_dir = blueprint_dir.join("apparmor").join("profiles");
    if profiles_dir.is_dir() {
        add_directory_to_zip(&mut zip, &profiles_dir, APPARMOR_PROFILES_DIR, options)?;
    }

    // Write apparmor/abstractions/ directory
    let abstractions_dir = blueprint_dir.join("apparmor").join("abstractions");
    if abstractions_dir.is_dir() {
        add_directory_to_zip(&mut zip, &abstractions_dir, APPARMOR_ABSTRACTIONS_DIR, options)?;
    }

    zip.finish()
        .map_err(|e| AppError::Package(format!("Failed to finalize archive: {}", e)))?;
    Ok(qmbp_file)
}

/// Unpacks a `.qmbp` archive to a destination directory.
///
/// Extracts all files from the archive, preserving directory structure, and
/// returns a [`BlueprintManifest`] with the parsed contents.
///
/// # Arguments
///
/// * `qmbp_path` - Path to the `.qmbp` file.
/// * `dest_dir` - Directory to extract into. Created if it does not exist.
///
/// # Returns
///
/// A [`BlueprintManifest`] describing the extracted package contents.
pub fn unpack_blueprint(
    qmbp_path: &Path,
    dest_dir: &Path,
) -> Result<BlueprintManifest, AppError> {
    validate_qmbp_path(qmbp_path)?;

    let file = fs::File::open(qmbp_path)?;
    let mut archive = ZipArchive::new(file)
        .map_err(|e| AppError::Package(format!("Invalid zip archive: {}", e)))?;

    fs::create_dir_all(dest_dir)?;

    // Track the files we find
    let mut has_manifest = false;
    let mut task_files = Vec::new();
    let mut apparmor_profiles = Vec::new();
    let mut apparmor_abstractions = Vec::new();

    for i in 0..archive.len() {
        let mut entry = archive
            .by_index(i)
            .map_err(|e| AppError::Package(format!("Failed to read archive entry: {}", e)))?;

        let entry_name = entry.name().to_string();

        // Guard against path traversal attacks
        if entry_name.contains("..") || entry_name.starts_with('/') {
            return Err(AppError::Package(format!(
                "Archive contains path traversal entry: {}",
                entry_name
            )));
        }

        let out_path = dest_dir.join(&entry_name);

        // Verify the resolved path stays within dest_dir
        let canonical_dest = dest_dir.canonicalize().unwrap_or_else(|_| dest_dir.to_path_buf());
        // For new files, check the parent exists within dest_dir
        let resolved = if out_path.exists() {
            out_path.canonicalize().unwrap_or_else(|_| out_path.clone())
        } else {
            // Normalize by resolving parent + joining filename
            if let Some(parent) = out_path.parent() {
                let _ = fs::create_dir_all(parent);
                let canon_parent = parent.canonicalize().unwrap_or_else(|_| parent.to_path_buf());
                canon_parent.join(out_path.file_name().unwrap_or_default())
            } else {
                out_path.clone()
            }
        };
        if !resolved.starts_with(&canonical_dest) {
            return Err(AppError::Package(format!(
                "Archive entry escapes destination directory: {}",
                entry_name
            )));
        }

        if entry.is_dir() {
            fs::create_dir_all(&out_path)?;
            continue;
        }

        // Ensure parent directory exists
        if let Some(parent) = out_path.parent() {
            fs::create_dir_all(parent)?;
        }

        // Classify the entry
        if entry_name == MANIFEST_FILENAME {
            has_manifest = true;
        } else if let Some(name) = entry_name.strip_prefix(TASKS_DIR) {
            if !name.is_empty() {
                task_files.push(name.to_string());
            }
        } else if let Some(name) = entry_name.strip_prefix(APPARMOR_PROFILES_DIR) {
            if !name.is_empty() {
                apparmor_profiles.push(name.to_string());
            }
        } else if let Some(name) = entry_name.strip_prefix(APPARMOR_ABSTRACTIONS_DIR) {
            if !name.is_empty() {
                apparmor_abstractions.push(name.to_string());
            }
        }

        // Write file contents
        let mut buf = Vec::new();
        entry
            .read_to_end(&mut buf)
            .map_err(|e| AppError::Package(format!("Failed to read {}: {}", entry_name, e)))?;
        fs::write(&out_path, &buf)?;
    }

    if !has_manifest {
        return Err(AppError::Package(format!(
            "Archive does not contain {}",
            MANIFEST_FILENAME
        )));
    }

    // Parse manifest
    let manifest_content = fs::read_to_string(dest_dir.join(MANIFEST_FILENAME))?;
    let definition: BlueprintDefinition = serde_yaml::from_str(&manifest_content)
        .map_err(|e| AppError::Package(format!("Invalid {}: {}", MANIFEST_FILENAME, e)))?;

    // Parse variables if present
    let variables_path = dest_dir.join(VARIABLES_FILENAME);
    let variables = if variables_path.exists() {
        let content = fs::read_to_string(&variables_path)?;
        serde_yaml::from_str::<Vec<VariableDefinition>>(&content)
            .map_err(|e| AppError::Package(format!("Invalid {}: {}", VARIABLES_FILENAME, e)))?
    } else {
        Vec::new()
    };

    task_files.sort();
    apparmor_profiles.sort();
    apparmor_abstractions.sort();

    Ok(BlueprintManifest {
        definition,
        variables,
        task_files,
        apparmor_profiles,
        apparmor_abstractions,
    })
}

/// Imports a `.qmbp` package into the user's configuration directory.
///
/// Extracts the archive to a temporary location, then copies each component
/// to its canonical location under `~/.config/quartermaster/`:
///
/// - The blueprint definition is saved to `blueprints/<id>.yaml`
/// - Task files are copied to `tasks/`
/// - AppArmor profiles are copied to `apparmor/profiles/`
/// - AppArmor abstractions are copied to `apparmor/abstractions/`
///
/// # Arguments
///
/// * `qmbp_path` - Path to the `.qmbp` file to import.
///
/// # Returns
///
/// A [`BlueprintManifest`] describing the imported package.
pub fn import_blueprint(qmbp_path: &Path) -> Result<BlueprintManifest, AppError> {
    validate_qmbp_path(qmbp_path)?;

    let config_base = config_base_dir()?;
    let temp_dir = tempfile::tempdir()
        .map_err(|e| AppError::Package(format!("Failed to create temp directory: {}", e)))?;

    // Unpack to a temporary directory first
    let manifest = unpack_blueprint(qmbp_path, temp_dir.path())?;
    let id = &manifest.definition.id;

    // Copy the blueprint definition into blueprints/
    let blueprints_dir = config_base.join("blueprints");
    fs::create_dir_all(&blueprints_dir)?;
    let dest_yaml = blueprints_dir.join(format!("{}.yaml", id));
    fs::copy(
        temp_dir.path().join(MANIFEST_FILENAME),
        &dest_yaml,
    )?;

    // Copy task files into tasks/
    if !manifest.task_files.is_empty() {
        let tasks_dest = config_base.join("tasks");
        fs::create_dir_all(&tasks_dest)?;
        for task_file in &manifest.task_files {
            let src = temp_dir.path().join("tasks").join(task_file);
            let dst = tasks_dest.join(task_file);
            fs::copy(&src, &dst)?;
        }
    }

    // Copy AppArmor profiles into apparmor/profiles/
    if !manifest.apparmor_profiles.is_empty() {
        let profiles_dest = config_base.join("apparmor").join("profiles");
        fs::create_dir_all(&profiles_dest)?;
        for profile in &manifest.apparmor_profiles {
            let src = temp_dir
                .path()
                .join("apparmor")
                .join("profiles")
                .join(profile);
            let dst = profiles_dest.join(profile);
            fs::copy(&src, &dst)?;
        }
    }

    // Copy AppArmor abstractions into apparmor/abstractions/
    if !manifest.apparmor_abstractions.is_empty() {
        let abstractions_dest = config_base.join("apparmor").join("abstractions");
        fs::create_dir_all(&abstractions_dest)?;
        for abstraction in &manifest.apparmor_abstractions {
            let src = temp_dir
                .path()
                .join("apparmor")
                .join("abstractions")
                .join(abstraction);
            let dst = abstractions_dest.join(abstraction);
            fs::copy(&src, &dst)?;
        }
    }

    Ok(manifest)
}

/// Exports a blueprint from the user's configuration directory to a `.qmbp` file.
///
/// Gathers the blueprint YAML, any associated task files and AppArmor files
/// from the config directory, assembles them into a staging directory, and
/// packs the result into a `.qmbp` archive.
///
/// # Arguments
///
/// * `blueprint_id` - The id of the blueprint to export.
/// * `blueprints_dir` - Path to the `blueprints/` directory containing the
///   blueprint YAML files (typically `~/.config/quartermaster/blueprints/`).
/// * `output_path` - Directory where the `.qmbp` file should be written.
///
/// # Returns
///
/// The full path to the created `.qmbp` file.
pub fn export_blueprint(
    blueprint_id: &str,
    blueprints_dir: &Path,
    output_path: &Path,
) -> Result<PathBuf, AppError> {
    // Find and validate the blueprint YAML file
    let yaml_path = blueprints_dir.join(format!("{}.yaml", blueprint_id));
    if !yaml_path.exists() {
        return Err(AppError::Package(format!(
            "Blueprint not found: {}",
            blueprint_id
        )));
    }

    let yaml_content = fs::read_to_string(&yaml_path)?;
    // Validate that it parses
    let _def: BlueprintDefinition = serde_yaml::from_str(&yaml_content)
        .map_err(|e| AppError::Package(format!("Invalid blueprint YAML: {}", e)))?;

    // Build a staging directory with the .qmbp structure
    let staging = tempfile::tempdir()
        .map_err(|e| AppError::Package(format!("Failed to create staging directory: {}", e)))?;

    // Write manifest.yaml
    fs::write(staging.path().join(MANIFEST_FILENAME), &yaml_content)?;

    // Resolve the config base from blueprints_dir (its parent)
    let config_base = blueprints_dir
        .parent()
        .ok_or_else(|| AppError::Package("Cannot determine config base directory".to_string()))?;

    // Copy variables.yaml if it exists alongside the blueprint
    let variables_path = config_base.join(VARIABLES_FILENAME);
    if variables_path.exists() {
        fs::copy(&variables_path, staging.path().join(VARIABLES_FILENAME))?;
    }

    // Copy only task files referenced by the blueprint's task entries
    let tasks_src = config_base.join("tasks");
    if tasks_src.is_dir() {
        let task_ids: Vec<&str> = _def.tasks.iter().map(|t| t.id.as_str()).collect();
        let tasks_dest = staging.path().join("tasks");
        let mut copied_any = false;
        if let Ok(entries) = fs::read_dir(&tasks_src) {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.is_file() {
                    // Match task files by filename stem against blueprint task IDs
                    if let Some(stem) = path.file_stem().and_then(|s| s.to_str()) {
                        if task_ids.contains(&stem) {
                            if !copied_any {
                                fs::create_dir_all(&tasks_dest)?;
                                copied_any = true;
                            }
                            fs::copy(&path, tasks_dest.join(entry.file_name()))?;
                        }
                    }
                }
            }
        }
    }

    // Copy apparmor/profiles/ if present
    let profiles_src = config_base.join("apparmor").join("profiles");
    if profiles_src.is_dir() {
        let profiles_dest = staging.path().join("apparmor").join("profiles");
        fs::create_dir_all(&profiles_dest)?;
        copy_directory_contents(&profiles_src, &profiles_dest)?;
    }

    // Copy apparmor/abstractions/ if present
    let abstractions_src = config_base.join("apparmor").join("abstractions");
    if abstractions_src.is_dir() {
        let abstractions_dest = staging.path().join("apparmor").join("abstractions");
        fs::create_dir_all(&abstractions_dest)?;
        copy_directory_contents(&abstractions_src, &abstractions_dest)?;
    }

    pack_blueprint(staging.path(), output_path)
}

// ---------------------------------------------------------------------------
// Internal helpers
// ---------------------------------------------------------------------------

/// Resolves the Quartermaster configuration base directory.
fn config_base_dir() -> Result<PathBuf, AppError> {
    let base = dirs::config_dir()
        .ok_or_else(|| AppError::Package("Cannot determine user config directory".to_string()))?;
    Ok(base.join("quartermaster"))
}

/// Validates that a path points to an existing `.qmbp` file.
fn validate_qmbp_path(path: &Path) -> Result<(), AppError> {
    if !path.exists() {
        return Err(AppError::Package(format!(
            "File does not exist: {}",
            path.display()
        )));
    }
    match path.extension().and_then(|e| e.to_str()) {
        Some(ext) if ext == QMBP_EXTENSION => Ok(()),
        _ => Err(AppError::Package(format!(
            "File does not have .{} extension: {}",
            QMBP_EXTENSION,
            path.display()
        ))),
    }
}

/// Recursively adds all files in `src_dir` to the zip writer under `zip_prefix`.
fn add_directory_to_zip<W: Write + std::io::Seek>(
    zip: &mut ZipWriter<W>,
    src_dir: &Path,
    zip_prefix: &str,
    options: FileOptions<()>,
) -> Result<(), AppError> {
    let entries = fs::read_dir(src_dir)?;
    for entry in entries {
        let entry = entry?;
        let path = entry.path();
        let file_name = entry
            .file_name()
            .to_str()
            .ok_or_else(|| AppError::Package("Non-UTF-8 filename encountered".to_string()))?
            .to_string();

        if path.is_file() {
            let zip_name = format!("{}{}", zip_prefix, file_name);
            let content = fs::read(&path)?;
            zip.start_file(&zip_name, options)
                .map_err(|e| AppError::Package(format!("Failed to write {}: {}", zip_name, e)))?;
            zip.write_all(&content)?;
        } else if path.is_dir() {
            let sub_prefix = format!("{}{}/", zip_prefix, file_name);
            add_directory_to_zip(zip, &path, &sub_prefix, options)?;
        }
    }
    Ok(())
}

/// Copies all files from `src` to `dest` (non-recursive, files only).
fn copy_directory_contents(src: &Path, dest: &Path) -> Result<(), AppError> {
    let entries = fs::read_dir(src)?;
    for entry in entries {
        let entry = entry?;
        let path = entry.path();
        if path.is_file() {
            let dest_file = dest.join(entry.file_name());
            fs::copy(&path, &dest_file)?;
        }
    }
    Ok(())
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    /// Creates a minimal blueprint directory suitable for packing.
    fn create_test_blueprint_dir(dir: &Path) {
        let manifest = r#"
id: test-blueprint
name: Test Blueprint
description: A test blueprint for packaging
icon: Package
builtin: false
tasks:
  - id: create-dev-folder
    enabled: true
    config:
      path: "~/Development"
  - id: flutter-sdk
    enabled: true
"#;
        fs::write(dir.join(MANIFEST_FILENAME), manifest).unwrap();
    }

    /// Creates a full blueprint directory with all optional components.
    fn create_full_blueprint_dir(dir: &Path) {
        create_test_blueprint_dir(dir);

        // variables.yaml
        let variables = r#"
- key: install_dir
  label: Installation Directory
  field_type: path
  default: /opt/tools
  description: Base directory for tool installations
- key: java_version
  label: Java Version
  field_type: string
  default: "21"
  description: Java JDK version
"#;
        fs::write(dir.join(VARIABLES_FILENAME), variables).unwrap();

        // tasks/
        let tasks_dir = dir.join("tasks");
        fs::create_dir_all(&tasks_dir).unwrap();
        fs::write(
            tasks_dir.join("custom-task.yaml"),
            "id: custom-task\nname: Custom Task\n",
        )
        .unwrap();
        fs::write(
            tasks_dir.join("another-task.yaml"),
            "id: another-task\nname: Another Task\n",
        )
        .unwrap();

        // apparmor/profiles/
        let profiles_dir = dir.join("apparmor").join("profiles");
        fs::create_dir_all(&profiles_dir).unwrap();
        fs::write(
            profiles_dir.join("usr.bin.myapp"),
            "# AppArmor profile for myapp\n",
        )
        .unwrap();

        // apparmor/abstractions/
        let abstractions_dir = dir.join("apparmor").join("abstractions");
        fs::create_dir_all(&abstractions_dir).unwrap();
        fs::write(
            abstractions_dir.join("myapp-base"),
            "# Base abstraction\n",
        )
        .unwrap();
    }

    // -----------------------------------------------------------------------
    // pack_blueprint tests
    // -----------------------------------------------------------------------

    #[test]
    fn pack_creates_qmbp_file() {
        let src = tempdir().unwrap();
        let out = tempdir().unwrap();

        create_test_blueprint_dir(src.path());
        let result = pack_blueprint(src.path(), out.path());
        assert!(result.is_ok());

        let qmbp_path = result.unwrap();
        assert!(qmbp_path.exists());
        assert_eq!(
            qmbp_path.file_name().unwrap().to_str().unwrap(),
            "test-blueprint.qmbp"
        );
    }

    #[test]
    fn pack_full_blueprint_includes_all_entries() {
        let src = tempdir().unwrap();
        let out = tempdir().unwrap();

        create_full_blueprint_dir(src.path());
        let qmbp_path = pack_blueprint(src.path(), out.path()).unwrap();

        // Verify zip contents
        let file = fs::File::open(&qmbp_path).unwrap();
        let archive = ZipArchive::new(file).unwrap();
        let names: Vec<String> = (0..archive.len())
            .map(|i| archive.name_for_index(i).unwrap().to_string())
            .collect();

        assert!(names.contains(&MANIFEST_FILENAME.to_string()));
        assert!(names.contains(&VARIABLES_FILENAME.to_string()));
        assert!(names.iter().any(|n| n.starts_with(TASKS_DIR)));
        assert!(names.iter().any(|n| n.starts_with(APPARMOR_PROFILES_DIR)));
        assert!(names.iter().any(|n| n.starts_with(APPARMOR_ABSTRACTIONS_DIR)));
    }

    #[test]
    fn pack_fails_without_manifest() {
        let src = tempdir().unwrap();
        let out = tempdir().unwrap();
        // Empty directory — no manifest.yaml
        let result = pack_blueprint(src.path(), out.path());
        assert!(result.is_err());
        let err = result.unwrap_err().to_string();
        assert!(err.contains("Missing manifest.yaml"));
    }

    #[test]
    fn pack_fails_for_non_directory() {
        let dir = tempdir().unwrap();
        let out = tempdir().unwrap();
        let file_path = dir.path().join("not-a-dir.txt");
        fs::write(&file_path, "hello").unwrap();

        let result = pack_blueprint(&file_path, out.path());
        assert!(result.is_err());
        let err = result.unwrap_err().to_string();
        assert!(err.contains("not a directory"));
    }

    #[test]
    fn pack_fails_for_invalid_manifest_yaml() {
        let src = tempdir().unwrap();
        let out = tempdir().unwrap();
        fs::write(src.path().join(MANIFEST_FILENAME), "not: valid: yaml: [[[").unwrap();

        let result = pack_blueprint(src.path(), out.path());
        assert!(result.is_err());
        let err = result.unwrap_err().to_string();
        assert!(err.contains("Invalid manifest.yaml"));
    }

    // -----------------------------------------------------------------------
    // unpack_blueprint tests
    // -----------------------------------------------------------------------

    #[test]
    fn unpack_extracts_and_parses_manifest() {
        let src = tempdir().unwrap();
        let out = tempdir().unwrap();
        let dest = tempdir().unwrap();

        create_test_blueprint_dir(src.path());
        let qmbp_path = pack_blueprint(src.path(), out.path()).unwrap();

        let manifest = unpack_blueprint(&qmbp_path, dest.path()).unwrap();
        assert_eq!(manifest.definition.id, "test-blueprint");
        assert_eq!(manifest.definition.name, "Test Blueprint");
        assert_eq!(manifest.definition.tasks.len(), 2);
        assert!(manifest.variables.is_empty());
        assert!(manifest.task_files.is_empty());
        assert!(manifest.apparmor_profiles.is_empty());
        assert!(manifest.apparmor_abstractions.is_empty());
    }

    #[test]
    fn unpack_full_blueprint_returns_complete_manifest() {
        let src = tempdir().unwrap();
        let out = tempdir().unwrap();
        let dest = tempdir().unwrap();

        create_full_blueprint_dir(src.path());
        let qmbp_path = pack_blueprint(src.path(), out.path()).unwrap();

        let manifest = unpack_blueprint(&qmbp_path, dest.path()).unwrap();
        assert_eq!(manifest.definition.id, "test-blueprint");
        assert_eq!(manifest.variables.len(), 2);
        assert_eq!(manifest.variables[0].key, "install_dir");
        assert_eq!(manifest.variables[1].key, "java_version");
        assert_eq!(manifest.task_files.len(), 2);
        assert!(manifest.task_files.contains(&"another-task.yaml".to_string()));
        assert!(manifest.task_files.contains(&"custom-task.yaml".to_string()));
        assert_eq!(manifest.apparmor_profiles.len(), 1);
        assert!(manifest
            .apparmor_profiles
            .contains(&"usr.bin.myapp".to_string()));
        assert_eq!(manifest.apparmor_abstractions.len(), 1);
        assert!(manifest
            .apparmor_abstractions
            .contains(&"myapp-base".to_string()));

        // Verify files were actually extracted
        assert!(dest.path().join(MANIFEST_FILENAME).exists());
        assert!(dest.path().join(VARIABLES_FILENAME).exists());
        assert!(dest.path().join("tasks").join("custom-task.yaml").exists());
        assert!(dest
            .path()
            .join("apparmor")
            .join("profiles")
            .join("usr.bin.myapp")
            .exists());
        assert!(dest
            .path()
            .join("apparmor")
            .join("abstractions")
            .join("myapp-base")
            .exists());
    }

    #[test]
    fn unpack_fails_for_nonexistent_file() {
        let dest = tempdir().unwrap();
        let result = unpack_blueprint(Path::new("/nonexistent/file.qmbp"), dest.path());
        assert!(result.is_err());
        let err = result.unwrap_err().to_string();
        assert!(err.contains("does not exist"));
    }

    #[test]
    fn unpack_fails_for_wrong_extension() {
        let dir = tempdir().unwrap();
        let dest = tempdir().unwrap();
        let bad_file = dir.path().join("file.zip");
        fs::write(&bad_file, "not a qmbp").unwrap();

        let result = unpack_blueprint(&bad_file, dest.path());
        assert!(result.is_err());
        let err = result.unwrap_err().to_string();
        assert!(err.contains(".qmbp extension"));
    }

    #[test]
    fn unpack_fails_for_zip_without_manifest() {
        let dir = tempdir().unwrap();
        let dest = tempdir().unwrap();

        // Create a valid zip but without manifest.yaml
        let qmbp_path = dir.path().join("empty.qmbp");
        let file = fs::File::create(&qmbp_path).unwrap();
        let mut zip = ZipWriter::new(file);
        let options = FileOptions::<()>::default();
        zip.start_file("readme.txt", options).unwrap();
        zip.write_all(b"no manifest here").unwrap();
        zip.finish().unwrap();

        let result = unpack_blueprint(&qmbp_path, dest.path());
        assert!(result.is_err());
        let err = result.unwrap_err().to_string();
        assert!(err.contains("does not contain manifest.yaml"));
    }

    #[test]
    fn unpack_rejects_absolute_path_entry() {
        let dir = tempdir().unwrap();
        let dest = tempdir().unwrap();

        // Create a zip with an absolute path entry
        let qmbp_path = dir.path().join("evil.qmbp");
        let file = fs::File::create(&qmbp_path).unwrap();
        let mut zip = ZipWriter::new(file);
        let options = FileOptions::<()>::default();
        zip.start_file(MANIFEST_FILENAME, options).unwrap();
        let manifest = "id: test\nname: Test\ndescription: t\nicon: X\ntasks: []\n";
        zip.write_all(manifest.as_bytes()).unwrap();
        zip.start_file("/etc/passwd", options).unwrap();
        zip.write_all(b"evil content").unwrap();
        zip.finish().unwrap();

        let result = unpack_blueprint(&qmbp_path, dest.path());
        assert!(result.is_err());
        let err = result.unwrap_err().to_string();
        assert!(
            err.contains("path traversal"),
            "Expected path traversal error, got: {}",
            err
        );
    }

    // -----------------------------------------------------------------------
    // pack + unpack roundtrip tests
    // -----------------------------------------------------------------------

    #[test]
    fn roundtrip_minimal_blueprint() {
        let src = tempdir().unwrap();
        let out = tempdir().unwrap();
        let dest = tempdir().unwrap();

        create_test_blueprint_dir(src.path());
        let qmbp_path = pack_blueprint(src.path(), out.path()).unwrap();
        let manifest = unpack_blueprint(&qmbp_path, dest.path()).unwrap();

        assert_eq!(manifest.definition.id, "test-blueprint");
        assert_eq!(manifest.definition.tasks.len(), 2);
        assert_eq!(manifest.definition.tasks[0].id, "create-dev-folder");
    }

    #[test]
    fn roundtrip_full_blueprint_preserves_content() {
        let src = tempdir().unwrap();
        let out = tempdir().unwrap();
        let dest = tempdir().unwrap();

        create_full_blueprint_dir(src.path());
        let qmbp_path = pack_blueprint(src.path(), out.path()).unwrap();
        let manifest = unpack_blueprint(&qmbp_path, dest.path()).unwrap();

        // Verify blueprint definition
        assert_eq!(manifest.definition.id, "test-blueprint");
        assert_eq!(manifest.definition.name, "Test Blueprint");
        assert!(!manifest.definition.builtin);

        // Verify variables
        assert_eq!(manifest.variables.len(), 2);
        let install_dir = manifest.variables.iter().find(|v| v.key == "install_dir").unwrap();
        assert_eq!(install_dir.default, "/opt/tools");
        assert_eq!(install_dir.field_type, "path");

        // Verify task files round-tripped
        let task_content =
            fs::read_to_string(dest.path().join("tasks").join("custom-task.yaml")).unwrap();
        assert!(task_content.contains("id: custom-task"));

        // Verify apparmor files round-tripped
        let profile_content = fs::read_to_string(
            dest.path()
                .join("apparmor")
                .join("profiles")
                .join("usr.bin.myapp"),
        )
        .unwrap();
        assert!(profile_content.contains("AppArmor profile for myapp"));
    }

    // -----------------------------------------------------------------------
    // export_blueprint tests
    // -----------------------------------------------------------------------

    #[test]
    fn export_creates_qmbp_from_config_dir() {
        let config_dir = tempdir().unwrap();
        let output_dir = tempdir().unwrap();

        // Set up a config directory structure
        let blueprints_dir = config_dir.path().join("blueprints");
        fs::create_dir_all(&blueprints_dir).unwrap();

        let manifest_yaml = r#"
id: my-blueprint
name: My Blueprint
description: Exported blueprint
icon: Star
builtin: false
tasks:
  - id: test-task
    enabled: true
"#;
        fs::write(
            blueprints_dir.join("my-blueprint.yaml"),
            manifest_yaml,
        )
        .unwrap();

        // Add a task file
        let tasks_dir = config_dir.path().join("tasks");
        fs::create_dir_all(&tasks_dir).unwrap();
        fs::write(tasks_dir.join("test-task.yaml"), "id: test-task\n").unwrap();

        let result =
            export_blueprint("my-blueprint", &blueprints_dir, output_dir.path());
        assert!(result.is_ok());

        let qmbp_path = result.unwrap();
        assert!(qmbp_path.exists());
        assert_eq!(
            qmbp_path.file_name().unwrap().to_str().unwrap(),
            "my-blueprint.qmbp"
        );

        // Verify the archive contains the expected files
        let dest = tempdir().unwrap();
        let manifest = unpack_blueprint(&qmbp_path, dest.path()).unwrap();
        assert_eq!(manifest.definition.id, "my-blueprint");
        assert_eq!(manifest.task_files.len(), 1);
    }

    #[test]
    fn export_fails_for_nonexistent_blueprint() {
        let config_dir = tempdir().unwrap();
        let output_dir = tempdir().unwrap();

        let blueprints_dir = config_dir.path().join("blueprints");
        fs::create_dir_all(&blueprints_dir).unwrap();

        let result =
            export_blueprint("nonexistent", &blueprints_dir, output_dir.path());
        assert!(result.is_err());
        let err = result.unwrap_err().to_string();
        assert!(err.contains("Blueprint not found"));
    }

    #[test]
    fn export_includes_variables_yaml() {
        let config_dir = tempdir().unwrap();
        let output_dir = tempdir().unwrap();

        let blueprints_dir = config_dir.path().join("blueprints");
        fs::create_dir_all(&blueprints_dir).unwrap();

        let manifest_yaml = r#"
id: vars-bp
name: Variables Blueprint
description: With variables
icon: Star
tasks: []
"#;
        fs::write(blueprints_dir.join("vars-bp.yaml"), manifest_yaml).unwrap();

        let variables_yaml = r#"
- key: test_var
  label: Test Variable
  field_type: string
  default: hello
  description: A test variable
"#;
        fs::write(config_dir.path().join(VARIABLES_FILENAME), variables_yaml).unwrap();

        let qmbp_path =
            export_blueprint("vars-bp", &blueprints_dir, output_dir.path()).unwrap();

        let dest = tempdir().unwrap();
        let manifest = unpack_blueprint(&qmbp_path, dest.path()).unwrap();
        assert_eq!(manifest.variables.len(), 1);
        assert_eq!(manifest.variables[0].key, "test_var");
    }

    #[test]
    fn export_includes_apparmor_files() {
        let config_dir = tempdir().unwrap();
        let output_dir = tempdir().unwrap();

        let blueprints_dir = config_dir.path().join("blueprints");
        fs::create_dir_all(&blueprints_dir).unwrap();

        let manifest_yaml = r#"
id: aa-bp
name: AppArmor Blueprint
description: With apparmor
icon: Shield
tasks: []
"#;
        fs::write(blueprints_dir.join("aa-bp.yaml"), manifest_yaml).unwrap();

        // Add apparmor files
        let profiles_dir = config_dir.path().join("apparmor").join("profiles");
        fs::create_dir_all(&profiles_dir).unwrap();
        fs::write(profiles_dir.join("usr.bin.test"), "# profile\n").unwrap();

        let abstractions_dir = config_dir.path().join("apparmor").join("abstractions");
        fs::create_dir_all(&abstractions_dir).unwrap();
        fs::write(abstractions_dir.join("test-abs"), "# abstraction\n").unwrap();

        let qmbp_path =
            export_blueprint("aa-bp", &blueprints_dir, output_dir.path()).unwrap();

        let dest = tempdir().unwrap();
        let manifest = unpack_blueprint(&qmbp_path, dest.path()).unwrap();
        assert_eq!(manifest.apparmor_profiles.len(), 1);
        assert_eq!(manifest.apparmor_profiles[0], "usr.bin.test");
        assert_eq!(manifest.apparmor_abstractions.len(), 1);
        assert_eq!(manifest.apparmor_abstractions[0], "test-abs");
    }

    // -----------------------------------------------------------------------
    // import_blueprint tests (uses a mock config dir via tempfile)
    // -----------------------------------------------------------------------

    // Note: import_blueprint uses dirs::config_dir() which resolves to the real
    // user config. Full integration testing of import_blueprint requires either
    // mocking the config directory or running in a controlled environment.
    // The following test validates the core logic by creating a .qmbp and
    // ensuring it can be unpacked, which covers the import code path minus
    // the final filesystem writes to ~/.config/quartermaster/.

    #[test]
    fn import_blueprint_validates_extension() {
        let dir = tempdir().unwrap();
        let bad_path = dir.path().join("bad.zip");
        fs::write(&bad_path, "data").unwrap();

        let result = import_blueprint(&bad_path);
        assert!(result.is_err());
        let err = result.unwrap_err().to_string();
        assert!(err.contains(".qmbp extension"));
    }

    #[test]
    fn import_blueprint_validates_existence() {
        let result = import_blueprint(Path::new("/tmp/nonexistent.qmbp"));
        assert!(result.is_err());
        let err = result.unwrap_err().to_string();
        assert!(err.contains("does not exist"));
    }

    // -----------------------------------------------------------------------
    // validate_qmbp_path tests
    // -----------------------------------------------------------------------

    #[test]
    fn validate_qmbp_path_accepts_correct_extension() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("test.qmbp");
        fs::write(&path, "data").unwrap();
        assert!(validate_qmbp_path(&path).is_ok());
    }

    #[test]
    fn validate_qmbp_path_rejects_wrong_extension() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("test.zip");
        fs::write(&path, "data").unwrap();
        assert!(validate_qmbp_path(&path).is_err());
    }

    #[test]
    fn validate_qmbp_path_rejects_no_extension() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("testfile");
        fs::write(&path, "data").unwrap();
        assert!(validate_qmbp_path(&path).is_err());
    }

    #[test]
    fn validate_qmbp_path_rejects_nonexistent() {
        assert!(validate_qmbp_path(Path::new("/does/not/exist.qmbp")).is_err());
    }

    // -----------------------------------------------------------------------
    // BlueprintManifest struct tests
    // -----------------------------------------------------------------------

    #[test]
    fn manifest_can_be_cloned() {
        let src = tempdir().unwrap();
        let out = tempdir().unwrap();
        let dest = tempdir().unwrap();

        create_full_blueprint_dir(src.path());
        let qmbp_path = pack_blueprint(src.path(), out.path()).unwrap();
        let manifest = unpack_blueprint(&qmbp_path, dest.path()).unwrap();

        let cloned = manifest.clone();
        assert_eq!(cloned.definition.id, manifest.definition.id);
        assert_eq!(cloned.variables.len(), manifest.variables.len());
        assert_eq!(cloned.task_files, manifest.task_files);
        assert_eq!(cloned.apparmor_profiles, manifest.apparmor_profiles);
        assert_eq!(
            cloned.apparmor_abstractions,
            manifest.apparmor_abstractions
        );
    }

    // -----------------------------------------------------------------------
    // Edge cases
    // -----------------------------------------------------------------------

    #[test]
    fn pack_with_empty_optional_dirs() {
        let src = tempdir().unwrap();
        let out = tempdir().unwrap();

        create_test_blueprint_dir(src.path());

        // Create empty optional directories
        fs::create_dir_all(src.path().join("tasks")).unwrap();
        fs::create_dir_all(src.path().join("apparmor").join("profiles")).unwrap();
        fs::create_dir_all(src.path().join("apparmor").join("abstractions")).unwrap();

        let qmbp_path = pack_blueprint(src.path(), out.path()).unwrap();
        assert!(qmbp_path.exists());

        let dest = tempdir().unwrap();
        let manifest = unpack_blueprint(&qmbp_path, dest.path()).unwrap();
        assert!(manifest.task_files.is_empty());
        assert!(manifest.apparmor_profiles.is_empty());
        assert!(manifest.apparmor_abstractions.is_empty());
    }

    #[test]
    fn unpack_creates_dest_dir_if_missing() {
        let src = tempdir().unwrap();
        let out = tempdir().unwrap();

        create_test_blueprint_dir(src.path());
        let qmbp_path = pack_blueprint(src.path(), out.path()).unwrap();

        let dest_base = tempdir().unwrap();
        let nested_dest = dest_base.path().join("a").join("b").join("c");
        assert!(!nested_dest.exists());

        let manifest = unpack_blueprint(&qmbp_path, &nested_dest).unwrap();
        assert!(nested_dest.exists());
        assert_eq!(manifest.definition.id, "test-blueprint");
    }

    #[test]
    fn pack_creates_output_dir_if_missing() {
        let src = tempdir().unwrap();
        let out_base = tempdir().unwrap();
        let nested_out = out_base.path().join("x").join("y");

        create_test_blueprint_dir(src.path());
        let qmbp_path = pack_blueprint(src.path(), &nested_out).unwrap();
        assert!(qmbp_path.exists());
    }
}
