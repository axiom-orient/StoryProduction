mod support;

use geultu::contracts::digest::jcs_sha256;
use geultu::contracts::{
    GeultuReviewedSynopsisHandoff, GeultuStoryPackage, GEULTU_STORY_PACKAGE_SCHEMA_ID,
};
use geultu::model::Project;
use geultu::story_package::{export_geultu_story_package, GeultuStoryExportInput};
use geultu::{Application, Error, Workspace};
use serde_json::{json, Value};
use sha2::Digest;
use std::fs;
use support::unique_temp_id;

fn fixture_path(label: &str) -> std::path::PathBuf {
    std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../docs/canonical/fixtures")
        .join(label)
}

fn read_fixture(label: &str) -> Value {
    let path = fixture_path(label);
    let raw = fs::read_to_string(&path)
        .unwrap_or_else(|error| panic!("read {}: {error}", path.display()));
    serde_json::from_str(&raw).unwrap_or_else(|error| panic!("parse {}: {error}", path.display()))
}

fn workspace_app(label: &str) -> (std::path::PathBuf, Application) {
    let root = std::env::temp_dir().join(format!(
        "geultu-contracts-{label}-{}-{}",
        std::process::id(),
        unique_temp_id()
    ));
    let _ = fs::remove_dir_all(&root);
    let workspace = Workspace::init(&root).unwrap();
    (root, Application::from_workspace(workspace))
}

#[test]
fn project_metadata_separates_title_from_list_label_and_preserves_export_meaning() {
    let (root, app) = workspace_app("project-metadata");
    app.call(
        "project.create",
        json!({
            "project_id":"book",
            "title":"비가 남긴 대출증",
            "display_label":"엄마 이야기 초안",
            "summary":"폐관 직전, 지안은 엄마 이름과 17년 전 날짜가 적힌 대출증을 발견한다.",
            "command_id":"create-book"
        }),
    )
    .unwrap();

    let created = app
        .call("project.get", json!({"project_id":"book"}))
        .unwrap();
    assert_eq!(created["title"], "비가 남긴 대출증");
    assert_eq!(created["display_label"], "엄마 이야기 초안");
    assert_eq!(
        created["summary"],
        "폐관 직전, 지안은 엄마 이름과 17년 전 날짜가 적힌 대출증을 발견한다."
    );
    assert_eq!(created["revision"], 1);
    assert_eq!(created["content_revision"], 0);

    let exported = app
        .call("project.export_markdown", json!({"project_id":"book"}))
        .unwrap();
    assert!(exported["markdown"]
        .as_str()
        .unwrap()
        .starts_with("# 비가 남긴 대출증\n"));
    assert!(!exported["markdown"]
        .as_str()
        .unwrap()
        .contains("엄마 이야기 초안"));

    app.call(
        "project.update_metadata",
        json!({
            "project_id":"book",
            "title":"엄마의 17년 전 대출증",
            "display_label":"엄마 이야기",
            "summary":"폐관 직전, 지안은 엄마 이름이 적힌 17년 전 대출증을 발견한다.",
            "expected_project_revision":1,
            "command_id":"metadata-update"
        }),
    )
    .unwrap();
    let updated = app
        .call("project.get", json!({"project_id":"book"}))
        .unwrap();
    assert_eq!(updated["title"], "엄마의 17년 전 대출증");
    assert_eq!(updated["display_label"], "엄마 이야기");
    assert_eq!(updated["revision"], 2);
    assert_eq!(updated["content_revision"], 0);

    let stale = app.call(
        "project.update_metadata",
        json!({
            "project_id":"book",
            "title":"잘못된 덮어쓰기",
            "display_label":"",
            "summary":"",
            "expected_project_revision":1,
            "command_id":"stale-metadata-update"
        }),
    );
    assert!(
        matches!(stale, Err(Error::Conflict(message)) if message.contains("project revision mismatch"))
    );

    app.call(
        "scene.add",
        json!({
            "project_id":"book",
            "scene_id":"scene_001",
            "title":"도서관 폐관 뒤",
            "text":"지안은 반납함에서 낡은 대출증을 발견했다.",
            "order":0,
            "command_id":"scene-add"
        }),
    )
    .unwrap();
    app.call(
        "voice.profile.create",
        json!({
            "project_id":"book",
            "profile_id":"voice-fixture",
            "expected_project_revision":3,
            "samples":["나는 매일 도서관에 갔다. 비가 오면 천천히 걸었다."],
            "command_id":"voice-create"
        }),
    )
    .unwrap();
    let guide = app
        .call(
            "voice.guide",
            json!({"project_id":"book","scene_id":"scene_001"}),
        )
        .unwrap();
    assert!(guide["instruction"]
        .as_str()
        .unwrap()
        .contains("폐관 직전, 지안은 엄마 이름이 적힌 17년 전 대출증을 발견한다."));
    assert!(guide["instruction"]
        .as_str()
        .unwrap()
        .contains("원문에 없는 사건·원인·감정을 추가하는 근거로 쓰지 마세요."));

    app.call("project.recover", json!({"project_id":"book"}))
        .unwrap();
    let recovered = app
        .call("project.get", json!({"project_id":"book"}))
        .unwrap();
    assert_eq!(
        recovered["summary"],
        "폐관 직전, 지안은 엄마 이름이 적힌 17년 전 대출증을 발견한다."
    );

    let mut legacy_snapshot =
        serde_json::to_value(Project::empty("legacy".into(), "기존 작품 제목".into())).unwrap();
    let legacy_object = legacy_snapshot.as_object_mut().unwrap();
    legacy_object.remove("display_label");
    legacy_object.remove("summary");
    let legacy: Project = serde_json::from_value(legacy_snapshot).unwrap();
    assert_eq!(legacy.display_label, None);
    assert_eq!(legacy.summary, None);
    let legacy_event: geultu::domain::Event = serde_json::from_value(json!({
        "type":"project_created",
        "id":"legacy",
        "title":"기존 작품 제목"
    }))
    .unwrap();
    let replayed_legacy = geultu::domain::evolve(None, &legacy_event).unwrap();
    assert_eq!(replayed_legacy.display_label, None);
    assert_eq!(replayed_legacy.summary, None);

    let _ = fs::remove_dir_all(root);
}

