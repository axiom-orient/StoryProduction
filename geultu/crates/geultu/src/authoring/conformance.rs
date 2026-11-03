//! Stored authoring graph and candidate-lineage consistency checks.

use super::validation::blocks_valid;
use super::{digest, Kind, State};
use crate::error::{Error, Result};
use std::collections::BTreeSet;

/// Called under the workspace lock, both before append and during event replay.
pub fn ensure_conformance(state: &State) -> Result<()> {
    for (kind, document) in &state.documents {
        if kind != &document.kind || document.revision == 0 {
            return Err(Error::Corrupt(
                "authoring document identity/revision mismatch".into(),
            ));
        }
        blocks_valid(&document.blocks)?;
        let expected: BTreeSet<_> = kind.upstream().iter().copied().collect();
        let actual: BTreeSet<_> = document.dependencies.keys().copied().collect();
        if expected != actual {
            return Err(Error::Corrupt("authoring dependency graph mismatch".into()));
        }
    }
    for (id, task) in &state.tasks {
        if id != &task.id
            || task.target == Kind::Source
            || task.inputs.keys().copied().collect::<BTreeSet<_>>()
                != task
                    .target
                    .upstream()
                    .iter()
                    .copied()
                    .collect::<BTreeSet<_>>()
        {
            return Err(Error::Corrupt(
                "authoring task identity/dependency mismatch".into(),
            ));
        }
    }
    for (id, candidate) in &state.candidates {
        if id != &candidate.id || !state.tasks.contains_key(&candidate.task_id) {
            return Err(Error::Corrupt(
                "authoring candidate identity/task mismatch".into(),
            ));
        }
        blocks_valid(&candidate.draft.blocks)?;
        if candidate
            .decision
            .as_ref()
            .is_some_and(|d| d.candidate_hash != digest(&candidate.draft))
        {
            return Err(Error::Corrupt(
                "authoring decision does not bind candidate bytes".into(),
            ));
        }
    }
    ensure_candidate_lineage(state)?;
    Ok(())
}

fn ensure_candidate_lineage(state: &State) -> Result<()> {
    let mut settled = BTreeSet::new();
    for (id, candidate) in &state.candidates {
        if settled.contains(id.as_str()) {
            continue;
        }

        let mut path = BTreeSet::new();
        let mut path_ids = Vec::new();
        let mut cursor = id.as_str();
        while !settled.contains(cursor) {
            if !path.insert(cursor) {
                return Err(Error::Corrupt(
                    "authoring candidate lineage contains a cycle".into(),
                ));
            }
            let current = state
                .candidates
                .get(cursor)
                .ok_or_else(|| Error::Corrupt("authoring candidate parent is missing".into()))?;
            if current.task_id != candidate.task_id {
                return Err(Error::Corrupt(
                    "authoring candidate lineage crosses tasks".into(),
                ));
            }
            path_ids.push(cursor);
            let Some(parent_id) = current.based_on.as_deref() else {
                break;
            };
            cursor = parent_id;
        }
        settled.extend(path_ids);
    }
    Ok(())
}
