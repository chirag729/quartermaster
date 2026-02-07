//! Download manager with caching, checksum verification, and archive extraction.
//!
//! Provides utilities for:
//! - Downloading files from URLs using `curl` (with resume support)
//! - Verifying SHA-256 checksums
//! - Extracting `tar.gz` and `zip` archives
//! - Caching downloads in `~/.cache/quartermaster/downloads/`
//!
//! All downloads are performed via `tokio::process::Command` calling `curl`,
//! avoiding the need for a Rust HTTP client dependency. Archive extraction
//! uses `tar` and `unzip` system commands.

use std::path::{Path, PathBuf};

use sha2::{Digest, Sha256};
use tokio::process::Command;

use crate::error::AppError;

/// Returns the cache directory for downloads.
///
/// Defaults to `~/.cache/quartermaster/downloads/`. Creates the directory
/// if it does not exist.
pub fn cache_dir() -> Result<PathBuf, AppError> {
    let dir = dirs::cache_dir()
        .unwrap_or_else(|| PathBuf::from("~/.cache"))
        .join("quartermaster")
        .join("downloads");
    std::fs::create_dir_all(&dir)?;
    Ok(dir)
}

/// Downloads a file from `url` to `dest`.
///
/// Uses `curl` with resume support (`-C -`). The `progress` callback receives
/// a completion fraction (0.0..=1.0) and a status message string.
///
/// If `dest` already exists as a complete file, the download is skipped.
/// Partial downloads are resumed automatically.
///
/// # Errors
///
/// Returns `AppError::Download` if `curl` fails or the destination cannot be written.
pub async fn download_file(
    url: &str,
    dest: &Path,
    progress: &dyn Fn(f32, String),
) -> Result<PathBuf, AppError> {
    // Ensure parent directory exists
    if let Some(parent) = dest.parent() {
        std::fs::create_dir_all(parent)?;
    }

    progress(0.0, format!("Starting download: {}", url));

    let output = Command::new("curl")
        .args([
            "-fSL",        // fail on HTTP errors, show errors, follow redirects
            "-C", "-",     // resume from where we left off
            "-o",
            dest.to_str().ok_or_else(|| {
                AppError::Download("Destination path contains invalid UTF-8".to_string())
            })?,
            url,
        ])
        .output()
        .await
        .map_err(|e| AppError::Download(format!("Failed to execute curl: {}", e)))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        // curl exit code 33 means the server doesn't support resume — retry without -C -
        if output.status.code() == Some(33) {
            progress(0.1, "Server does not support resume, restarting download".to_string());
            let retry_output = Command::new("curl")
                .args([
                    "-fSL",
                    "-o",
                    dest.to_str().unwrap(),
                    url,
                ])
                .output()
                .await
                .map_err(|e| AppError::Download(format!("Failed to execute curl: {}", e)))?;

            if !retry_output.status.success() {
                let retry_stderr = String::from_utf8_lossy(&retry_output.stderr);
                return Err(AppError::Download(format!(
                    "Download failed: {}",
                    retry_stderr.trim()
                )));
            }
        } else {
            return Err(AppError::Download(format!(
                "Download failed (exit code {}): {}",
                output.status.code().unwrap_or(-1),
                stderr.trim()
            )));
        }
    }

    progress(1.0, "Download complete".to_string());
    Ok(dest.to_path_buf())
}

/// Verifies that a file matches the expected SHA-256 checksum.
///
/// The `expected_sha256` should be a lowercase hex-encoded SHA-256 hash string.
///
/// # Returns
///
/// `Ok(true)` if the checksum matches, `Ok(false)` if it does not.
///
/// # Errors
///
/// Returns `AppError::Download` if the file cannot be read.
pub fn verify_checksum(file: &Path, expected_sha256: &str) -> Result<bool, AppError> {
    let data = std::fs::read(file).map_err(|e| {
        AppError::Download(format!(
            "Failed to read file for checksum verification '{}': {}",
            file.display(),
            e
        ))
    })?;

    let mut hasher = Sha256::new();
    hasher.update(&data);
    let result = hasher.finalize();
    let actual = hex::encode(result);

    Ok(actual == expected_sha256.to_lowercase())
}

