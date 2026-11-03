//! Immutable producer receipts and source observations remain separate authorities.
use serde_json::{json, Value};
use std::{
    fs,
    os::unix::fs::PermissionsExt,
    path::{Path, PathBuf},
    sync::{
        atomic::{AtomicUsize, Ordering},
        Arc,
    },
};
use storyctl::{
    error::{Error, Result},
    ports::{NarrativeOwner, WritingOwner},
    workflows::{animatic, text::TextContext},
};

fn repository() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .ancestors()
        .nth(3)
        .unwrap()
        .to_path_buf()
}
fn packet() -> Value {
    serde_json::from_slice(
        &fs::read(
            repository().join(
                "NarrativeCourier/crates/authoring/tests/fixtures/legacy-animatic-packet.json",
            ),
        )
        .unwrap(),
    )
    .unwrap()
}
fn candidate(attempt: &str) -> Value {
    let hash = format!("sha256:{}", "a".repeat(64));
    json!({"schemaVersion":"smc.animatic-result/1","attemptId":attempt,"status":"candidate","requestHash":hash,"contentHash":hash,
        "finalApproval":false,"audio":"none","frameCount":1,"fps":12,"width":64,"height":64,
        "artifacts":(0..5).map(|index|json!({"path":format!("animatic/renders/{attempt}/{index}"),"sha256":hash})).collect::<Vec<_>>()})
}
struct Writing {
    mode: &'static str,
    calls: AtomicUsize,
}
impl WritingOwner for Writing {
    fn initialize(&self) -> Result<()> {
        Ok(())
    }
    fn call(&self, _: &str, _: &Value) -> Result<Value> {
        let call = self.calls.fetch_add(1, Ordering::SeqCst);
        if call > 0 && self.mode == "unobserved" {
            return Err(Error::unknown("Writing observation unavailable"));
        }
        Ok(
            json!({"markdown_sha256": if call > 0 && self.mode == "changed" { "b".repeat(64) } else {"a".repeat(64)}}),
        )
    }
}
struct Narrative {
    mode: &'static str,
    calls: Arc<AtomicUsize>,
    asset_root: PathBuf,
}
impl NarrativeOwner for Narrative {
    fn text(&self, _: &str, _: &str, _: &[String]) -> Result<String> {
        Ok(json!({"reviewed_synopsis":{"source_project_id":"test","synopsis_sha256":"a".repeat(64)}}).to_string())
    }
    fn tool(&self, _: &str, _: &[String], _: &Path) -> Result<Value> {
        let call = self.calls.fetch_add(1, Ordering::SeqCst);
        let mut packet = packet();
        if (call == 0 && self.mode == "root-before-dispatch")
            || (call == 1 && self.mode == "root-after-observation")
        {
            fs::rename(
                &self.asset_root,
                self.asset_root.with_file_name("original-assets"),
            )
            .unwrap();
            fs::create_dir(&self.asset_root).unwrap();
        }
        if call > 0 {
            match self.mode {
                "packet-drift" => {
                    packet["captions"][0]["soundNotes"] =
                        json!("changed with the same claimed digest")
                }
                "packet-unknown" => {
                    packet = json!({"status":"outcome_unknown","contentHash":packet["contentHash"]})
                }
                "packet-rejected" => packet["review"]["decision"] = json!("rejected"),
                "packet-unreachable" => return Err(Error::unknown("NVC unavailable")),
                _ => {}
            }
        }
        Ok(packet)
    }
}
struct Fixture {
    _temp: tempfile::TempDir,
    root: PathBuf,
    ctx: TextContext,
    owner: PathBuf,
    exports: Arc<AtomicUsize>,
}
impl Fixture {
    fn new(mode: &'static str, render: &Value, observed: &Value, code: i32) -> Self {
        let temp = tempfile::tempdir().unwrap();
        let workspace = temp.path().canonicalize().unwrap();
        let root = workspace.join("assets");
        fs::create_dir(&root).unwrap();
        fs::write(
            workspace.join("story-project.json"),
            br#"{"schemaVersion":"story.text-project/1","projectId":"test"}"#,
        )
        .unwrap();
        let owner = workspace.join("smc");
        fs::write(
            workspace.join("render.json"),
            serde_json::to_vec(render).unwrap(),
        )
        .unwrap();
        fs::write(
            workspace.join("status.json"),
            serde_json::to_vec(observed).unwrap(),
        )
        .unwrap();
        let replace = if mode == "root-after-dispatch" {
            format!("if [ \"$1\" != status ]; then /bin/mv '{}' '{}'; /bin/mkdir '{}'; printf original-effect > producer-call; fi\n", root.display(), root.with_file_name("original-assets").display(), root.display())
        } else {
            String::new()
        };
        fs::write(&owner, format!("#!/bin/sh\n{replace}if [ \"$1\" = status ]; then /bin/cat '{}'; else /bin/cat '{}'; fi\nexit {code}\n", workspace.join("status.json").display(), workspace.join("render.json").display())).unwrap();
        fs::set_permissions(&owner, fs::Permissions::from_mode(0o700)).unwrap();
        let exports = Arc::new(AtomicUsize::new(0));
        let ctx = TextContext::with_owners(
            repository(),
            workspace,
            Box::new(Writing {
                mode,
                calls: AtomicUsize::new(0),
            }),
            Box::new(Narrative {
                mode,
                calls: exports.clone(),
                asset_root: root.clone(),
            }),
            None,
        )
        .unwrap();
        Self {
            _temp: temp,
            root,
            ctx,
            owner,
            exports,
        }
    }
    fn render(&self) -> Result<Value> {
        let args = [
            "render",
            "--asset-root",
            self.root.to_str().unwrap(),
            "--smc-bin",
            self.owner.to_str().unwrap(),
            "--board",
            "board",
            "--board-review",
            "board-review",
            "--plan",
            "plan",
            "--review",
            "review",
            "--assets",
            "assets",
            "--asset-review",
            "asset-review",
            "--attempt",
            "selected",
        ];
        animatic::run(&self.ctx, &args.map(str::to_owned))
    }
}

