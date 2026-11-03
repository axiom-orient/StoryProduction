use std::collections::BTreeSet;

use crate::domain::validation::validate_id;
use crate::model::MAX_TEXT_BYTES;
use crate::{Error, Result};

use super::super::validation::{blocks_valid, text};
use super::super::{Candidate, Draft, State};

pub(super) fn submit(
    state: &mut State,
    candidate_id: &str,
    task_id: &str,
    draft: &Draft,
    based_on: Option<&str>,
) -> Result<()> {
    validate_id("candidate id", candidate_id)?;
    blocks_valid(&draft.blocks)?;
    text(&draft.change_summary, "change summary")?;
    if serde_json::to_vec(draft)?.len() > MAX_TEXT_BYTES {
        return Err(Error::InvalidInput("draft exceeds text limit".into()));
    }
    if state.candidates.contains_key(candidate_id) {
        return Err(Error::Conflict(
            "candidate id already exists; submit edits under a new id".into(),
        ));
    }
    let task = state
        .tasks
        .get(task_id)
        .ok_or_else(|| Error::NotFound("task not found".into()))?;
    if !task.selection.is_empty() {
        let actual: BTreeSet<_> = draft.blocks.iter().map(|b| &b.id).collect();
        let expected: BTreeSet<_> = task.selection.iter().collect();
        if actual != expected {
            return Err(Error::InvalidInput(
                "부분 수정은 선택된 블록만, 빠짐없이 제출해야 합니다".into(),
            ));
        }
    }
    if let Some(parent) = based_on {
        let original = state
            .candidates
            .get(parent)
            .ok_or_else(|| Error::NotFound("parent candidate not found".into()))?;
        if original.task_id != task_id {
            return Err(Error::Conflict("parent belongs to a different task".into()));
        }
    }
    // A late result is retained for comparison, even after source change/cancel.
    state.candidates.insert(
        candidate_id.to_owned(),
        Candidate {
            id: candidate_id.to_owned(),
            task_id: task_id.to_owned(),
            draft: draft.clone(),
            based_on: based_on.map(str::to_owned),
            decision: None,
        },
    );
    Ok(())
}
