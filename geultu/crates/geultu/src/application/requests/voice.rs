use crate::voice::VoiceSample;
use serde::Deserialize;

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(in crate::application) struct VoiceProfileCreate {
    pub(in crate::application) project_id: String,
    pub(in crate::application) profile_id: Option<String>,
    pub(in crate::application) expected_project_revision: u64,
    pub(in crate::application) samples: Vec<VoiceSampleInput>,
    pub(in crate::application) command_id: Option<String>,
}

#[derive(Deserialize)]
#[serde(untagged)]
pub(in crate::application) enum VoiceSampleInput {
    Text(String),
    Object(VoiceSampleObject),
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(in crate::application) struct VoiceSampleObject {
    pub(in crate::application) id: Option<String>,
    pub(in crate::application) text: String,
}

impl VoiceSampleInput {
    pub(in crate::application) fn into_voice_sample(self, index: usize) -> VoiceSample {
        match self {
            Self::Text(text) => VoiceSample {
                id: format!("sample-{index}"),
                text,
            },
            Self::Object(sample) => VoiceSample {
                id: sample.id.unwrap_or_else(|| format!("sample-{index}")),
                text: sample.text,
            },
        }
    }
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(in crate::application) struct VoiceCompare {
    pub(in crate::application) project_id: String,
    pub(in crate::application) candidate_text: String,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(in crate::application) struct VoiceGuideRequest {
    pub(in crate::application) project_id: String,
    pub(in crate::application) scene_id: String,
}
