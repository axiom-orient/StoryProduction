//! M4 integration tests: NVC admission boundary (H7) — fail-closed
//! `visual.admit_package` preflight, identity recomputation, and run-state
//! journaling.

mod support;

use geultu::{Application, Error, Workspace};
use serde_json::{json, Value};
use std::fs;
use support::unique_temp_id;

fn workspace_app(label: &str) -> (std::path::PathBuf, Application) {
    let root = std::env::temp_dir().join(format!(
        "geultu-m4-{label}-{}-{}",
        std::process::id(),
        unique_temp_id()
    ));
    let _ = fs::remove_dir_all(&root);
    let workspace = Workspace::init(&root).unwrap();
    (root.clone(), Application::from_workspace(workspace))
}

fn journal(root: &std::path::Path, project_id: &str) -> String {
    fs::read_to_string(root.join(format!(".geultu/projects/{project_id}/events.ndjson")))
        .unwrap_or_default()
}

/// Builds a project through the M1 exporter path and returns the exported
/// flat package (with its deterministic np_ identity).
fn exported_package(app: &Application) -> Value {
    app.call(
        "project.create",
        json!({"project_id": "book", "title": "Example", "command_id": "create"}),
    )
    .unwrap();
    app.call(
        "scene.add",
        json!({
            "project_id": "book",
            "scene_id": "scene_001",
            "title": "시작 장면",
            "text": "주인공이 문을 연다.",
            "order": 1,
            "command_id": "scene-1"
        }),
    )
    .unwrap();
    let export = app
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
    export["story_package"].clone()
}

fn route_admission() -> Value {
    json!({
        "route_id": "NVC",
        "command_id": "visual.admit_package",
        "input_refs": {
            "synopsis": {"id": "text:synopsis", "kind": "synopsis", "revision": 1,
                         "digest": "sha256:aa000000000000000000000000000000000000000000000000000000000000ee"}
        },
        "required_receipts": [],
        "capabilities": [],
        "budget": {"time_ms": 1000, "output_bytes": 4096}
    })
}

fn admit_request(package: Value) -> Value {
    json!({
        "project_id": "book",
        "run_id": "vrun1",
        "stage": "S00",
        "package": package,
        "route_admission": route_admission(),
        "command_id": "admit-1"
    })
}

#[test]
fn admit_valid_package_records_ready_run_with_receipt() {
    let (root, app) = workspace_app("valid");
    let package = exported_package(&app);

    let result = app
        .call("visual.admit_package", admit_request(package.clone()))
        .expect("admission of a verified package must succeed");

    let checks: Vec<&str> = result["preflight"]
        .as_array()
        .unwrap()
        .iter()
        .map(|check| check["check"].as_str().unwrap())
        .collect();
    assert_eq!(
        checks,
        vec![
            "input_shape",
            "t33_contract",
            "package_id_recomputation",
            "current_source_binding",
            "route_admission_shape",
            "run_state_shape",
            "not_already_admitted",
        ],
        "preflight checklist must be explicit and complete: {checks:?}"
    );

    let run = &result["run"];
    assert_eq!(run["schema_id"], "geultu.dev/visual-run-state/v1");
    assert_eq!(run["status"], "READY");
    assert_eq!(run["stage"], "S00");
    assert_eq!(run["revision"], 0);
    let package_id = package["package_id"].as_str().unwrap();
    assert_eq!(
        run["narrative_package"]["id"],
        format!("story:{package_id}")
    );
    assert_eq!(
        run["narrative_package"]["digest"],
        format!("sha256:{}", &package_id[3..]),
        "the run's package reference must commit to the exact np_ identity bytes"
    );
    assert_eq!(result["event_count"], 2);
    assert_eq!(result["idempotent"], false);

    let receipt = &result["receipt"];
    let receipt_id = receipt["receipt_id"].as_str().unwrap();
    assert!(receipt_id.starts_with("vr_") && receipt_id.len() == 67);
    assert_eq!(receipt["outcome"], "PASS");
    assert_eq!(receipt["validator"]["id"], "geultu-nvc-admission");
    assert_eq!(receipt["subject"], run["narrative_package"]);

    let after = app
        .call("project.get", json!({"project_id": "book"}))
        .unwrap();
    assert_eq!(after["visual_runs"]["vrun1"]["status"], "READY");
    assert_eq!(
        after["validation_receipts"][receipt_id]["outcome"], "PASS",
        "the admission receipt must be recorded alongside the transition"
    );

    let journaled = journal(&root, "book");
    assert!(
        journaled.contains("\"visual_run_admitted\""),
        "transition must be journaled: {journaled}"
    );
    assert!(
        journaled.contains("\"validation_receipt_recorded\""),
        "receipt must be journaled with the transition"
    );
    let _ = fs::remove_dir_all(root);
}

