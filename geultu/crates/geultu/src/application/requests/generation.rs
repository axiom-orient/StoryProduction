use crate::generation::GenerationRequest;
use serde::Deserialize;

#[derive(Deserialize)]
pub(in crate::application) struct GenerationGenerate {
    #[serde(flatten)]
    pub(in crate::application) request: GenerationRequest,
    pub(in crate::application) provider_id: String,
}
