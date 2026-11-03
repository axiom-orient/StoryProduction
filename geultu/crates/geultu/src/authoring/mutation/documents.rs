use std::collections::BTreeMap;

use crate::model::Project;
use crate::{Error, Result};

use super::super::freshness::Freshness;
use super::super::validation::{blocks_valid, text};
use super::super::{Block, Document, Kind, State};

pub(super) fn start(state: &mut State, source: &str, actor: &str) -> Result<()> {
    text(source, "source")?;
    text(actor, "actor")?;
    if state.documents.contains_key(&Kind::Source) {
        return Err(Error::Conflict(
            "원문이 이미 있습니다. authoring.edit를 사용하세요".into(),
        ));
    }
    state.documents.insert(
        Kind::Source,
        Document {
            kind: Kind::Source,
            revision: 1,
            blocks: vec![Block {
                id: "source".into(),
                heading: "원문".into(),
                text: source.to_owned(),
            }],
            dependencies: BTreeMap::new(),
            guidance_hash: String::new(),
            last_actor: actor.to_owned(),
        },
    );
    Ok(())
}

pub(super) fn edit(
    project: &Project,
    state: &mut State,
    kind: Kind,
    blocks: &[Block],
    expected_revision: u64,
    actor: &str,
) -> Result<()> {
    text(actor, "actor")?;
    blocks_valid(blocks)?;
    let current = state
        .documents
        .get(&kind)
        .ok_or_else(|| Error::NotFound("document not found".into()))?;
    if current.revision != expected_revision {
        return Err(Error::Conflict("문서 revision이 변경되었습니다".into()));
    }
    let mut freshness = Freshness::new(project);
    let deps = freshness.dependencies(kind)?;
    state.documents.insert(
        kind,
        Document {
            kind,
            revision: current.revision + 1,
            blocks: blocks.to_vec(),
            dependencies: deps,
            guidance_hash: freshness.guidance_hash(),
            last_actor: actor.to_owned(),
        },
    );
    Ok(())
}

pub(super) fn restore(
    state: &mut State,
    document: &Document,
    expected_revision: u64,
    actor: &str,
) -> Result<()> {
    text(actor, "actor")?;
    let current = state
        .documents
        .get(&document.kind)
        .ok_or_else(|| Error::NotFound("document not found".into()))?;
    if current.revision != expected_revision {
        return Err(Error::Conflict("복원 대상 문서가 변경되었습니다".into()));
    }
    let mut restored = document.clone();
    restored.revision = current.revision + 1;
    restored.last_actor = actor.to_owned();
    // Keep historical source/guidance bindings: restoring text is not a re-review.
    state.documents.insert(document.kind, restored);
    Ok(())
}
