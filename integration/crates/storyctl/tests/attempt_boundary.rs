//! Public executable seams for attempt acknowledgments and run/recover races.
//! Fixtures never invoke an actual model or mutate a user workspace.
use serde_json::{json, Value};
use std::{
    fs,
    os::unix::fs::PermissionsExt,
    path::{Path, PathBuf},
    process::{Command, Output, Stdio},
    time::{Duration, Instant},
};
use storyctl::{
    adapters::owners::{
        validate_geultu_preparation, validate_geultu_submission, validate_geultu_task,
        validate_geultu_task_binding, validate_narrative_artifact, validate_narrative_submission,
        validate_storyboard_submission, validate_storyboard_task,
    },
    effects::json as codec,
};

fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .ancestors()
        .nth(3)
        .unwrap()
        .to_path_buf()
}
fn draft() -> Value {
    json!({"blocks":[{"id":"main","heading":"story","text":"synthetic body"}],
        "change_summary":"test only","assumptions":[],"open_questions":[]})
}
fn geultu_receipt() -> Value {
    json!({"commit":{"project_id":"probe","revision":3,"event_count":1,"idempotent":false},
        "result":{"candidate":{"id":"draft-one","task_id":"story-1","draft":draft()},
            "candidate_hash":"a".repeat(64)}})
}
fn geultu_preparation() -> Value {
    json!({"commit":{"project_id":"probe","revision":2,"event_count":1,"idempotent":false},
        "result":{"task":{"id":"story-1","target":"story","instruction":"test brief","cancelled":false,
            "inputs":{"source":{"kind":"source","blocks":[{"id":"source","heading":"source","text":"source fixture"}]}},
            "selection":[],"base":null,"brief":{},"guidance_hash":"b".repeat(64),"voice_guidelines":[]},
            "adoption_blockers":[],"output_schema":{"type":"object"}}})
}
struct Fixture {
    _temp: tempfile::TempDir,
    workspace: PathBuf,
    owner: PathBuf,
    nvc: PathBuf,
    writer: PathBuf,
    response: Vec<u8>,
}
impl Fixture {
    fn new() -> Self {
        let temp = tempfile::tempdir().unwrap();
        let workspace = temp.path().canonicalize().unwrap();
        fs::create_dir(workspace.join("writing")).unwrap();
        fs::write(
            workspace.join("story-project.json"),
            br#"{"schemaVersion":"story.text-project/1","projectId":"probe"}"#,
        )
        .unwrap();
        let response = serde_json::to_vec(&json!({"schemaVersion":"story.text-writer-result/1",
            "attemptId":"one","status":"succeeded","content":serde_json::to_string(&draft()).unwrap(),"error":null})).unwrap();
        let fixture = Self {
            owner: workspace.join("owner"),
            nvc: workspace.join("nvc"),
            writer: workspace.join("writer"),
            workspace,
            response,
            _temp: temp,
        };
        fixture.executable(
            &fixture.writer,
            &format!(
                "/bin/cat >/dev/null\nprintf called >> '{}'\nprintf '%s' '{}'",
                fixture.workspace.join("provider-calls").display(),
                String::from_utf8_lossy(&fixture.response)
            ),
        );
        fixture.owner(&format!(
            "printf '%s' '{}'",
            json!({"ok":true,"result":geultu_receipt()})
        ));
        fixture
    }
    fn executable(&self, path: &Path, body: &str) {
        fs::write(path, format!("#!/bin/sh\n{body}\n")).unwrap();
        fs::set_permissions(path, fs::Permissions::from_mode(0o700)).unwrap();
    }
    fn owner(&self, submit: &str) {
        self.owner_prepare(submit, &geultu_preparation());
    }
    fn owner_prepare(&self, submit: &str, prepare: &Value) {
        self.executable(&self.owner, &format!("/bin/cat >/dev/null\ncase \"$4\" in\n authoring.status) printf '%s' '{{\"ok\":true,\"result\":{{\"project_revision\":1,\"next\":{{\"kind\":\"story\"}},\"pending_candidates\":[],\"tasks\":[]}}}}';;\n authoring.prepare) printf '%s' '{}';;\n authoring.task) printf '%s' '{}';;\n authoring.submit) {submit};;\n *) printf '%s' '{{\"ok\":true,\"result\":{{}}}}';;\nesac",json!({"ok":true,"result":prepare}),json!({"ok":true,"result":geultu_preparation()["result"]})));
    }
    fn narrative(&self, submit: &Value, artifact: &Value, token: &str) {
        fs::create_dir_all(self.workspace.join("narrative/probe")).unwrap();
        self.executable(&self.owner, &format!("/bin/cat >/dev/null\ncase \"$4\" in\n authoring.status) printf '%s' '{{\"ok\":true,\"result\":{{\"pending_candidates\":[],\"tasks\":[],\"next\":{{}}}}}}';;\n authoring.export) printf '%s' '{{\"ok\":true,\"result\":{{\"markdown_sha256\":\"{}\"}}}}';;\nesac", "a".repeat(64)));
        self.executable(&self.nvc, &format!("/bin/cat >/dev/null\ncase \"$1\" in\n source-binding) printf '%s' '{}';;\n agent-task) printf '%s' '{}';;\n next) printf 'task text';;\n submit) printf '%s' '{}';;\n artifact) printf '%s' '{}';;\nesac",
            json!({"reviewed_synopsis":{"source_project_id":"probe","synopsis_sha256":"a".repeat(64)}}),
            json!({"status":"agent_task","node":"step_00","task_token":token,"output":"output/step_00.md"}), submit, artifact));
    }
    fn command(&self) -> Command {
        let mut command = Command::new(env!("CARGO_BIN_EXE_storyctl"));
        command.args([
            "--story-root",
            root().to_str().unwrap(),
            "text",
            "--workspace",
            self.workspace.to_str().unwrap(),
            "--geultu-bin",
            self.owner.to_str().unwrap(),
            "--nvc-bin",
            self.nvc.to_str().unwrap(),
        ]);
        command.env("PATH", "/story-test-no-interpreters");
        command
    }
    fn run(&self, writer: &Path) -> Command {
        let mut command = self.command();
        command.args([
            "run",
            "--attempt",
            "one",
            "--writer-program",
            writer.to_str().unwrap(),
            "--instruction",
            "test brief",
        ]);
        command
    }
    fn value(output: &Output) -> Value {
        codec::parse_object(&output.stdout)
            .unwrap_or_else(|_| panic!("{}", String::from_utf8_lossy(&output.stderr)))
    }
}
fn wait_for(path: &Path) {
    let deadline = Instant::now() + Duration::from_secs(5);
    while !path.exists() && Instant::now() < deadline {
        std::thread::sleep(Duration::from_millis(10));
    }
    assert!(path.exists(), "fixture did not reach {}", path.display());
}

