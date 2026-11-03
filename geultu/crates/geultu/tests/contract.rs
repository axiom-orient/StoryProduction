mod support;

use geultu::domain::Command;
use geultu::generation::{
    GeneratedDraft, GenerationInput, GenerationPlan, GenerationProvider, WritingAdapter,
    WritingType, FICTION,
};
use geultu::model::{CandidateStatus, ChangeSetStatus, VisualPlan};
use geultu::{Application, Error, Workspace};
use serde_json::json;
use std::collections::BTreeSet;
use std::fs;
use support::unique_temp_id;

#[derive(Debug)]
struct FixtureWritingAdapter {
    writing_type: WritingType,
}

impl WritingAdapter for FixtureWritingAdapter {
    fn id(&self) -> &str {
        "fixture-fiction"
    }

    fn writing_type(&self) -> &WritingType {
        &self.writing_type
    }

    fn prepare(&self, input: &GenerationInput) -> geultu::Result<GenerationPlan> {
        Ok(GenerationPlan {
            project_id: input.request.project_id.clone(),
            writing_type: self.writing_type.clone(),
            stage: input
                .request
                .stage
                .clone()
                .unwrap_or_else(|| geultu::WritingStage::new("draft").unwrap()),
            adapter_id: self.id().into(),
            source_content_revision: input.context.source_content_revision,
            prompt: format!("fixture: {}", input.request.brief),
            output_format: geultu::generation::OutputFormat::Markdown,
            input_hash: "fixture-hash".into(),
            craftpack: None,
        })
    }
}

#[derive(Debug)]
struct FixtureProvider;

impl GenerationProvider for FixtureProvider {
    fn id(&self) -> &str {
        "fixture"
    }

    fn generate(&self, plan: &GenerationPlan) -> geultu::Result<GeneratedDraft> {
        Ok(GeneratedDraft {
            content: format!("draft for {}", plan.writing_type.as_str()),
            model: "fixture-model".into(),
            request_id: Some("fixture-request".into()),
        })
    }
}

fn fixture(label: &str) -> (std::path::PathBuf, Application) {
    let root = std::env::temp_dir().join(format!(
        "geultu-contract-{label}-{}-{}",
        std::process::id(),
        unique_temp_id()
    ));
    let _ = fs::remove_dir_all(&root);
    let workspace = Workspace::init(&root).unwrap();
    (root, Application::from_workspace(workspace))
}

fn candidate_subject_claims(app: &Application, candidate_id: &str) -> (u64, String) {
    let subject = app
        .call(
            "candidate.subject",
            json!({"project_id":"book","candidate_id":candidate_id}),
        )
        .unwrap();
    (
        subject["revision"].as_u64().unwrap(),
        subject["digest"].as_str().unwrap().to_string(),
    )
}

#[test]
fn tool_names_are_unique() {
    let (root, app) = fixture("tools");
    let tools = app.tool_definitions();
    let unique: BTreeSet<_> = tools.iter().map(|tool| tool.name).collect();
    assert_eq!(tools.len(), unique.len());
    assert!(unique.contains("scene.set_text"));
    assert!(unique.contains("candidate.subject"));
    assert!(unique.contains("scene.prepare_candidate"));
    assert!(unique.contains("changeset.approve"));
    assert!(unique.contains("proof.verify"));
    let candidate_subject = tools
        .iter()
        .find(|tool| tool.name == "candidate.subject")
        .unwrap();
    for required in ["project_id", "candidate_id"] {
        assert!(candidate_subject.input_schema["required"]
            .as_array()
            .unwrap()
            .contains(&json!(required)));
    }
    let prepare_candidate = tools
        .iter()
        .find(|tool| tool.name == "scene.prepare_candidate")
        .unwrap();
    for required in [
        "project_id",
        "scene_id",
        "expected_content_revision",
        "expected_scene_revision",
        "candidate_id",
        "changeset_id",
        "kind",
        "content",
        "provider",
        "model",
    ] {
        assert!(prepare_candidate.input_schema["required"]
            .as_array()
            .unwrap()
            .contains(&json!(required)));
    }
    assert!(!geultu::application::is_human_only_tool(
        "scene.prepare_candidate"
    ));
    for name in ["candidate.reject", "changeset.approve", "changeset.reject"] {
        let decision_tool = tools.iter().find(|tool| tool.name == name).unwrap();
        for required in [
            "actor_id",
            "expected_artifact_revision",
            "expected_artifact_digest",
        ] {
            assert!(decision_tool.input_schema["required"]
                .as_array()
                .unwrap()
                .contains(&json!(required)));
        }
    }
    let _ = fs::remove_dir_all(root);
}

