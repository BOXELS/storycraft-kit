use std::path::PathBuf;

pub type Result<T, E = Error> = std::result::Result<T, E>;

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("{path}: {source}")]
    Io { path: PathBuf, source: std::io::Error },
    #[error("{0}")]
    Recipe(String),
    #[error("{app} is not installed or its CLI was not found ({hint})")]
    AppMissing { app: String, hint: String },
    #[error("{app} {found} is older than the {min} this recipe needs")]
    AppTooOld { app: String, found: String, min: String },
    #[error("{app} {version} has no command `{command}`; run `sc commands {app} --search {command}`")]
    CommandMissing { app: String, version: String, command: String },
    #[error("step `{step}` ({app}) failed: {message}")]
    Step { step: String, app: String, message: String },
    #[error("verification failed: {0}")]
    Verify(String),
    #[error("{0}")]
    Other(String),
}

impl Error {
    pub fn io(path: impl Into<PathBuf>, source: std::io::Error) -> Self {
        Self::Io { path: path.into(), source }
    }

    /// The CLI exit code for this error: 1 operation failed, 3 verifier failed.
    pub fn exit_code(&self) -> u8 {
        match self {
            Self::Verify(_) => 3,
            _ => 1,
        }
    }
}

impl From<serde_json::Error> for Error {
    fn from(e: serde_json::Error) -> Self {
        Self::Other(format!("JSON: {e}"))
    }
}

impl From<serde_yaml::Error> for Error {
    fn from(e: serde_yaml::Error) -> Self {
        Self::Recipe(format!("YAML: {e}"))
    }
}