#[test]
fn public_geultu_commit_and_candidate_must_match_the_submission() {
    let input = json!({"project_id":"probe","candidate_id":"draft-one","task_id":"story-1","draft":draft()});
    validate_geultu_submission(&geultu_receipt(), &input).unwrap();
    let mut replay = geultu_receipt();
    replay["commit"]["event_count"] = json!(0);
    replay["commit"]["idempotent"] = json!(true);
    validate_geultu_submission(&replay, &input).unwrap();
    for pointer in [
        "/commit",
        "/commit/revision",
        "/commit/idempotent",
        "/result/candidate/id",
        "/result/candidate/task_id",
        "/result/candidate/draft",
        "/result/candidate_hash",
    ] {
        let mut receipt = geultu_receipt();
        *receipt.pointer_mut(pointer).unwrap() = Value::Null;
        assert_eq!(
            validate_geultu_submission(&receipt, &input)
                .unwrap_err()
                .outcome
                .as_str(),
            "outcome_unknown",
            "{pointer}"
        );
    }
}

#[test]
fn public_geultu_prepare_and_task_reads_require_the_requested_packet_identity() {
    let input = json!({"project_id":"probe","task_id":"story-1","target":"story","instruction":"test brief"});
    let receipt = geultu_preparation();
    validate_geultu_preparation(&receipt, &input).unwrap();
    for pointer in [
        "/commit",
        "/result/task/id",
        "/result/task/target",
        "/result/task/instruction",
        "/result/task/cancelled",
        "/result/adoption_blockers",
        "/result/output_schema",
    ] {
        let mut bad = receipt.clone();
        *bad.pointer_mut(pointer).unwrap() = Value::Null;
        assert_eq!(
            validate_geultu_preparation(&bad, &input)
                .unwrap_err()
                .outcome
                .as_str(),
            "outcome_unknown",
            "{pointer}"
        );
    }
    let mut cancelled = receipt["result"].clone();
    cancelled["task"]["cancelled"] = json!(true);
    validate_geultu_task(&cancelled, "story-1").unwrap(); // Public cancelled-task inspection remains readable.
}

