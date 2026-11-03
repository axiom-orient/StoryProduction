mod support;

use geultu::{Application, Error, Workspace};
use serde_json::json;
use std::fs;
use support::unique_temp_id;

#[test]
fn authoring_task_prepare_compares_revision_and_replays_by_command_id() {
    let root = std::env::temp_dir().join(format!(
        "geultu-authoring-revision-{}-{}",
        std::process::id(),
        unique_temp_id()
    ));
    let workspace = Workspace::init(&root).unwrap();
    let app = Application::from_workspace(workspace);

    app.call(
        "project.create",
        json!({"project_id":"book","title":"Book","command_id":"create-book"}),
    )
    .unwrap();
    app.call(
        "authoring.start",
        json!({"project_id":"book","command_id":"source-1","source":"Seed text","actor":"author"}),
    )
    .unwrap();
    let prepared_revision = app
        .call("authoring.status", json!({"project_id":"book"}))
        .unwrap()["project_revision"]
        .as_u64()
        .unwrap();
    app.call(
        "authoring.brief",
        json!({"project_id":"book","command_id":"brief-1","brief":{"tone":"calm"},"expected_revision":0}),
    )
    .unwrap();

    let prepare = |expected_project_revision| {
        json!({
            "project_id":"book",
            "command_id":"prepare-1",
            "task_id":"story-task-1",
            "target":"story",
            "instruction":"Draft the next story revision",
            "expected_project_revision":expected_project_revision
        })
    };
    let stale = app.call("authoring.prepare", prepare(prepared_revision));
    assert!(
        matches!(stale, Err(Error::Conflict(message)) if message.contains("project revision mismatch"))
    );

    let current_revision = app
        .call("authoring.status", json!({"project_id":"book"}))
        .unwrap()["project_revision"]
        .as_u64()
        .unwrap();
    let created = app
        .call("authoring.prepare", prepare(current_revision))
        .unwrap();
    assert_eq!(created["result"]["task"]["id"], "story-task-1");
    assert_eq!(created["commit"]["idempotent"], false);

    let replayed = app
        .call("authoring.prepare", prepare(prepared_revision))
        .unwrap();
    assert_eq!(replayed["commit"]["idempotent"], true);
    let status = app
        .call("authoring.status", json!({"project_id":"book"}))
        .unwrap();
    assert_eq!(status["tasks"].as_array().unwrap().len(), 1);

    let _ = fs::remove_dir_all(root);
}