#[test]
fn actual_source_drift_and_unreachable_observations_are_distinct() {
    let render = candidate("selected");
    for (mode, status, source) in [
        ("current", "candidate", "current"),
        ("changed", "source_stale", "changed"),
        ("unobserved", "outcome_unknown", "unobserved"),
        ("packet-drift", "source_stale", "changed"),
        ("packet-unknown", "outcome_unknown", "unobserved"),
        ("packet-rejected", "outcome_unknown", "unobserved"),
        ("packet-unreachable", "outcome_unknown", "unobserved"),
    ] {
        let fixture = Fixture::new(mode, &render, &render, 0);
        let result = fixture.render().unwrap();
        assert_eq!(result["status"], status, "{mode}: {result}");
        assert_eq!(result["sourceStatus"], source, "{mode}: {result}");
        assert_eq!(result["render"], render);
    }
}

#[test]
fn wrong_attempt_malformed_candidate_or_unconfirmed_request_never_becomes_current() {
    let render = candidate("selected");
    for bad in [
        json!({}),
        json!({"status":"candidate"}),
        candidate("another"),
    ] {
        let fixture = Fixture::new("current", &bad, &bad, 0);
        assert!(fixture.render().is_err());
    }
    for observed in [
        candidate("another"),
        {
            let mut v = render.clone();
            v["requestHash"] = json!(format!("sha256:{}", "b".repeat(64)));
            v
        },
        json!({}),
    ] {
        let result = Fixture::new("current", &render, &observed, 0)
            .render()
            .unwrap();
        assert_eq!(result["status"], "outcome_unknown");
        assert_eq!(result["render"], render);
    }
}

#[test]
fn nonzero_terminal_receipts_are_returned_without_resealing_or_reclassifying() {
    for status in ["failed", "cancelled", "outcome_unknown"] {
        let mut receipt = candidate("selected");
        receipt.as_object_mut().unwrap().retain(|key, _| {
            [
                "schemaVersion",
                "attemptId",
                "requestHash",
                "contentHash",
                "status",
            ]
            .contains(&key.as_str())
        });
        receipt["status"] = json!(status);
        receipt["detail"] = json!("Exact producer observation");
        let result = Fixture::new("current", &receipt, &receipt, 3)
            .render()
            .unwrap();
        assert_eq!(result, receipt);
    }
    for bad in [
        json!({}),
        json!({"status":"running","attemptId":"selected"}),
    ] {
        let fixture = Fixture::new("current", &bad, &bad, 0);
        let args = [
            "status",
            "--asset-root",
            fixture.root.to_str().unwrap(),
            "--smc-bin",
            fixture.owner.to_str().unwrap(),
            "--attempt",
            "selected",
        ];
        assert!(animatic::run(&fixture.ctx, &args.map(str::to_owned)).is_err());
    }
}

