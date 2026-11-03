use serde::Deserialize;

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(in crate::application) struct ProjectCreate {
    pub(in crate::application) project_id: String,
    pub(in crate::application) title: String,
    #[serde(default)]
    pub(in crate::application) display_label: Option<String>,
    #[serde(default)]
    pub(in crate::application) summary: Option<String>,
    pub(in crate::application) command_id: Option<String>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(in crate::application) struct ProjectMetadataUpdate {
    pub(in crate::application) project_id: String,
    pub(in crate::application) title: String,
    /// Empty values clear optional metadata; the whole metadata record is replaced.
    pub(in crate::application) display_label: String,
    pub(in crate::application) summary: String,
    pub(in crate::application) expected_project_revision: u64,
    pub(in crate::application) command_id: Option<String>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(in crate::application) struct RecoverProject {
    pub(in crate::application) project_id: String,
    #[serde(default)]
    pub(in crate::application) remove_lock: bool,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(in crate::application) struct ImportMarkdown {
    pub(in crate::application) project_id: String,
    pub(in crate::application) markdown: String,
    pub(in crate::application) command_id: Option<String>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(in crate::application) struct ReviewedSynopsisExportRequest {
    pub(in crate::application) project_id: String,
    pub(in crate::application) actor_id: String,
    pub(in crate::application) language: String,
    #[serde(default)]
    pub(in crate::application) reason: Option<String>,
    pub(in crate::application) expected_source_revision: u64,
    pub(in crate::application) expected_synopsis_sha256: String,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(in crate::application) struct GeultuStoryExportRequest {
    pub(in crate::application) project_id: String,
    pub(in crate::application) language: String,
    pub(in crate::application) logline: String,
    #[serde(default)]
    pub(in crate::application) global_constraints: Vec<String>,
}