#[test]
fn task_binding_keeps_immutable_source_and_allows_owner_cancelled_candidate_retention() {
    let captured = geultu_preparation()["result"].clone();
    let mut late = captured.clone();
    late["task"]["cancelled"] = json!(true);
    late["adoption_blockers"] = json!(["task_cancelled"]);
    validate_geultu_task_binding(&captured, &late, "story-1").unwrap();
    for pointer in [
        "/task/inputs/source/blocks/0/text",
        "/task/instruction",
        "/task/brief",
        "/task/selection",
        "/task/base",
    ] {
        let mut changed = captured.clone();
        *changed.pointer_mut(pointer).unwrap() = json!("changed");
        assert_eq!(
            validate_geultu_task_binding(&captured, &changed, "story-1")
                .unwrap_err()
                .outcome
                .as_str(),
            "outcome_unknown",
            "{pointer}"
        );
    }
}

#[test]
fn malformed_prepare_is_unknown_before_the_writer_can_dispatch() {
    let fixture = Fixture::new();
    fixture.owner_prepare("printf '%s' '{\"ok\":true,\"result\":{}}'", &json!({}));
    let output = fixture.run(&fixture.writer).output().unwrap();
    assert!(String::from_utf8_lossy(&output.stderr).contains("outcome_unknown"));
    assert!(!fixture.workspace.join("provider-calls").exists());
    assert!(!fixture.workspace.join("attempts/one/request.json").exists());
}

#[test]
fn malformed_existing_task_packet_is_unknown_before_the_writer_can_dispatch() {
    let fixture = Fixture::new();
    fixture.executable(&fixture.owner,"/bin/cat >/dev/null\ncase \"$4\" in\n authoring.status) printf '%s' '{\"ok\":true,\"result\":{\"project_revision\":1,\"next\":{\"kind\":\"story\"},\"pending_candidates\":[],\"tasks\":[{\"id\":\"story-1\",\"target\":\"story\",\"cancelled\":false,\"has_candidates\":false}]}}';;\n authoring.task) printf '%s' '{\"ok\":true,\"result\":{}}';;\nesac");
    let output = fixture.run(&fixture.writer).output().unwrap();
    assert!(String::from_utf8_lossy(&output.stderr).contains("outcome_unknown"));
    assert!(!fixture.workspace.join("provider-calls").exists());
}

#[test]
fn public_nvc_receipt_requires_matching_node_output_version_and_observed_bytes() {
    let packet = json!({"node":"step_00","output":"output/step_00.md"});
    let receipt =
        json!({"node":"step_00","output":"output/step_00.md","version":1,"status":"in_progress"});
    let observed =
        json!({"item":{"node":"step_00","path":"output/step_00.md","version":1},"content":"body"});
    validate_narrative_submission(&receipt, &packet).unwrap();
    validate_narrative_artifact(&observed, &packet, "body", Some(&receipt)).unwrap();
    for invalid in [
        json!({}),
        json!({"node":"other","output":"output/step_00.md","version":1,"status":"in_progress"}),
        json!({"node":"step_00","output":"other","version":1,"status":"in_progress"}),
        json!({"node":"step_00","output":"output/step_00.md","version":0,"status":"in_progress"}),
    ] {
        assert!(validate_narrative_submission(&invalid, &packet).is_err());
    }
    for invalid in [
        json!({"content":"body"}),
        json!({"item":{"node":"other","path":"output/step_00.md","version":1},"content":"body"}),
        json!({"item":{"node":"step_00","path":"output/step_00.md","version":2},"content":"body"}),
        json!({"item":{"node":"step_00","path":"output/step_00.md","version":1},"content":"changed"}),
    ] {
        assert!(validate_narrative_artifact(&invalid, &packet, "body", Some(&receipt)).is_err());
    }
}

