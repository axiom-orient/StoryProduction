//! Reviewed synopsis export and hash-bound cross-owner handoff publication.

use super::file_io::{atomic_json, atomic_write, ensure_directory};
use super::lock::acquire_lock;
use super::{now_ms, validate_project_id, Workspace};
use crate::contracts::digest::sha256_hex;
use crate::contracts::{
    GeultuReviewedSynopsisHandoff, SynopsisApproval, REVIEWED_SYNOPSIS_HANDOFF_SCHEMA_ID,
    REVIEWED_SYNOPSIS_HANDOFF_SCHEMA_VERSION,
};
use crate::error::{Error, Result};
use crate::markdown::export_markdown;
use std::fs::{self, File, OpenOptions};
use std::io::{self, Read, Write};
use std::path::Path;
use std::sync::atomic::{AtomicU64, Ordering};

impl Workspace {
    /// Emits the only geultu artifact intended for the NarrativeCourier main
    /// edge: a raw UTF-8 synopsis and a human approval bound to that exact
    /// byte stream and project revision.  This method intentionally does not
    /// emit an NVC release package; that contract belongs to NarrativeCourier.
    pub fn export_reviewed_synopsis(
        &self,
        id: &str,
        actor_id: &str,
        language: &str,
        reason: Option<&str>,
        expected_source_revision: u64,
        expected_synopsis_sha256: &str,
    ) -> Result<GeultuReviewedSynopsisHandoff> {
        validate_project_id(id)?;
        if actor_id.trim().is_empty() {
            return Err(Error::InvalidInput(
                "approval actor_id must not be empty".into(),
            ));
        }
        if !crate::contracts::validate::valid_language_tag(language, 1) {
            return Err(Error::InvalidInput(
                "language must be a BCP-47-like language tag".into(),
            ));
        }
        if expected_source_revision == 0
            || !crate::contracts::validate::valid_sha256_hex(expected_synopsis_sha256)
        {
            return Err(Error::InvalidInput(
                "review requires a positive source revision and lowercase SHA-256".into(),
            ));
        }
        if !self.events_path(id).is_file() {
            return Err(Error::NotFound(format!("project '{id}' was not found")));
        }
        // Serialize export with canonical writes and other exports. The two
        // public handoff files still require consumer hash verification.
        let _lock = acquire_lock(&self.project_dir(id).join("lock"))?;
        let project = self.load_project(id)?;
        let source_revision = project.revision;
        if source_revision == 0 {
            return Err(Error::Corrupt("project revision is zero".into()));
        }
        if expected_source_revision != source_revision {
            return Err(Error::Conflict(format!(
                "source revision mismatch: expected {expected_source_revision}, actual {source_revision}"
            )));
        }
        let synopsis = export_markdown(&project);
        let synopsis_bytes = synopsis.as_bytes();
        let synopsis_sha256 = sha256_hex(synopsis_bytes);
        if expected_synopsis_sha256 != synopsis_sha256 {
            return Err(Error::Conflict(format!(
                "synopsis hash mismatch: expected {expected_synopsis_sha256}, actual {synopsis_sha256}"
            )));
        }
        let handoff = reviewed_handoff(&project, actor_id, language, reason, synopsis_sha256)?;

        let project_dir = self.project_dir(id);
        let handoff_dir = project_dir.join("handoff");
        ensure_directory(&handoff_dir)?;
        atomic_write(&handoff_dir.join("synopsis.md"), synopsis_bytes)?;
        atomic_json(
            &handoff_dir.join("reviewed-synopsis-handoff.v1.json"),
            &handoff,
        )?;
        Ok(handoff)
    }

