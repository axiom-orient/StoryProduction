//! Exact NVC text projection; occurrence text always comes from the current release.

use crate::codec::Value;
use crate::{project::Release, require, Result, MAX_JSON_BYTES};
use std::collections::BTreeMap;

pub(super) fn render(board: &Value, release: &Release, label: &str) -> Result<String> {
    let scenes: BTreeMap<_, _> = release.scene_map["scenes"]
        .as_array()
        .expect("source scenes admitted")
        .iter()
        .map(|scene| {
            (
                scene["scene_id"].as_str().expect("scene id admitted"),
                scene,
            )
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
    let mut lines: Vec<String> = Vec::new();
    let mut byte_count = 0;
    let mut append = |values: Vec<String>| -> Result<()> {
        for line in values {
            byte_count += line.len() + usize::from(!lines.is_empty());
            require(
                byte_count <= MAX_JSON_BYTES,
                "STORYBOARD_PREVIEW_TOO_LARGE: select fewer scenes",
            )?;
            lines.push(line);
        }
        Ok(())
    };
    append(vec![
        "# 텍스트 스토리보드".into(),
        "".into(),
        format!("상태: {label}"),
        "".into(),
        "구도·동선·전환을 글로 작성한 콘티입니다.".into(),
        "".into(),
    ])?;
    for scene in board["scenes"].as_array().expect("board scenes admitted") {
        let id = scene["sceneId"].as_str().expect("scene id admitted");
        let original = scenes[id];
        append(vec![
            format!(
                "## {id} · {}",
                original["heading"].as_str().expect("heading admitted")
            ),
            "".into(),
            format!(
                "장면 의도: {}",
                scene["intent"].as_str().expect("intent admitted")
            ),
            "".into(),
        ])?;
        for (index, panel) in scene["panels"]
            .as_array()
            .expect("panels admitted")
            .iter()
            .enumerate()
        {
            append(vec![
                format!(
                    "### {}. {}",
                    index + 1,
                    panel["panelId"].as_str().expect("panel id admitted")
                ),
                "".into(),
                format!(
                    "**구도** {}",
                    panel["composition"].as_str().expect("composition admitted")
                ),
                "".into(),
                format!(
                    "**동선·시선** {}",
                    panel["staging"].as_str().expect("staging admitted")
                ),
                "".into(),
                "**원 장면의 행동·대사·지시**".into(),
                "".into(),
            ])?;
            for reference in panel["sourceRefs"].as_array().expect("refs admitted") {
                let entry = semantics[reference["semanticId"]
                    .as_str()
                    .expect("semantic id admitted")];
                let prefix = entry["source_locator"]["character"]
                    .as_str()
                    .filter(|speaker| !speaker.is_empty())
                    .map(|speaker| format!("{speaker}: "))
                    .unwrap_or_default();
                append(vec![format!(
                    "- [{}] {prefix}{}",
                    entry["kind"].as_str().expect("entry kind admitted"),
                    entry["source_text"].as_str().expect("source text admitted")
                )])?;
            }
            append(vec![
                "".into(),
                format!(
                    "**소리** {}",
                    panel["soundNotes"].as_str().expect("sound admitted")
                ),
                "".into(),
                format!(
                    "**연결** {}",
                    panel["transition"].as_str().expect("transition admitted")
                ),
                "".into(),
            ])?;
        }
        append(vec!["---".into(), "".into()])?;
    }
    for (key, heading) in [
        ("assumptions", "## 연출 가정"),
        ("openQuestions", "## 미결 질문"),
    ] {
        let values = board[key].as_array().expect("board list admitted");
        if !values.is_empty() {
            let mut section = vec![heading.into(), "".into()];
            section.extend(
                values
                    .iter()
                    .map(|item| format!("- {}", item.as_str().expect("list string admitted"))),
            );
            section.push("".into());
            append(section)?;
        }
    }
    Ok(lines.join("\n"))
}