#[test]
fn storyboard_candidate_acknowledgment_requires_the_exact_public_path_and_hash() {
    let path = "storyboards/boards/one.json";
    let receipt = json!({"status":"candidate","path":path,"contentHash":format!("sha256:{}","a".repeat(64)),
        "sceneCount":1,"markdown":"candidate","diff":"","openQuestions":[]});
    validate_storyboard_submission(&receipt, path).unwrap();
    for pointer in ["/status", "/path", "/contentHash", "/sceneCount"] {
        let mut invalid = receipt.clone();
        *invalid.pointer_mut(pointer).unwrap() = Value::Null;
        assert!(
            validate_storyboard_submission(&invalid, path).is_err(),
            "{pointer}"
        );
    }
    let task = json!({"status":"ready_for_writing","task":{"contentHash":format!("sha256:{}","a".repeat(64))}});
    validate_storyboard_task(&task, &task["task"]).unwrap();
    validate_storyboard_task(&task, &Value::Null).unwrap();
    assert!(validate_storyboard_task(&json!({}), &task["task"]).is_err());
    assert_eq!(
        validate_storyboard_task(
            &task,
            &json!({"contentHash":format!("sha256:{}","b".repeat(64))})
        )
        .unwrap_err()
        .outcome
        .as_str(),
        "not_applied"
    );
}

#[test]
fn missing_owner_candidate_evidence_is_unknown_and_preserves_raw_writer_response() {
    let fixture = Fixture::new();
    fixture.owner("printf '%s' '{\"ok\":true,\"result\":{}}'");
    let output = fixture.run(&fixture.writer).output().unwrap();
    let value = Fixture::value(&output);
    assert_eq!(value["status"], "outcome_unknown");
    assert_eq!(output.status.code(), Some(3));
    assert_eq!(
        fs::read(fixture.workspace.join("attempts/one/response.json")).unwrap(),
        fixture.response
    );
    assert_eq!(
        fs::read_to_string(fixture.workspace.join("provider-calls")).unwrap(),
        "called"
    );
}

#[test]
fn invalid_saved_candidate_is_unknown_and_recovery_keeps_the_old_record_bytes() {
    let fixture = Fixture::new();
    let first = fixture.run(&fixture.writer).output().unwrap();
    assert_eq!(Fixture::value(&first)["status"], "candidate_stored");
    let result_path = fixture.workspace.join("attempts/one/result.json");
    let bad = br#"{"status":"candidate_stored","attempt":"one","result":{}}"#;
    fs::write(&result_path, bad).unwrap(); // Model a receipt emitted by the former bug.
    let replay = fixture.run(&fixture.writer).output().unwrap();
    assert_eq!(Fixture::value(&replay)["status"], "outcome_unknown");
    let recovered = fixture
        .command()
        .args(["recover", "--attempt", "one"])
        .output()
        .unwrap();
    assert_eq!(Fixture::value(&recovered)["status"], "candidate_stored");
    assert_eq!(fs::read(result_path).unwrap(), bad);
    assert_eq!(
        fs::read(fixture.workspace.join("attempts/one/response.json")).unwrap(),
        fixture.response
    );
    assert_eq!(
        fs::read_to_string(fixture.workspace.join("provider-calls")).unwrap(),
        "called"
    );
}

#[test]
fn an_unpinned_legacy_request_can_be_inspected_but_cannot_automatically_submit() {
    let fixture = Fixture::new();
    let submits = fixture.workspace.join("legacy-submits");
    fixture.owner(&format!(
        "printf submitted >> '{}'; printf '%s' '{}'",
        submits.display(),
        json!({"ok":true,"result":geultu_receipt()})
    ));
    let first = fixture.run(&fixture.writer).output().unwrap();
    assert_eq!(Fixture::value(&first)["status"], "candidate_stored");
    let request_path = fixture.workspace.join("attempts/one/request.json");
    let mut request = codec::parse_object(&fs::read(&request_path).unwrap()).unwrap();
    request["task"]
        .as_object_mut()
        .unwrap()
        .remove("storyProjectId");
    let legacy = codec::stable_bytes(&request).unwrap();
    fs::write(&request_path, &legacy).unwrap();
    fs::remove_file(fixture.workspace.join("attempts/one/result.json")).unwrap();
    let recovered = fixture
        .command()
        .args(["recover", "--attempt", "one"])
        .output()
        .unwrap();
    assert!(String::from_utf8_lossy(&recovered.stderr).contains("outcome_unknown"));
    assert!(
        String::from_utf8_lossy(&recovered.stderr).contains("no proven project routing identity")
    );
    assert_eq!(fs::read(&request_path).unwrap(), legacy);
    assert_eq!(
        fs::read(fixture.workspace.join("attempts/one/response.json")).unwrap(),
        fixture.response
    );
    assert_eq!(fs::read_to_string(submits).unwrap(), "submitted");
    assert_eq!(
        fs::read_to_string(fixture.workspace.join("provider-calls")).unwrap(),
        "called"
    );
}

