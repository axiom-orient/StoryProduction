use serde::Serialize;
use std::fmt;
use std::io;

pub type Result<T> = std::result::Result<T, Error>;

#[derive(Debug)]
pub enum Error {
    InvalidInput(String),
    NotFound(String),
    Conflict(String),
    Busy(String),
    AdapterUnavailable(String),
    Corrupt(String),
    /// A command would exceed a declared text-run budget
    /// (`revision_remaining` / `pivot_remaining`). Raised fail-closed before
    /// any event is journaled.
    BudgetExhausted(String),
    Io(io::Error),
    Json(serde_json::Error),
    CommittedButSnapshotFailed {
        revision: u64,
        message: String,
    },
}

#[derive(Debug, Serialize)]
pub struct ErrorBody {
    pub code: &'static str,
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub committed_revision: Option<u64>,
}

impl Error {
    pub fn body(&self) -> ErrorBody {
        match self {
            Self::InvalidInput(message) => ErrorBody {
                code: "invalid_input",
                message: message.clone(),
                committed_revision: None,
            },
            Self::NotFound(message) => ErrorBody {
                code: "not_found",
                message: message.clone(),
                committed_revision: None,
            },
            Self::Conflict(message) => ErrorBody {
                code: "conflict",
                message: message.clone(),
                committed_revision: None,
            },
            Self::Busy(message) => ErrorBody {
                code: "busy",
                message: message.clone(),
                committed_revision: None,
            },
            Self::AdapterUnavailable(message) => ErrorBody {
                code: "adapter_unavailable",
                message: message.clone(),
                committed_revision: None,
            },
            Self::Corrupt(message) => ErrorBody {
                code: "corrupt",
                message: message.clone(),
                committed_revision: None,
            },
            Self::BudgetExhausted(message) => ErrorBody {
                code: "budget_exhausted",
                message: message.clone(),
                committed_revision: None,
            },
            Self::Io(error) => ErrorBody {
                code: "io",
                message: error.to_string(),
                committed_revision: None,
            },
            Self::Json(error) => ErrorBody {
                code: "json",
                message: error.to_string(),
                committed_revision: None,
            },
            Self::CommittedButSnapshotFailed { revision, message } => ErrorBody {
                code: "committed_snapshot_failed",
                message: message.clone(),
                committed_revision: Some(*revision),
            },
        }
    }
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidInput(message)
            | Self::NotFound(message)
            | Self::Conflict(message)
            | Self::Busy(message)
            | Self::AdapterUnavailable(message)
            | Self::Corrupt(message)
            | Self::BudgetExhausted(message) => f.write_str(message),
            Self::Io(error) => write!(f, "I/O error: {error}"),
            Self::Json(error) => write!(f, "JSON error: {error}"),
            Self::CommittedButSnapshotFailed { revision, message } => {
                write!(
                    f,
                    "event committed at revision {revision}, but snapshot update failed: {message}"
                )
            }
        }
    }
}

impl std::error::Error for Error {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Io(error) => Some(error),
            Self::Json(error) => Some(error),
            _ => None,
        }
    }
}

impl From<io::Error> for Error {
    fn from(value: io::Error) -> Self {
        Self::Io(value)
    }
}

impl From<serde_json::Error> for Error {
    fn from(value: serde_json::Error) -> Self {
        Self::Json(value)
    }
}