#[test]
fn scene_prepare_candidate_is_atomic_and_replay_payload_bound() {
    let (root, app) = fixture("scene-prepare-candidate");
    app.call(
        "project.create",
        json!({"project_id":"book","title":"Book","command_id":"prepare-project"}),
    )
    .unwrap();
    app.call(
        "scene.add",
        json!({"project_id":"book","scene_id":"scene_1","title":"Scene","text":"저장된 원문","command_id":"prepare-scene"}),
    )
    .unwrap();

    let request = json!({
        "project_id":"book",
        "scene_id":"scene_1",
        "expected_content_revision":1,
        "expected_scene_revision":0,
        "candidate_id":"prepared-candidate",
        "changeset_id":"prepared-changeset",
        "kind":"manual_rewrite",
        "content":"사람이 확인할 수정안",
        "provider":"external_editor",
        "model":"user_provided",
        "request_id":"manual-request",
        "input_hash":"source-hash",
        "command_id":"prepare-pair"
    });
    let first = app
        .call("scene.prepare_candidate", request.clone())
        .unwrap();
    assert_eq!(first["project_id"], "book");
    assert_eq!(first["event_count"], 2);
    assert_eq!(first["idempotent"], false);

    let project = app
        .call("project.get", json!({"project_id":"book"}))
        .unwrap();
    let candidate = &project["candidates"]["prepared-candidate"];
    let changeset = &project["changesets"]["prepared-changeset"];
    assert_eq!(candidate["source_revision"], 1);
    assert_eq!(candidate["status"], "pending");
    assert_eq!(candidate["content"], "사람이 확인할 수정안");
    assert_eq!(candidate["receipt"]["provider"], "external_editor");
    assert_eq!(candidate["receipt"]["model"], "user_provided");
    assert_eq!(candidate["receipt"]["request_id"], "manual-request");
    assert_eq!(candidate["receipt"]["input_hash"], "source-hash");
    assert_eq!(changeset["candidate_id"], "prepared-candidate");
    assert_eq!(changeset["source_revision"], 1);
    assert_eq!(changeset["status"], "pending");
    assert_eq!(changeset["operations"].as_array().unwrap().len(), 1);
    assert_eq!(changeset["operations"][0]["type"], "replace_scene_text");
    assert_eq!(changeset["operations"][0]["scene_id"], "scene_1");
    assert_eq!(changeset["operations"][0]["expected_scene_revision"], 0);
    assert_eq!(changeset["operations"][0]["text"], "사람이 확인할 수정안");
    assert_eq!(project["scenes"]["scene_1"]["text"], "저장된 원문");
    assert_eq!(project["scenes"]["scene_1"]["revision"], 0);

    let journal_path = root.join(".geultu/projects/book/events.ndjson");
    let committed_journal = fs::read_to_string(&journal_path).unwrap();
    let matching: Vec<_> = committed_journal
        .lines()
        .map(|line| serde_json::from_str::<serde_json::Value>(line).unwrap())
        .filter(|event| event["command_id"] == "prepare-pair")
        .collect();
    assert_eq!(matching.len(), 2);
    assert_eq!(matching[0]["event"]["type"], "candidate_added");
    assert_eq!(matching[1]["event"]["type"], "change_set_added");

    let replay = app
        .call("scene.prepare_candidate", request.clone())
        .unwrap();
    assert_eq!(replay["idempotent"], true);
    assert_eq!(replay["event_count"], 0);
    assert_eq!(
        fs::read_to_string(&journal_path).unwrap(),
        committed_journal
    );

    for (field, value) in [
        ("candidate_id", json!("other-candidate")),
        ("changeset_id", json!("other-changeset")),
        ("kind", json!("different-kind")),
        ("content", json!("different proposal")),
        ("provider", json!("different-provider")),
        ("model", json!("different-model")),
        ("request_id", json!("different-request")),
        ("input_hash", json!("different-hash")),
        ("expected_content_revision", json!(0)),
        ("expected_scene_revision", json!(1)),
    ] {
        let mut conflicting = request.clone();
        conflicting[field] = value;
        let error = app
            .call("scene.prepare_candidate", conflicting)
            .unwrap_err();
        assert!(
            matches!(&error, Error::Conflict(message) if message.contains("different payload")),
            "field {field} did not bind replay payload: {error:?}"
        );
        assert_eq!(
            fs::read_to_string(&journal_path).unwrap(),
            committed_journal
        );
    }
    let _ = fs::remove_dir_all(root);
}