#[test]
fn a_legacy_request_with_its_stored_public_commit_keeps_exact_acknowledgment_bytes() {
    let fixture = Fixture::new();
    let first = fixture.run(&fixture.writer).output().unwrap();
    assert_eq!(Fixture::value(&first)["status"], "candidate_stored");
    let request_path = fixture.workspace.join("attempts/one/request.json");
    let mut request = codec::parse_object(&fs::read(&request_path).unwrap()).unwrap();
    request["task"]
        .as_object_mut()
        .unwrap()
        .remove("storyProjectId");
    let legacy = codec::stable_bytes(&request).unwrap();
    fs::write(&request_path, &legacy).unwrap();
    let result_path = fixture.workspace.join("attempts/one/result.json");
    let acknowledgment = fs::read(&result_path).unwrap();
    let recovered = fixture
        .command()
        .args(["recover", "--attempt", "one"])
        .output()
        .unwrap();
    assert_eq!(Fixture::value(&recovered), Fixture::value(&first));
    assert_eq!(fs::read(result_path).unwrap(), acknowledgment);
    assert_eq!(fs::read(request_path).unwrap(), legacy);
    assert_eq!(
        fs::read_to_string(fixture.workspace.join("provider-calls")).unwrap(),
        "called"
    );
}

#[test]
fn invalid_saved_matching_artifact_is_observed_again_without_provider_execution() {
    let fixture = Fixture::new();
    let receipt =
        json!({"node":"step_00","output":"output/step_00.md","version":1,"status":"in_progress"});
    let artifact = json!({"item":{"node":"step_00","path":"output/step_00.md","version":1},"content":serde_json::to_string(&draft()).unwrap()});
    fixture.narrative(&receipt, &artifact, "token");
    let first = fixture.run(&fixture.writer).output().unwrap();
    assert_eq!(Fixture::value(&first)["status"], "candidate_stored");
    let result_path = fixture.workspace.join("attempts/one/result.json");
    let bad = br#"{"status":"matching_artifact_observed","attempt":"one","artifact":{}}"#;
    fs::write(&result_path, bad).unwrap();
    let replay = fixture.run(&fixture.writer).output().unwrap();
    assert_eq!(Fixture::value(&replay)["status"], "outcome_unknown");
    fixture.narrative(&receipt, &artifact, "changed-token");
    let recovered = fixture
        .command()
        .args(["recover", "--attempt", "one"])
        .output()
        .unwrap();
    assert_eq!(
        Fixture::value(&recovered)["status"],
        "matching_artifact_observed"
    );
    assert_eq!(fs::read(result_path).unwrap(), bad);
    assert_eq!(
        fs::read_to_string(fixture.workspace.join("provider-calls")).unwrap(),
        "called"
    );
}

#[test]
fn nvc_writer_submission_requires_a_public_receipt_and_matching_artifact() {
    for (receipt, artifact) in [
        (json!({}), json!({})),
        (
            json!({"node":"step_00","output":"output/step_00.md","version":1,"status":"in_progress"}),
            json!({"content":"same"}),
        ),
    ] {
        let fixture = Fixture::new();
        fixture.narrative(&receipt, &artifact, "token");
        let output = fixture.run(&fixture.writer).output().unwrap();
        assert_eq!(Fixture::value(&output)["status"], "outcome_unknown");
        assert_eq!(
            fs::read(fixture.workspace.join("attempts/one/response.json")).unwrap(),
            fixture.response
        );
    }
}

