use serde::Deserialize;

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(in crate::application) struct ProjectRef {
    pub(in crate::application) project_id: String,
}
