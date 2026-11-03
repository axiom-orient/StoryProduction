mod support;

use geultu::{Application, Error, Workspace};
use serde_json::{json, Value};
use std::{fs, path::PathBuf};
use support::unique_temp_id;

fn fixture() -> (PathBuf, Application, Value) {
    let root = std::env::temp_dir().join(format!(
        "geultu-authoring-review-{}-{}",
        std::process::id(),
        unique_temp_id()
    ));
    let app = Application::from_workspace(Workspace::init(&root).unwrap());
    for (operation, input) in [
        (
            "project.create",
            json!({"project_id":"book","title":"Test","command_id":"create"}),
        ),
        (
            "authoring.start",
            json!({"project_id":"book","command_id":"source","source":"Original source","actor":"synthetic-test"}),
        ),
        (
            "authoring.prepare",
            json!({"project_id":"book","command_id":"prepare","task_id":"task","target":"story","instruction":"Draft the story"}),
        ),
        (
            "authoring.submit",
            json!({"project_id":"book","command_id":"submit","candidate_id":"candidate","task_id":"task","draft":{"blocks":[{"id":"main","heading":"Story","text":"Actual fixture draft"}],"change_summary":"Synthetic lifecycle test only","assumptions":[],"open_questions":[]}}),
        ),
    ] {
        app.call(operation, input).unwrap();
    }
    let candidate = app
        .call(
            "authoring.candidate",
            json!({"project_id":"book","id":"candidate"}),
        )
        .unwrap();
    let review = json!({"project_id":"book","command_id":"review","candidate_id":"candidate", "expected_candidate_hash":candidate["candidate_hash"],"accept":true,"actor":"synthetic-test"});
    (root, app, review)
}

fn change_source(app: &Application) {
    app.call("authoring.edit", json!({"project_id":"book","command_id":"edit-source","kind":"source","expected_revision":1,"actor":"synthetic-test","blocks":[{"id":"source","heading":"Source","text":"Changed source"}]})).unwrap();
}

#[test]
fn accepted_readback_does_not_treat_its_own_adoption_as_a_conflict() {
    let (root, app, review) = fixture();
    let accepted = app.call("authoring.review", review.clone()).unwrap();
    assert_eq!(
        accepted["result"]["candidate"]["decision"]["accepted"],
        true
    );
    assert_eq!(accepted["result"]["adoption_blockers"], json!([]));
    assert_eq!(accepted["result"]["current_target_reasons"], json!([]));
    assert_eq!(accepted["result"]["next_action"], "authoring.status");
    let replay = app.call("authoring.review", review).unwrap();
    assert_eq!(replay["commit"]["idempotent"], true);
    assert_eq!(replay["result"]["current_target"]["revision"], 1);

    change_source(&app);
    let historical = app
        .call(
            "authoring.candidate",
            json!({"project_id":"book","id":"candidate"}),
        )
        .unwrap();
    assert_eq!(historical["candidate"]["decision"]["accepted"], true);
    assert!(!historical["current_target_reasons"]
        .as_array()
        .unwrap()
        .is_empty());
    assert_eq!(historical["next_action"], "authoring.prepare");
    assert!(app
        .call(
            "authoring.export",
            json!({"project_id":"book","kind":"story"})
        )
        .is_err());
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn pending_source_conflict_still_blocks_acceptance_but_not_rejection() {
    let (root, app, mut review) = fixture();
    change_source(&app);
    let pending = app
        .call(
            "authoring.candidate",
            json!({"project_id":"book","id":"candidate"}),
        )
        .unwrap();
    assert!(!pending["adoption_blockers"].as_array().unwrap().is_empty());
    assert!(matches!(
        app.call("authoring.review", review.clone()),
        Err(Error::Conflict(_))
    ));
    review["accept"] = json!(false);
    let rejected = app.call("authoring.review", review).unwrap();
    assert_eq!(
        rejected["result"]["candidate"]["decision"]["accepted"],
        false
    );
    assert_eq!(rejected["result"]["adoption_blockers"], json!([]));
    assert_eq!(rejected["result"]["next_action"], "authoring.prepare");
    assert!(rejected["result"]["current_target"].is_null());
    fs::remove_dir_all(root).unwrap();
}
