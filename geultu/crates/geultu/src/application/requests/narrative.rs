use crate::model::{EvidenceRef, ThreadStatus};
use serde::Deserialize;

#[derive(Deserialize)]
pub(in crate::application) struct SceneAdd {
    pub(in crate::application) project_id: String,
    pub(in crate::application) scene_id: String,
    pub(in crate::application) title: String,
    pub(in crate::application) text: Option<String>,
    pub(in crate::application) order: Option<i64>,
    pub(in crate::application) command_id: Option<String>,
}

#[derive(Deserialize)]
pub(in crate::application) struct SceneRef {
    pub(in crate::application) project_id: String,
    pub(in crate::application) scene_id: String,
}

#[derive(Deserialize)]
pub(in crate::application) struct SceneSetText {
    pub(in crate::application) project_id: String,
    pub(in crate::application) scene_id: String,
    pub(in crate::application) text: String,
    pub(in crate::application) expected_scene_revision: Option<u64>,
    pub(in crate::application) run_id: Option<String>,
    pub(in crate::application) command_id: Option<String>,
}

#[derive(Deserialize)]
pub(in crate::application) struct FactAdd {
    pub(in crate::application) project_id: String,
    pub(in crate::application) fact_id: String,
    pub(in crate::application) subject: String,
    pub(in crate::application) predicate: String,
    pub(in crate::application) object: String,
    #[serde(default)]
    pub(in crate::application) evidence: Vec<EvidenceRef>,
    pub(in crate::application) command_id: Option<String>,
}

#[derive(Deserialize)]
pub(in crate::application) struct TimelineAdd {
    pub(in crate::application) project_id: String,
    pub(in crate::application) entry_id: String,
    pub(in crate::application) order: i64,
    pub(in crate::application) label: String,
    pub(in crate::application) description: String,
    pub(in crate::application) scene_id: Option<String>,
    pub(in crate::application) command_id: Option<String>,
}

#[derive(Deserialize)]
pub(in crate::application) struct ThreadAdd {
    pub(in crate::application) project_id: String,
    pub(in crate::application) thread_id: String,
    pub(in crate::application) title: String,
    pub(in crate::application) status: Option<ThreadStatus>,
    #[serde(default)]
    pub(in crate::application) evidence: Vec<EvidenceRef>,
    pub(in crate::application) command_id: Option<String>,
}

#[derive(Deserialize)]
pub(in crate::application) struct ThreadSetStatus {
    pub(in crate::application) project_id: String,
    pub(in crate::application) thread_id: String,
    pub(in crate::application) status: ThreadStatus,
    pub(in crate::application) command_id: Option<String>,
}
