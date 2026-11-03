use std::collections::{BTreeMap, BTreeSet};

use crate::domain::validation::validate_id;
use crate::model::{Project, MAX_TEXT_BYTES};
use crate::{Error, Result};

use super::super::freshness::Freshness;
use super::super::validation::text;
use super::super::{Brief, Kind, State, Task};

pub(super) fn set_brief(state: &mut State, brief: &Brief, expected_revision: u64) -> Result<()> {
    if state.brief_revision != expected_revision {
        return Err(Error::Conflict("브리프 revision이 변경되었습니다".into()));
    }
    if serde_json::to_vec(brief)?.len() > 64 * 1024 {
        return Err(Error::InvalidInput("brief exceeds 64 KiB".into()));
    }
    state.brief = brief.clone();
    state.brief_revision += 1;
    Ok(())
}

pub(super) fn prepare(
    project: &Project,
    state: &mut State,
    task_id: &str,
    target: Kind,
    instruction: &str,
    selection: &[String],
) -> Result<()> {
    validate_id("task id", task_id)?;
    text(instruction, "instruction")?;
    if target == Kind::Source {
        return Err(Error::InvalidInput("원문은 생성 대상이 아닙니다".into()));
    }
    if state.tasks.contains_key(task_id) {
        return Err(Error::Conflict("task id already exists".into()));
    }
    let mut freshness = Freshness::new(project);
    let deps = freshness.dependencies(target)?;
    let base = state.documents.get(&target);
    let base_ids: BTreeSet<_> = base
        .into_iter()
        .flat_map(|document| &document.blocks)
        .map(|block| block.id.as_str())
        .collect();
    let mut unique = BTreeSet::new();
    for id in selection {
        if !unique.insert(id) || !base_ids.contains(id.as_str()) {
            return Err(Error::InvalidInput(format!(
                "unknown or duplicate selected block: {id}"
            )));
        }
    }
    // Partial repair cannot make unrelated stale blocks current.
    if !selection.is_empty()
        && base.is_some_and(|document| !freshness.document_reasons(document).is_empty())
    {
        return Err(Error::Conflict(
            "상위 자료가 바뀐 문서는 전체 갱신이 필요합니다".into(),
        ));
    }
    let inputs = deps
        .keys()
        .map(|kind| {
            state
                .documents
                .get(kind)
                .cloned()
                .map(|document| (*kind, document))
                .ok_or_else(|| {
                    Error::Corrupt(
                        "validated authoring dependency is missing from task inputs".into(),
                    )
                })
        })
        .collect::<Result<BTreeMap<_, _>>>()?;
    let task = Task {
        id: task_id.to_owned(),
        target,
        instruction: instruction.to_owned(),
        selection: selection.to_vec(),
        base: base.cloned(),
        inputs,
        brief: state.brief.clone(),
        guidance_hash: freshness.guidance_hash(),
        voice_guidelines: project
            .voice_profile
            .as_ref()
            .map(|v| v.guidelines.clone())
            .unwrap_or_default(),
        cancelled: false,
    };
    if serde_json::to_vec(&task)?.len() > MAX_TEXT_BYTES {
        return Err(Error::InvalidInput(
            "task context exceeds 16 MiB; reduce source scope".into(),
        ));
    }
    state.tasks.insert(task_id.to_owned(), task);
    Ok(())
}

pub(super) fn cancel(state: &mut State, task_id: &str) -> Result<()> {
    let task = state
        .tasks
        .get_mut(task_id)
        .ok_or_else(|| Error::NotFound("task not found".into()))?;
    task.cancelled = true;
    Ok(())
}