#[test]
fn tampered_package_id_is_rejected_before_any_journal_write() {
    let (root, app) = workspace_app("tampered");
    let mut package = exported_package(&app);
    let forged = format!("np_{}", "b".repeat(64));
    package["package_id"] = json!(forged);

    let journal_before = journal(&root, "book");
    let error = app
        .call("visual.admit_package", admit_request(package))
        .unwrap_err();
    assert!(
        matches!(&error, Error::InvalidInput(message)
            if message.contains("package_id_recomputation") && message.contains("recomputed")),
        "unexpected error: {error:?}"
    );
    assert_eq!(
        journal(&root, "book"),
        journal_before,
        "rejected admission must leave the journal untouched"
    );
    let _ = fs::remove_dir_all(root);
}

#[test]
fn correctly_rehashed_package_from_stale_source_is_rejected_before_journaling() {
    let (root, app) = workspace_app("stale-source");
    let mut package = exported_package(&app);
    package["scenes"][0]["narrative"] = json!("stale narrative");
    let mut identity_body = package.clone();
    identity_body.as_object_mut().unwrap().remove("package_id");
    package["package_id"] = json!(format!(
        "np_{}",
        geultu::contracts::digest::jcs_sha256(&identity_body).unwrap()
    ));

    let journal_before = journal(&root, "book");
    let error = app
        .call("visual.admit_package", admit_request(package))
        .unwrap_err();
    assert!(
        matches!(&error, Error::Conflict(message) if message.contains("current_source_binding")),
        "unexpected error: {error:?}"
    );
    assert_eq!(
        journal(&root, "book"),
        journal_before,
        "a correctly rehashed but stale package must not be admitted"
    );
    let _ = fs::remove_dir_all(root);
}

#[test]
fn legacy_envelope_shape_is_rejected_fail_closed() {
    let (root, app) = workspace_app("legacy");
    exported_package(&app);
    let legacy = json!({
        "schema_id": "geultu.dev/geultu-story-package-envelope/v1",
        "artifact_ref": {
            "id": "text:master-001", "kind": "text_master", "revision": 1,
            "digest": "sha256:1111111111111111111111111111111111111111111111111111111111111111"
        },
        "body": {"scenes": []}
    });

    let journal_before = journal(&root, "book");
    let error = app
        .call("visual.admit_package", admit_request(legacy))
        .unwrap_err();
    assert!(
        matches!(&error, Error::InvalidInput(message) if message.contains("unknown field")),
        "legacy envelope must be rejected at deserialization: {error:?}"
    );
    assert_eq!(
        journal(&root, "book"),
        journal_before,
        "rejected admission must leave the journal untouched"
    );
    let _ = fs::remove_dir_all(root);
}

#[test]
fn double_admission_is_idempotent_for_identical_state_and_conflict_otherwise() {
    let (root, app) = workspace_app("double-admit");
    let package = exported_package(&app);

    let first = app
        .call("visual.admit_package", admit_request(package.clone()))
        .unwrap();
    assert_eq!(first["idempotent"], false);

    // Identical replay under a fresh command id: zero new events.
    let mut replay = admit_request(package.clone());
    replay["command_id"] = json!("admit-replay");
    let second = app.call("visual.admit_package", replay).unwrap();
    assert_eq!(second["idempotent"], true);
    assert_eq!(second["event_count"], 0);
    assert_eq!(second["run"], first["run"]);
    let journal_after_first = journal(&root, "book");
    assert_eq!(
        journal(&root, "book"),
        journal_after_first,
        "identical replay must not append events"
    );

    // Same package under a different run id is an explicit conflict.
    let mut other_run = admit_request(package.clone());
    other_run["run_id"] = json!("vrun2");
    other_run["command_id"] = json!("admit-other-run");
    let error = app.call("visual.admit_package", other_run).unwrap_err();
    assert!(
        matches!(&error, Error::Conflict(message) if message.contains("already admitted under visual run 'vrun1'")),
        "unexpected error: {error:?}"
    );

    // Same run id with different content is an explicit conflict.
    let mut tampered_stage = admit_request(package);
    tampered_stage["stage"] = json!("S01");
    tampered_stage["command_id"] = json!("admit-tampered-stage");
    let error = app
        .call("visual.admit_package", tampered_stage)
        .unwrap_err();
    assert!(
        matches!(&error, Error::Conflict(message) if message.contains("different content")),
        "unexpected error: {error:?}"
    );
    assert_eq!(
        journal(&root, "book"),
        journal_after_first,
        "conflicting admissions must leave the journal untouched"
    );
    let _ = fs::remove_dir_all(root);
}

