use serde::Serialize;
use serde::ser::SerializeStruct;

#[derive(Debug, thiserror::Error)]
pub enum AppError {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Serialization error: {0}")]
    Serde(#[from] serde_json::Error),

    #[error("Command failed: {0}")]
    Command(String),

    #[error("Task error: {0}")]
    Task(String),

    #[error("AppArmor error: {0}")]
    AppArmor(String),

    #[error("Polkit error: {0}")]
    Polkit(String),

    #[error("Config error: {0}")]
    Config(String),

    #[error("SSH error: {0}")]
    Ssh(String),

    #[error("Fleet error: {0}")]
    Fleet(String),

    #[error("Blueprint error: {0}")]
    Blueprint(String),

    #[error("Connection error: {0}")]
    Connection(String),

    #[error("Download error: {0}")]
    Download(String),

    #[error("Desktop integration error: {0}")]
    Desktop(String),

    #[error("Variable resolution error: {0}")]
    Variable(String),

    #[error("Vault error: {0}")]
    Vault(String),

    #[error("YubiKey error: {0}")]
    YubiKey(String),

    #[error("Package error: {0}")]
    Package(String),

    #[error("{0}")]
    Other(String),
}

impl AppError {
    /// Returns a static string identifying the error variant.
    pub fn kind(&self) -> &'static str {
        match self {
            AppError::Io(_) => "io",
            AppError::Serde(_) => "serialization",
            AppError::Command(_) => "command",
            AppError::Task(_) => "task",
            AppError::AppArmor(_) => "apparmor",
            AppError::Polkit(_) => "polkit",
            AppError::Config(_) => "config",
            AppError::Ssh(_) => "ssh",
            AppError::Fleet(_) => "fleet",
            AppError::Blueprint(_) => "blueprint",
            AppError::Connection(_) => "connection",
            AppError::Download(_) => "download",
            AppError::Desktop(_) => "desktop",
            AppError::Variable(_) => "variable",
            AppError::Vault(_) => "vault",
            AppError::YubiKey(_) => "yubikey",
            AppError::Package(_) => "package",
            AppError::Other(_) => "other",
        }
    }

    /// Returns a user-friendly recovery hint based on the error variant and message content.
    pub fn user_hint(&self) -> Option<&'static str> {
        let msg_lower = self.to_string().to_lowercase();

        match self {
            AppError::Io(_) => {
                if msg_lower.contains("permission denied") {
                    Some("Try running with elevated privileges or check file permissions")
                } else if msg_lower.contains("no such file") {
                    Some("Check that the file path exists")
                } else {
                    None
                }
            }
            AppError::Ssh(_) => {
                if msg_lower.contains("connection refused") {
                    Some("Check that the SSH server is running and the host is reachable")
                } else if msg_lower.contains("authentication") {
                    Some("Verify your SSH credentials, key path, or agent configuration")
                } else {
                    None
                }
            }
            AppError::Connection(_) => {
                if msg_lower.contains("timeout") {
                    Some("The host may be unreachable or a firewall may be blocking the connection")
                } else {
                    None
                }
            }
            AppError::Vault(_) => {
                if msg_lower.contains("wrong password") {
                    Some("The master password is incorrect. Try again")
                } else {
                    None
                }
            }
            AppError::YubiKey(_) => {
                Some("Ensure your YubiKey is inserted and ykman is installed")
            }
            AppError::Polkit(_) => {
                Some("PolicyKit authorization was denied. You may need to enter your password")
            }
            AppError::Download(_) => {
                Some("Check your internet connection and try again")
            }
            AppError::Config(_) => {
                Some("The configuration file may be corrupted. Check ~/.config/quartermaster/")
            }
            _ => None,
        }
    }
}

impl Serialize for AppError {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        let mut state = serializer.serialize_struct("AppError", 3)?;
        state.serialize_field("kind", self.kind())?;
        state.serialize_field("message", &self.to_string())?;
        state.serialize_field("hint", &self.user_hint())?;
        state.end()
    }
}
