use crate::model::{Finding, StyleMetrics};
use serde::{Deserialize, Serialize};

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(in crate::application) struct ProofVerify {
    pub(in crate::application) source: String,
    pub(in crate::application) candidate: String,
    #[serde(default)]
    pub(in crate::application) evidence: Vec<String>,
    #[serde(default)]
    pub(in crate::application) protected_terms: Vec<String>,
    /// Optional journal anchor; when present the receipt is recorded into
    /// this project's event journal.
    pub(in crate::application) project_id: Option<String>,
    pub(in crate::application) command_id: Option<String>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(in crate::application) struct CandidateCreate {
    pub(in crate::application) project_id: String,
    pub(in crate::application) candidate_id: Option<String>,
    pub(in crate::application) kind: String,
    pub(in crate::application) writing_type: Option<crate::generation::WritingType>,
    pub(in crate::application) stage: Option<crate::generation::WritingStage>,
    pub(in crate::application) source_revision: Option<u64>,
    pub(in crate::application) content: String,
    pub(in crate::application) provider: String,
    pub(in crate::application) model: String,
    pub(in crate::application) request_id: Option<String>,
    pub(in crate::application) input_hash: Option<String>,
    pub(in crate::application) command_id: Option<String>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(in crate::application) struct ScenePrepareCandidate {
    pub(in crate::application) project_id: String,
    pub(in crate::application) scene_id: String,
    pub(in crate::application) expected_content_revision: u64,
    pub(in crate::application) expected_scene_revision: u64,
    pub(in crate::application) candidate_id: String,
    pub(in crate::application) changeset_id: String,
    pub(in crate::application) kind: String,
    pub(in crate::application) content: String,
    pub(in crate::application) provider: String,
    pub(in crate::application) model: String,
    pub(in crate::application) request_id: Option<String>,
    pub(in crate::application) input_hash: Option<String>,
    pub(in crate::application) command_id: Option<String>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(in crate::application) struct CandidateRef {
    pub(in crate::application) project_id: String,
    pub(in crate::application) candidate_id: String,
    pub(in crate::application) actor_id: String,
    pub(in crate::application) reason: Option<String>,
    pub(in crate::application) expected_artifact_revision: u64,
    pub(in crate::application) expected_artifact_digest: String,
    pub(in crate::application) command_id: Option<String>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(in crate::application) struct CandidateSubjectRef {
    pub(in crate::application) project_id: String,
    pub(in crate::application) candidate_id: String,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(in crate::application) struct ChangeSetCreate {
    pub(in crate::application) project_id: String,
    pub(in crate::application) candidate_id: String,
    pub(in crate::application) changeset_id: Option<String>,
    pub(in crate::application) operations: Vec<crate::model::ChangeOperation>,
    pub(in crate::application) command_id: Option<String>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(in crate::application) struct ChangeSetApprove {
    pub(in crate::application) project_id: String,
    pub(in crate::application) changeset_id: String,
    pub(in crate::application) expected_project_revision: u64,
    pub(in crate::application) run_id: Option<String>,
    pub(in crate::application) actor_id: String,
    pub(in crate::application) reason: Option<String>,
    pub(in crate::application) expected_artifact_revision: u64,
    pub(in crate::application) expected_artifact_digest: String,
    pub(in crate::application) command_id: Option<String>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(in crate::application) struct ChangeSetRef {
    pub(in crate::application) project_id: String,
    pub(in crate::application) changeset_id: String,
    pub(in crate::application) actor_id: String,
    pub(in crate::application) reason: Option<String>,
    pub(in crate::application) expected_artifact_revision: u64,
    pub(in crate::application) expected_artifact_digest: String,
    pub(in crate::application) command_id: Option<String>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(in crate::application) struct RunOpen {
    pub(in crate::application) project_id: String,
    pub(in crate::application) run_id: String,
    pub(in crate::application) stage: String,
    pub(in crate::application) budgets: crate::contracts::RunBudgets,
    pub(in crate::application) frozen_inputs: crate::contracts::RunFrozenInputs,
    pub(in crate::application) accepted_outputs:
        Option<std::collections::BTreeMap<String, crate::contracts::ArtifactRef>>,
    pub(in crate::application) command_id: Option<String>,
}

/// review.run response: historical report fields plus the additive receipt.
#[derive(Serialize)]
pub(in crate::application) struct ReviewRunResponse {
    pub(in crate::application) project_id: String,
    pub(in crate::application) source_revision: u64,
    pub(in crate::application) metrics: StyleMetrics,
    pub(in crate::application) findings: Vec<Finding>,
    pub(in crate::application) receipt: crate::contracts::ValidationReceipt,
    pub(in crate::application) idempotent: bool,
}