#[test]
fn route_admission_shape_is_enforced_fail_closed() {
    let (root, app) = workspace_app("route-shape");
    let package = exported_package(&app);
    let journal_before = journal(&root, "book");

    let mut wrong_route = admit_request(package.clone());
    wrong_route["route_admission"]["route_id"] = json!("TEXT");
    let error = app.call("visual.admit_package", wrong_route).unwrap_err();
    assert!(
        matches!(&error, Error::InvalidInput(message)
            if message.contains("route_admission_shape") && message.contains("'NVC'")),
        "unexpected error: {error:?}"
    );

    let mut missing_budget_field = admit_request(package.clone());
    missing_budget_field["route_admission"]["budget"]
        .as_object_mut()
        .unwrap()
        .remove("output_bytes");
    let error = app
        .call("visual.admit_package", missing_budget_field)
        .unwrap_err();
    assert!(matches!(&error, Error::InvalidInput(_)));

    let mut wrong_command = admit_request(package.clone());
    wrong_command["route_admission"]["command_id"] = json!("visual.plan");
    let error = app.call("visual.admit_package", wrong_command).unwrap_err();
    assert!(
        matches!(&error, Error::InvalidInput(message)
            if message.contains("command_id") && message.contains("visual.admit_package")),
        "unexpected error: {error:?}"
    );

    // Stage outside ^S(?:0[0-9]|1[0-2])$ fails the run-state shape check.
    let mut text_stage = admit_request(package);
    text_stage["stage"] = json!("T01");
    let error = app.call("visual.admit_package", text_stage).unwrap_err();
    assert!(
        matches!(&error, Error::InvalidInput(message)
            if message.contains("run_state_shape") && message.contains("stage")),
        "unexpected error: {error:?}"
    );

    assert_eq!(
        journal(&root, "book"),
        journal_before,
        "every rejected admission must leave the journal untouched"
    );
    let state = app
        .call("project.get", json!({"project_id": "book"}))
        .unwrap();
    assert!(
        state["visual_runs"].as_object().unwrap().is_empty(),
        "no visual run may be recorded from rejected admissions"
    );
    let _ = fs::remove_dir_all(root);
}

#[test]
fn retired_package_id_reference_form_requires_inline_body() {
    let (root, app) = workspace_app("reference-form");
    exported_package(&app);

    let malformed = json!({
        "project_id": "book",
        "run_id": "vrun1",
        "stage": "S00",
        "package_id": "np_not-hex",
        "route_admission": route_admission(),
        "command_id": "admit-ref-bad"
    });
    let error = app.call("visual.admit_package", malformed).unwrap_err();
    assert!(
        matches!(&error, Error::InvalidInput(message) if message.contains("unknown field `package_id`")),
        "unexpected error: {error:?}"
    );

    let well_formed = json!({
        "project_id": "book",
        "run_id": "vrun1",
        "stage": "S00",
        "package_id": format!("np_{}", "c".repeat(64)),
        "route_admission": route_admission(),
        "command_id": "admit-ref-good"
    });
    let error = app.call("visual.admit_package", well_formed).unwrap_err();
    assert!(
        matches!(&error, Error::InvalidInput(message) if message.contains("unknown field `package_id`")),
        "the retired package_id argument must be rejected: {error:?}"
    );

    let neither = json!({
        "project_id": "book",
        "run_id": "vrun1",
        "stage": "S00",
        "route_admission": route_admission(),
        "command_id": "admit-neither"
    });
    let error = app.call("visual.admit_package", neither).unwrap_err();
    assert!(matches!(&error, Error::InvalidInput(_)));
    let _ = fs::remove_dir_all(root);
}

#[test]
fn admit_package_surfaces_in_tool_registry_and_survives_recovery() {
    let (root, app) = workspace_app("registry-recovery");
    let definition = app
        .tool_definitions()
        .into_iter()
        .find(|tool| tool.name == "visual.admit_package")
        .expect("visual.admit_package must be listed as an MCP tool");
    assert_eq!(
        definition.input_schema["required"],
        json!([
            "project_id",
            "run_id",
            "stage",
            "package",
            "route_admission"
        ])
    );
    assert!(definition.input_schema["properties"]["package_id"].is_null());
    assert_eq!(
        definition.input_schema["properties"]["route_admission"]["properties"]["route_id"]["enum"],
        json!(["NVC"])
    );

    let package = exported_package(&app);
    app.call("visual.admit_package", admit_request(package))
        .unwrap();

    let recovered = app.workspace().recover_project("book", false).unwrap();
    let recovered = serde_json::to_value(&recovered).unwrap();
    assert_eq!(recovered["visual_runs"]["vrun1"]["status"], "READY");
    assert_eq!(
        recovered["visual_runs"]["vrun1"]["narrative_package"]["kind"],
        "geultu_story_package"
    );
    let _ = fs::remove_dir_all(root);
}
