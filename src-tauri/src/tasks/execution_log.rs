//! Task execution logging — persists stdout/stderr per task run.
//!
//! Logs are stored in `~/.local/state/quartermaster/logs/` as JSON files
//! with the naming convention `{task_id}_{timestamp}.log`.

use std::fs;
use std::path::{Path, PathBuf};

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::error::AppError;

/// Returns the default base directory for execution logs.
pub fn default_logs_dir() -> PathBuf {
    dirs::state_dir()
        .or_else(|| dirs::home_dir().map(|h| h.join(".local/state")))
        .unwrap_or_else(|| PathBuf::from("~/.local/state"))
        .join("quartermaster")
        .join("logs")
}

/// Format a `DateTime<Utc>` into a filename-safe ISO 8601 string.
fn format_timestamp_for_filename(dt: &DateTime<Utc>) -> String {
    dt.format("%Y-%m-%dT%H-%M-%S").to_string()
}

/// A single step within a task execution.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StepLog {
    pub name: String,
    pub stdout: String,
    pub stderr: String,
    pub exit_code: i32,
    pub duration_ms: u64,
}

/// A complete execution log for a task run, containing metadata and per-step output.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecutionLog {
    pub task_id: String,
    pub node_id: Option<String>,
    pub started_at: DateTime<Utc>,
    pub finished_at: Option<DateTime<Utc>>,
    pub success: bool,
    pub steps: Vec<StepLog>,

    /// Override logs directory (used for testing). When `None`, uses `default_logs_dir()`.
    #[serde(skip)]
    logs_dir_override: Option<PathBuf>,
}

impl ExecutionLog {
    /// Creates a new log entry with the current UTC timestamp.
    pub fn new(task_id: &str, node_id: Option<&str>) -> Self {
        Self {
            task_id: task_id.to_string(),
            node_id: node_id.map(|s| s.to_string()),
            started_at: Utc::now(),
            finished_at: None,
            success: false,
            steps: Vec::new(),
            logs_dir_override: None,
        }
    }

    /// Creates a new log entry that writes to a custom directory (for testing).
    #[cfg(test)]
    fn new_with_dir(task_id: &str, node_id: Option<&str>, logs_dir: PathBuf) -> Self {
        let mut log = Self::new(task_id, node_id);
        log.logs_dir_override = Some(logs_dir);
        log
    }

    fn effective_logs_dir(&self) -> PathBuf {
        self.logs_dir_override
            .clone()
            .unwrap_or_else(default_logs_dir)
    }

    /// Appends a step log entry.
    pub fn add_step(
        &mut self,
        name: &str,
        stdout: &str,
        stderr: &str,
        exit_code: i32,
        duration_ms: u64,
    ) {
        self.steps.push(StepLog {
            name: name.to_string(),
            stdout: stdout.to_string(),
            stderr: stderr.to_string(),
            exit_code,
            duration_ms,
        });
    }

    /// Marks the execution as finished, recording the outcome and current timestamp.
    pub fn finish(&mut self, success: bool) {
        self.finished_at = Some(Utc::now());
        self.success = success;
    }

    /// Serializes this log to JSON and writes it to the logs directory.
    /// Returns the path to the written file.
    pub fn save(&self) -> Result<PathBuf, AppError> {
        let dir = self.effective_logs_dir();
        fs::create_dir_all(&dir)?;

        let timestamp = format_timestamp_for_filename(&self.started_at);
        let filename = format!("{}_{}.log", self.task_id, timestamp);
        let path = dir.join(filename);

        let json = serde_json::to_string_pretty(self)?;
        fs::write(&path, json)?;

        Ok(path)
    }
}

/// Lists all log files for a given task, sorted by timestamp newest first.
pub fn list_logs(task_id: &str) -> Result<Vec<PathBuf>, AppError> {
    list_logs_in(task_id, &default_logs_dir())
}