/// Extracts an archive to the specified destination directory.
///
/// Supported formats:
/// - `"tar.gz"` / `"tgz"` — extracted via `tar xzf`
/// - `"tar.xz"` / `"txz"` — extracted via `tar xJf`
/// - `"tar.bz2"` / `"tbz2"` — extracted via `tar xjf`
/// - `"zip"` — extracted via `unzip`
///
/// # Errors
///
/// Returns `AppError::Download` if the extraction command fails or the format
/// is not recognized.
pub async fn extract_archive(
    archive: &Path,
    dest: &Path,
    format: &str,
) -> Result<(), AppError> {
    std::fs::create_dir_all(dest)?;

    let archive_str = archive.to_str().ok_or_else(|| {
        AppError::Download("Archive path contains invalid UTF-8".to_string())
    })?;
    let dest_str = dest.to_str().ok_or_else(|| {
        AppError::Download("Destination path contains invalid UTF-8".to_string())
    })?;

    let output = match format {
        "tar.gz" | "tgz" => {
            Command::new("tar")
                .args(["xzf", archive_str, "-C", dest_str])
                .output()
                .await
                .map_err(|e| AppError::Download(format!("Failed to execute tar: {}", e)))?
        }
        "tar.xz" | "txz" => {
            Command::new("tar")
                .args(["xJf", archive_str, "-C", dest_str])
                .output()
                .await
                .map_err(|e| AppError::Download(format!("Failed to execute tar: {}", e)))?
        }
        "tar.bz2" | "tbz2" => {
            Command::new("tar")
                .args(["xjf", archive_str, "-C", dest_str])
                .output()
                .await
                .map_err(|e| AppError::Download(format!("Failed to execute tar: {}", e)))?
        }
        "zip" => {
            Command::new("unzip")
                .args(["-o", archive_str, "-d", dest_str])
                .output()
                .await
                .map_err(|e| AppError::Download(format!("Failed to execute unzip: {}", e)))?
        }
        _ => {
            return Err(AppError::Download(format!(
                "Unsupported archive format: '{}'",
                format
            )));
        }
    };

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(AppError::Download(format!(
            "Extraction failed (exit code {}): {}",
            output.status.code().unwrap_or(-1),
            stderr.trim()
        )));
    }

    Ok(())
}

