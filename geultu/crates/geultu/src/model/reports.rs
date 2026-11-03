use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum Severity {
    Info,
    Warning,
    Error,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Finding {
    pub code: String,
    pub severity: Severity,
    pub message: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub scene_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub evidence: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct StyleMetrics {
    pub character_count: usize,
    pub sentence_count: usize,
    pub average_sentence_length: f64,
    pub dialogue_line_ratio: f64,
    pub paragraph_count: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ReviewReport {
    pub project_id: String,
    pub source_revision: u64,
    pub metrics: StyleMetrics,
    pub findings: Vec<Finding>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ProofReport {
    pub fidelity: f64,
    pub grounding: f64,
    pub naturalness: f64,
    pub protected_terms_preserved: bool,
    pub findings: Vec<Finding>,
}