/// Lists log files for a task in a specific directory.
pub fn list_logs_in(task_id: &str, logs_dir: &Path) -> Result<Vec<PathBuf>, AppError> {
    if !logs_dir.exists() {
        return Ok(Vec::new());
    }

    let prefix = format!("{}_", task_id);
    let mut logs: Vec<PathBuf> = fs::read_dir(logs_dir)?
        .filter_map(|entry| entry.ok())
        .map(|entry| entry.path())
        .filter(|path| {
            path.file_name()
                .and_then(|n| n.to_str())
                .map(|n| n.starts_with(&prefix) && n.ends_with(".log"))
                .unwrap_or(false)
        })
        .collect();

    // Sort by filename descending (newest first)
    logs.sort_by(|a, b| b.file_name().cmp(&a.file_name()));

    Ok(logs)
}

/// Reads and deserializes a log file from disk.
pub fn read_log(path: &Path) -> Result<ExecutionLog, AppError> {
    let content = fs::read_to_string(path)?;
    let log: ExecutionLog = serde_json::from_str(&content)?;
    Ok(log)
}

/// Keeps only the `keep` most recent log files for a task, deleting the rest.
pub fn cleanup_old_logs(task_id: &str, keep: usize) -> Result<(), AppError> {
    cleanup_old_logs_in(task_id, keep, &default_logs_dir())
}

