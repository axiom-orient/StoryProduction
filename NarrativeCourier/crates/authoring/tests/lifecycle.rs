#[path = "../../../test_support/sts_workflow_fixture.rs"]
mod sts_workflow_fixture;

use narrative_courier_authoring::{execute, Error};
use narrative_courier_runtime::{store::sha256_bytes, token::hash_jcs};
use serde_json::{json, Value};
use std::fs;
use std::path::{Path, PathBuf};
use sts_workflow::{Engine, NewProject, NextPacket};
use tempfile::TempDir;

struct Harness {
    _temporary: TempDir,
    project: PathBuf,
}
impl Harness {
    fn new() -> Self {
        Self::with_two_scenes(false)
    }
    fn with_two_scenes(two_scenes: bool) -> Self {
        let temporary = TempDir::new().unwrap();
        let owner = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");
        copy_tree(&owner.join("skills"), &temporary.path().join("skills"));
        let engine = Engine::new(temporary.path()).unwrap();
        engine
            .create_project(&NewProject {
                project_name: "demo".into(),
                title: "Synthetic test only".into(),
                synopsis: sts_workflow_fixture::SYNOPSIS.into(),
                source_format: "original".into(),
                target_duration: 100,
                genre: vec!["드라마".into()],
                tone: "절제".into(),
                theme_direction: "의심과 사실의 구분".into(),
                references: vec![],
            })
            .unwrap();
        for _ in 0..80 {
            match engine.next("demo").unwrap() {
                NextPacket::AgentTask {
                    node, task_token, ..
                } => {
                    let original = sts_workflow_fixture::artifact_for(&node);
                    let artifact = if two_scenes
                        && matches!(node.as_str(), "step_06" | "step_07_full" | "step_08_apply")
                    {
                        format!("{original}\n{}", original.replace("SCN-001", "SCN-002"))
                    } else {
                        original.to_owned()
                    };
                    engine.submit("demo", &artifact, &task_token).unwrap();
                }
                NextPacket::AwaitingHuman { gate_token, .. } => {
                    engine
                        .decide("demo", "approve", None, None, &gate_token)
                        .unwrap();
                }
                NextPacket::Completed { .. } => {
                    let project = engine.project_dir("demo").unwrap();
                    engine.release_bundle("demo").unwrap();
                    return Self {
                        _temporary: temporary,
                        project,
                    };
                }
                other => panic!("unexpected fixture state: {other:?}"),
            }
        }
        panic!("synthetic fixture did not complete");
    }
    fn call(&self, lane: &str, arguments: &[&str]) -> Result<Value, Error> {
        execute(
            lane,
            arguments
                .iter()
                .copied()
                .chain(["--project-root", self.project.to_str().unwrap()])
                .map(str::to_owned),
        )
    }
    fn write(&self, name: &str, value: &Value) {
        fs::write(
            self.project.join(name),
            serde_json::to_vec_pretty(value).unwrap(),
        )
        .unwrap();
    }
    fn read(&self, path: &str) -> Value {
        serde_json::from_slice(&fs::read(self.project.join(path)).unwrap()).unwrap()
    }
    fn task(&self) -> Value {
        self.call(
            "storyboard",
            &[
                "prepare",
                "--task-id",
                "synthetic",
                "--brief",
                "보존 검증",
                "--output",
                "storyboards/task.json",
            ],
        )
        .unwrap()
    }
    fn draft(&self, packet: &Value) -> Value {
        let occurrences = packet["semanticOccurrences"].as_array().unwrap();
        let scenes: Vec<_> = packet["scenes"].as_array().unwrap().iter().enumerate().map(|(index, scene)| {
            let refs: Vec<_> = occurrences.iter().filter(|entry| entry["scene_id"] == scene["scene_id"]).map(|entry| json!({"semanticId": entry["semantic_id"], "entryHash": entry["entry_hash"]})).collect();
            json!({"sceneId": scene["scene_id"], "intent": "의심과 확인의 순서를 보존한다", "panels": [{"panelId": format!("panel-{index}"), "composition": "기록과 지우를 함께 본다", "staging": "지우가 기록에서 문으로 시선을 옮긴다", "sourceRefs": refs, "soundNotes": "원문에 없는 대사를 추가하지 않는다", "transition": "기록 확인 뒤 출발한다"}]})
        }).collect();
        json!({"scenes": scenes, "changeSummary": "Synthetic fixture only", "assumptions": [], "openQuestions": []})
    }
    fn board(&self) -> Value {
        let task = self.task();
        self.write("draft.json", &self.draft(&task));
        self.call(
            "storyboard",
            &[
                "submit",
                "--task",
                "storyboards/task.json",
                "--draft",
                "draft.json",
                "--output",
                "storyboards/board.json",
            ],
        )
        .unwrap()
    }
    fn approve_board(&self, board: &Value) -> Value {
        self.call(
            "storyboard",
            &[
                "review",
                "--board",
                "storyboards/board.json",
                "--expected-hash",
                board["contentHash"].as_str().unwrap(),
                "--reviewer",
                " SYNTHETIC_TEST_ONLY ",
                "--decision",
                "approved",
                "--output",
                "storyboards/review.json",
            ],
        )
        .unwrap()
    }
    fn approved(&self) -> (Value, Value) {
        let board = self.board();
        let review = self.approve_board(&board);
        (board, review)
    }
    fn animatic_draft(&self) -> Value {
        let source = self
            .call(
                "animatic",
                &[
                    "source",
                    "--board",
                    "storyboards/board.json",
                    "--board-review",
                    "storyboards/review.json",
                ],
            )
            .unwrap();
        let panels: Vec<_> = source["board"]["scenes"].as_array().unwrap().iter().flat_map(|scene| scene["panels"].as_array().unwrap().iter().map(move |panel| {
            json!({"sceneId": scene["sceneId"], "panelId": panel["panelId"], "durationFrames": 12,
                "camera": [{"frame": 0, "x": 0, "y": 0, "scalePerMille": 1000, "easing": "hold"}, {"frame": 11, "x": 0, "y": 0, "scalePerMille": 1000, "easing": "hold"}],
                "layers": [{"layerId": "background", "depthPerMille": 0, "poses": [{"frame": 0, "assetId": "background"}],
                    "motion": [{"frame": 0, "x": 0, "y": 0, "scalePerMille": 1000, "easing": "hold", "rotationMilliDeg": 0, "opacityPerMille": 1000},
                               {"frame": 11, "x": 0, "y": 0, "scalePerMille": 1000, "easing": "hold", "rotationMilliDeg": 0, "opacityPerMille": 1000}]}],
                "captions": panel["sourceRefs"].as_array().unwrap().iter().map(|reference| reference["semanticId"].clone()).collect::<Vec<_>>()})
        })).collect();
        json!({"schemaVersion": "nvc.animatic-plan/1", "source": source["source"], "boardHash": source["board"]["contentHash"], "boardReviewHash": source["boardReview"]["contentHash"],
            "fps": 12, "canvas": {"width": 64, "height": 64, "background": "#ffffff"}, "assets": [{"assetId": "background", "width": 64, "height": 64, "alpha": "opaque", "brief": "Synthetic PNG only"}], "panels": panels, "openQuestions": []})
    }
}

