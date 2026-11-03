use serde::Deserialize;

use super::super::Kind;

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct ProjectRef {
    pub(super) project_id: String,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct DocumentRef {
    pub(super) project_id: String,
    pub(super) kind: Kind,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct ItemRef {
    pub(super) project_id: String,
    pub(super) id: String,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct Handoff {
    pub(super) project_id: String,
    pub(super) actor: String,
    pub(super) language: String,
    pub(super) expected_revision: u64,
    pub(super) expected_markdown_sha256: String,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct History {
    pub(super) project_id: String,
    pub(super) kind: Kind,
    #[serde(default)]
    pub(super) revision: Option<u64>,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct Restore {
    pub(super) project_id: String,
    pub(super) kind: Kind,
    pub(super) revision: u64,
    pub(super) expected_revision: u64,
    pub(super) actor: String,
    pub(super) command_id: String,
}