#[test]
fn scene_prepare_candidate_rejects_stale_content_or_scene_without_partial_events() {
    let (root, app) = fixture("scene-prepare-stale");
    app.call(
        "project.create",
        json!({"project_id":"book","title":"Book","command_id":"prepare-project"}),
    )
    .unwrap();
    app.call(
        "scene.add",
        json!({"project_id":"book","scene_id":"scene_1","title":"Scene","text":"원문","command_id":"prepare-scene"}),
    )
    .unwrap();

    let stale_content_request = json!({
        "project_id":"book","scene_id":"scene_1",
        "expected_content_revision":1,"expected_scene_revision":0,
        "candidate_id":"stale-content","changeset_id":"stale-content-change",
        "kind":"rewrite","content":"proposal","provider":"external_editor","model":"user_provided",
        "command_id":"stale-content-prepare"
    });
    app.call(
        "scene.set_text",
        json!({"project_id":"book","scene_id":"scene_1","text":"다른 곳에서 저장한 원고","expected_scene_revision":0,"command_id":"owner-drift"}),
    )
    .unwrap();
    let journal_path = root.join(".geultu/projects/book/events.ndjson");
    let after_drift = fs::read_to_string(&journal_path).unwrap();
    let content_error = app
        .call("scene.prepare_candidate", stale_content_request)
        .unwrap_err();
    assert!(
        matches!(content_error, Error::Conflict(message) if message.contains("content revision"))
    );
    assert_eq!(fs::read_to_string(&journal_path).unwrap(), after_drift);

    let stale_scene_request = json!({
        "project_id":"book","scene_id":"scene_1",
        "expected_content_revision":2,"expected_scene_revision":0,
        "candidate_id":"stale-scene","changeset_id":"stale-scene-change",
        "kind":"rewrite","content":"another proposal","provider":"external_editor","model":"user_provided",
        "command_id":"stale-scene-prepare"
    });
    let stale_scene_error = app
        .call("scene.prepare_candidate", stale_scene_request)
        .unwrap_err();
    assert!(
        matches!(stale_scene_error, Error::Conflict(message) if message.contains("revision mismatch"))
    );
    assert_eq!(fs::read_to_string(&journal_path).unwrap(), after_drift);
    let project = app
        .call("project.get", json!({"project_id":"book"}))
        .unwrap();
    assert!(!project["candidates"]
        .as_object()
        .unwrap()
        .contains_key("stale-content"));
    assert!(!project["candidates"]
        .as_object()
        .unwrap()
        .contains_key("stale-scene"));
    assert!(project["changesets"].as_object().unwrap().is_empty());
    let _ = fs::remove_dir_all(root);
}

#[test]
fn generation_is_type_and_provider_extensible_without_persisting_draft() {
    let (root, workspace_app) = fixture("generation");
    let mut registry = geultu::GenerationRegistry::default();
    registry.add_writing_adapter(FixtureWritingAdapter {
        writing_type: WritingType::new(FICTION).unwrap(),
    });
    registry.add_provider(FixtureProvider);
    let app = workspace_app.with_generation_registry(registry);
    app.call(
        "project.create",
        json!({"project_id":"book","title":"Book","command_id":"one"}),
    )
    .unwrap();

    let types = app.call("writing.types.list", json!({})).unwrap();
    assert_eq!(types["writing_types"], json!([FICTION]));
    assert_eq!(types["providers"], json!(["fixture"]));

    let prepared = app
        .call(
            "generation.prepare",
            json!({
                "project_id":"book",
                "writing_type":FICTION,
                "brief":"Write a complete short story",
                "expected_content_revision":0
            }),
        )
        .unwrap();
    assert_eq!(prepared["writing_type"], FICTION);
    assert_eq!(prepared["source_content_revision"], 0);

    let generated = app
        .call(
            "generation.generate",
            json!({
                "project_id":"book",
                "writing_type":FICTION,
                "brief":"Write a complete short story",
                "provider_id":"fixture"
            }),
        )
        .unwrap();
    assert_eq!(generated["draft"]["content"], "draft for fiction");
    assert_eq!(generated["candidate"]["kind"], "fiction:draft");
    assert_eq!(generated["candidate"]["writing_type"], FICTION);
    assert_eq!(generated["candidate"]["stage"], "draft");
    assert_eq!(
        app.call("project.get", json!({"project_id":"book"}))
            .unwrap()["revision"],
        1
    );
    let _ = fs::remove_dir_all(root);
}

#[test]
fn candidate_metadata_does_not_advance_content_revision() {
    let (root, app) = fixture("content-revision");
    app.call(
        "project.create",
        json!({"project_id":"book","title":"Book","command_id":"one"}),
    )
    .unwrap();
    app.call("scene.add", json!({"project_id":"book","scene_id":"scene_1","title":"Scene","text":"original","command_id":"two"})).unwrap();
    let before = app
        .call("project.get", json!({"project_id":"book"}))
        .unwrap();
    app.call("candidate.create", json!({"project_id":"book","candidate_id":"candidate","kind":"rewrite","content":"proposal","provider":"fixture","model":"fixture","command_id":"three"})).unwrap();
    let after = app
        .call("project.get", json!({"project_id":"book"}))
        .unwrap();
    assert!(after["revision"].as_u64().unwrap() > before["revision"].as_u64().unwrap());
    assert_eq!(after["content_revision"], before["content_revision"]);
    assert_eq!(after["scenes"]["scene_1"]["text"], "original");
    assert_eq!(
        after["candidates"]["candidate"]["status"],
        serde_json::to_value(CandidateStatus::Pending).unwrap()
    );
    let _ = fs::remove_dir_all(root);
}