#[test]
fn selected_scene_revision_preserves_unselected_scene_and_release_order() {
    let harness = Harness::with_two_scenes(true);
    harness.board();
    let before = harness.read("storyboards/board.json");
    assert_eq!(before["scenes"].as_array().unwrap().len(), 2);
    let packet = harness
        .call(
            "storyboard",
            &[
                "prepare",
                "--task-id",
                "partial",
                "--brief",
                "두 번째 장면의 구도만 바꾼다",
                "--base",
                "storyboards/board.json",
                "--scene",
                "SCN-002",
                "--output",
                "storyboards/partial-task.json",
            ],
        )
        .unwrap();
    assert_eq!(packet["task"]["sceneIds"], json!(["SCN-002"]));
    let mut draft = harness.draft(&packet);
    draft["scenes"][0]["panels"][0]["panelId"] = "panel-1".into();
    draft["scenes"][0]["panels"][0]["composition"] = "두 번째 장면에서 출발을 더 멀리 본다".into();
    harness.write("partial-draft.json", &draft);
    harness
        .call(
            "storyboard",
            &[
                "submit",
                "--task",
                "storyboards/partial-task.json",
                "--draft",
                "partial-draft.json",
                "--output",
                "storyboards/partial-board.json",
            ],
        )
        .unwrap();
    let after = harness.read("storyboards/partial-board.json");
    assert_eq!(after["scenes"][0], before["scenes"][0]);
    assert_eq!(after["scenes"][1]["sceneId"], "SCN-002");
    assert_eq!(after["baseHash"], before["contentHash"]);
    assert_ne!(after["scenes"][1], before["scenes"][1]);
}
fn copy_tree(source: &Path, target: &Path) {
    fs::create_dir_all(target).unwrap();
    for entry in fs::read_dir(source).unwrap() {
        let entry = entry.unwrap();
        let target = target.join(entry.file_name());
        if entry.file_type().unwrap().is_dir() {
            copy_tree(&entry.path(), &target);
        } else {
            fs::copy(entry.path(), target).unwrap();
        }
    }
}
fn assert_hash(value: &Value) {
    let mut body = value.clone();
    body.as_object_mut().unwrap().remove("contentHash");
    assert_eq!(
        value["contentHash"],
        format!("sha256:{}", hash_jcs(&body).unwrap())
    );
}