/// Builds a project whose logical composition mirrors
/// docs/canonical/fixtures/t33/valid-minimal.json.
fn seed_fixture_composition(app: &Application, project_id: &str) {
    app.call(
        "project.create",
        json!({"project_id": project_id, "title": "Example", "command_id": "create"}),
    )
    .unwrap();
    app.call(
        "scene.add",
        json!({
            "project_id": project_id,
            "scene_id": "scene_001",
            "title": "시작 장면",
            "text": "주인공이 문을 연다.",
            "order": 1,
            "command_id": "scene-1"
        }),
    )
    .unwrap();
}

#[test]
fn valid_minimal_fixture_deserializes_into_contract_type() {
    let value = read_fixture("t33/valid-minimal.json");
    let package: GeultuStoryPackage =
        serde_json::from_value(value).expect("valid-minimal must deserialize");
    package.validate().expect("valid-minimal must validate");
    assert_eq!(package.schema_id, GEULTU_STORY_PACKAGE_SCHEMA_ID);
    assert_eq!(package.language, "ko");
    assert_eq!(package.scenes.len(), 1);
    assert_eq!(package.scenes[0].scene_id, "scene_001");
    assert_eq!(package.scenes[0].visual_priority, 0.8);
}

#[test]
fn invalid_t33_fixtures_are_rejected_fail_closed() {
    // Unknown top-level key "passed" violates additionalProperties: false.
    let caller_pass = read_fixture("t33/invalid-caller-pass.json");
    assert!(
        serde_json::from_value::<GeultuStoryPackage>(caller_pass).is_err(),
        "caller-pass envelope key must be rejected at deserialization"
    );

    // Known fields but out-of-contract values are rejected by validation.
    let score = read_fixture("t33/invalid-score-out-of-range.json");
    let package: GeultuStoryPackage = serde_json::from_value(score).unwrap();
    let error = package
        .validate()
        .expect_err("visual_priority 1.1 must be rejected");
    assert!(
        matches!(error, geultu::Error::InvalidInput(message) if message.contains("visual_priority"))
    );

    let digest = read_fixture("t33/invalid-digest-format.json");
    let package: GeultuStoryPackage = serde_json::from_value(digest).unwrap();
    let error = package
        .validate()
        .expect_err("non-sha256 digest must be rejected");
    assert!(matches!(error, geultu::Error::InvalidInput(message) if message.contains("digest")));

    // Wrapped envelope fields are outside the flat package contract.
    let legacy = read_fixture("t33/invalid-legacy-envelope.json");
    assert!(
        serde_json::from_value::<GeultuStoryPackage>(legacy).is_err(),
        "wrapped envelope fields must be rejected at deserialization"
    );
}