/// Downloads a file to the cache directory, using the filename from the URL.
///
/// If the file already exists in the cache and matches the expected checksum
/// (when provided), the cached copy is returned without re-downloading.
///
/// # Arguments
///
/// * `url` — the URL to download
/// * `expected_sha256` — optional SHA-256 checksum to verify; if `None`, no check is performed
/// * `progress` — callback for progress updates
///
/// # Errors
///
/// Returns `AppError::Download` on network errors, checksum mismatches, or
/// cache directory issues.
pub async fn download_to_cache(
    url: &str,
    expected_sha256: Option<&str>,
    progress: &dyn Fn(f32, String),
) -> Result<PathBuf, AppError> {
    let cache = cache_dir()?;

    // Derive filename from URL
    let filename = url
        .rsplit('/')
        .next()
        .filter(|s| !s.is_empty())
        .unwrap_or("download");
    let dest = cache.join(filename);

    // Check cache: if file exists and checksum matches, skip download
    if dest.exists() {
        if let Some(expected) = expected_sha256 {
            if verify_checksum(&dest, expected)? {
                progress(1.0, "Using cached file (checksum verified)".to_string());
                return Ok(dest);
            }
            // Checksum mismatch — re-download
            progress(0.0, "Cached file has wrong checksum, re-downloading".to_string());
        } else {
            progress(1.0, "Using cached file".to_string());
            return Ok(dest);
        }
    }

    download_file(url, &dest, progress).await?;

    // Verify checksum after download
    if let Some(expected) = expected_sha256 {
        if !verify_checksum(&dest, expected)? {
            // Remove the corrupted file
            let _ = std::fs::remove_file(&dest);
            return Err(AppError::Download(format!(
                "Checksum verification failed for '{}'",
                filename
            )));
        }
    }

    Ok(dest)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::tempdir;

    #[test]
    fn verify_checksum_correct() {
        let dir = tempdir().unwrap();
        let file = dir.path().join("test.txt");
        fs::write(&file, b"hello world").unwrap();

        // SHA-256 of "hello world"
        let expected = "b94d27b9934d3e08a52e52d7da7dabfac484efe37a5380ee9088f7ace2efcde9";
        assert!(verify_checksum(&file, expected).unwrap());
    }

    #[test]
    fn verify_checksum_incorrect() {
        let dir = tempdir().unwrap();
        let file = dir.path().join("test.txt");
        fs::write(&file, b"hello world").unwrap();

        let wrong = "0000000000000000000000000000000000000000000000000000000000000000";
        assert!(!verify_checksum(&file, wrong).unwrap());
    }

    #[test]
    fn verify_checksum_case_insensitive() {
        let dir = tempdir().unwrap();
        let file = dir.path().join("test.txt");
        fs::write(&file, b"hello world").unwrap();

        let expected_upper = "B94D27B9934D3E08A52E52D7DA7DABFAC484EFE37A5380EE9088F7ACE2EFCDE9";
        assert!(verify_checksum(&file, expected_upper).unwrap());
    }

    #[test]
    fn verify_checksum_empty_file() {
        let dir = tempdir().unwrap();
        let file = dir.path().join("empty.txt");
        fs::write(&file, b"").unwrap();

        // SHA-256 of empty string
        let expected = "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855";
        assert!(verify_checksum(&file, expected).unwrap());
    }

    #[test]
    fn verify_checksum_nonexistent_file() {
        let dir = tempdir().unwrap();
        let file = dir.path().join("nonexistent.txt");

        let result = verify_checksum(&file, "abc");
        assert!(result.is_err());
    }

    #[test]
    fn verify_checksum_binary_data() {
        let dir = tempdir().unwrap();
        let file = dir.path().join("binary.dat");
        let data: Vec<u8> = (0..=255).collect();
        fs::write(&file, &data).unwrap();

        // Compute expected hash
        let mut hasher = Sha256::new();
        hasher.update(&data);
        let expected = hex::encode(hasher.finalize());

        assert!(verify_checksum(&file, &expected).unwrap());
    }

    #[test]
    fn cache_dir_is_under_quartermaster() {
        let dir = cache_dir().unwrap();
        let path_str = dir.to_string_lossy();
        assert!(path_str.contains("quartermaster"));
        assert!(path_str.ends_with("downloads"));
        assert!(dir.exists());
    }

    #[tokio::test]
    async fn extract_tar_gz_archive() {
        let dir = tempdir().unwrap();
        let archive_dir = dir.path().join("archive_src");
        let extract_dir = dir.path().join("extracted");
        let archive_path = dir.path().join("test.tar.gz");

        // Create a file to archive
        fs::create_dir_all(&archive_dir).unwrap();
        fs::write(archive_dir.join("hello.txt"), "hello from archive").unwrap();

        // Create the tar.gz archive using system tar
        let output = std::process::Command::new("tar")
            .args([
                "czf",
                archive_path.to_str().unwrap(),
                "-C",
                dir.path().to_str().unwrap(),
                "archive_src",
            ])
            .output()
            .unwrap();
        assert!(output.status.success(), "Failed to create test tar.gz");

        // Extract it
        extract_archive(&archive_path, &extract_dir, "tar.gz")
            .await
            .unwrap();

        let extracted_file = extract_dir.join("archive_src").join("hello.txt");
        assert!(extracted_file.exists());
        assert_eq!(
            fs::read_to_string(extracted_file).unwrap(),
            "hello from archive"
        );
    }

    #[tokio::test]
    async fn extract_zip_archive() {
        let dir = tempdir().unwrap();
        let source_file = dir.path().join("hello.txt");
        let archive_path = dir.path().join("test.zip");
        let extract_dir = dir.path().join("extracted");

        fs::write(&source_file, "zipped content").unwrap();

        // Create zip using system zip command
        let output = std::process::Command::new("zip")
            .args([
                "-j",
                archive_path.to_str().unwrap(),
                source_file.to_str().unwrap(),
            ])
            .output();

        // zip might not be installed in all test environments
        match output {
            Ok(o) if o.status.success() => {
                extract_archive(&archive_path, &extract_dir, "zip")
                    .await
                    .unwrap();

                let extracted_file = extract_dir.join("hello.txt");
                assert!(extracted_file.exists());
                assert_eq!(fs::read_to_string(extracted_file).unwrap(), "zipped content");
            }
            _ => {
                // Skip test if zip is not available
                eprintln!("Skipping zip extraction test: zip command not available");
            }
        }
    }

    #[tokio::test]
    async fn extract_unsupported_format_returns_error() {
        let dir = tempdir().unwrap();
        let archive = dir.path().join("test.rar");
        fs::write(&archive, b"not a real archive").unwrap();

        let result = extract_archive(&archive, dir.path(), "rar").await;
        assert!(result.is_err());
        let err = result.unwrap_err().to_string();
        assert!(err.contains("Unsupported archive format"));
    }

    #[tokio::test]
    async fn extract_nonexistent_archive_returns_error() {
        let dir = tempdir().unwrap();
        let archive = dir.path().join("nonexistent.tar.gz");

        let result = extract_archive(&archive, dir.path(), "tar.gz").await;
        assert!(result.is_err());
    }
}