#[test]
fn full_storyboard_lifecycle_and_exact_six_file_bundle() {
    let harness = Harness::new();
    let (board, review) = harness.approved();
    let bytes = fs::read(harness.project.join("storyboards/review.json")).unwrap();
    assert_eq!(harness.approve_board(&board), review);
    assert_eq!(
        fs::read(harness.project.join("storyboards/review.json")).unwrap(),
        bytes
    );
    assert_eq!(review["review"]["reviewer"], "SYNTHETIC_TEST_ONLY");
    assert_hash(&harness.read("storyboards/board.json"));
    let export = harness
        .call(
            "storyboard",
            &[
                "export",
                "--board",
                "storyboards/board.json",
                "--review",
                "storyboards/review.json",
                "--output",
                "storyboards/approved.md",
            ],
        )
        .unwrap();
    assert_eq!(export["status"], "approved_exported");
    assert_eq!(export["coverage"], "full");
    let bundle = harness
        .call(
            "storyboard",
            &[
                "bundle",
                "--board",
                "storyboards/board.json",
                "--review",
                "storyboards/review.json",
            ],
        )
        .unwrap();
    assert_eq!(bundle["status"], "approved_text_package");
    let files = bundle["files"].as_object().unwrap();
    assert_eq!(files.len(), 6);
    for name in [
        "screenplay.md",
        "scene-blueprint.md",
        "scene-map.json",
        "storyboard.md",
        "storyboard.json",
        "storyboard-review.json",
    ] {
        let artifact = &files[name];
        assert_eq!(
            artifact["sha256"],
            format!(
                "sha256:{}",
                sha256_bytes(artifact["text"].as_str().unwrap().as_bytes())
            )
        );
    }
    assert_eq!(
        bundle["files"]["storyboard.md"]["text"]
            .as_str()
            .unwrap()
            .as_bytes(),
        fs::read(harness.project.join("storyboards/approved.md")).unwrap()
    );
}

#[test]
fn preparation_replay_base_revision_and_immutable_conflicts() {
    let harness = Harness::new();
    let initial = harness.task();
    let task_bytes = fs::read(harness.project.join("storyboards/task.json")).unwrap();
    assert_eq!(harness.task(), initial);
    assert_eq!(
        fs::read(harness.project.join("storyboards/task.json")).unwrap(),
        task_bytes
    );
    let board = harness.board();
    let revised = harness
        .call(
            "storyboard",
            &[
                "prepare",
                "--task-id",
                "revision",
                "--brief",
                "구도만 변경",
                "--base",
                "storyboards/board.json",
                "--scene",
                "SCN-001",
                "--output",
                "storyboards/revision-task.json",
            ],
        )
        .unwrap();
    let mut draft = harness.draft(&revised);
    draft["scenes"][0]["panels"][0]["composition"] =
        "기록의 내용에서 지우의 표정으로 구도를 옮긴다".into();
    harness.write("revised.json", &draft);
    let result = harness
        .call(
            "storyboard",
            &[
                "submit",
                "--task",
                "storyboards/revision-task.json",
                "--draft",
                "revised.json",
                "--output",
                "storyboards/revised-board.json",
            ],
        )
        .unwrap();
    assert_ne!(result["contentHash"], board["contentHash"]);
    assert_eq!(
        harness.read("storyboards/revised-board.json")["baseHash"],
        board["contentHash"]
    );
    assert!(result["diff"]
        .as_str()
        .unwrap()
        .contains("-**구도** 기록과 지우를 함께 본다"));
    let bytes = fs::read(harness.project.join("storyboards/board.json")).unwrap();
    let error = harness
        .call(
            "storyboard",
            &[
                "submit",
                "--task",
                "storyboards/revision-task.json",
                "--draft",
                "revised.json",
                "--output",
                "storyboards/board.json",
            ],
        )
        .unwrap_err();
    assert_eq!(error.outcome, "not_applied");
    assert_eq!(
        fs::read(harness.project.join("storyboards/board.json")).unwrap(),
        bytes
    );
}