#[test]
fn recovery_cannot_claim_an_observed_nvc_artifact_without_its_public_item() {
    let fixture = Fixture::new();
    let receipt =
        json!({"node":"step_00","output":"output/step_00.md","version":1,"status":"in_progress"});
    let content = serde_json::to_string(&draft()).unwrap();
    let artifact =
        json!({"item":{"node":"step_00","path":"output/step_00.md","version":1},"content":content});
    fixture.narrative(&receipt, &artifact, "token");
    let output = fixture.run(&fixture.writer).output().unwrap();
    assert_eq!(Fixture::value(&output)["status"], "candidate_stored");
    fs::remove_file(fixture.workspace.join("attempts/one/result.json")).unwrap();
    fixture.narrative(&receipt, &json!({"content":content}), "changed-token");
    let recovered = fixture
        .command()
        .args(["recover", "--attempt", "one"])
        .output()
        .unwrap();
    assert!(String::from_utf8_lossy(&recovered.stderr).contains("outcome_unknown"));
    assert!(!fixture
        .workspace
        .join("attempts/one/recovery.json")
        .exists());
    assert_eq!(
        fs::read_to_string(fixture.workspace.join("provider-calls")).unwrap(),
        "called"
    );
}

#[test]
fn a_followup_owner_spawn_failure_cannot_make_a_dispatched_writer_not_started() {
    let fixture = Fixture::new();
    fixture.executable(
        &fixture.writer,
        &format!(
            "/bin/cat >/dev/null\nprintf called >> '{}'\n/bin/mv '{}' '{}'\nprintf '%s' '{}'",
            fixture.workspace.join("provider-calls").display(),
            fixture.owner.display(),
            fixture.workspace.join("owner-disabled").display(),
            String::from_utf8_lossy(&fixture.response)
        ),
    );
    let output = fixture.run(&fixture.writer).output().unwrap();
    let value = Fixture::value(&output);
    assert_eq!(value["status"], "outcome_unknown");
    assert_eq!(value["providerStatus"], "succeeded");
    assert_eq!(
        fs::read(fixture.workspace.join("attempts/one/response.json")).unwrap(),
        fixture.response
    );
    let recover = fixture
        .command()
        .args(["recover", "--attempt", "one"])
        .output()
        .unwrap();
    assert!(String::from_utf8_lossy(&recover.stderr).contains("outcome_unknown"));
}

#[test]
fn a_genuine_provider_spawn_failure_remains_not_started() {
    let fixture = Fixture::new();
    let output = fixture
        .run(&fixture.workspace.join("missing-provider"))
        .output()
        .unwrap();
    assert_eq!(Fixture::value(&output)["status"], "not_started");
    assert!(!fixture.workspace.join("provider-calls").exists());
}

#[test]
fn a_writer_success_followed_by_explicit_owner_refusal_exits_two() {
    let fixture = Fixture::new();
    fixture.owner("printf '%s' '{\"status\":\"not_applied\",\"error\":\"fixture refusal before commit\"}' >&2; exit 2");
    let output = fixture.run(&fixture.writer).output().unwrap();
    let value = Fixture::value(&output);
    assert_eq!(value["status"], "not_applied");
    assert_eq!(value["providerStatus"], "succeeded");
    assert_eq!(output.status.code(), Some(2));
    assert_eq!(
        fs::read(fixture.workspace.join("attempts/one/response.json")).unwrap(),
        fixture.response
    );
    assert_eq!(
        fs::read_to_string(fixture.workspace.join("provider-calls")).unwrap(),
        "called"
    );
}

#[test]
fn early_recover_cannot_leave_a_lock_in_a_run_reservation_without_a_request() {
    let fixture = Fixture::new();
    let ready = fixture.workspace.join("status-ready");
    let release = fixture.workspace.join("release-status");
    fixture.executable(&fixture.owner,&format!("/bin/cat >/dev/null\ncase \"$4\" in\n authoring.status) : > '{}'; while [ ! -e '{}' ]; do /bin/sleep 0.01; done; printf '%s' '{{\"ok\":true,\"result\":{{\"pending_candidates\":[],\"tasks\":[],\"next\":{{}}}}}}';;\n authoring.export) printf '%s' '{{\"ok\":true,\"result\":{{\"markdown\":\"synopsis\"}}}}';;\nesac",ready.display(),release.display()));
    let running = fixture
        .run(&fixture.writer)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    wait_for(&ready);
    let recover = fixture
        .command()
        .args(["recover", "--attempt", "one"])
        .output()
        .unwrap();
    assert_eq!(recover.status.code(), Some(2));
    assert!(!fixture
        .workspace
        .join("attempts/one/submission.lock")
        .exists());
    fs::write(release, b"release").unwrap();
    let output = running.wait_with_output().unwrap();
    assert_eq!(Fixture::value(&output)["status"], "awaiting_handoff");
    assert!(!fixture.workspace.join("attempts/one").exists());
    assert!(!fixture.workspace.join("provider-calls").exists());
}

