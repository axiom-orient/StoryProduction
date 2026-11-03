//! Journal envelope validation, append, and replay primitives.

use super::file_io::atomic_replace;
use crate::domain::validation::validate_id;
use crate::domain::{evolve, Event, EventEnvelope};
use crate::error::{Error, Result};
use crate::model::{Project, SCHEMA_VERSION};
use std::fs::{self, File};
use std::io::{self, BufRead, BufReader, Read, Write};
use std::path::Path;

fn validate_envelope(envelope: &EventEnvelope) -> Result<()> {
    if envelope.schema_version != SCHEMA_VERSION {
        return Err(Error::Corrupt(format!(
            "unsupported journal schema {}",
            envelope.schema_version
        )));
    }
    validate_id("journal command id", &envelope.command_id)
        .map_err(|error| Error::Corrupt(error.to_string()))
}

pub(super) fn ensure_project_identity(id: &str, project: &Project) -> Result<()> {
    if project.id != id {
        return Err(Error::Corrupt(format!(
            "project identity mismatch: expected {id}, actual {}",
            project.id
        )));
    }
    Ok(())
}

pub(super) fn ensure_created_project(id: &str, event: &Event) -> Result<()> {
    match event {
        Event::ProjectCreated { id: created_id, .. } if created_id == id => Ok(()),
        _ => Err(Error::Corrupt(
            "journal must start with the requested project identity".into(),
        )),
    }
}

/// Decode each committed line through one schema and sequence admission owner.
pub(super) fn journal_entries(path: &Path) -> Result<impl Iterator<Item = Result<EventEnvelope>>> {
    let reader = BufReader::new(File::open(path)?);
    Ok(reader.lines().enumerate().map(|(line_index, line)| {
        let line = line?;
        if line.trim().is_empty() {
            return Err(Error::Corrupt(format!(
                "blank journal line at {}",
                line_index + 1
            )));
        }
        let envelope: EventEnvelope = serde_json::from_str(&line).map_err(|error| {
            Error::Corrupt(format!("invalid journal line {}: {error}", line_index + 1))
        })?;
        validate_envelope(&envelope)?;
        let expected = line_index as u64 + 1;
        if envelope.seq != expected {
            return Err(Error::Corrupt(format!(
                "journal sequence mismatch: expected {expected}, actual {}",
                envelope.seq
            )));
        }
        Ok(envelope)
    }))
}

pub(super) fn replay_journal(path: &Path) -> Result<(u64, Project)> {
    if !path.exists() {
        return Err(Error::NotFound(format!(
            "journal was not found: {}",
            path.display()
        )));
    }
    let mut project = None;
    let mut count = 0;
    for entry in journal_entries(path)? {
        let envelope = entry?;
        count = envelope.seq;
        project = Some(evolve(project, &envelope.event)?);
        if let Some(state) = project.as_mut() {
            state
                .applied_commands
                .insert(envelope.command_id, state.revision);
        }
    }
    Ok((
        count,
        project.ok_or_else(|| Error::Corrupt("project journal is empty".into()))?,
    ))
}

pub(super) fn append_envelopes(path: &Path, envelopes: &[EventEnvelope]) -> Result<()> {
    if envelopes.is_empty() {
        return Err(Error::InvalidInput(
            "journal publication requires at least one event".into(),
        ));
    }
    let mut bytes = Vec::new();
    for envelope in envelopes {
        serde_json::to_writer(&mut bytes, envelope)?;
        bytes.push(b'\n');
    }
    // Preserve the exact committed prefix while publishing this whole command's
    // event group atomically. A torn write never becomes a committed journal.
    let previous = match fs::symlink_metadata(path) {
        Ok(metadata) => {
            if !metadata.is_file() || metadata.file_type().is_symlink() {
                return Err(Error::Corrupt("journal is not a regular file".into()));
            }
            let file = File::open(path)?;
            let opened = file.metadata()?;
            if !opened.is_file() || opened.len() != metadata.len() {
                return Err(Error::Corrupt("journal changed during open".into()));
            }
            #[cfg(unix)]
            {
                use std::os::unix::fs::MetadataExt;
                if (opened.dev(), opened.ino()) != (metadata.dev(), metadata.ino()) {
                    return Err(Error::Corrupt(
                        "journal identity changed during open".into(),
                    ));
                }
            }
            Some((file, opened.len()))
        }
        Err(error) if error.kind() == io::ErrorKind::NotFound => {
            if envelopes[0].seq != 1 {
                return Err(Error::Corrupt(
                    "committed journal disappeared before append".into(),
                ));
            }
            None
        }
        Err(error) => return Err(error.into()),
    };
    atomic_replace(path, move |target| {
        if let Some((file, length)) = previous {
            let mut source = file.take(length);
            let copied = io::copy(&mut source, target)?;
            if copied != length || source.get_ref().metadata()?.len() != length {
                return Err(Error::Corrupt("journal size changed during staging".into()));
            }
        }
        target.write_all(&bytes)?;
        Ok(())
    })
}