#[test]
fn stale_scene_revision_rejects_approval_without_partial_mutation() {
    let (root, app) = fixture("stale");
    app.call(
        "project.create",
        json!({"project_id":"book","title":"Book","command_id":"one"}),
    )
    .unwrap();
    app.call("scene.add", json!({"project_id":"book","scene_id":"scene_1","title":"Scene","text":"v0","command_id":"two"})).unwrap();
    app.call("candidate.create", json!({"project_id":"book","candidate_id":"candidate","kind":"rewrite","content":"candidate","provider":"fixture","model":"fixture","command_id":"three"})).unwrap();
    app.call("changeset.create", json!({"project_id":"book","candidate_id":"candidate","changeset_id":"change","operations":[{"type":"replace_scene_text","scene_id":"scene_1","expected_scene_revision":0,"text":"candidate"}],"command_id":"four"})).unwrap();
    app.call("scene.set_text", json!({"project_id":"book","scene_id":"scene_1","text":"manual","expected_scene_revision":0,"command_id":"five"})).unwrap();
    let project = app
        .call("project.get", json!({"project_id":"book"}))
        .unwrap();
    let (artifact_revision, artifact_digest) = candidate_subject_claims(&app, "candidate");
    let error = app
        .call(
            "changeset.approve",
            json!({
                "project_id":"book",
                "changeset_id":"change",
                "expected_project_revision":project["revision"],
                "actor_id":"editor",
                "expected_artifact_revision":artifact_revision,
                "expected_artifact_digest":artifact_digest,
                "command_id":"six"
            }),
        )
        .unwrap_err();
    assert!(matches!(error, Error::Conflict(_)));
    let after = app
        .call("project.get", json!({"project_id":"book"}))
        .unwrap();
    assert_eq!(after["scenes"]["scene_1"]["text"], "manual");
    assert_eq!(
        after["changesets"]["change"]["status"],
        serde_json::to_value(ChangeSetStatus::Pending).unwrap()
    );
    let _ = fs::remove_dir_all(root);
}

#[test]
fn visual_plan_uses_content_revision_for_staleness() {
    let (root, app) = fixture("visual-revision");
    app.call(
        "project.create",
        json!({"project_id":"book","title":"Book","command_id":"one"}),
    )
    .unwrap();
    app.call(
        "scene.add",
        json!({"project_id":"book","scene_id":"scene_1","title":"Scene","text":"Opening","command_id":"two"}),
    )
    .unwrap();
    app.call(
        "visual.plan",
        json!({"project_id":"book","scene_id":"scene_1","plan_id":"plan","command_id":"three"}),
    )
    .unwrap();

    let plans = app
        .call("visual.list", json!({"project_id":"book"}))
        .unwrap();
    assert_eq!(plans[0]["source_revision"], 1);
    let review = app
        .call("review.run", json!({"project_id":"book"}))
        .unwrap();
    assert!(!review["findings"]
        .as_array()
        .unwrap()
        .iter()
        .any(|finding| finding["code"] == "visual.stale"));
    let _ = fs::remove_dir_all(root);
}

#[test]
fn stale_visual_plan_is_rejected_before_commit() {
    let (root, app) = fixture("visual-conflict");
    app.call(
        "project.create",
        json!({"project_id":"book","title":"Book","command_id":"one"}),
    )
    .unwrap();
    app.call(
        "scene.add",
        json!({"project_id":"book","scene_id":"scene_1","title":"Scene","command_id":"two"}),
    )
    .unwrap();
    let error = app
        .workspace()
        .transact(
            "book",
            "three",
            Command::AddVisualPlan {
                plan: VisualPlan {
                    id: "stale-plan".into(),
                    scene_id: "scene_1".into(),
                    source_revision: 0,
                    shots: Vec::new(),
                },
            },
        )
        .unwrap_err();
    assert!(
        matches!(error, Error::Conflict(message) if message.contains("source revision mismatch"))
    );
    let plans = app
        .call("visual.list", json!({"project_id":"book"}))
        .unwrap();
    assert!(plans.as_array().unwrap().is_empty());
    let _ = fs::remove_dir_all(root);
}

#[test]
fn changeset_operations_use_direct_command_validation() {
    let (root, app) = fixture("changeset-validation");
    app.call(
        "project.create",
        json!({"project_id":"book","title":"Book","command_id":"one"}),
    )
    .unwrap();
    app.call(
        "scene.add",
        json!({"project_id":"book","scene_id":"scene_1","title":"Scene","command_id":"two"}),
    )
    .unwrap();
    app.call(
        "candidate.create",
        json!({"project_id":"book","candidate_id":"candidate","kind":"rewrite","content":"proposal","provider":"fixture","model":"fixture","command_id":"three"}),
    )
    .unwrap();

    let error = app
        .call(
            "changeset.create",
            json!({"project_id":"book","candidate_id":"candidate","changeset_id":"invalid","operations":[{"type":"add_fact","fact":{"id":"fact-1","subject":"","predicate":"name","object":"value"}}],"command_id":"four"}),
        )
        .unwrap_err();
    assert!(
        matches!(error, Error::InvalidInput(message) if message == "fact subject must not be empty")
    );

    let error = app
        .call(
            "changeset.create",
            json!({"project_id":"book","candidate_id":"candidate","changeset_id":"duplicate","operations":[{"type":"add_fact","fact":{"id":"fact-1","subject":"hero","predicate":"name","object":"one"}},{"type":"add_fact","fact":{"id":"fact-1","subject":"hero","predicate":"name","object":"two"}}],"command_id":"five"}),
        )
        .unwrap_err();
    assert!(
        matches!(error, Error::Conflict(message) if message.contains("appears more than once"))
    );

    let project = app
        .call("project.get", json!({"project_id":"book"}))
        .unwrap();
    assert_eq!(project["revision"], 3);
    assert!(project["facts"].as_object().unwrap().is_empty());
    assert!(project["changesets"].as_object().unwrap().is_empty());
    let _ = fs::remove_dir_all(root);
}