#[test]
fn concurrent_run_and_recover_submit_only_once_and_replay_the_saved_response() {
    let fixture = Fixture::new();
    let ready = fixture.workspace.join("submit-ready");
    let release = fixture.workspace.join("release-submit");
    let calls = fixture.workspace.join("submit-calls");
    fixture.owner(&format!("printf submitted >> '{}'; : > '{}'; while [ ! -e '{}' ]; do /bin/sleep 0.01; done; printf '%s' '{}'",
        calls.display(),ready.display(),release.display(),json!({"ok":true,"result":geultu_receipt()})));
    let running = fixture
        .run(&fixture.writer)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    wait_for(&ready);
    let recover = fixture
        .command()
        .args(["recover", "--attempt", "one"])
        .output()
        .unwrap();
    assert!(String::from_utf8_lossy(&recover.stderr).contains("outcome_unknown"));
    fs::write(release, b"release").unwrap();
    let result = running.wait_with_output().unwrap();
    assert_eq!(Fixture::value(&result)["status"], "candidate_stored");
    let recovered = fixture
        .command()
        .args(["recover", "--attempt", "one"])
        .output()
        .unwrap();
    assert_eq!(Fixture::value(&recovered), Fixture::value(&result));
    assert_eq!(fs::read_to_string(calls).unwrap(), "submitted");
    assert_eq!(
        fs::read_to_string(fixture.workspace.join("provider-calls")).unwrap(),
        "called"
    );
}

#[test]
fn cancel_after_owner_ack_helper() {
    if std::env::var("STORY_TEST_CANCEL_AFTER_OWNER_ACK").as_deref() != Ok("1") {
        return;
    }
    use storyctl::{
        error::Result,
        ports::{NarrativeOwner, WritingOwner},
        workflows::{
            attempts::{self, RunOptions},
            text::TextContext,
        },
    };
    struct Writing;
    impl WritingOwner for Writing {
        fn initialize(&self) -> Result<()> {
            Ok(())
        }
        fn call(&self, operation: &str, _: &Value) -> Result<Value> {
            Ok(match operation {
                "authoring.status" => {
                    json!({"project_revision":1,"next":{"kind":"story"},"pending_candidates":[],"tasks":[]})
                }
                "authoring.prepare" => geultu_preparation(),
                "authoring.task" => geultu_preparation()["result"].clone(),
                "authoring.submit" => {
                    // A real signal at the port-return boundary models cancellation
                    // after transport completion and before durable acknowledgment.
                    assert_eq!(unsafe { libc::raise(libc::SIGTERM) }, 0);
                    geultu_receipt()
                }
                other => panic!("unexpected owner operation {other}"),
            })
        }
    }
    struct Narrative;
    impl NarrativeOwner for Narrative {
        fn text(&self, _: &str, _: &str, _: &[String]) -> Result<String> {
            unreachable!()
        }
        fn tool(&self, _: &str, _: &[String], _: &Path) -> Result<Value> {
            unreachable!()
        }
    }
    let fixture = Fixture::new();
    storyctl::effects::process::install_signal_handlers();
    let context = TextContext::with_owners(
        root(),
        fixture.workspace.clone(),
        Box::new(Writing),
        Box::new(Narrative),
        None,
    )
    .unwrap();
    let value = attempts::run(
        &context,
        &RunOptions {
            attempt: "one".into(),
            writer: fixture.writer.clone(),
            arguments: vec![],
            timeout: 5,
            instruction: "test brief".into(),
            storyboard_task: None,
        },
    )
    .unwrap();
    assert!(storyctl::effects::process::cancellation_requested());
    assert_eq!(value["status"], "outcome_unknown");
    assert_eq!(value["providerStatus"], "succeeded");
    assert_eq!(
        fs::read(fixture.workspace.join("attempts/one/response.json")).unwrap(),
        fixture.response
    );
    assert_eq!(
        fs::read_to_string(fixture.workspace.join("provider-calls")).unwrap(),
        "called"
    );
}

#[test]
fn late_local_cancellation_cannot_publish_a_fresh_candidate_success() {
    let output = Command::new(std::env::current_exe().unwrap())
        .args(["--exact", "cancel_after_owner_ack_helper", "--nocapture"])
        .env("STORY_TEST_CANCEL_AFTER_OWNER_ACK", "1")
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "{}\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
}