#[test]
fn jcs_digest_matches_golden_body_vector() {
    let vector = read_fixture("jcs/body-digest-vector.json");
    let body = &vector["body"];
    let canonical = serde_json_canonicalizer::to_string(body).unwrap();
    assert_eq!(
        canonical, vector["canonical_utf8"],
        "canonical form mismatch"
    );
    let digest = jcs_sha256(body).expect("golden vector must canonicalize");
    let expected = vector["sha256"]
        .as_str()
        .unwrap()
        .strip_prefix("sha256:")
        .unwrap();
    assert_eq!(digest, expected);
}

#[test]
fn story_export_package_id_is_deterministic_and_prefixed() {
    let export = |label: &str| {
        let (root, app) = workspace_app(label);
        seed_fixture_composition(&app, "book");
        let result = app
            .call(
                "story.export",
                json!({
                    "project_id": "book",
                    "language": "ko",
                    "logline": "검증용 서사 패키지.",
                    "global_constraints": ["character identity must remain stable"]
                }),
            )
            .expect("story.export must succeed");
        let _ = fs::remove_dir_all(root);
        result
    };

    let first = export("det-a");
    let second = export("det-b");
    assert_eq!(
        first, second,
        "same logical input must yield identical output"
    );
    let package_id = first["story_package_id"].as_str().unwrap();

    assert!(
        package_id.starts_with("np_"),
        "package_id must start with np_"
    );
    let hex = &package_id[3..];
    assert_eq!(hex.len(), 64);
    assert!(
        hex.bytes()
            .all(|byte| byte.is_ascii_hexdigit() && !byte.is_ascii_uppercase()),
        "package_id suffix must be lowercase hex"
    );

    // The identity must be recomputable from the emitted flat package.
    let mut body = first["story_package"].clone();
    body.as_object_mut()
        .expect("package is an object")
        .remove("package_id")
        .expect("package carries package_id");
    assert_eq!(format!("np_{}", jcs_sha256(&body).unwrap()), package_id);

    // The emitted package round-trips through the contract type.
    let package: GeultuStoryPackage =
        serde_json::from_value(first["story_package"].clone()).unwrap();
    package.validate().expect("emitted package must validate");
    assert_eq!(package.package_id, package_id);
    assert_eq!(first["source_project_revision"], 2);
}

#[test]
fn story_export_is_a_pure_read_without_journal_writes() {
    let (root, app) = workspace_app("pure-read");
    seed_fixture_composition(&app, "book");
    let journal_before =
        fs::read_to_string(root.join(".geultu/projects/book/events.ndjson")).unwrap();
    let before = app
        .call("project.get", json!({"project_id": "book"}))
        .unwrap();

    app.call(
        "story.export",
        json!({"project_id": "book", "language": "ko", "logline": "로그라인."}),
    )
    .unwrap();

    assert_eq!(
        fs::read_to_string(root.join(".geultu/projects/book/events.ndjson")).unwrap(),
        journal_before,
        "story.export must not write the event journal"
    );
    assert_eq!(
        app.call("project.get", json!({"project_id": "book"}))
            .unwrap(),
        before
    );
    let _ = fs::remove_dir_all(root);
}