/// Keeps only the `keep` most recent log files for a task in a specific directory.
pub fn cleanup_old_logs_in(
    task_id: &str,
    keep: usize,
    logs_dir: &Path,
) -> Result<(), AppError> {
    let logs = list_logs_in(task_id, logs_dir)?;

    if logs.len() <= keep {
        return Ok(());
    }

    for path in &logs[keep..] {
        fs::remove_file(path)?;
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn create_new_log_and_finish() {
        let mut log = ExecutionLog::new("test-task", Some("node-1"));

        assert_eq!(log.task_id, "test-task");
        assert_eq!(log.node_id, Some("node-1".to_string()));
        assert!(!log.success);
        assert!(log.finished_at.is_none());
        assert!(log.steps.is_empty());

        log.finish(true);

        assert!(log.success);
        assert!(log.finished_at.is_some());
        assert!(log.finished_at.unwrap() >= log.started_at);
    }

    #[test]
    fn add_steps() {
        let mut log = ExecutionLog::new("test-task", None);

        log.add_step("install deps", "installed 42 packages", "", 0, 1500);
        log.add_step("compile", "", "warning: unused var", 0, 3200);
        log.add_step("test", "3 passed", "1 failed", 1, 800);

        assert_eq!(log.steps.len(), 3);
        assert_eq!(log.steps[0].name, "install deps");
        assert_eq!(log.steps[0].stdout, "installed 42 packages");
        assert_eq!(log.steps[0].exit_code, 0);
        assert_eq!(log.steps[0].duration_ms, 1500);
        assert_eq!(log.steps[2].name, "test");
        assert_eq!(log.steps[2].exit_code, 1);
    }

    #[test]
    fn save_and_read_roundtrip() {
        let dir = tempdir().unwrap();
        let mut log = ExecutionLog::new_with_dir("test-task", Some("remote-node"), dir.path().to_path_buf());
        log.add_step("step-1", "hello stdout", "hello stderr", 0, 100);
        log.finish(true);

        let path = log.save().unwrap();
        assert!(path.exists());

        let loaded = read_log(&path).unwrap();
        assert_eq!(loaded.task_id, "test-task");
        assert_eq!(loaded.node_id, Some("remote-node".to_string()));
        assert!(loaded.success);
        assert!(loaded.finished_at.is_some());
        assert_eq!(loaded.steps.len(), 1);
        assert_eq!(loaded.steps[0].name, "step-1");
        assert_eq!(loaded.steps[0].stdout, "hello stdout");
    }

    #[test]
    fn list_logs_returns_correct_files_sorted() {
        let dir = tempdir().unwrap();
        let task_id = "my-task";

        let mut log1 = ExecutionLog::new_with_dir(task_id, None, dir.path().to_path_buf());
        log1.started_at = chrono::DateTime::parse_from_rfc3339("2024-01-01T10:00:00Z")
            .unwrap()
            .with_timezone(&Utc);
        log1.save().unwrap();

        let mut log2 = ExecutionLog::new_with_dir(task_id, None, dir.path().to_path_buf());
        log2.started_at = chrono::DateTime::parse_from_rfc3339("2024-06-15T14:30:00Z")
            .unwrap()
            .with_timezone(&Utc);
        log2.save().unwrap();

        let mut log3 = ExecutionLog::new_with_dir(task_id, None, dir.path().to_path_buf());
        log3.started_at = chrono::DateTime::parse_from_rfc3339("2024-03-10T08:15:00Z")
            .unwrap()
            .with_timezone(&Utc);
        log3.save().unwrap();

        // Different task — should not appear
        let other_log = ExecutionLog::new_with_dir("other-task", None, dir.path().to_path_buf());
        other_log.save().unwrap();

        let logs = list_logs_in(task_id, dir.path()).unwrap();
        assert_eq!(logs.len(), 3);

        let names: Vec<String> = logs
            .iter()
            .map(|p| p.file_name().unwrap().to_str().unwrap().to_string())
            .collect();
        assert!(names[0].contains("2024-06-15"));
        assert!(names[1].contains("2024-03-10"));
        assert!(names[2].contains("2024-01-01"));
    }

    #[test]
    fn cleanup_old_logs_keeps_only_n() {
        let dir = tempdir().unwrap();
        let task_id = "cleanup-task";

        for month in 1..=5 {
            let mut log = ExecutionLog::new_with_dir(task_id, None, dir.path().to_path_buf());
            log.started_at =
                chrono::DateTime::parse_from_rfc3339(&format!("2024-{:02}-01T00:00:00Z", month))
                    .unwrap()
                    .with_timezone(&Utc);
            log.save().unwrap();
        }

        assert_eq!(list_logs_in(task_id, dir.path()).unwrap().len(), 5);

        cleanup_old_logs_in(task_id, 2, dir.path()).unwrap();

        let remaining = list_logs_in(task_id, dir.path()).unwrap();
        assert_eq!(remaining.len(), 2);

        let names: Vec<String> = remaining
            .iter()
            .map(|p| p.file_name().unwrap().to_str().unwrap().to_string())
            .collect();
        assert!(names[0].contains("2024-05-01"));
        assert!(names[1].contains("2024-04-01"));
    }

    #[test]
    fn cleanup_old_logs_fewer_than_n_is_noop() {
        let dir = tempdir().unwrap();
        let task_id = "noop-task";

        let log = ExecutionLog::new_with_dir(task_id, None, dir.path().to_path_buf());
        log.save().unwrap();

        assert_eq!(list_logs_in(task_id, dir.path()).unwrap().len(), 1);

        cleanup_old_logs_in(task_id, 5, dir.path()).unwrap();

        assert_eq!(list_logs_in(task_id, dir.path()).unwrap().len(), 1);
    }

    #[test]
    fn log_file_naming_format() {
        let dir = tempdir().unwrap();
        let task_id = "naming-task";
        let mut log = ExecutionLog::new_with_dir(task_id, None, dir.path().to_path_buf());
        log.started_at = chrono::DateTime::parse_from_rfc3339("2024-12-15T10:30:00Z")
            .unwrap()
            .with_timezone(&Utc);

        let path = log.save().unwrap();
        let filename = path.file_name().unwrap().to_str().unwrap();

        let expected = format!("{}_2024-12-15T10-30-00.log", task_id);
        assert_eq!(filename, expected);
        assert!(!filename.contains(':'));
    }

    #[test]
    fn empty_log_serialization() {
        let dir = tempdir().unwrap();
        let task_id = "empty-task";
        let log = ExecutionLog::new_with_dir(task_id, None, dir.path().to_path_buf());

        let path = log.save().unwrap();
        let loaded = read_log(&path).unwrap();

        assert_eq!(loaded.task_id, task_id);
        assert_eq!(loaded.node_id, None);
        assert!(!loaded.success);
        assert!(loaded.finished_at.is_none());
        assert!(loaded.steps.is_empty());

        let content = fs::read_to_string(&path).unwrap();
        let value: serde_json::Value = serde_json::from_str(&content).unwrap();
        assert!(value.get("task_id").is_some());
        assert!(value.get("steps").is_some());
        assert_eq!(value["steps"].as_array().unwrap().len(), 0);
    }
}
