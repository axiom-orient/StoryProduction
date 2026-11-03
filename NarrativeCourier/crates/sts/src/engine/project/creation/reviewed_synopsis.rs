//! Hash-bound geultu reviewed-synopsis ingress into a new STS project.

use super::super::super::{
    generated_project_name, read_reference_bytes, resolve_regular_file_within_root, Engine,
};
use super::ReviewedSynopsisInput;
use crate::model::project::{
    CreateReviewedSynopsisProjectRequest, NewProject, ReviewedSynopsisBinding,
};
use crate::model::views::ProjectView;
use narrative_courier_contracts::{
    validate_reviewed_synopsis_handoff_identity, ReviewedSynopsisHandoff,
};
use narrative_courier_runtime::store::sha256_bytes;
use narrative_courier_runtime::{Error, Result};
use std::fs;
use std::path::PathBuf;

impl Engine {
    /// Verifies and ingests a geultu-reviewed synopsis. The handoff and its
    /// synopsis are resolved relative to the declared handoff root, with
    /// realpath/lstat checks at the boundary. No screenplay or NarrativePackage
    /// is accepted at this ingress.
    pub fn create_project_from_reviewed_synopsis(
        &self,
        request: &CreateReviewedSynopsisProjectRequest,
    ) -> Result<ProjectView> {
        let handoff_root = PathBuf::from(&request.handoff_root);
        let handoff_path = resolve_regular_file_within_root(&handoff_root, &request.handoff_path)?;
        let handoff_bytes = read_reference_bytes(&handoff_path)?;
        let handoff: ReviewedSynopsisHandoff = serde_json::from_slice(&handoff_bytes)?;
        validate_reviewed_synopsis_handoff_identity(&handoff)?;
        let handoff_hash = sha256_bytes(&handoff_bytes);
        let handoff_dir = handoff_path
            .parent()
            .ok_or_else(|| Error::Contract("reviewed synopsis handoff has no parent".into()))?;
        let synopsis_path = resolve_regular_file_within_root(handoff_dir, &handoff.synopsis_path)?;
        let synopsis_bytes = fs::read(&synopsis_path)?;
        if sha256_bytes(&synopsis_bytes) != handoff.synopsis_sha256 {
            return Err(Error::Contract(
                "reviewed synopsis raw hash does not match handoff".into(),
            ));
        }
        let synopsis = std::str::from_utf8(&synopsis_bytes)
            .map_err(|error| Error::Contract(format!("reviewed synopsis is not UTF-8: {error}")))?;
        if synopsis.trim().is_empty() {
            return Err(Error::Contract(
                "reviewed synopsis must not be empty".into(),
            ));
        }
        let title = request
            .title
            .as_deref()
            .unwrap_or(handoff.title.as_str())
            .trim()
            .to_string();
        if title != handoff.title.trim() {
            return Err(Error::Contract(
                "project title must match the reviewed synopsis handoff".into(),
            ));
        }
        let project_name = if request.project_name.trim().is_empty() {
            generated_project_name(&title)
        } else {
            request.project_name.clone()
        };
        let binding = ReviewedSynopsisBinding::from_verified_handoff(&handoff, handoff_hash);
        let new_project = NewProject {
            project_name,
            title,
            synopsis: synopsis.to_string(),
            source_format: "original".into(),
            target_duration: request.target_duration,
            genre: request.genre.clone(),
            tone: request.tone.clone(),
            theme_direction: request.theme_direction.clone(),
            references: Vec::new(),
        };
        self.create_project_with_synopsis_bytes(
            &new_project,
            &synopsis_bytes,
            Some(ReviewedSynopsisInput {
                binding,
                handoff_bytes,
                reference_name: handoff_path
                    .file_name()
                    .and_then(|name| name.to_str())
                    .ok_or_else(|| {
                        Error::Contract("handoff reference filename is not UTF-8".into())
                    })?
                    .to_owned(),
            }),
        )
    }
}