#[test]
fn cancellation_preserves_review_but_blocks_future_publication_and_direction() {
    let harness = Harness::new();
    harness.approved();
    let receipt = fs::read(harness.project.join("storyboards/review.json")).unwrap();
    harness
        .call("storyboard", &["cancel", "--task", "storyboards/task.json"])
        .unwrap();
    for (lane, arguments) in [
        (
            "storyboard",
            vec![
                "bundle",
                "--board",
                "storyboards/board.json",
                "--review",
                "storyboards/review.json",
            ],
        ),
        (
            "storyboard",
            vec![
                "export",
                "--board",
                "storyboards/board.json",
                "--review",
                "storyboards/review.json",
                "--output",
                "storyboards/cancelled-export.md",
            ],
        ),
        (
            "storyboard",
            vec![
                "export",
                "--board",
                "storyboards/board.json",
                "--candidate",
                "--output",
                "storyboards/cancelled-candidate.md",
            ],
        ),
        (
            "animatic",
            vec![
                "source",
                "--board",
                "storyboards/board.json",
                "--board-review",
                "storyboards/review.json",
            ],
        ),
    ] {
        assert!(harness
            .call(lane, &arguments)
            .unwrap_err()
            .message
            .contains("TASK_CANCELLED"));
    }
    assert_eq!(
        fs::read(harness.project.join("storyboards/review.json")).unwrap(),
        receipt
    );
    assert!(!harness
        .project
        .join("storyboards/cancelled-export.md")
        .exists());
    assert!(!harness
        .project
        .join("storyboards/cancelled-candidate.md")
        .exists());
}

#[test]
fn exact_review_open_questions_and_cancelled_task_block_adoption() {
    let harness = Harness::new();
    let board = harness.board();
    let error = harness
        .call(
            "storyboard",
            &[
                "review",
                "--board",
                "storyboards/board.json",
                "--expected-hash",
                &format!("sha256:{}", "0".repeat(64)),
                "--reviewer",
                "TEST",
                "--decision",
                "approved",
                "--output",
                "storyboards/review.json",
            ],
        )
        .unwrap_err();
    assert!(error.message.contains("REVIEWED_CONTENT_CHANGED"));
    assert!(!harness.project.join("storyboards/review.json").exists());
    let mut unresolved = harness.draft(&harness.task());
    unresolved["openQuestions"] = json!(["장소는 사람의 확인이 필요하다"]);
    harness.write("unresolved.json", &unresolved);
    let candidate = harness
        .call(
            "storyboard",
            &[
                "submit",
                "--task",
                "storyboards/task.json",
                "--draft",
                "unresolved.json",
                "--output",
                "storyboards/unresolved.json",
            ],
        )
        .unwrap();
    let error = harness
        .call(
            "storyboard",
            &[
                "review",
                "--board",
                "storyboards/unresolved.json",
                "--expected-hash",
                candidate["contentHash"].as_str().unwrap(),
                "--reviewer",
                "TEST",
                "--decision",
                "approved",
                "--output",
                "storyboards/unresolved-review.json",
            ],
        )
        .unwrap_err();
    assert!(error
        .message
        .contains("RESOLVE_OPEN_QUESTIONS_BEFORE_APPROVAL"));
    assert!(!harness
        .project
        .join("storyboards/unresolved-review.json")
        .exists());
    let cancelled = harness
        .call("storyboard", &["cancel", "--task", "storyboards/task.json"])
        .unwrap();
    assert_eq!(cancelled["status"], "cancelled");
    assert_eq!(cancelled["externalRequestCancelled"], false);
    let error = harness
        .call("storyboard", &["task", "--task", "storyboards/task.json"])
        .unwrap_err();
    assert!(error.message.contains("TASK_CANCELLED"));
    let error = harness
        .call(
            "storyboard",
            &[
                "review",
                "--board",
                "storyboards/board.json",
                "--expected-hash",
                board["contentHash"].as_str().unwrap(),
                "--reviewer",
                "TEST",
                "--decision",
                "approved",
                "--output",
                "storyboards/review.json",
            ],
        )
        .unwrap_err();
    assert!(error.message.contains("TASK_CANCELLED"));
    assert!(!harness.project.join("storyboards/review.json").exists());
}

