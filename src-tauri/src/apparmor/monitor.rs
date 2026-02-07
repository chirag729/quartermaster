use std::sync::Arc;
use std::io::{BufRead, Seek, SeekFrom};
use tokio::sync::Mutex;
use tauri::{AppHandle, Emitter};
use notify::{Watcher, RecursiveMode, Event, EventKind};

use super::log_parser::parse_denial_line;
use crate::error::AppError;

pub async fn start_monitoring(
    app: AppHandle,
    running: Arc<Mutex<bool>>,
) -> Result<(), AppError> {
    let mut guard = running.lock().await;
    if *guard {
        return Err(AppError::AppArmor(
            "Log monitor is already running".to_string(),
        ));
    }
    *guard = true;
    drop(guard);

    let running_clone = running.clone();
    let app_clone = app.clone();

    tokio::spawn(async move {
        let audit_path = "/var/log/audit/audit.log";

        if !std::path::Path::new(audit_path).exists() {
            monitor_via_journalctl(app_clone, running_clone).await;
            return;
        }

        let initial_size = std::fs::metadata(audit_path)
            .map(|m| m.len())
            .unwrap_or(0);
        let last_size = Arc::new(Mutex::new(initial_size));

        let (tx, rx) = std::sync::mpsc::channel();

        let mut watcher = match notify::recommended_watcher(move |res: Result<Event, notify::Error>| {
            if let Ok(event) = res {
                if matches!(event.kind, EventKind::Modify(_)) {
                    let _ = tx.send(());
                }
            }
        }) {
            Ok(w) => w,
            Err(e) => {
                eprintln!("Failed to create watcher: {}", e);
                return;
            }
        };

        if let Err(e) = watcher.watch(std::path::Path::new(audit_path), RecursiveMode::NonRecursive) {
            eprintln!("Failed to watch file: {}", e);
            return;
        }

        loop {
            if !*running_clone.lock().await {
                break;
            }

            if rx.recv_timeout(std::time::Duration::from_secs(1)).is_ok() {
                let mut size = last_size.lock().await;
                let current_size = *size;

                // Run blocking file I/O in a dedicated thread
                let read_result = tokio::task::spawn_blocking(move || -> Option<(Vec<String>, u64)> {
                    let file = std::fs::File::open(audit_path).ok()?;
                    let current_len = file.metadata().map(|m| m.len()).unwrap_or(0);

                    // Handle log rotation: if file shrunk, start from beginning
                    let seek_pos = if current_len < current_size { 0 } else { current_size };

                    let mut reader = std::io::BufReader::new(file);
                    reader.seek(SeekFrom::Start(seek_pos)).ok()?;

                    let mut denials = Vec::new();
                    let mut line = String::new();
                    while reader.read_line(&mut line).unwrap_or(0) > 0 {
                        if line.contains("apparmor=\"DENIED\"") {
                            denials.push(line.clone());
                        }
                        line.clear();
                    }

                    Some((denials, current_len))
                }).await;

                if let Ok(Some((denial_lines, new_size))) = read_result {
                    for line in &denial_lines {
                        if let Some(denial) = parse_denial_line(line) {
                            let _ = app_clone.emit("apparmor-denial", serde_json::json!({
                                "denial": denial,
                            }));
                        }
                    }
                    *size = new_size;
                }
            }
        }

        drop(watcher);
    });

    Ok(())
}

async fn monitor_via_journalctl(app: AppHandle, running: Arc<Mutex<bool>>) {
    use tokio::io::AsyncBufReadExt;

    let mut child = match tokio::process::Command::new("journalctl")
        .args(["--no-pager", "-f", "-g", "apparmor.*DENIED"])
        .stdout(std::process::Stdio::piped())
        .spawn()
    {
        Ok(c) => c,
        Err(e) => {
            eprintln!("Failed to start journalctl: {}", e);
            return;
        }
    };

    let stdout = match child.stdout.take() {
        Some(s) => s,
        None => return,
    };

    let reader = tokio::io::BufReader::new(stdout);
    let mut lines = reader.lines();

    loop {
        if !*running.lock().await {
            let _ = child.kill().await;
            break;
        }

        tokio::select! {
            line = lines.next_line() => {
                match line {
                    Ok(Some(line)) => {
                        if line.contains("DENIED") {
                            if let Some(denial) = parse_denial_line(&line) {
                                let _ = app.emit("apparmor-denial", serde_json::json!({
                                    "denial": denial,
                                }));
                            }
                        }
                    }
                    Ok(None) => break,
                    Err(_) => break,
                }
            }
            _ = tokio::time::sleep(std::time::Duration::from_secs(1)) => {}
        }
    }
}

pub async fn stop_monitoring(running: Arc<Mutex<bool>>) {
    *running.lock().await = false;
}
