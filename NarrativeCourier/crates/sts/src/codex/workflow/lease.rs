//! STS-owned active-run identity and cancellation marker lifecycle.

use std::io::Read;
use std::path::Path;
use std::time::{SystemTime, UNIX_EPOCH};

use narrative_courier_runtime::store::atomic_write_text;
use narrative_courier_runtime::{Error, Result};
use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub(super) struct AgentRunIdentity {
    pub(super) run_id: String,
    pub(super) node: String,
    pub(super) task_token: String,
}

impl AgentRunIdentity {
    pub(super) fn new(node: String, task_token: String) -> Result<Self> {
        let started_at = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_err(|_| Error::Contract("system clock is before the Unix epoch".into()))?;
        Ok(Self {
            run_id: format!("{}-{}", std::process::id(), started_at.as_nanos()),
            node,
            task_token,
        })
    }
}

pub(super) fn begin_agent_run(project: &Path, identity: &AgentRunIdentity) -> Result<()> {
    remove_if_exists(&project.join(".agent-cancel"))?;
    atomic_write_text(
        &project.join(".agent-active"),
        &serde_json::to_string(identity)?,
    )
}

pub(super) fn read_active_identity(project: &Path) -> Result<AgentRunIdentity> {
    let active =
        read_marker(&project.join(".agent-active")).map_err(|error| match error.kind() {
            std::io::ErrorKind::NotFound => {
                Error::Contract("Agent run is starting; retry cancellation".into())
            }
            _ => Error::Io(error),
        })?;
    let identity: AgentRunIdentity = serde_json::from_str(&active)
        .map_err(|_| Error::Contract("active Agent run identity is invalid".into()))?;
    if identity.run_id.trim().is_empty()
        || identity.node.trim().is_empty()
        || identity.task_token.trim().is_empty()
    {
        return Err(Error::Contract(
            "active Agent run identity is invalid".into(),
        ));
    }
    Ok(identity)
}

pub(super) fn request_cancel(project: &Path, identity: &AgentRunIdentity) -> Result<()> {
    atomic_write_text(
        &project.join(".agent-cancel"),
        &format!("{}\n", identity.run_id),
    )
}

pub(super) fn ensure_active_identity(project: &Path, expected: &AgentRunIdentity) -> Result<()> {
    let current = read_active_identity(project)?;
    if &current != expected {
        return Err(Error::Contract(
            "active Agent run identity changed before its result was committed".into(),
        ));
    }
    Ok(())
}

pub(super) fn cancellation_requested(project: &Path, identity: &AgentRunIdentity) -> Result<bool> {
    let marker = match read_marker(&project.join(".agent-cancel")) {
        Ok(marker) => marker,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(false),
        Err(error) => return Err(Error::Io(error)),
    };
    if marker.trim() == identity.run_id {
        return Ok(true);
    }
    Err(Error::Contract(
        "Agent cancellation marker belongs to a different run".into(),
    ))
}

pub(super) fn finish_agent_run(
    project: &Path,
    identity: &AgentRunIdentity,
    clear_cancellation: bool,
) -> Result<()> {
    ensure_active_identity(project, identity)?;
    if clear_cancellation && cancellation_requested(project, identity)? {
        remove_if_exists(&project.join(".agent-cancel"))?;
    }
    remove_if_exists(&project.join(".agent-active"))
}

pub(super) fn remove_if_exists(path: &Path) -> Result<()> {
    match std::fs::remove_file(path) {
        Ok(()) => Ok(()),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(error) => Err(Error::Io(error)),
    }
}

fn read_marker(path: &Path) -> std::io::Result<String> {
    let metadata = std::fs::symlink_metadata(path)?;
    if metadata.file_type().is_symlink() || !metadata.is_file() {
        return Err(std::io::Error::new(
            std::io::ErrorKind::InvalidData,
            "Agent marker must be a regular file",
        ));
    }
    let file = std::fs::File::open(path)?;
    if !file.metadata()?.is_file() {
        return Err(std::io::Error::new(
            std::io::ErrorKind::InvalidData,
            "Agent marker must be a regular file",
        ));
    }
    let mut text = String::new();
    file.take(4097).read_to_string(&mut text)?;
    if text.len() > 4096 {
        return Err(std::io::Error::new(
            std::io::ErrorKind::InvalidData,
            "Agent marker exceeds 4096 bytes",
        ));
    }
    Ok(text)
}
