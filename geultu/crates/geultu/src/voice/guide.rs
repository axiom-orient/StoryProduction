//! Scene-bound, copyable instructions for external editors.

use super::profile::validate_profile;
use crate::contracts::digest::sha256_hex;
use crate::error::{Error, Result};
use crate::model::{Project, VoiceGuide};

/// Creates a copyable instruction for an external AI/editor. No rewrite is
/// generated here; the selected scene remains canonical until approval.
pub fn build_guide(project: &Project, scene_id: &str) -> Result<VoiceGuide> {
    let profile = project.voice_profile.as_ref().ok_or_else(|| {
        Error::NotFound("active voice profile was not found; create 글투 first".into())
    })?;
    validate_profile(profile)?;
    let scene = project
        .scenes
        .get(scene_id)
        .ok_or_else(|| Error::NotFound(format!("scene '{scene_id}' was not found")))?;

    let fact_lines: Vec<_> = project
        .facts
        .values()
        .filter(|fact| {
            fact.evidence.is_empty()
                || fact
                    .evidence
                    .iter()
                    .any(|evidence| evidence.scene_id == scene_id)
        })
        .map(|fact| format!("- {}: {} = {}", fact.subject, fact.predicate, fact.object))
        .collect();
    let facts = if fact_lines.is_empty() {
        "- 별도로 기록된 사실은 없습니다. 아래 장면 원문을 유일한 내용 기준으로 삼으세요.".into()
    } else {
        fact_lines.join("\n")
    };
    let guidelines = profile
        .guidelines
        .iter()
        .map(|guideline| format!("- {guideline}"))
        .collect::<Vec<_>>()
        .join("\n");
    let story_context = project
        .summary
        .as_deref()
        .map(|summary| {
            format!(
                "\n\n작품 맥락\n작품 제목: {}\n작가의 한 줄 소개: {}\n이 정보는 이야기 방향을 이해하기 위한 참고입니다. 장면 원문과 기록된 사실을 바꾸거나, 원문에 없는 사건·원인·감정을 추가하는 근거로 쓰지 마세요.",
                project.title, summary
            )
        })
        .unwrap_or_default();
    let source_scene_digest = sha256_hex(scene.text.as_bytes());
    let instruction = format!(
        concat!(
            "외부 AI/편집기용 작업 지시\n\n이 지시문은 외부 AI 또는 편집기에 붙여 넣어 사용할 작업 지시입니다. 글투 엔진은 이 자리에서 새 글을 생성하거나 원문을 자동으로 바꾸지 않습니다.\n\n",
            "목표\n선택한 장면을 아래에 적힌 관찰된 글투의 방향으로 다듬되, 원문이 가진 사실과 이름, 사건 순서, 관점, 의미를 그대로 보존하세요.\n\n",
            "관찰된 글투 요약\n{summary}\n{guidelines}{story_context}\n\n",
            "반드시 보존할 것\n- 장면의 사실, 인물 이름, 지명, 고유명사, 숫자, 관계와 사건 순서를 바꾸지 마세요.\n- 정보나 원인, 감정을 새로 만들어 내거나 삭제하지 마세요. 표현만 다듬고 의미를 바꾸지 마세요.\n- 확신할 수 없는 부분은 추측으로 채우지 말고 원문을 유지하세요.\n\n",
            "프로젝트에 기록된 관련 사실\n{facts}\n\n",
            "작업 대상 장면\n제목: {title}\n원문:\n{scene_text}\n\n",
            "결과 형식\n보존 규칙을 먼저 확인한 뒤, 장면의 의미와 사실을 유지한 수정안만 제시하세요. 변경한 표현이 글투에 맞는지 설명할 수 있어야 하며, 이 엔진의 정본에는 사용자가 명시적으로 승인하기 전까지 아무것도 반영하지 마세요."
        ),
        summary = profile.summary,
        guidelines = guidelines,
        facts = facts,
        story_context = story_context,
        title = scene.title,
        scene_text = scene.text,
    );
    Ok(VoiceGuide {
        project_id: project.id.clone(),
        scene_id: scene.id.clone(),
        scene_title: scene.title.clone(),
        source_project_revision: project.revision,
        source_content_revision: project.content_revision,
        source_scene_revision: scene.revision,
        source_scene_digest,
        external_editor_notice:
            "외부 AI/편집기에서 사용할 지시문입니다. 글투 엔진이 글을 대신 생성하지 않습니다."
                .into(),
        instruction,
    })
}
