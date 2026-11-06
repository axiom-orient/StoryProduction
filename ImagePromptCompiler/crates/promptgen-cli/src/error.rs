use std::fmt;
use std::io;
use std::path::Path;

pub(super) const EXIT_OK: u8 = 0;
pub(super) const EXIT_USAGE_OR_IO: u8 = 2;
pub(super) const EXIT_VALIDATION: u8 = 3;
pub(super) const EXIT_EXECUTION: u8 = 4;
pub(super) const EXIT_PROVIDER_REJECTED: u8 = 5;

#[derive(Debug)]
pub(super) struct CliError {
    pub(super) exit_code: u8,
    pub(super) code: &'static str,
    pub(super) message: String,
}

impl CliError {
    pub(super) fn usage(message: impl Into<String>) -> Self {
        Self {
            exit_code: EXIT_USAGE_OR_IO,
            code: "CLI_USAGE",
            message: message.into(),
        }
    }

    pub(super) fn validation(message: impl Into<String>) -> Self {
        Self {
            exit_code: EXIT_VALIDATION,
            code: "CLI_VALIDATION",
            message: message.into(),
        }
    }

    pub(super) fn execution(message: impl Into<String>) -> Self {
        Self {
            exit_code: EXIT_EXECUTION,
            code: "CLI_EXECUTION",
            message: message.into(),
        }
    }

    pub(super) fn io(operation: &'static str, path: &Path, error: io::Error) -> Self {
        Self {
            exit_code: EXIT_USAGE_OR_IO,
            code: "CLI_IO",
            message: format!(
                "operation={operation}, path={}, error={error}",
                path.display()
            ),
        }
    }

    pub(super) fn printed_help() -> Self {
        Self {
            exit_code: EXIT_OK,
            code: "CLI_HELP",
            message: "help displayed".to_owned(),
        }
    }
}

impl fmt::Display for CliError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.message)
    }
}
