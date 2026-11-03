//! Producer shape/hash validation plus exact current scene/occurrence coverage.

use crate::codec::Value;
use crate::{
    project::{Project, Release},
    require, schemas, Error, Result,
};
use std::collections::{BTreeMap, BTreeSet};

pub(crate) fn validate_board(board: &Value, release: &Release) -> Result<()> {
    schemas::sealed("text-storyboard-v1", board)?;
    require(board["source"] == release.binding, "BOARD_SOURCE_CHANGED")?;
    let original_scenes = release.scene_map["scenes"]
        .as_array()
        .expect("source scenes admitted");
    let source_ids: Vec<_> = original_scenes
        .iter()
        .map(|scene| {
            scene["scene_id"]
                .as_str()
                .expect("source scene id admitted")
        })
        .collect();
    let semantics: BTreeMap<_, _> = release.scene_map["semantic_manifest"]["entries"]
        .as_array()
        .expect("source semantics admitted")
        .iter()
        .map(|entry| {
            (
                entry["semantic_id"].as_str().expect("semantic id admitted"),
                entry,
            )
        })
        .collect();
    let scenes = board["scenes"].as_array().expect("board schema admitted");
    let ids: Vec<_> = scenes
        .iter()
        .map(|scene| scene["sceneId"].as_str().expect("board scene id admitted"))
        .collect();
    let included: BTreeSet<_> = ids.iter().copied().collect();
    require(
        ids.len() == included.len() && included.iter().all(|id| source_ids.contains(id)),
        "BOARD_SCENE_IDENTITY",
    )?;
    require(
        ids == source_ids
            .into_iter()
            .filter(|id| included.contains(id))
            .collect::<Vec<_>>(),
        "BOARD_SCENE_ORDER",
    )?;
    let mut seen_panels = BTreeSet::new();
    for scene in scenes {
        let mut covered = BTreeSet::new();
        for panel in scene["panels"].as_array().expect("panels admitted") {
            require(
                seen_panels.insert(panel["panelId"].as_str().expect("panel id admitted")),
                "DUPLICATE_PANEL_ID",
            )?;
            for reference in panel["sourceRefs"]
                .as_array()
                .expect("source refs admitted")
            {
                let key = reference["semanticId"]
                    .as_str()
                    .expect("semantic id admitted");
                let entry = semantics
                    .get(key)
                    .ok_or_else(|| Error::refusal("PANEL_SOURCE_OUTSIDE_SCENE"))?;
                require(
                    entry["scene_id"] == scene["sceneId"],
                    "PANEL_SOURCE_OUTSIDE_SCENE",
                )?;
                require(
                    entry["entry_hash"] == reference["entryHash"],
                    "PANEL_SOURCE_HASH_CHANGED",
                )?;
                covered.insert(key);
            }
        }
        let missing: Vec<_> = semantics
            .iter()
            .filter(|(key, entry)| {
                entry["scene_id"] == scene["sceneId"] && !covered.contains(**key)
            })
            .map(|(key, _)| *key)
            .collect();
        require(
            missing.is_empty(),
            format!(
                "UNCOVERED_SCENE_OCCURRENCES:{}:{}",
                scene["sceneId"].as_str().expect("scene id admitted"),
                missing.join(",")
            ),
        )?;
    }
    Ok(())
}
pub(crate) fn current_approval(
    project: &Project,
    relative: &str,
    board: &Value,
    release: &Release,
) -> Result<Value> {
    // Cancellation preserves an existing review as evidence, but prevents future
    // exports, packages, and direction from consuming this task's candidate.
    super::task_live(
        project,
        board["taskHash"]
            .as_str()
            .expect("board task hash admitted"),
    )?;
    let receipt = project.artifact(relative, None)?;
    schemas::sealed("text-storyboard-review-v1", &receipt)?;
    require(
        receipt["source"] == release.binding
            && receipt["boardHash"] == board["contentHash"]
            && receipt["decision"] == "approved",
        "CURRENT_APPROVAL_REQUIRED",
    )?;
    Ok(receipt)
}
