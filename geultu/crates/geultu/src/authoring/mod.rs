//! Story and synopsis workbench. STS remains the screenplay/scene authority.
//! All mutations are replayable journal commands; model output never applies itself.
pub mod api;
mod conformance;
mod freshness;
mod mutation;
mod read;
mod validation;

pub use conformance::ensure_conformance;
pub use freshness::stale_reasons;
pub use mutation::apply;
pub use read::{candidate_view, draft_schema, export, packet, status};

use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
#[serde(rename_all = "snake_case")]
pub enum Kind {
    Source,
    Story,
    Synopsis,
}

impl Kind {
    pub fn label(self) -> &'static str {
        match self {
            Self::Source => "원문",
            Self::Story => "스토리",
            Self::Synopsis => "시놉시스",
        }
    }
    fn upstream(self) -> &'static [Kind] {
        match self {
            Self::Source => &[],
            Self::Story => &[Self::Source],
            Self::Synopsis => &[Self::Source, Self::Story],
        }
    }
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct Brief {
    #[serde(default)]
    pub audience: String,
    #[serde(default)]
    pub genre: String,
    #[serde(default)]
    pub tone: String,
    #[serde(default)]
    pub point_of_view: String,
    #[serde(default)]
    pub tense: String,
    #[serde(default)]
    pub voice_notes: String,
    #[serde(default)]
    pub preserve: Vec<String>,
    #[serde(default)]
    pub avoid: Vec<String>,
    #[serde(default)]
    pub ending: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct Block {
    pub id: String,
    pub heading: String,
    pub text: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct Document {
    pub kind: Kind,
    pub revision: u64,
    pub blocks: Vec<Block>,
    pub dependencies: BTreeMap<Kind, String>,
    pub guidance_hash: String,
    pub last_actor: String,
}

impl Document {
    pub fn markdown(&self) -> String {
        if self.kind == Kind::Source {
            return self
                .blocks
                .iter()
                .map(|b| b.text.as_str())
                .collect::<Vec<_>>()
                .join("\n\n");
        }
        self.blocks
            .iter()
            .map(|b| format!("## {}\n\n{}", b.heading, b.text))
            .collect::<Vec<_>>()
            .join("\n\n")
            + "\n"
    }
    pub fn content_hash(&self) -> String {
        digest(&self.blocks)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct Task {
    pub id: String,
    pub target: Kind,
    pub instruction: String,
    pub selection: Vec<String>,
    pub base: Option<Document>,
    pub inputs: BTreeMap<Kind, Document>,
    pub brief: Brief,
    pub guidance_hash: String,
    pub voice_guidelines: Vec<String>,
    pub cancelled: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct Draft {
    pub blocks: Vec<Block>,
    pub change_summary: String,
    pub assumptions: Vec<String>,
    pub open_questions: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct Candidate {
    pub id: String,
    pub task_id: String,
    pub draft: Draft,
    pub based_on: Option<String>,
    pub decision: Option<Decision>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct Decision {
    pub actor: String,
    pub accepted: bool,
    pub candidate_hash: String,
    pub note: String,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct State {
    pub brief: Brief,
    pub brief_revision: u64,
    pub documents: BTreeMap<Kind, Document>,
    pub tasks: BTreeMap<String, Task>,
    pub candidates: BTreeMap<String, Candidate>,
    pub requests: BTreeMap<String, String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Mutation {
    pub command_id: String,
    pub action: Action,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "action", rename_all = "snake_case", deny_unknown_fields)]
pub enum Action {
    Start {
        source: String,
        actor: String,
    },
    SetBrief {
        brief: Brief,
        expected_revision: u64,
    },
    Prepare {
        task_id: String,
        target: Kind,
        instruction: String,
        #[serde(default)]
        selection: Vec<String>,
    },
    Submit {
        candidate_id: String,
        task_id: String,
        draft: Draft,
        #[serde(default)]
        based_on: Option<String>,
    },
    Edit {
        kind: Kind,
        blocks: Vec<Block>,
        expected_revision: u64,
        actor: String,
    },
    Restore {
        document: Document,
        expected_revision: u64,
        actor: String,
    },
    Review {
        candidate_id: String,
        expected_candidate_hash: String,
        accept: bool,
        actor: String,
        #[serde(default)]
        note: String,
    },
    Cancel {
        task_id: String,
    },
}

pub fn digest<T: Serialize>(value: &T) -> String {
    crate::contracts::digest::serde_json_sha256(value).expect("authoring values are serializable")
}
