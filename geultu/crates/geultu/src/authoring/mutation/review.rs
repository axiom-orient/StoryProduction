use std::collections::BTreeMap;

use crate::model::Project;
use crate::{Error, Result};

use super::super::freshness::Freshness;
use super::super::validation::text;
use super::super::{digest, Decision, Document, State};

pub(super) fn review(
    project: &Project,
    state: &mut State,
    candidate_id: &str,
    expected_candidate_hash: &str,
    accept: bool,
    actor: &str,
    note: &str,
) -> Result<()> {
    text(actor, "actor")?;
    let candidate = state
        .candidates
        .get_mut(candidate_id)
        .ok_or_else(|| Error::NotFound("candidate not found".into()))?;
    if candidate.decision.is_some() {
        return Err(Error::Conflict("candidate already decided".into()));
    }
    if digest(&candidate.draft) != expected_candidate_hash {
        return Err(Error::Conflict("검토한 후보 내용과 해시가 다릅니다".into()));
    }
    let task = state
        .tasks
        .get(&candidate.task_id)
        .ok_or_else(|| Error::Corrupt("candidate task missing".into()))?;
    if accept {
        let mut freshness = Freshness::new(project);
        let reasons = freshness.task_reasons(task);
        if !reasons.is_empty() {
            return Err(Error::Conflict(reasons.join("; ")));
        }
        if !candidate.draft.open_questions.is_empty() {
            return Err(Error::Conflict(
                "미결 질문을 해결한 새 후보를 제출한 뒤 채택하세요".into(),
            ));
        }
        let blocks = if task.selection.is_empty() {
            candidate.draft.blocks.clone()
        } else {
            let current = state
                .documents
                .get(&task.target)
                .ok_or_else(|| Error::Conflict("target document missing".into()))?;
            let replacements: BTreeMap<_, _> = candidate
                .draft
                .blocks
                .iter()
                .map(|block| (block.id.as_str(), block))
                .collect();
            current
                .blocks
                .iter()
                .map(|block| {
                    replacements
                        .get(block.id.as_str())
                        .copied()
                        .unwrap_or(block)
                        .clone()
                })
                .collect()
        };
        let revision = state
            .documents
            .get(&task.target)
            .map(|d| d.revision + 1)
            .unwrap_or(1);
        let dependencies = freshness.dependencies(task.target)?;
        state.documents.insert(
            task.target,
            Document {
                kind: task.target,
                revision,
                blocks,
                dependencies,
                guidance_hash: task.guidance_hash.clone(),
                last_actor: actor.to_owned(),
            },
        );
    }
    candidate.decision = Some(Decision {
        actor: actor.to_owned(),
        accepted: accept,
        candidate_hash: expected_candidate_hash.to_owned(),
        note: note.to_owned(),
    });
    Ok(())
}