#[test]
fn active_project_lock_fails_explicitly() {
    let (root, app) = fixture("lock");
    app.call(
        "project.create",
        json!({"project_id":"book","title":"Book","command_id":"one"}),
    )
    .unwrap();
    let lock = root.join(".geultu/projects/book/lock");
    fs::write(&lock, "pid=fixture").unwrap();
    let error = app
        .call(
            "scene.add",
            json!({"project_id":"book","scene_id":"scene_1","title":"Scene","command_id":"two"}),
        )
        .unwrap_err();
    assert!(matches!(error, Error::Busy(_)));
    fs::remove_file(lock).unwrap();
    let _ = fs::remove_dir_all(root);
}

#[test]
fn voice_profile_create_get_and_raw_samples_are_not_persisted() {
    let (root, app) = fixture("voice-persistence");
    app.call(
        "project.create",
        json!({"project_id":"book","title":"Book","command_id":"one"}),
    )
    .unwrap();
    let sample_one = "UNIQUE_SAMPLE_RAIN_7f3a. 비가 내렸다. 나는 창가에 앉았다.";
    let sample_two = "UNIQUE_SAMPLE_TRAIN_91bc. 기차가 떠났다. 나는 오래 손을 흔들었다.";
    let created = app
        .call(
            "voice.profile.create",
            json!({
                "project_id":"book",
                "profile_id":"voice-main",
                "expected_project_revision":1,
                "samples":[
                    {"id":"sample-a","text":sample_one},
                    {"id":"sample-b","text":sample_two}
                ],
                "command_id":"voice-create"
            }),
        )
        .unwrap();
    assert_eq!(created["revision"], 2);
    assert_eq!(created["idempotent"], false);

    let profile = app
        .call("voice.profile.get", json!({"project_id":"book"}))
        .unwrap();
    assert_eq!(profile["profile_id"], "voice-main");
    assert_eq!(profile["sample_count"], 2);
    assert!(profile["summary"].as_str().unwrap().contains("샘플"));
    assert!(!profile["guidelines"].as_array().unwrap().is_empty());

    let project_json = serde_json::to_string(
        &app.call("project.get", json!({"project_id":"book"}))
            .unwrap(),
    )
    .unwrap();
    let journal = fs::read_to_string(root.join(".geultu/projects/book/events.ndjson")).unwrap();
    let snapshot = fs::read_to_string(root.join(".geultu/projects/book/snapshot.json")).unwrap();
    for serialized in [&project_json, &journal, &snapshot] {
        assert!(!serialized.contains(sample_one));
        assert!(!serialized.contains(sample_two));
        assert!(!serialized.contains("UNIQUE_SAMPLE_"));
    }

    let mut without_profile = app
        .call("project.get", json!({"project_id":"book"}))
        .unwrap();
    without_profile
        .as_object_mut()
        .unwrap()
        .remove("voice_profile");
    let project_without_profile: geultu::model::Project =
        serde_json::from_value(without_profile).unwrap();
    assert_eq!(
        project_without_profile.schema_version,
        geultu::model::SCHEMA_VERSION
    );
    assert!(project_without_profile.voice_profile.is_none());
    let _ = fs::remove_dir_all(root);
}

#[test]
fn voice_profile_bounds_and_empty_samples_leave_state_unchanged() {
    let (root, app) = fixture("voice-bounds");
    app.call(
        "project.create",
        json!({"project_id":"book","title":"Book","command_id":"one"}),
    )
    .unwrap();
    let before = app
        .call("project.get", json!({"project_id":"book"}))
        .unwrap();

    let error = app
        .call(
            "voice.profile.create",
            json!({
                "project_id":"book",
                "expected_project_revision":1,
                "samples":[],
                "command_id":"voice-empty"
            }),
        )
        .unwrap_err();
    assert!(matches!(error, Error::InvalidInput(message) if message.contains("1..=32")));
    assert_eq!(
        app.call("project.get", json!({"project_id":"book"}))
            .unwrap(),
        before
    );

    let oversized = "x".repeat(2 * 1024 * 1024 + 1);
    let error = app
        .call(
            "voice.profile.create",
            json!({
                "project_id":"book",
                "expected_project_revision":1,
                "samples":[{"id":"too-large","text":oversized}],
                "command_id":"voice-oversized"
            }),
        )
        .unwrap_err();
    assert!(matches!(error, Error::InvalidInput(message) if message.contains("exceeds")));
    assert_eq!(
        app.call("project.get", json!({"project_id":"book"}))
            .unwrap(),
        before
    );

    let mut over_limit = Vec::with_capacity(11);
    for index in 0..10 {
        over_limit.push(json!({
            "id": format!("corpus-{index}"),
            "text": "x".repeat(2 * 1024 * 1024)
        }));
    }
    over_limit.push(json!({"id":"corpus-extra","text":"x"}));
    let error = app
        .call(
            "voice.profile.create",
            json!({
                "project_id":"book",
                "expected_project_revision":1,
                "samples":over_limit,
                "command_id":"voice-corpus-over-limit"
            }),
        )
        .unwrap_err();
    assert!(matches!(error, Error::InvalidInput(message) if message.contains("corpus exceeds")));
    assert_eq!(
        app.call("project.get", json!({"project_id":"book"}))
            .unwrap(),
        before
    );
    let _ = fs::remove_dir_all(root);
}

