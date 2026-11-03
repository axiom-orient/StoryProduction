pub mod document;
pub mod paths;
pub mod process;
pub mod store;
pub mod token;

use thiserror::Error;

#[derive(Debug, Error)]
pub enum Error {
    #[error("{0}")]
    Contract(String),
    #[error("I/O: {0}")]
    Io(#[from] std::io::Error),
    #[error("JSON: {0}")]
    Json(#[from] serde_json::Error),
    #[error("YAML: {0}")]
    Yaml(#[from] serde_yaml::Error),
    #[error("ZIP: {0}")]
    Zip(#[from] zip::result::ZipError),
    #[error("Executor: {0}")]
    Executor(String),
}

pub type Result<T> = std::result::Result<T, Error>;

/// Process-local directory selection for executable transports.
pub mod working_directory;
