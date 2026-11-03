//! M2 integration tests: validation receipts (H3), human decisions bound to
//! artifact digests (H3), text-run budget admission and snapshot SSOT
//! conformance (H6).

mod support;

use geultu::error::Error;
use geultu::{Application, Workspace};
use serde_json::{json, Value};
use std::fs;
use support::unique_temp_id;

fn workspace_app(label: &str) -> (std::path::PathBuf, Application) {
    let root = std::env::temp_dir().join(format!(
        "geultu-receipts-{label}-{}-{}",
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

const HEX64: &str = "aa000000000000000000000000000000000000000000000000000000000000ee";

fn frozen_inputs() -> Value {
    json!({
        "synopsis": {"id": "text:synopsis", "kind": "synopsis", "revision": 1, "digest": format!("sha256:{HEX64}")},
        "style": {"id": "text:style", "kind": "style", "revision": 1, "digest": format!("sha256:{HEX64}")},
        "canon": {"id": "text:canon", "kind": "canon", "revision": 1, "digest": format!("sha256:{HEX64}")},
        "genre": {"id": "text:genre", "kind": "genre", "revision": 1, "digest": format!("sha256:{HEX64}")}
    })
}

/// Seeds project `book` with one scene, one pending candidate, and one
/// pending changeset; returns the application.
fn seeded_flow(label: &str) -> (std::path::PathBuf, Application) {
    let (root, app) = workspace_app(label);
    app.call(
        "project.create",
        json!({"project_id": "book", "title": "Book", "command_id": "create"}),
    )
    .unwrap();
    app.call(
        "scene.add",
        json!({"project_id": "book", "scene_id":"scene_1", "title": "One", "text": "old", "command_id": "scene"}),
    )
    .unwrap();
    app.call(
        "candidate.create",
        json!({"project_id": "book", "candidate_id": "cand", "kind": "rewrite", "content": "new", "provider": "fixture", "model": "fixture", "command_id": "cand-create"}),
    )
    .unwrap();
    app.call(
        "changeset.create",
        json!({"project_id": "book", "candidate_id": "cand", "changeset_id": "change", "operations": [{"type": "replace_scene_text", "scene_id":"scene_1", "expected_scene_revision": 0, "text": "new"}], "command_id": "change-create"}),
    )
    .unwrap();
    (root, app)
}

/// Reads the canonical subject through the public candidate review entrypoint.
fn stored_candidate_binding(app: &Application) -> (u64, String) {
    let subject = app
        .call(
            "candidate.subject",
            json!({"project_id":"book","candidate_id":"cand"}),
        )
        .unwrap();
    (
        subject["revision"].as_u64().unwrap(),
        subject["digest"].as_str().unwrap().to_string(),
    )
}

#[test]
fn approve_with_digest_mismatch_is_rejected_fail_closed() {
    let (root, app) = seeded_flow("mismatch");
    let project = app
        .call("project.get", json!({"project_id": "book"}))
        .unwrap();
    let journal_before = journal(&root, "book");
    let (artifact_revision, _correct_digest) = stored_candidate_binding(&app);

    let error = app
        .call(
            "changeset.approve",
            json!({
                "project_id": "book",
                "changeset_id": "change",
                "expected_project_revision": project["revision"],
                "actor_id": "editor",
                "expected_artifact_revision": artifact_revision,
                "expected_artifact_digest": format!("sha256:{HEX64}"),
                "command_id": "approve-bad"
            }),
        )
        .unwrap_err();
    assert!(
        matches!(&error, Error::Conflict(message) if message.contains("digest mismatch")),
        "unexpected error: {error:?}"
    );

    assert_eq!(
        journal(&root, "book"),
        journal_before,
        "rejected approval must leave the journal untouched"
    );
    let after = app
        .call("project.get", json!({"project_id": "book"}))
        .unwrap();
    assert_eq!(after["changesets"]["change"]["status"], "pending");
    assert_eq!(after["candidates"]["cand"]["status"], "pending");
    let _ = fs::remove_dir_all(root);
}

#[test]
fn approve_with_matching_digest_records_human_decision() {
    let (root, app) = seeded_flow("decision");
    let project = app
        .call("project.get", json!({"project_id": "book"}))
        .unwrap();
    let (artifact_revision, digest) = stored_candidate_binding(&app);

    let result = app
        .call(
            "changeset.approve",
            json!({
                "project_id": "book",
                "changeset_id": "change",
                "expected_project_revision": project["revision"],
                "actor_id": "editor",
                "reason": "copy fits the scene",
                "expected_artifact_revision": artifact_revision,
                "expected_artifact_digest": digest,
                "command_id": "approve-good"
            }),
        )
        .unwrap();
    let decision = &result["decision"];
    let decision_id = decision["decision_id"].as_str().unwrap();
    assert!(decision_id.starts_with("human_"));
    assert_eq!(decision_id.len(), 70);
    assert_eq!(decision["decision"], "APPROVE");
    assert_eq!(decision["subject"]["id"], "candidate:cand");
    assert_eq!(decision["scope"], "project:book/changeset:change");

    let after = app
        .call("project.get", json!({"project_id": "book"}))
        .unwrap();
    assert_eq!(after["changesets"]["change"]["status"], "approved");
    let recorded = &after["human_decisions"][decision_id];
    assert_eq!(recorded, decision, "decision must be retrievable verbatim");

    // Declaring a decision without the complete artifact claims fails
    // closed before any lookup or journal write.
    let error = app
        .call(
            "changeset.reject",
            json!({
                "project_id": "book",
                "changeset_id": "nonexistent",
                "actor_id": "editor"
            }),
        )
        .unwrap_err();
    assert!(
        matches!(&error, Error::InvalidInput(message) if message.contains("expected_artifact")),
        "unexpected error: {error:?}"
    );
    let _ = fs::remove_dir_all(root);
}

#[test]
fn review_pass_receipt_is_recorded_and_retrievable_and_idempotent() {
    let (root, app) = workspace_app("pass");
    app.call(
        "project.create",
        json!({"project_id": "book", "title": "Book", "command_id": "create"}),
    )
    .unwrap();
    app.call(
        "scene.add",
        json!({"project_id": "book", "scene_id":"scene_1", "title": "One", "text": "본문", "command_id": "scene"}),
    )
    .unwrap();

    let first = app
        .call("review.run", json!({"project_id": "book"}))
        .unwrap();
    let receipt = &first["receipt"];
    assert_eq!(receipt["outcome"], "PASS");
    let receipt_id = receipt["receipt_id"].as_str().unwrap();
    assert!(receipt_id.starts_with("vr_") && receipt_id.len() == 67);
    assert_eq!(receipt["validator"]["id"], "geultu-review");
    assert_eq!(receipt["subject"]["id"], "text:book");
    assert_eq!(first["findings"], receipt["findings"]);

    let after = app
        .call("project.get", json!({"project_id": "book"}))
        .unwrap();
    assert_eq!(
        after["validation_receipts"][receipt_id]["outcome"], "PASS",
        "receipt must be recorded in project state"
    );

    let journal_after_first = journal(&root, "book");
    let second = app
        .call("review.run", json!({"project_id": "book"}))
        .unwrap();
    assert_eq!(second["receipt"]["receipt_id"], receipt["receipt_id"]);
    assert_eq!(second["idempotent"], true);
    assert_eq!(
        journal(&root, "book"),
        journal_after_first,
        "re-reviewing unchanged state must not append events"
    );
    let _ = fs::remove_dir_all(root);
}

#[test]
fn review_fail_receipt_carries_error_findings() {
    let (root, app) = workspace_app("fail");
    app.call(
        "project.create",
        json!({"project_id": "book", "title": "Book", "command_id": "create"}),
    )
    .unwrap();
    app.call(
        "fact.add",
        json!({"project_id": "book", "fact_id": "f1", "subject": "민수", "predicate": "눈색", "object": "갈색", "command_id": "f1"}),
    )
    .unwrap();
    app.call(
        "fact.add",
        json!({"project_id": "book", "fact_id": "f2", "subject": "민수", "predicate": "눈색", "object": "파랑", "command_id": "f2"}),
    )
    .unwrap();

    let result = app
        .call("review.run", json!({"project_id": "book"}))
        .unwrap();
    assert_eq!(result["receipt"]["outcome"], "FAIL");
    let codes: Vec<&str> = result["receipt"]["findings"]
        .as_array()
        .unwrap()
        .iter()
        .map(|finding| finding["code"].as_str().unwrap())
        .collect();
    assert!(
        codes.contains(&"CONTINUITY_FACT_CONFLICT"),
        "mapped finding codes must follow the schema pattern: {codes:?}"
    );
    let _ = fs::remove_dir_all(root);
}

#[test]
fn proof_verify_issues_receipt_and_records_only_when_anchored() {
    let (root, app) = workspace_app("proof");
    app.call(
        "project.create",
        json!({"project_id": "book", "title": "Book", "command_id": "create"}),
    )
    .unwrap();

    let passing = app
        .call(
            "proof.verify",
            json!({"source": "Axiom은 정본이다.", "candidate": "Axiom은 정본이다.", "protected_terms": ["Axiom"]}),
        )
        .unwrap();
    assert_eq!(passing["receipt"]["outcome"], "PASS");
    assert_eq!(passing["recorded"], false);

    let anchored = app
        .call(
            "proof.verify",
            json!({"source": "Axiom은 정본이다.", "candidate": "Axiom은 정본이다.", "protected_terms": ["Axiom"], "project_id": "book"}),
        )
        .unwrap();
    assert_eq!(anchored["recorded"], true);
    let receipt_id = anchored["receipt"]["receipt_id"].as_str().unwrap();
    let after = app
        .call("project.get", json!({"project_id": "book"}))
        .unwrap();
    assert_eq!(after["validation_receipts"][receipt_id]["outcome"], "PASS");

    let failing = app
        .call(
            "proof.verify",
            json!({"source": "Axiom은 정본이다.", "candidate": "정본이다.", "protected_terms": ["Axiom"], "project_id": "book"}),
        )
        .unwrap();
    assert_eq!(failing["receipt"]["outcome"], "FAIL");
    let _ = fs::remove_dir_all(root);
}

#[test]
fn candidate_reject_records_bound_reject_decision() {
    let (root, app) = seeded_flow("reject");
    let (artifact_revision, digest) = stored_candidate_binding(&app);
    let result = app
        .call(
            "candidate.reject",
            json!({
                "project_id": "book",
                "candidate_id": "cand",
                "actor_id": "editor",
                "reason": "wrong direction",
                "expected_artifact_revision": artifact_revision,
                "expected_artifact_digest": digest,
                "command_id": "reject-decision"
            }),
        )
        .unwrap();
    assert_eq!(result["decision"]["decision"], "REJECT");
    let after = app
        .call("project.get", json!({"project_id": "book"}))
        .unwrap();
    assert_eq!(after["candidates"]["cand"]["status"], "rejected");
    assert_eq!(
        after["human_decisions"].as_object().map(|map| map.len()),
        Some(1)
    );
    let _ = fs::remove_dir_all(root);
}

#[test]
fn budget_exhaustion_rejects_with_explicit_error_and_leaves_journal_untouched() {
    let (root, app) = workspace_app("budget");
    app.call(
        "project.create",
        json!({"project_id": "book", "title": "Book", "command_id": "create"}),
    )
    .unwrap();
    app.call(
        "scene.add",
        json!({"project_id": "book", "scene_id":"scene_1", "title": "One", "text": "v0", "command_id": "scene"}),
    )
    .unwrap();
    let opened = app
        .call(
            "run.open",
            json!({
                "project_id": "book",
                "run_id": "run1",
                "stage": "T01",
                "budgets": {"revision_remaining": 1, "pivot_remaining": 2},
                "frozen_inputs": frozen_inputs(),
                "command_id": "run-open"
            }),
        )
        .unwrap();
    assert_eq!(opened["run"]["status"], "READY");

    app.call(
        "scene.set_text",
        json!({"project_id": "book", "scene_id":"scene_1", "text": "v1", "expected_scene_revision": 0, "run_id": "run1", "command_id": "set-1"}),
    )
    .unwrap();
    let after_first = app
        .call("project.get", json!({"project_id": "book"}))
        .unwrap();
    assert_eq!(
        after_first["text_runs"]["run1"]["budgets"]["revision_remaining"],
        0
    );

    let journal_before = journal(&root, "book");
    let error = app
        .call(
            "scene.set_text",
            json!({"project_id": "book", "scene_id":"scene_1", "text": "v2", "expected_scene_revision": 1, "run_id": "run1", "command_id": "set-2"}),
        )
        .unwrap_err();
    assert!(
        matches!(&error, Error::BudgetExhausted(message) if message.contains("run1")),
        "unexpected error: {error:?}"
    );
    assert_eq!(
        journal(&root, "book"),
        journal_before,
        "rejected command must leave the journal untouched"
    );

    // Unknown runs and terminal statuses are also rejected fail-closed.
    let error = app
        .call(
            "scene.set_text",
            json!({"project_id": "book", "scene_id":"scene_1", "text": "v3", "expected_scene_revision": 1, "run_id": "ghost"}),
        )
        .unwrap_err();
    assert!(matches!(error, Error::NotFound(_)));
    let _ = fs::remove_dir_all(root);
}

#[test]
fn snapshot_roundtrip_passes_project_state_predicates() {
    let (root, app) = seeded_flow("roundtrip");
    let (artifact_revision, digest) = stored_candidate_binding(&app);
    let project = app
        .call("project.get", json!({"project_id": "book"}))
        .unwrap();
    app.call(
        "changeset.approve",
        json!({
            "project_id": "book",
            "changeset_id": "change",
            "expected_project_revision": project["revision"],
            "actor_id": "editor",
            "expected_artifact_revision": artifact_revision,
            "expected_artifact_digest": digest,
            "command_id": "approve"
        }),
    )
    .unwrap();
    app.call("review.run", json!({"project_id": "book"}))
        .unwrap();
    app.call(
        "run.open",
        json!({
            "project_id": "book",
            "run_id": "run1",
            "stage": "T00",
            "budgets": {"revision_remaining": 3, "pivot_remaining": 1},
            "frozen_inputs": frozen_inputs()
        }),
    )
    .unwrap();

    let before = app
        .call("project.get", json!({"project_id": "book"}))
        .unwrap();
    let recovered = app.workspace().recover_project("book", false).unwrap();
    let reloaded = app
        .call("project.get", json!({"project_id": "book"}))
        .unwrap();
    assert_eq!(
        serde_json::to_value(&recovered).unwrap(),
        before,
        "journal replay must reproduce the pre-recovery state"
    );
    assert_eq!(reloaded, before, "snapshot reload must be lossless");
    assert_eq!(
        reloaded["validation_receipts"]
            .as_object()
            .map(|map| map.len()),
        Some(1)
    );
    assert_eq!(
        reloaded["human_decisions"].as_object().map(|map| map.len()),
        Some(1)
    );
    assert_eq!(reloaded["text_runs"]["run1"]["stage"], "T00");
    let _ = fs::remove_dir_all(root);
}

#[test]
fn corrupted_snapshot_fails_load_fail_closed() {
    let snapshot_of = |label: &str| {
        let (root, app) = seeded_flow(label);
        let _ = app
            .call("project.get", json!({"project_id": "book"}))
            .unwrap();
        root
    };

    // Case 1: hand-edited revision below the schema minimum of 1.
    let root = snapshot_of("corrupt-revision");
    let path = root.join(".geultu/projects/book/snapshot.json");
    let raw = fs::read_to_string(&path).unwrap();
    let edited = raw.replace("\"revision\": 4,", "\"revision\": 0,");
    assert_ne!(raw, edited, "fixture must contain the edited field");
    fs::write(&path, edited).unwrap();
    let reopened = Application::open(&root).unwrap();
    let error = reopened
        .call("project.get", json!({"project_id": "book"}))
        .unwrap_err();
    assert!(
        matches!(&error, Error::Corrupt(message) if message.contains("project-state contract")),
        "unexpected error: {error:?}"
    );
    let _ = fs::remove_dir_all(root);

    // Case 2: unknown top-level snapshot key violates the closed shape.
    let root = snapshot_of("corrupt-unknown-key");
    let path = root.join(".geultu/projects/book/snapshot.json");
    let raw = fs::read_to_string(&path).unwrap();
    let edited = raw.replace(
        "{\n  \"event_count\"",
        "{\n  \"hand_edited\": true,\n  \"event_count\"",
    );
    assert_ne!(raw, edited, "fixture must contain the edited field");
    fs::write(&path, edited).unwrap();
    let reopened = Application::open(&root).unwrap();
    let error = reopened
        .call("project.get", json!({"project_id": "book"}))
        .unwrap_err();
    assert!(matches!(error, Error::Corrupt(_)));
    let _ = fs::remove_dir_all(root);

    // Case 3: map key detached from its entity id.
    let root = snapshot_of("corrupt-keys");
    let path = root.join(".geultu/projects/book/snapshot.json");
    let raw = fs::read_to_string(&path).unwrap();
    let edited = raw.replace(
        "\"cand\": {\n        \"id\": \"cand\"",
        "\"forged\": {\n        \"id\": \"cand\"",
    );
    assert_ne!(raw, edited, "fixture must contain the edited field");
    fs::write(&path, edited).unwrap();
    let reopened = Application::open(&root).unwrap();
    let error = reopened
        .call("project.get", json!({"project_id": "book"}))
        .unwrap_err();
    assert!(
        matches!(&error, Error::Corrupt(message) if message.contains("mismatched entity id")),
        "unexpected error: {error:?}"
    );
    let _ = fs::remove_dir_all(root);
}
