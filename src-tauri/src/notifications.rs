//! Desktop notification helpers.
//!
//! Wraps tauri-plugin-notification to send platform-native notifications
//! for key events (task completion, blueprint applied, errors, etc.).

use tauri::AppHandle;
use tauri_plugin_notification::NotificationExt;

/// Send a notification for a completed task.
pub fn notify_task_completed(app: &AppHandle, task_name: &str, node_name: &str) {
    let _ = app
        .notification()
        .builder()
        .title("Task Completed")
        .body(format!("{} finished on {}", task_name, node_name))
        .show();
}

/// Send a notification for a blueprint apply completion.
pub fn notify_blueprint_applied(app: &AppHandle, blueprint_name: &str, node_name: &str) {
    let _ = app
        .notification()
        .builder()
        .title("Blueprint Applied")
        .body(format!("{} applied to {}", blueprint_name, node_name))
        .show();
}

/// Send a notification for a bulk operation completion.
pub fn notify_bulk_complete(
    app: &AppHandle,
    blueprint_name: &str,
    succeeded: usize,
    failed: usize,
) {
    let title = if failed == 0 {
        "Bulk Apply Complete"
    } else {
        "Bulk Apply Finished with Errors"
    };
    let body = format!(
        "{}: {} succeeded, {} failed",
        blueprint_name, succeeded, failed
    );
    let _ = app
        .notification()
        .builder()
        .title(title)
        .body(body)
        .show();
}

/// Send a notification for a blueprint uninstall completion.
pub fn notify_blueprint_uninstalled(app: &AppHandle, blueprint_name: &str, node_name: &str) {
    let _ = app
        .notification()
        .builder()
        .title("Blueprint Uninstalled")
        .body(format!("{} uninstalled from {}", blueprint_name, node_name))
        .show();
}

/// Send an error notification.
pub fn notify_error(app: &AppHandle, title: &str, message: &str) {
    let _ = app
        .notification()
        .builder()
        .title(title)
        .body(message.to_string())
        .show();
}
