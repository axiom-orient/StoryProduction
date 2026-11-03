use crate::ToolDefinition;
use serde_json::{json, Value};

pub fn tools() -> Vec<ToolDefinition> {
    let string = json!({"type":"string","minLength":1});
    let kind = json!({"enum":["source","story","synopsis"]});
    let revision = json!({"type":"integer","minimum":0});
    let draft = super::super::draft_schema();
    let block_list = draft["properties"]["blocks"].clone();
    let brief = json!({"type":"object","additionalProperties":false,"properties":{
        "audience":{"type":"string"},"genre":{"type":"string"},"tone":{"type":"string"},"point_of_view":{"type":"string"},
        "tense":{"type":"string"},"voice_notes":{"type":"string"},"ending":{"type":"string"},
        "preserve":{"type":"array","items":{"type":"string"}},"avoid":{"type":"array","items":{"type":"string"}}}});
    vec![
        tool("authoring.history","이전 집필본 읽기","List the last 20 document revisions, total/omitted count; supply revision to read an exact historical document.",&["project_id","kind"],json!({"kind":kind,"revision":revision})),
        tool("authoring.restore","이전 집필본 복원","Human editor: restore an exact historical document as a new revision with compare-and-set. Old source bindings remain stale when appropriate. CLI only.",&["project_id","command_id","kind","revision","expected_revision","actor"],json!({"kind":kind,"revision":revision,"expected_revision":revision,"actor":string})),
        tool("authoring.bundle","집필 문서 묶음 읽기","Read current source/story/synopsis from one saved project revision; no filesystem export or external effect.",&["project_id"],json!({})),
        tool("authoring.status","집필 이어하기","Read saved source/story/synopsis progress, stale dependencies, pending candidates and next action.",&["project_id"],json!({})),
        tool("authoring.document","작업물 읽기","Read the actual document and stale reasons, including source bytes.",&["project_id","kind"],json!({"kind":kind})),
        tool("authoring.task","집필 작업 읽기","Read a durable writing packet. No provider is called. Cancelled/stale tasks disclose adoption blockers.",&["project_id","id"],json!({"id":string})),
        tool("authoring.candidate","원문과 후보 비교","Read exact before/after blocks, order changes, assumptions, questions and candidate digest.",&["project_id","id"],json!({"id":string})),
        tool("authoring.export","집필 문서 내보내기","Return current adopted Markdown with its byte hash. Stale documents fail; no narrative release is implied.",&["project_id","kind"],json!({"kind":kind})),
        tool("authoring.start","원문으로 시작","Persist exact source text in the project journal; never generate replacement source.",&["project_id","command_id","source","actor"],json!({"source":string,"actor":string})),
        tool("authoring.brief","집필 방향 저장","Update audience, voice, POV, ending and protected details using brief revision. Dependent documents become stale.",&["project_id","command_id","brief","expected_revision"],json!({"brief":brief,"expected_revision":revision})),
        tool("authoring.prepare","집필 후보 준비","Save a source-bound story/synopsis task for an agent; selection restricts replacement to existing block ids. No external call. Optionally compare-and-set against the exact project revision under the workspace lock.",&["project_id","command_id","task_id","target","instruction"],json!({"task_id":string,"target":{"enum":["story","synopsis"]},"instruction":string,"selection":{"type":"array","items":{"type":"string"},"uniqueItems":true},"expected_project_revision":revision})),
        tool("authoring.submit","집필 후보 저장","Save an untrusted draft. For human edits submit a new candidate with based_on. Late/cancelled results are retained but cannot be adopted.",&["project_id","command_id","candidate_id","task_id","draft"],json!({"candidate_id":string,"task_id":string,"draft":draft,"based_on":string})),
        tool("authoring.cancel","집필 작업 중단","Prevent future adoption of this task; keep all drafts. Does not cancel an external model request.",&["project_id","command_id","task_id"],json!({"task_id":string})),
        tool("authoring.edit","문서 직접 수정","Human editor: replace the exact document revision; changes downstream freshness. Available through local CLI, not MCP.",&["project_id","command_id","kind","blocks","expected_revision","actor"],json!({"kind":kind,"blocks":block_list,"expected_revision":revision,"actor":string})),
        tool("authoring.review","후보 채택 또는 거절","Human review: bind decision to exact candidate hash, source and current guidance. Available through local CLI, not MCP.",&["project_id","command_id","candidate_id","expected_candidate_hash","accept","actor"],json!({"candidate_id":string,"expected_candidate_hash":string,"accept":{"type":"boolean"},"actor":string,"note":{"type":"string"}})),
        tool("authoring.handoff","시놉시스 STS 인계","Human action: export the current synopsis under an immutable handoff path. Does not approve STS scenes. Local CLI only.",&["project_id","actor","language","expected_revision","expected_markdown_sha256"],json!({"actor":string,"language":string,"expected_revision":revision,"expected_markdown_sha256":string})),
    ]
}

fn tool(
    name: &'static str,
    title: &'static str,
    description: &'static str,
    required: &[&str],
    mut properties: Value,
) -> ToolDefinition {
    properties["project_id"] = json!({"type":"string","minLength":1});
    if required.contains(&"command_id") {
        properties["command_id"] = json!({"type":"string","pattern":"^[A-Za-z0-9_.-]{1,64}$"});
    }
    ToolDefinition {
        name,
        title,
        description,
        input_schema: json!({"type":"object","additionalProperties":false,"required":required,"properties":properties}),
    }
}
