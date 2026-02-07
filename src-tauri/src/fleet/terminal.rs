use std::process::Command;

use crate::error::AppError;

/// Ordered list of terminal emulators to search for.
const TERMINAL_CANDIDATES: &[&str] = &[
    "gnome-terminal",
    "konsole",
    "xfce4-terminal",
    "alacritty",
    "kitty",
    "wezterm",
    "tilix",
    "xterm",
];

/// Detects the first available terminal emulator on the system.
///
/// Checks each candidate in priority order using `which`. Returns `Some(name)`
/// for the first binary found on `$PATH`, or `None` if no supported terminal
/// is installed.
pub fn detect_terminal() -> Option<String> {
    detect_terminal_with(|name| {
        Command::new("which")
            .arg(name)
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .status()
            .map(|s| s.success())
            .unwrap_or(false)
    })
}

/// Inner detection logic, parameterised over a checker function so tests
/// can supply a mock.
fn detect_terminal_with<F>(checker: F) -> Option<String>
where
    F: Fn(&str) -> bool,
{
    for &candidate in TERMINAL_CANDIDATES {
        if checker(candidate) {
            return Some(candidate.to_string());
        }
    }
    None
}

/// Builds the argument list for an SSH command.
///
/// The returned vec always starts with `"ssh"` and includes `-p <port>`
/// and `<username>@<host>`. If an identity file is provided, `-i <path>`
/// is inserted before the destination.
pub fn build_ssh_command(
    host: &str,
    port: u16,
    username: &str,
    identity_file: Option<&str>,
) -> Vec<String> {
    let mut args = vec![
        "ssh".to_string(),
        "-p".to_string(),
        port.to_string(),
    ];

    if let Some(key_path) = identity_file {
        args.push("-i".to_string());
        args.push(key_path.to_string());
    }

    args.push(format!("{}@{}", username, host));
    args
}

/// Opens the detected terminal emulator with an SSH session to the
/// specified remote host.
///
/// Each supported terminal has its own syntax for executing a command
/// on launch. Returns `AppError::Ssh` if no terminal emulator can be
/// found.
pub fn open_terminal_with_ssh(
    host: &str,
    port: u16,
    username: &str,
    identity_file: Option<&str>,
) -> Result<(), AppError> {
    let terminal = detect_terminal()
        .ok_or_else(|| AppError::Ssh("No terminal emulator found".to_string()))?;

    let ssh_args = build_ssh_command(host, port, username, identity_file);
    let ssh_full = ssh_args.join(" ");

    launch_terminal(&terminal, &ssh_args, &ssh_full)
}

/// Opens a plain terminal window (no SSH), useful for the local node.
pub fn open_local_terminal() -> Result<(), AppError> {
    let terminal = detect_terminal()
        .ok_or_else(|| AppError::Ssh("No terminal emulator found".to_string()))?;

    let mut cmd = Command::new(&terminal);

    // Most terminals open a default shell when invoked without arguments.
    // No extra flags needed.
    cmd.spawn()
        .map_err(|e| AppError::Ssh(format!("Failed to open terminal '{}': {}", terminal, e)))?;

    Ok(())
}

/// Spawns the terminal process with the correct flag convention for the
/// given emulator.
fn launch_terminal(
    terminal: &str,
    ssh_args: &[String],
    ssh_full: &str,
) -> Result<(), AppError> {
    let mut cmd = Command::new(terminal);

    match terminal {
        "gnome-terminal" => {
            // gnome-terminal -- ssh -p 22 user@host
            cmd.arg("--");
            cmd.args(ssh_args);
        }
        "konsole" => {
            // konsole -e ssh -p 22 user@host
            cmd.arg("-e");
            cmd.args(ssh_args);
        }
        "xfce4-terminal" => {
            // xfce4-terminal -e "ssh -p 22 user@host"
            cmd.arg("-e");
            cmd.arg(ssh_full);
        }
        "alacritty" => {
            // alacritty -e ssh -p 22 user@host
            cmd.arg("-e");
            cmd.args(ssh_args);
        }
        "kitty" => {
            // kitty ssh -p 22 user@host
            cmd.args(ssh_args);
        }
        "wezterm" => {
            // wezterm start -- ssh -p 22 user@host
            cmd.arg("start");
            cmd.arg("--");
            cmd.args(ssh_args);
        }
        "tilix" => {
            // tilix -e "ssh -p 22 user@host"
            cmd.arg("-e");
            cmd.arg(ssh_full);
        }
        "xterm" => {
            // xterm -e ssh -p 22 user@host
            cmd.arg("-e");
            cmd.args(ssh_args);
        }
        _ => {
            return Err(AppError::Ssh(format!(
                "Unsupported terminal emulator: {}",
                terminal
            )));
        }
    }

    cmd.spawn()
        .map_err(|e| AppError::Ssh(format!("Failed to launch '{}': {}", terminal, e)))?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn build_ssh_command_with_all_args() {
        let args = build_ssh_command("example.com", 2222, "admin", Some("/home/admin/.ssh/id_ed25519"));
        assert_eq!(
            args,
            vec![
                "ssh".to_string(),
                "-p".to_string(),
                "2222".to_string(),
                "-i".to_string(),
                "/home/admin/.ssh/id_ed25519".to_string(),
                "admin@example.com".to_string(),
            ]
        );
    }

    #[test]
    fn build_ssh_command_without_identity_file() {
        let args = build_ssh_command("10.0.0.5", 22, "deploy", None);
        assert_eq!(
            args,
            vec![
                "ssh".to_string(),
                "-p".to_string(),
                "22".to_string(),
                "deploy@10.0.0.5".to_string(),
            ]
        );
    }

    #[test]
    fn detect_terminal_returns_option() {
        // On CI or a headless system no terminal may be installed; on a
        // desktop at least one should be present. Either way the function
        // must not panic.
        let result = detect_terminal();
        match result {
            Some(name) => assert!(!name.is_empty()),
            None => {} // perfectly valid on a headless box
        }
    }

    #[test]
    fn detect_terminal_with_mock_returns_first_match() {
        // Pretend only "alacritty" and "xterm" are installed.
        let result = detect_terminal_with(|name| {
            name == "alacritty" || name == "xterm"
        });
        // "alacritty" comes before "xterm" in the candidate list.
        assert_eq!(result, Some("alacritty".to_string()));
    }

    #[test]
    fn detect_terminal_with_mock_none_installed() {
        let result = detect_terminal_with(|_name| false);
        assert!(result.is_none());
    }

    #[test]
    fn open_terminal_ssh_fails_when_no_terminal_found() {
        // We cannot easily mock `detect_terminal` inside
        // `open_terminal_with_ssh`, but we can verify the inner logic
        // by confirming that `detect_terminal_with` returning None
        // would yield the correct error path. Simulate via a direct
        // call to the fallback:
        let terminal: Option<String> = detect_terminal_with(|_| false);
        let err = terminal
            .ok_or_else(|| AppError::Ssh("No terminal emulator found".to_string()));
        assert!(err.is_err());
        let msg = format!("{}", err.unwrap_err());
        assert!(msg.contains("No terminal emulator found"));
    }
}
