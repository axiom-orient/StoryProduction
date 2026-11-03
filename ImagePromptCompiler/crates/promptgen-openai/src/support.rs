//! Shared I/O error formatting for OpenAI adapter owners.

use std::io;
use std::path::Path;

use super::OpenAiExecutionError;

pub(super) fn io_error(
    operation: &'static str,
    path: &Path,
    error: io::Error,
) -> OpenAiExecutionError {
    OpenAiExecutionError::new(
        "OPENAI_IO",
        format!(
            "operation={operation}, path={}, error={error}",
            path.display()
        ),
    )
}