#[test]
fn not_started_receipt_requires_the_selected_attempt_and_preserves_producer_terminal() {
    let matching = json!({
        "status":"not_started",
        "attemptId":"selected",
        "detail":"SMC did not start this attempt"
    });
    let result = Fixture::new("current", &matching, &matching, 2)
        .render()
        .unwrap();
    assert_eq!(result, matching);

    for receipt in [
        json!({"status":"not_started","detail":"SMC did not start this attempt"}),
        json!({"status":"not_started","attemptId":"another","detail":"SMC did not start this attempt"}),
    ] {
        let error = Fixture::new("current", &receipt, &receipt, 2)
            .render()
            .unwrap_err();
        assert_eq!(error.outcome, story_integration_core::Outcome::Unknown);
        assert_eq!(
            serde_json::from_slice::<Value>(&error.stdout).unwrap(),
            receipt
        );
    }
}

#[test]
fn invalid_render_options_never_export_or_publish_a_packet() {
    let receipt = candidate("selected");
    for ending in [
        vec![],
        vec!["--assets", "--asset-review", "review"],
        vec![
            "--assets",
            "assets",
            "--asset-review",
            "review",
            "--attempt",
            "selected",
            "--unknown",
            "value",
        ],
        vec![
            "--assets",
            "assets",
            "--asset-review",
            "review",
            "--attempt",
            "selected",
            "--motion",
            "invalid",
        ],
    ] {
        let fixture = Fixture::new("current", &receipt, &receipt, 0);
        let mut args = vec![
            "render",
            "--asset-root",
            fixture.root.to_str().unwrap(),
            "--smc-bin",
            fixture.owner.to_str().unwrap(),
            "--board",
            "board",
            "--board-review",
            "board-review",
            "--plan",
            "plan",
            "--review",
            "review",
        ];
        args.extend(ending);
        assert!(animatic::run(
            &fixture.ctx,
            &args.into_iter().map(str::to_owned).collect::<Vec<_>>()
        )
        .is_err());
        assert_eq!(fixture.exports.load(Ordering::SeqCst), 0);
        assert!(!fixture.root.join("animatic").exists());
    }
}

#[test]
fn explicit_equals_options_keep_flag_shaped_values_as_values() {
    let receipt = candidate("selected");
    let fixture = Fixture::new("current", &receipt, &receipt, 0);
    let args = vec![
        "render".into(),
        format!("--asset-root={}", fixture.root.display()),
        format!("--smc-bin={}", fixture.owner.display()),
        "--board=--board-value".into(),
        "--board-review=board-review".into(),
        "--plan=plan".into(),
        "--review=review".into(),
        "--assets=assets".into(),
        "--asset-review=asset-review".into(),
        "--attempt=selected".into(),
    ];
    assert_eq!(
        animatic::run(&fixture.ctx, &args).unwrap()["sourceStatus"],
        "current"
    );
}

#[test]
fn large_unobserved_detail_stays_within_the_story_envelope_without_changing_receipt() {
    let receipt = candidate("selected");
    let observation =
        animatic::observe(receipt.clone(), "unobserved", &"관찰 실패".repeat(10000)).unwrap();
    storyctl::contracts::SchemaRegistry::new(&repository().join("integration/contracts"))
        .unwrap()
        .validate(
            "https://story-production.local/contracts/animatic-observation-v1",
            &observation,
        )
        .unwrap();
    assert_eq!(observation["render"], receipt);
}

#[test]
fn asset_acknowledgments_need_the_requested_public_identity_but_keep_terminal_receipts() {
    let hash = format!("sha256:{}", "a".repeat(64));
    for receipt in [
        json!({}),
        json!({"status":"candidate","path":"wrong","contentHash":hash}),
    ] {
        let fixture = Fixture::new("current", &receipt, &receipt, 0);
        let args = [
            "assets",
            "--asset-root",
            fixture.root.to_str().unwrap(),
            "--sac-bin",
            fixture.owner.to_str().unwrap(),
            "submit",
            "--output",
            "selected.json",
        ];
        assert!(animatic::run(&fixture.ctx, &args.map(str::to_owned)).is_err());
    }
    for receipt in [
        json!({"status":"candidate","path":"selected.json","contentHash":hash}),
        json!({"status":"outcome_unknown","detail":"Original owner observation"}),
    ] {
        let fixture = Fixture::new("current", &receipt, &receipt, 0);
        let args = [
            "assets",
            "--asset-root",
            fixture.root.to_str().unwrap(),
            "--sac-bin",
            fixture.owner.to_str().unwrap(),
            "submit",
            "--output",
            "selected.json",
        ];
        assert_eq!(
            animatic::run(&fixture.ctx, &args.map(str::to_owned)).unwrap(),
            receipt
        );
    }
}