    /// Export only the adopted workbench synopsis, never the entire source/story project.
    /// Each source revision has an immutable directory; repeated review reuses its receipt.
    pub fn export_authoring_synopsis(
        &self,
        id: &str,
        actor: &str,
        language: &str,
        expected_revision: u64,
        expected_markdown_sha256: &str,
    ) -> Result<serde_json::Value> {
        validate_project_id(id)?;
        if actor.trim().is_empty() || !crate::contracts::validate::valid_language_tag(language, 1) {
            return Err(Error::InvalidInput(
                "actor and a valid language tag are required".into(),
            ));
        }
        if expected_revision == 0
            || !crate::contracts::validate::valid_sha256_hex(expected_markdown_sha256)
        {
            return Err(Error::InvalidInput(
                "review requires a positive synopsis revision and lowercase SHA-256".into(),
            ));
        }
        let _lock = acquire_lock(&self.project_dir(id).join("lock"))?;
        let project = self.load_project(id)?;
        let exported = crate::authoring::export(&project, crate::authoring::Kind::Synopsis)?;
        if exported["revision"].as_u64() != Some(expected_revision)
            || exported["markdown_sha256"].as_str() != Some(expected_markdown_sha256)
        {
            return Err(Error::Conflict(
                "reviewed synopsis revision or bytes have changed".into(),
            ));
        }
        let synopsis = exported["markdown"]
            .as_str()
            .ok_or_else(|| Error::Corrupt("synopsis export missing".into()))?;
        let handoff_root = self.project_dir(id).join("handoff");
        ensure_directory(&handoff_root)?;
        let directory = handoff_root.join(format!(
            "authoring-{}-{}",
            project.revision, expected_markdown_sha256
        ));
        ensure_directory(&directory)?;
        File::open(self.project_dir(id))?.sync_all()?;
        File::open(&handoff_root)?.sync_all()?;
        let manifest = directory.join("reviewed-synopsis-handoff.v1.json");
        let intent = directory.join(".review-intent.json");
        let handoff = if handoff_entry_exists(&manifest)? {
            let prior = read_handoff(&manifest)?;
            require_authoring_review(&prior, &project, actor, language, expected_markdown_sha256)?;
            require_same_handoff_bytes(&directory.join("synopsis.md"), synopsis.as_bytes())?;
            prior
        } else {
            let handoff = if handoff_entry_exists(&intent)? {
                let pending = read_handoff(&intent)?;
                require_authoring_review(
                    &pending,
                    &project,
                    actor,
                    language,
                    expected_markdown_sha256,
                )?;
                pending
            } else {
                // This explicit human request approves the current bytes. A body left by
                // an interrupted export may be reused only when it is byte-identical.
                if handoff_entry_exists(&directory.join("synopsis.md"))? {
                    require_same_handoff_bytes(
                        &directory.join("synopsis.md"),
                        synopsis.as_bytes(),
                    )?;
                }
                reviewed_handoff(
                    &project,
                    actor,
                    language,
                    Some(AUTHORING_REVIEW_REASON),
                    expected_markdown_sha256.into(),
                )?
            };
            let receipt_bytes = serde_json::to_vec_pretty(&handoff)?;
            publish_handoff_file(&intent, &receipt_bytes)?;
            publish_handoff_file(&directory.join("synopsis.md"), synopsis.as_bytes())?;
            // Only the public receipt admits this pair to a consumer.
            publish_handoff_file(&manifest, &receipt_bytes)?;
            handoff
        };
        if handoff_entry_exists(&intent)? {
            let pending = read_handoff(&intent)?;
            if pending != handoff {
                return Err(Error::Conflict(
                    "published handoff has a different pending review; inspect it before cleanup"
                        .into(),
                ));
            }
            fs::remove_file(&intent).map_err(|error| {
                Error::Io(io::Error::new(
                    error.kind(),
                    format!("handoff is published, but review-intent cleanup failed: {error}"),
                ))
            })?;
        }
        File::open(&directory)?.sync_all()?;
        Ok(
            serde_json::json!({"handoff":handoff,"handoff_root":directory,"handoff_path":"reviewed-synopsis-handoff.v1.json"}),
        )
    }
}

const AUTHORING_REVIEW_REASON: &str = "Authoring synopsis reviewed for STS handoff";

/// Shared producer receipt assembly; each caller retains its source/export authority.
fn reviewed_handoff(
    project: &crate::model::Project,
    actor: &str,
    language: &str,
    reason: Option<&str>,
    synopsis_sha256: String,
) -> Result<GeultuReviewedSynopsisHandoff> {
    let mut handoff = GeultuReviewedSynopsisHandoff {
        schema_id: REVIEWED_SYNOPSIS_HANDOFF_SCHEMA_ID.into(),
        schema_version: REVIEWED_SYNOPSIS_HANDOFF_SCHEMA_VERSION,
        handoff_id: String::new(),
        project_id: project.id.clone(),
        source_revision: project.revision,
        synopsis_path: "synopsis.md".into(),
        approval: SynopsisApproval {
            actor_id: actor.trim().into(),
            decision: "APPROVE".into(),
            synopsis_sha256: synopsis_sha256.clone(),
            source_revision: project.revision,
            approved_at_ms: now_ms(),
            reason: reason
                .map(str::trim)
                .filter(|value| !value.is_empty())
                .map(str::to_owned),
        },
        synopsis_sha256,
        title: project.title.clone(),
        language: language.into(),
    };
    handoff.handoff_id = handoff.expected_handoff_id()?;
    handoff.validate_identity()?;
    Ok(handoff)
}

fn require_regular_handoff_file(path: &Path) -> Result<fs::Metadata> {
    let metadata = fs::symlink_metadata(path)?;
    if !metadata.is_file() || metadata.file_type().is_symlink() {
        return Err(Error::Conflict(format!(
            "immutable handoff file is not a regular file: {}",
            path.display()
        )));
    }
    Ok(metadata)
}