#[test]
fn reviewed_synopsis_handoff_is_raw_hash_and_human_approval_bound() {
    let (root, app) = workspace_app("reviewed-handoff");
    seed_fixture_composition(&app, "book");
    let exported = app
        .call("project.export_markdown", json!({"project_id":"book"}))
        .unwrap();
    let synopsis_sha256 = format!(
        "{:x}",
        sha2::Sha256::digest(exported["markdown"].as_str().unwrap().as_bytes())
    );
    let result = app
        .call(
            "synopsis.export_reviewed_handoff",
            json!({
                "project_id": "book",
                "actor_id": "human-author",
                "language": "ko",
                "reason": "검토 완료",
                "expected_source_revision": exported["revision"],
                "expected_synopsis_sha256": synopsis_sha256
            }),
        )
        .expect("reviewed synopsis handoff must succeed");
    let handoff: GeultuReviewedSynopsisHandoff =
        serde_json::from_value(result.clone()).expect("handoff must round-trip");
    handoff
        .validate_identity()
        .expect("handoff identity must bind to its body");
    assert_eq!(handoff.schema_version, 1);
    assert_eq!(handoff.source_revision, 2);
    assert_eq!(handoff.approval.decision, "APPROVE");
    assert_eq!(
        handoff.approval.synopsis_sha256, handoff.synopsis_sha256,
        "human approval must bind to the raw synopsis hash"
    );

    let synopsis_path = root.join(".geultu/projects/book/handoff/synopsis.md");
    let synopsis = fs::read(&synopsis_path).expect("synopsis.md must be emitted");
    assert_eq!(
        format!("{:x}", sha2::Sha256::digest(&synopsis)),
        handoff.synopsis_sha256,
        "handoff hash must cover exact emitted UTF-8 bytes"
    );
    let persisted: GeultuReviewedSynopsisHandoff = serde_json::from_slice(
        &fs::read(root.join(".geultu/projects/book/handoff/reviewed-synopsis-handoff.v1.json"))
            .unwrap(),
    )
    .unwrap();
    assert_eq!(persisted, handoff);

    let journal_before =
        fs::read_to_string(root.join(".geultu/projects/book/events.ndjson")).unwrap();
    let stale = app.call(
        "synopsis.export_reviewed_handoff",
        json!({
            "project_id": "book",
            "actor_id": "human-author",
            "language": "ko",
            "expected_source_revision": 1,
            "expected_synopsis_sha256": handoff.synopsis_sha256
        }),
    );
    assert!(stale.is_err(), "stale source revision must fail closed");
    assert_eq!(
        fs::read_to_string(root.join(".geultu/projects/book/events.ndjson")).unwrap(),
        journal_before,
        "handoff export must not mutate the canonical journal"
    );
    let _ = fs::remove_dir_all(root);
}

#[test]
fn story_export_surfaces_in_tool_registry() {
    let (root, app) = workspace_app("tools");
    let tools = app.tool_definitions();
    let definition = tools
        .iter()
        .find(|tool| tool.name == "story.export")
        .expect("story.export must be listed as an MCP tool");
    assert_eq!(
        definition.input_schema["required"],
        json!(["project_id", "language", "logline"])
    );
    assert!(
        tools.iter().all(|tool| tool.name != "narrative.export"),
        "the pre-rename public command must not remain registered"
    );
    assert!(
        app.call(
            "narrative.export",
            json!({"project_id": "book", "language": "ko", "logline": "로그라인."})
        )
        .is_err(),
        "the pre-rename public command must fail closed rather than shim"
    );
    let _ = fs::remove_dir_all(root);
}