#[test]
fn asset_root_replacement_during_export_never_receives_a_packet_or_producer_dispatch() {
    let receipt = candidate("selected");
    let fixture = Fixture::new("root-before-dispatch", &receipt, &receipt, 0);
    let error = fixture.render().unwrap_err();
    assert_eq!(error.outcome, story_integration_core::Outcome::NotApplied);
    assert!(!fixture.root.join("animatic").exists());
    assert!(!fixture
        .root
        .with_file_name("original-assets")
        .join("animatic")
        .exists());
}

#[test]
fn asset_root_replacement_after_dispatch_keeps_effects_in_original_root_and_receipt_unknown() {
    let receipt = candidate("selected");
    let fixture = Fixture::new("root-after-dispatch", &receipt, &receipt, 0);
    let observed = fixture.render().unwrap();
    assert_eq!(observed["status"], "outcome_unknown");
    assert_eq!(observed["render"], receipt);
    assert!(fixture
        .root
        .with_file_name("original-assets")
        .join("producer-call")
        .exists());
    assert!(!fixture.root.join("producer-call").exists());
    assert!(!fixture.root.join("animatic").exists());
    let fixture = Fixture::new("root-after-observation", &receipt, &receipt, 0);
    let observed = fixture.render().unwrap();
    assert_eq!(observed["status"], "outcome_unknown");
    assert_eq!(observed["render"], receipt);
    assert!(!fixture.root.join("animatic").exists());
}

#[test]
fn missing_effectful_lane_executable_stops_before_source_queries_or_packet_publication() {
    let receipt = candidate("selected");
    for operation in ["render", "assets"] {
        let fixture = Fixture::new("current", &receipt, &receipt, 0);
        fs::remove_file(&fixture.owner).unwrap();
        let result = if operation == "render" {
            fixture.render()
        } else {
            animatic::run(
                &fixture.ctx,
                &[
                    "assets".into(),
                    "--asset-root".into(),
                    fixture.root.display().to_string(),
                    "--sac-bin".into(),
                    fixture.owner.display().to_string(),
                    "submit".into(),
                    "--output".into(),
                    "selected.json".into(),
                ],
            )
        };
        assert_eq!(
            result.unwrap_err().outcome,
            story_integration_core::Outcome::NotStarted
        );
        assert_eq!(fixture.exports.load(Ordering::SeqCst), 0);
        assert!(!fixture.root.join("animatic").exists());
    }
    let fixture = Fixture::new("current", &receipt, &receipt, 0);
    fs::remove_file(&fixture.owner).unwrap();
    let args = [
        "packet",
        "--asset-root",
        fixture.root.to_str().unwrap(),
        "--smc-bin",
        fixture.owner.to_str().unwrap(),
        "--board",
        "board",
        "--board-review",
        "board-review",
        "--plan",
        "plan",
        "--review",
        "review",
    ];
    assert_eq!(
        animatic::run(&fixture.ctx, &args.map(str::to_owned)).unwrap()["status"],
        "exported"
    );
}

#[test]
fn late_root_change_keeps_terminal_and_asset_receipts_in_unknown_error_observation() {
    let hash = format!("sha256:{}", "a".repeat(64));
    let terminal = json!({"schemaVersion":"smc.animatic-result/1","attemptId":"selected","requestHash":hash,"contentHash":hash,
        "status":"failed","detail":"Original producer failure"});
    let fixture = Fixture::new("root-after-dispatch", &terminal, &terminal, 0);
    let error = fixture.render().unwrap_err();
    assert_eq!(error.outcome, story_integration_core::Outcome::Unknown);
    assert_eq!(
        storyctl::effects::json::parse_object(&error.stdout).unwrap(),
        terminal
    );
    let asset = json!({"status":"candidate","path":"selected.json","contentHash":hash});
    let fixture = Fixture::new("root-after-dispatch", &asset, &asset, 0);
    let args = [
        "assets",
        "--asset-root",
        fixture.root.to_str().unwrap(),
        "--sac-bin",
        fixture.owner.to_str().unwrap(),
        "submit",
        "--output",
        "selected.json",
    ];
    let error = animatic::run(&fixture.ctx, &args.map(str::to_owned)).unwrap_err();
    assert_eq!(error.outcome, story_integration_core::Outcome::Unknown);
    assert_eq!(
        storyctl::effects::json::parse_object(&error.stdout).unwrap(),
        asset
    );
    assert!(!fixture.root.join("producer-call").exists());
    assert!(fixture
        .root
        .with_file_name("original-assets")
        .join("producer-call")
        .exists());
}