#[test]
fn source_drift_and_stale_occurrence_hash_are_refused_before_publication() {
    let harness = Harness::new();
    let task = harness.task();
    let mut draft = harness.draft(&task);
    draft["scenes"][0]["panels"][0]["sourceRefs"][0]["entryHash"] =
        format!("sha256:{}", "0".repeat(64)).into();
    harness.write("draft.json", &draft);
    let error = harness
        .call(
            "storyboard",
            &[
                "submit",
                "--task",
                "storyboards/task.json",
                "--draft",
                "draft.json",
                "--output",
                "storyboards/board.json",
            ],
        )
        .unwrap_err();
    assert!(error.message.contains("PANEL_SOURCE_HASH_CHANGED"));
    assert!(!harness.project.join("storyboards/board.json").exists());
    let state = harness.read("state.json");
    fs::write(
        harness.project.join(
            state["source_snapshot"]["synopsis"]["path"]
                .as_str()
                .unwrap(),
        ),
        "source changed",
    )
    .unwrap();
    let error = harness.call("storyboard", &["scenes"]).unwrap_err();
    assert!(error.message.contains("SOURCE_CHANGED"));
}

#[cfg(unix)]
#[test]
fn held_sts_lock_and_nonregular_or_symlink_inputs_are_refused() {
    use fs2::FileExt;
    use std::os::unix::fs::symlink;
    let harness = Harness::new();
    let lock = fs::OpenOptions::new()
        .read(true)
        .write(true)
        .open(harness.project.join(".workflow.lock"))
        .unwrap();
    lock.lock_exclusive().unwrap();
    assert!(harness
        .call("storyboard", &["scenes"])
        .unwrap_err()
        .message
        .contains("PROJECT_BUSY"));
    fs2::FileExt::unlock(&lock).unwrap();
    let task = harness.task();
    harness.write("draft.json", &harness.draft(&task));
    symlink(
        harness.project.join("draft.json"),
        harness.project.join("symlink.json"),
    )
    .unwrap();
    let error = harness
        .call(
            "storyboard",
            &[
                "submit",
                "--task",
                "storyboards/task.json",
                "--draft",
                "symlink.json",
                "--output",
                "storyboards/board.json",
            ],
        )
        .unwrap_err();
    assert!(error.message.contains("REGULAR_FILE_REQUIRED"));
    let fifo = std::ffi::CString::new(harness.project.join("fifo.json").to_str().unwrap()).unwrap();
    assert_eq!(unsafe { libc::mkfifo(fifo.as_ptr(), 0o600) }, 0);
    let error = harness
        .call(
            "storyboard",
            &[
                "submit",
                "--task",
                "storyboards/task.json",
                "--draft",
                "fifo.json",
                "--output",
                "storyboards/board.json",
            ],
        )
        .unwrap_err();
    assert!(error.message.contains("REGULAR_FILE_REQUIRED"));
    fs::create_dir(harness.project.join("real-dir")).unwrap();
    symlink(
        harness.project.join("real-dir"),
        harness.project.join("storyboards/linked"),
    )
    .unwrap();
    assert!(harness
        .call(
            "storyboard",
            &[
                "prepare",
                "--task-id",
                "other",
                "--brief",
                "test",
                "--output",
                "storyboards/linked/task.json"
            ]
        )
        .is_err());
    assert!(!harness.project.join("real-dir/task.json").exists());
}

