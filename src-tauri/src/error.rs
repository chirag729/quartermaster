use serde::Serialize;

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

    #[error("{0}")]
    Other(String),
}

impl Serialize for AppError {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.serialize_str(&self.to_string())
    }
}