#[test]
fn story_export_rejects_invalid_projection_inputs() {
    let (root, app) = workspace_app("reject-inputs");
    seed_fixture_composition(&app, "book");

    // Scene ids outside the t33 pattern fail closed at ingestion.
    let error = app
        .call(
            "scene.add",
            json!({"project_id": "book", "scene_id": "s1", "title": "나쁜 아이디", "text": "본문", "order": 2, "command_id": "bad-id"}),
        )
        .unwrap_err();
    assert!(matches!(error, geultu::Error::InvalidInput(message) if message.contains("scene_id")));

    // Invalid language tags fail closed.
    let error = app
        .call(
            "story.export",
            json!({"project_id": "book", "language": "not a tag!", "logline": "로그라인."}),
        )
        .unwrap_err();
    assert!(matches!(error, geultu::Error::InvalidInput(message) if message.contains("language")));

    // Missing required metadata fails closed instead of inventing values.
    let error = app
        .call("story.export", json!({"project_id": "book"}))
        .unwrap_err();
    assert!(matches!(error, geultu::Error::InvalidInput(_)));

    let _ = fs::remove_dir_all(root);

    // Empty narrative text violates the schema minimum.
    let (empty_root, empty_app) = workspace_app("reject-empty-text");
    empty_app
        .call(
            "project.create",
            json!({"project_id": "book", "title": "Example", "command_id": "create"}),
        )
        .unwrap();
    empty_app
        .call(
            "scene.add",
            json!({"project_id": "book", "scene_id": "scene_001", "title": "빈 장면", "order": 1, "command_id": "empty"}),
        )
        .unwrap();
    let error = empty_app
        .call(
            "story.export",
            json!({"project_id": "book", "language": "ko", "logline": "로그라인."}),
        )
        .unwrap_err();
    assert!(matches!(error, geultu::Error::InvalidInput(message) if message.contains("narrative")));
    let _ = fs::remove_dir_all(empty_root);
}

#[test]
fn direct_story_projection_matches_command() {
    let (root, app) = workspace_app("direct");
    seed_fixture_composition(&app, "book");
    let command_package = app
        .call(
            "story.export",
            json!({
                "project_id": "book",
                "language": "ko",
                "logline": "검증용 서사 패키지.",
                "global_constraints": ["character identity must remain stable"]
            }),
        )
        .unwrap();

    let state = app
        .call("project.get", json!({"project_id": "book"}))
        .unwrap();
    let input = GeultuStoryExportInput {
        language: "ko".into(),
        logline: "검증용 서사 패키지.".into(),
        global_constraints: vec!["character identity must remain stable".into()],
    };
    let direct =
        export_geultu_story_package(&serde_json::from_value(state).unwrap(), &input).unwrap();
    assert_eq!(
        serde_json::to_value(&direct).unwrap(),
        command_package["story_package"],
        "direct projection and command output must agree"
    );

    let _ = fs::remove_dir_all(root);
}

#[test]
fn route_id_serde_matches_schema_spelling() {
    use geultu::contracts::RouteId;

    let parsed: RouteId = serde_json::from_value(json!("TEXT_TO_NVC"))
        .expect("schema spelling TEXT_TO_NVC must deserialize");
    assert_eq!(parsed, RouteId::TextToNvc);
    assert_eq!(
        serde_json::to_value(&parsed).expect("serialize RouteId"),
        json!("TEXT_TO_NVC"),
        "serialized spelling must equal the route-admission schema enum"
    );

    for wrong in ["TEXTTONVC", "text_to_nvc", "TextToNvc"] {
        assert!(
            serde_json::from_value::<RouteId>(json!(wrong)).is_err(),
            "non-schema spelling '{wrong}' must be rejected"
        );
    }
}

#[test]
fn markdown_import_generates_t33_conforming_scene_ids() {
    let scenes =
        geultu::markdown::import_markdown("# 비 오는 역\n\n본문.\n\n# Chapter 2\n\n이어짐.\n", 0)
            .expect("import simple markdown");
    let ids: Vec<&str> = scenes.iter().map(|scene| scene.id.as_str()).collect();
    assert_eq!(ids, vec!["scene_1", "scene_chapter-2"]);
}
