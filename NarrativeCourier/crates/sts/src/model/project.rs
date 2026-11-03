//! Project configuration and explicit project import/handoff request DTOs.

use super::views::ProjectView;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ProjectConfig {
    pub title: String,
    pub source_format: String,
    pub target_duration: u32,
    #[serde(default)]
    pub genre: Vec<String>,
    #[serde(default)]
    pub tone: String,
    #[serde(default)]
    pub theme_direction: String,
    pub synopsis_file: String,
    #[serde(default)]
    pub reference_files: Vec<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub reviewed_synopsis: Option<ReviewedSynopsisBinding>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ReferenceInput {
    pub source_path: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct NewProject {
    #[serde(default)]
    pub project_name: String,
    pub title: String,
    pub synopsis: String,
    pub source_format: String,
    pub target_duration: u32,
    #[serde(default)]
    pub genre: Vec<String>,
    #[serde(default)]
    pub tone: String,
    #[serde(default)]
    pub theme_direction: String,
    #[serde(default)]
    pub references: Vec<ReferenceInput>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CreateReviewedSynopsisProjectRequest {
    #[serde(default)]
    pub project_name: String,
    #[serde(default)]
    pub title: Option<String>,
    pub handoff_root: String,
    pub handoff_path: String,
    pub target_duration: u32,
    #[serde(default)]
    pub genre: Vec<String>,
    #[serde(default)]
    pub tone: String,
    #[serde(default)]
    pub theme_direction: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct ReviewedSynopsisBinding {
    pub handoff_id: String,
    pub handoff_sha256: String,
    pub source_project_id: String,
    pub source_revision: u64,
    pub synopsis_sha256: String,
    pub title: String,
    pub language: String,
    pub actor_id: String,
}

impl ReviewedSynopsisBinding {
    pub(crate) fn from_verified_handoff(
        handoff: &narrative_courier_contracts::ReviewedSynopsisHandoff,
        handoff_sha256: String,
    ) -> Self {
        Self {
            handoff_id: handoff.handoff_id.clone(),
            handoff_sha256,
            source_project_id: handoff.project_id.clone(),
            source_revision: handoff.source_revision,
            synopsis_sha256: handoff.synopsis_sha256.clone(),
            title: handoff.title.clone(),
            language: handoff.language.clone(),
            actor_id: handoff.approval.actor_id.clone(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ImportScreenplayRequest {
    #[serde(default)]
    pub project_name: String,
    pub title: String,
    /// Human-approved semantic source. Import never derives it from the script.
    pub synopsis: String,
    #[serde(default = "default_source_format")]
    pub source_format: String,
    pub target_duration: u32,
    #[serde(default)]
    pub genre: Vec<String>,
    #[serde(default)]
    pub tone: String,
    #[serde(default)]
    pub theme_direction: String,
    /// Path to a `.fountain` or `.fdx` screenplay readable by this process.
    pub script_path: String,
}

fn default_source_format() -> String {
    "original".into()
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImportReport {
    pub project: ProjectView,
    pub summary: crate::fountain::ScreenplaySummary,
    pub reference_files: Vec<String>,
}