#[test]
fn stale_voice_activation_does_not_mutate_and_retry_is_idempotent() {
    let (root, app) = fixture("voice-stale");
    app.call(
        "project.create",
        json!({"project_id":"book","title":"Book","command_id":"one"}),
    )
    .unwrap();
    app.call(
        "scene.add",
        json!({"project_id":"book","scene_id":"scene_1","title":"Scene","text":"본문","command_id":"two"}),
    )
    .unwrap();
    let before = app
        .call("project.get", json!({"project_id":"book"}))
        .unwrap();
    let journal_before =
        fs::read_to_string(root.join(".geultu/projects/book/events.ndjson")).unwrap();

    let error = app
        .call(
            "voice.profile.create",
            json!({
                "project_id":"book",
                "profile_id":"voice-main",
                "expected_project_revision":1,
                "samples":["비가 내렸다."],
                "command_id":"voice-stale"
            }),
        )
        .unwrap_err();
    assert!(matches!(error, Error::Conflict(message) if message.contains("revision mismatch")));
    assert_eq!(
        app.call("project.get", json!({"project_id":"book"}))
            .unwrap(),
        before
    );
    assert_eq!(
        fs::read_to_string(root.join(".geultu/projects/book/events.ndjson")).unwrap(),
        journal_before
    );

    let created = app
        .call(
            "voice.profile.create",
            json!({
                "project_id":"book",
                "profile_id":"voice-main",
                "expected_project_revision":2,
                "samples":["비가 내렸다. 나는 걸었다."],
                "command_id":"voice-once"
            }),
        )
        .unwrap();
    let profile_after_create = app
        .call("voice.profile.get", json!({"project_id":"book"}))
        .unwrap();
    let retried = app
        .call(
            "voice.profile.create",
            json!({
                "project_id":"book",
                "profile_id":"different-id",
                "expected_project_revision":999,
                "samples":["다른 원문."],
                "command_id":"voice-once"
            }),
        )
        .unwrap();
    assert_eq!(retried["idempotent"], true);
    assert_eq!(retried["revision"], created["revision"]);
    assert_eq!(
        app.call("voice.profile.get", json!({"project_id":"book"}))
            .unwrap(),
        profile_after_create
    );
    let _ = fs::remove_dir_all(root);
}

#[test]
fn voice_compare_distinguishes_near_and_different_text() {
    let (root, app) = fixture("voice-compare");
    app.call(
        "project.create",
        json!({"project_id":"book","title":"Book","command_id":"one"}),
    )
    .unwrap();
    app.call(
        "voice.profile.create",
        json!({
            "project_id":"book",
            "profile_id":"voice-main",
            "expected_project_revision":1,
            "samples":[
                "“비가 조용히 내렸다.”\n“나는 창가에 앉아 오래 바라보았다.”",
                "밤이 깊었다. 방 안에는 작은 숨소리만 남았다."
            ],
            "command_id":"voice-create"
        }),
    )
    .unwrap();
    let near = app
        .call(
            "voice.compare",
            json!({"project_id":"book","candidate_text":"비가 천천히 내렸다. 나는 창가에 앉아 잠시 바라보았다."}),
        )
        .unwrap();
    let different = app
        .call(
            "voice.compare",
            json!({"project_id":"book","candidate_text":"THE QUICK BROWN FOX JUMPS OVER THE LAZY DOG 123!!!"}),
        )
        .unwrap();
    assert!(near["score"].as_u64().unwrap() > different["score"].as_u64().unwrap());
    assert!(near["label"].as_str().is_some());
    assert!(!different["tips"].as_array().unwrap().is_empty());
    let _ = fs::remove_dir_all(root);
}

#[test]
fn voice_guide_contains_scene_and_external_editor_preservation_rules() {
    let (root, app) = fixture("voice-guide");
    app.call(
        "project.create",
        json!({"project_id":"book","title":"Book","command_id":"one"}),
    )
    .unwrap();
    app.call(
        "scene.add",
        json!({"project_id":"book","scene_id":"scene_1","title":"비 오는 역","text":"민서가 서울역에서 편지를 접었다.","command_id":"two"}),
    )
    .unwrap();
    app.call(
        "fact.add",
        json!({
            "project_id":"book",
            "fact_id":"fact-1",
            "subject":"민서",
            "predicate":"장소",
            "object":"서울역",
            "evidence":[{"scene_id":"scene_1"}],
            "command_id":"three"
        }),
    )
    .unwrap();
    app.call(
        "voice.profile.create",
        json!({
            "project_id":"book",
            "profile_id":"voice-main",
            "expected_project_revision":3,
            "samples":["비가 내렸다. 민서는 편지를 접었다."],
            "command_id":"voice-create"
        }),
    )
    .unwrap();
    let guide = app
        .call(
            "voice.guide",
            json!({"project_id":"book","scene_id":"scene_1"}),
        )
        .unwrap();
    let instruction = guide["instruction"].as_str().unwrap();
    for phrase in [
        "비 오는 역",
        "민서가 서울역에서 편지를 접었다.",
        "외부 AI",
        "사실",
        "이름",
        "의미",
    ] {
        assert!(
            instruction.contains(phrase),
            "missing guide phrase: {phrase}"
        );
    }
    assert!(guide["external_editor_notice"]
        .as_str()
        .unwrap()
        .contains("생성하지"));
    assert_eq!(guide["source_project_revision"], 4);
    assert_eq!(guide["source_content_revision"], 2);
    assert_eq!(guide["source_scene_revision"], 0);
    assert_eq!(guide["source_scene_digest"].as_str().unwrap().len(), 64);
    let _ = fs::remove_dir_all(root);
}

