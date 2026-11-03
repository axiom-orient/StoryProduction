//! Shared input decoding, transaction result, and command identity helpers.

use crate::domain::Command;
use crate::error::{Error, Result};
use crate::store::Workspace;
use serde::de::DeserializeOwned;
use serde_json::Value;
use sha2::{Digest, Sha256};
use std::sync::atomic::{AtomicU64, Ordering};

static ID_COUNTER: AtomicU64 = AtomicU64::new(1);

pub(super) fn commit(
    workspace: &Workspace,
    project_id: &str,
    command: Option<String>,
    command_value: Command,
) -> Result<Value> {
    Ok(serde_json::to_value(workspace.transact(
        project_id,
        &command_id(command),
        command_value,
    )?)?)
}

pub(crate) fn parse<T: DeserializeOwned>(value: Value) -> Result<T> {
    serde_json::from_value(value).map_err(|error| Error::InvalidInput(error.to_string()))
}

pub(super) fn command_id(value: Option<String>) -> String {
    value.unwrap_or_else(|| generated_id("cmd"))
}

/// Creates a process-local unique identifier for new commands and records.
pub(super) fn generated_id(prefix: &str) -> String {
    let counter = ID_COUNTER.fetch_add(1, Ordering::Relaxed);
    let mut hasher = Sha256::new();
    hasher.update(prefix.as_bytes());
    hasher.update(crate::store::now_ms().to_le_bytes());
    hasher.update(std::process::id().to_le_bytes());
    hasher.update(counter.to_le_bytes());
    let digest = hasher.finalize();
    let suffix = u64::from_be_bytes([
        digest[0], digest[1], digest[2], digest[3], digest[4], digest[5], digest[6], digest[7],
    ]);
    format!("{prefix}-{suffix:016x}")
}
