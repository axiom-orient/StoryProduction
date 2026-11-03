use std::io;

#[derive(Debug, Clone)]
pub(crate) struct CustodyError {
    pub(crate) code: String,
    pub(crate) message: String,
    pub(crate) retryable: bool,
}

impl CustodyError {
    pub(crate) fn new(
        code: impl Into<String>,
        message: impl Into<String>,
        retryable: bool,
    ) -> Self {
        Self {
            code: code.into(),
            message: message.into(),
            retryable,
        }
    }

    pub(crate) fn input(message: impl Into<String>) -> Self {
        Self::new("trackb-input-invalid", message, false)
    }

    pub(crate) fn storage(message: impl Into<String>) -> Self {
        Self::new("trackb-storage-integrity", message, false)
    }

    pub(crate) fn recovery(message: impl Into<String>) -> Self {
        Self::new("trackb-lock-recovery-required", message, false)
    }

    pub(crate) fn io(label: &str, error: &io::Error) -> Self {
        Self::storage(format!("{label}: {error}"))
    }
}

pub(crate) type Result<T> = std::result::Result<T, CustodyError>;