#[test]
fn stale_voice_guide_after_same_scene_change_cannot_approve_old_candidate() {
    let (root, app) = fixture("voice-guide-stale-scene");
    app.call(
        "project.create",
        json!({"project_id":"book","title":"Book","command_id":"one"}),
    )
    .unwrap();
    app.call(
        "scene.add",
        json!({"project_id":"book","scene_id":"scene_1","title":"Scene","text":"원문","command_id":"two"}),
    )
    .unwrap();
    app.call(
        "voice.profile.create",
        json!({
            "project_id":"book",
            "profile_id":"voice-main",
            "expected_project_revision":2,
            "samples":["원문을 조용히 적었다."],
            "command_id":"voice-create"
        }),
    )
    .unwrap();
    let guide = app
        .call(
            "voice.guide",
            json!({"project_id":"book","scene_id":"scene_1"}),
        )
        .unwrap();
    assert_eq!(guide["source_project_revision"], 3);
    assert_eq!(guide["source_content_revision"], 1);
    assert_eq!(guide["source_scene_revision"], 0);

    app.call(
        "scene.set_text",
        json!({"project_id":"book","scene_id":"scene_1","text":"사용자가 먼저 고친 원고","expected_scene_revision":0,"command_id":"scene-change"}),
    )
    .unwrap();

    let candidate = app
        .call(
            "candidate.create",
            json!({
                "project_id":"book",
                "candidate_id":"stale-candidate",
                "kind":"style_aware_rewrite",
                "source_revision":guide["source_content_revision"],
                "content":"오래된 작업 지시에서 온 후보",
                "provider":"external_editor",
                "model":"user_provided",
                "command_id":"stale-candidate-create"
            }),
        )
        .unwrap();
    assert_eq!(candidate["idempotent"], false);
    app.call(
        "changeset.create",
        json!({
            "project_id":"book",
            "candidate_id":"stale-candidate",
            "changeset_id":"stale-change",
            "operations":[{"type":"replace_scene_text","scene_id":"scene_1","expected_scene_revision":1,"text":"오래된 후보가 덮어쓸 수 없음"}],
            "command_id":"stale-change-create"
        }),
    )
    .unwrap();
    let before_approval = app
        .call("project.get", json!({"project_id":"book"}))
        .unwrap();
    let (artifact_revision, artifact_digest) = candidate_subject_claims(&app, "stale-candidate");
    let error = app
        .call(
            "changeset.approve",
            json!({
                "project_id":"book",
                "changeset_id":"stale-change",
                "expected_project_revision":before_approval["revision"],
                "actor_id":"editor",
                "expected_artifact_revision":artifact_revision,
                "expected_artifact_digest":artifact_digest,
                "command_id":"stale-change-approve"
            }),
        )
        .unwrap_err();
    assert!(matches!(error, Error::Conflict(message) if message.contains("content revision")));
    let after = app
        .call("project.get", json!({"project_id":"book"}))
        .unwrap();
    assert_eq!(after["revision"], before_approval["revision"]);
    assert_eq!(after["content_revision"], 2);
    assert_eq!(
        after["scenes"]["scene_1"]["text"],
        "사용자가 먼저 고친 원고"
    );
    assert_eq!(after["candidates"]["stale-candidate"]["status"], "pending");
    assert_eq!(after["changesets"]["stale-change"]["status"], "pending");
    let _ = fs::remove_dir_all(root);
}