fn handoff_entry_exists(path: &Path) -> Result<bool> {
    match fs::symlink_metadata(path) {
        Ok(_) => Ok(true),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(false),
        Err(error) => Err(error.into()),
    }
}

fn read_handoff(path: &Path) -> Result<GeultuReviewedSynopsisHandoff> {
    let bytes = read_handoff_bytes(path, 64 * 1024)?;
    let value: GeultuReviewedSynopsisHandoff = serde_json::from_slice(&bytes)?;
    value.validate_identity()?;
    Ok(value)
}

fn require_authoring_review(
    handoff: &GeultuReviewedSynopsisHandoff,
    project: &crate::model::Project,
    actor: &str,
    language: &str,
    synopsis_sha256: &str,
) -> Result<()> {
    if handoff.project_id != project.id
        || handoff.title != project.title
        || handoff.synopsis_path != "synopsis.md"
        || handoff.approval.reason.as_deref() != Some(AUTHORING_REVIEW_REASON)
        || handoff.synopsis_sha256 != synopsis_sha256
        || handoff.approval.actor_id != actor.trim()
        || handoff.language != language
        || handoff.source_revision != project.revision
    {
        return Err(Error::Conflict(
            "immutable synopsis handoff differs from this review".into(),
        ));
    }
    Ok(())
}

fn read_handoff_bytes(path: &Path, limit: usize) -> Result<Vec<u8>> {
    let before = require_regular_handoff_file(path)?;
    let file = File::open(path)?;
    let opened = file.metadata()?;
    if !opened.is_file() || opened.len() != before.len() {
        return Err(Error::Conflict(
            "handoff input type or size changed while opening".into(),
        ));
    }
    #[cfg(unix)]
    {
        use std::os::unix::fs::MetadataExt;
        if (opened.dev(), opened.ino()) != (before.dev(), before.ino()) {
            return Err(Error::Conflict(
                "handoff input identity changed while opening".into(),
            ));
        }
    }
    let mut bytes = Vec::new();
    file.take(limit as u64 + 1).read_to_end(&mut bytes)?;
    if bytes.len() > limit {
        return Err(Error::Conflict(
            "handoff input exceeds its byte limit".into(),
        ));
    }
    if bytes.len() as u64 != opened.len() {
        return Err(Error::Conflict(
            "handoff input size changed while reading".into(),
        ));
    }
    Ok(bytes)
}

fn require_same_handoff_bytes(path: &Path, expected: &[u8]) -> Result<()> {
    if read_handoff_bytes(path, expected.len())? != expected {
        return Err(Error::Conflict(format!(
            "immutable handoff bytes differ: {}",
            path.display()
        )));
    }
    Ok(())
}

static HANDOFF_STAGE_SEQUENCE: AtomicU64 = AtomicU64::new(0);

fn publish_handoff_file(path: &Path, bytes: &[u8]) -> Result<()> {
    let parent = path
        .parent()
        .ok_or_else(|| Error::InvalidInput("handoff path has no parent".into()))?;
    if handoff_entry_exists(path)? {
        require_same_handoff_bytes(path, bytes)?;
        File::open(parent)?.sync_all()?;
        return Ok(());
    }
    let sequence = HANDOFF_STAGE_SEQUENCE
        .fetch_update(Ordering::Relaxed, Ordering::Relaxed, |value| {
            value.checked_add(1)
        })
        .map_err(|_| Error::Conflict("handoff staging sequence exhausted".into()))?;
    let name = path
        .file_name()
        .and_then(|value| value.to_str())
        .ok_or_else(|| Error::InvalidInput("handoff file name must be UTF-8".into()))?;
    let temporary = parent.join(format!(
        ".{name}.{}.{}.{}.tmp",
        std::process::id(),
        now_ms(),
        sequence
    ));
    let mut file = OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&temporary)?;
    let result = (|| -> Result<()> {
        file.write_all(bytes)?;
        file.sync_all()?;
        match fs::hard_link(&temporary, path) {
            Ok(()) => {}
            Err(error) if error.kind() == io::ErrorKind::AlreadyExists => {
                require_same_handoff_bytes(path, bytes)?;
            }
            Err(error) => return Err(error.into()),
        }
        File::open(parent)?.sync_all()?;
        Ok(())
    })();
    drop(file);
    let cleanup = fs::remove_file(&temporary);
    match (result, cleanup) {
        (result, Ok(())) => result,
        (result, Err(error)) => Err(Error::Io(io::Error::new(error.kind(), format!(
            "handoff publication may have completed; staging cleanup failed: {error}; operation: {result:?}"
        )))),
    }
}
