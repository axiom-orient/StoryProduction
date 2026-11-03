use serde::Deserialize;

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(in crate::application) struct RefinementDiagnose {
    pub(in crate::application) project_id: String,
    pub(in crate::application) scene_id: Option<String>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(in crate::application) struct RefinementPropose {
    pub(in crate::application) project_id: String,
    pub(in crate::application) scene_id: String,
    pub(in crate::application) candidate_id: Option<String>,
    pub(in crate::application) changeset_id: Option<String>,
    pub(in crate::application) command_id: Option<String>,
}