#[test]
fn stale_voice_guide_after_other_content_change_cannot_overwrite_scene() {
    let (root, app) = fixture("voice-guide-stale-content");
    app.call(
        "project.create",
        json!({"project_id":"book","title":"Book","command_id":"one"}),
    )
    .unwrap();
    app.call(
        "scene.add",
        json!({"project_id":"book","scene_id":"scene_1","title":"Scene","text":"원문","command_id":"two"}),
    )
    .unwrap();
    app.call(
        "voice.profile.create",
        json!({
            "project_id":"book",
            "profile_id":"voice-main",
            "expected_project_revision":2,
            "samples":["원문을 조용히 적었다."],
            "command_id":"voice-create"
        }),
    )
    .unwrap();
    let guide = app
        .call(
            "voice.guide",
            json!({"project_id":"book","scene_id":"scene_1"}),
        )
        .unwrap();
    app.call(
        "fact.add",
        json!({
            "project_id":"book",
            "fact_id":"fact-1",
            "subject":"민서",
            "predicate":"이름",
            "object":"민서",
            "evidence":[{"scene_id":"scene_1"}],
            "command_id":"fact-change"
        }),
    )
    .unwrap();

    app.call(
        "candidate.create",
        json!({
            "project_id":"book",
            "candidate_id":"stale-content-candidate",
            "kind":"style_aware_rewrite",
            "source_revision":guide["source_content_revision"],
            "content":"사실 변경 뒤의 오래된 후보",
            "provider":"external_editor",
            "model":"user_provided",
            "command_id":"stale-content-candidate-create"
        }),
    )
    .unwrap();
    app.call(
        "changeset.create",
        json!({
            "project_id":"book",
            "candidate_id":"stale-content-candidate",
            "changeset_id":"stale-content-change",
            "operations":[{"type":"replace_scene_text","scene_id":"scene_1","expected_scene_revision":0,"text":"오래된 후보"}],
            "command_id":"stale-content-change-create"
        }),
    )
    .unwrap();
    let before_approval = app
        .call("project.get", json!({"project_id":"book"}))
        .unwrap();
    let (artifact_revision, artifact_digest) =
        candidate_subject_claims(&app, "stale-content-candidate");
    let error = app
        .call(
            "changeset.approve",
            json!({
                "project_id":"book",
                "changeset_id":"stale-content-change",
                "expected_project_revision":before_approval["revision"],
                "actor_id":"editor",
                "expected_artifact_revision":artifact_revision,
                "expected_artifact_digest":artifact_digest,
                "command_id":"stale-content-change-approve"
            }),
        )
        .unwrap_err();
    assert!(matches!(error, Error::Conflict(message) if message.contains("content revision")));
    let after = app
        .call("project.get", json!({"project_id":"book"}))
        .unwrap();
    assert_eq!(after["scenes"]["scene_1"]["text"], "원문");
    assert_eq!(after["facts"]["fact-1"]["object"], "민서");
    assert_eq!(
        after["changesets"]["stale-content-change"]["status"],
        "pending"
    );
    let _ = fs::remove_dir_all(root);
}

#[test]
fn current_voice_guide_approves_and_candidate_future_content_revision_is_rejected() {
    let (root, app) = fixture("voice-guide-current");
    app.call(
        "project.create",
        json!({"project_id":"book","title":"Book","command_id":"one"}),
    )
    .unwrap();
    app.call(
        "scene.add",
        json!({"project_id":"book","scene_id":"scene_1","title":"Scene","text":"원문","command_id":"two"}),
    )
    .unwrap();
    app.call(
        "voice.profile.create",
        json!({
            "project_id":"book",
            "profile_id":"voice-main",
            "expected_project_revision":2,
            "samples":["원문을 조용히 적었다."],
            "command_id":"voice-create"
        }),
    )
    .unwrap();
    let guide = app
        .call(
            "voice.guide",
            json!({"project_id":"book","scene_id":"scene_1"}),
        )
        .unwrap();
    app.call(
        "candidate.create",
        json!({
            "project_id":"book",
            "candidate_id":"current-candidate",
            "kind":"style_aware_rewrite",
            "source_revision":guide["source_content_revision"],
            "content":"현재 작업 지시를 따른 후보",
            "provider":"external_editor",
            "model":"user_provided",
            "command_id":"current-candidate-create"
        }),
    )
    .unwrap();
    app.call(
        "changeset.create",
        json!({
            "project_id":"book",
            "candidate_id":"current-candidate",
            "changeset_id":"current-change",
            "operations":[{"type":"replace_scene_text","scene_id":"scene_1","expected_scene_revision":guide["source_scene_revision"],"text":"현재 작업 지시를 따른 후보"}],
            "command_id":"current-change-create"
        }),
    )
    .unwrap();
    let before_approval = app
        .call("project.get", json!({"project_id":"book"}))
        .unwrap();
    let (artifact_revision, artifact_digest) = candidate_subject_claims(&app, "current-candidate");
    app.call(
        "changeset.approve",
        json!({
            "project_id":"book",
            "changeset_id":"current-change",
            "expected_project_revision":before_approval["revision"],
            "actor_id":"editor",
            "expected_artifact_revision":artifact_revision,
            "expected_artifact_digest":artifact_digest,
            "command_id":"current-change-approve"
        }),
    )
    .unwrap();
    let approved = app
        .call("project.get", json!({"project_id":"book"}))
        .unwrap();
    assert_eq!(
        approved["scenes"]["scene_1"]["text"],
        "현재 작업 지시를 따른 후보"
    );

    let future_error = app
        .call(
            "candidate.create",
            json!({
                "project_id":"book",
                "candidate_id":"future-candidate",
                "kind":"rewrite",
                "source_revision":approved["revision"],
                "content":"감사 revision을 content revision으로 착각한 후보",
                "provider":"external_editor",
                "model":"user_provided",
                "command_id":"future-candidate-create"
            }),
        )
        .unwrap_err();
    assert!(
        matches!(future_error, Error::InvalidInput(message) if message.contains("content revision"))
    );
    assert!(!app
        .call("project.get", json!({"project_id":"book"}))
        .unwrap()["candidates"]
        .as_object()
        .unwrap()
        .contains_key("future-candidate"));
    let _ = fs::remove_dir_all(root);
}