#[test]
fn animatic_candidate_review_packet_preserve_identity_and_original_captions() {
    let harness = Harness::new();
    harness.approved();
    let draft = harness.animatic_draft();
    harness.write("direction.json", &draft);
    let submitted = harness
        .call(
            "animatic",
            &[
                "submit",
                "--board",
                "storyboards/board.json",
                "--board-review",
                "storyboards/review.json",
                "--draft",
                "direction.json",
                "--output",
                "animatic/plan.json",
            ],
        )
        .unwrap();
    let plan = harness.read("animatic/plan.json");
    assert_hash(&plan);
    assert_eq!(submitted["status"], "candidate");
    let review_args = [
        "review",
        "--board",
        "storyboards/board.json",
        "--board-review",
        "storyboards/review.json",
        "--plan",
        "animatic/plan.json",
        "--reviewer",
        "SYNTHETIC_TEST_ONLY",
        "--expected-hash",
        submitted["contentHash"].as_str().unwrap(),
        "--decision",
        "approved",
        "--output",
        "animatic/review.json",
    ];
    let reviewed = harness.call("animatic", &review_args).unwrap();
    let bytes = fs::read(harness.project.join("animatic/review.json")).unwrap();
    assert_eq!(harness.call("animatic", &review_args).unwrap(), reviewed);
    assert_eq!(
        fs::read(harness.project.join("animatic/review.json")).unwrap(),
        bytes
    );
    let packet = harness
        .call(
            "animatic",
            &[
                "export",
                "--board",
                "storyboards/board.json",
                "--board-review",
                "storyboards/review.json",
                "--plan",
                "animatic/plan.json",
                "--review",
                "animatic/review.json",
                "--output",
                "animatic/packet.json",
            ],
        )
        .unwrap();
    assert_hash(&packet);
    assert_eq!(packet["plan"], plan);
    assert_eq!(packet["review"], reviewed);
    let scenes = harness.call("storyboard", &["scenes"]).unwrap();
    for caption in packet["captions"]
        .as_array()
        .unwrap()
        .iter()
        .flat_map(|panel| panel["lines"].as_array().unwrap())
    {
        let original = scenes["semanticOccurrences"]
            .as_array()
            .unwrap()
            .iter()
            .find(|entry| entry["semantic_id"] == caption["semanticId"])
            .unwrap();
        assert_eq!(caption["text"], original["source_text"]);
        assert_eq!(caption["entryHash"], original["entry_hash"]);
    }
    let mut canonical = narrative_courier_runtime::token::jcs_bytes(&packet).unwrap();
    canonical.push(b'\n');
    assert_eq!(
        fs::read(harness.project.join("animatic/packet.json")).unwrap(),
        canonical
    );
}

#[test]
fn animatic_invalid_keyframes_assets_and_unresolved_approval_are_refused() {
    let harness = Harness::new();
    harness.approved();
    let original = harness.animatic_draft();
    let mut wrong = original.clone();
    wrong["panels"][0]["camera"][1]["frame"] = 10.into();
    harness.write("direction.json", &wrong);
    let submit = [
        "submit",
        "--board",
        "storyboards/board.json",
        "--board-review",
        "storyboards/review.json",
        "--draft",
        "direction.json",
        "--output",
        "animatic/plan.json",
    ];
    assert!(harness
        .call("animatic", &submit)
        .unwrap_err()
        .message
        .contains("KEYFRAME_ENDPOINTS_ORDER"));
    let mut wrong = original.clone();
    wrong["assets"][0]["assetId"] = "unused".into();
    harness.write("direction.json", &wrong);
    assert!(harness
        .call("animatic", &submit)
        .unwrap_err()
        .message
        .contains("ASSET_COVERAGE"));
    let mut wrong = original;
    wrong["openQuestions"] = json!(["未解決"]);
    harness.write("direction.json", &wrong);
    let plan = harness.call("animatic", &submit).unwrap();
    assert!(harness
        .call(
            "animatic",
            &[
                "review",
                "--board",
                "storyboards/board.json",
                "--board-review",
                "storyboards/review.json",
                "--plan",
                "animatic/plan.json",
                "--reviewer",
                "TEST",
                "--expected-hash",
                plan["contentHash"].as_str().unwrap(),
                "--decision",
                "approved",
                "--output",
                "animatic/review.json"
            ]
        )
        .unwrap_err()
        .message
        .contains("UNRESOLVED_DIRECTION"));
    assert!(!harness.project.join("animatic/review.json").exists());
}

#[test]
fn malformed_cli_options_are_rejected_before_project_access() {
    for args in [
        vec!["scenes", "--project-root", "/missing", "--draft", "no"],
        vec![
            "prepare",
            "--project-root",
            "/missing",
            "--task-id",
            "a",
            "--task-id",
            "b",
            "--brief",
            "x",
            "--output",
            "storyboards/task.json",
        ],
        vec![
            "export",
            "--project-root",
            "/missing",
            "--board",
            "a",
            "--candidate",
            "--review",
            "b",
            "--output",
            "storyboards/board.md",
        ],
    ] {
        let error = execute("storyboard", args.into_iter().map(str::to_owned)).unwrap_err();
        assert!(!error.message.contains("NOT_FOUND"));
    }
}
