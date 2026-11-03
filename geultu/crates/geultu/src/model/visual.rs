use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct Shot {
    pub index: usize,
    pub shot_type: String,
    pub description: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct VisualPlan {
    pub id: String,
    pub scene_id: String,
    pub source_revision: u64,
    pub shots: Vec<Shot>,
}
