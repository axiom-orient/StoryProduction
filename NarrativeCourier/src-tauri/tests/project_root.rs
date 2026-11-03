//! Actual public STS executable project selection; no model or user project.
#[path = "../../test_support/sts_workflow_fixture.rs"]
mod sts_workflow_fixture;
use serde_json::Value;
use std::{
    fs,
    os::unix::fs::PermissionsExt,
    path::{Path, PathBuf},
    process::Command,
};
use sts_workflow::{Engine, NewProject, NextPacket};

fn repository() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .to_path_buf()
}
fn fixture(root: &Path) -> (Engine, PathBuf, String) {
    let engine = Engine::with_roots(repository(), root.join("projects")).unwrap();
    engine
        .create_project(&NewProject {
            project_name: "demo".into(),
            title: "test".into(),
            synopsis: sts_workflow_fixture::SYNOPSIS.into(),
            source_format: "original".into(),
            target_duration: 100,
            genre: vec![],
            tone: String::new(),
            theme_direction: String::new(),
            references: vec![],
        })
        .unwrap();
    let project = engine.project_dir("demo").unwrap();
    let token = match engine.next("demo").unwrap() {
        NextPacket::AgentTask { task_token, .. } => task_token,
        other => panic!("{other:?}"),
    };
    (engine, project, token)
}
fn command(binary: &Path, cwd: &Path) -> Command {
    let mut command = Command::new(binary);
    command
        .current_dir(cwd)
        .env("STS_ASSETS_ROOT", repository())
        .env_remove("STS_PROJECTS_ROOT");
    command
}
fn binary() -> PathBuf {
    PathBuf::from(env!("CARGO_BIN_EXE_narrative-courier-cli"))
}

#[test]
fn selected_project_preserves_task_token_and_rejects_the_wrong_logical_project() {
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path().canonicalize().unwrap();
    let (_, project, token) = fixture(&root);
    let output = command(&binary(), &root)
        .args([
            "agent-task",
            "demo",
            "--project-root",
            project.to_str().unwrap(),
        ])
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let packet: Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(packet["task_token"], token);
    assert_eq!(packet["node"], "step_00");
    let alias = root.join("project-alias");
    std::os::unix::fs::symlink(&project, &alias).unwrap();
    let symlink = command(&binary(), &root)
        .args([
            "agent-task",
            "demo",
            "--project-root",
            alias.to_str().unwrap(),
        ])
        .output()
        .unwrap();
    assert!(!symlink.status.success());
    let state = fs::read(project.join("state.json")).unwrap();
    let refused = command(&binary(), &project)
        .args(["agent-task", "other", "--project-root=."])
        .output()
        .unwrap();
    assert!(!refused.status.success());
    assert!(String::from_utf8_lossy(&refused.stderr).contains("state project identity mismatch"));
    assert_eq!(fs::read(project.join("state.json")).unwrap(), state);
    assert!(!project.join("other").exists());
}

#[test]
fn selected_project_mutation_remains_on_original_inode_after_leaf_replacement() {
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path().canonicalize().unwrap();
    let (_, project, token) = fixture(&root);
    let moved = root.join("original");
    fs::write(
        project.join("input/draft.md"),
        sts_workflow_fixture::artifact_for("step_00"),
    )
    .unwrap();
    let wrapper = root.join("swap-and-submit");
    fs::write(
        &wrapper,
        format!(
            "#!/bin/sh\n/bin/mv '{}' '{}'\n/bin/mkdir '{}'\nexec '{}' \"$@\"\n",
            project.display(),
            moved.display(),
            project.display(),
            binary().display()
        ),
    )
    .unwrap();
    fs::set_permissions(&wrapper, fs::Permissions::from_mode(0o700)).unwrap();
    let output = command(&wrapper, &project)
        .args([
            "submit",
            "demo",
            "input/draft.md",
            "--task-token",
            &token,
            "--project-root=.",
        ])
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let receipt: Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(receipt["node"], "step_00");
    let state: Value =
        serde_json::from_slice(&fs::read(moved.join("state.json")).unwrap()).unwrap();
    assert_eq!(state["project_name"], "demo");
    assert_eq!(state["nodes"]["step_00"]["status"], "completed");
    assert_eq!(
        fs::read_to_string(moved.join(receipt["output"].as_str().unwrap())).unwrap(),
        sts_workflow_fixture::artifact_for("step_00")
    );
    assert!(fs::read_dir(&project).unwrap().next().is_none());
    let packet = command(&binary(), &moved)
        .args(["agent-task", "demo", "--project-root=."])
        .output()
        .unwrap();
    assert!(
        packet.status.success(),
        "{}",
        String::from_utf8_lossy(&packet.stderr)
    );
    assert_eq!(
        serde_json::from_slice::<Value>(&packet.stdout).unwrap()["status"],
        "awaiting_human"
    );
}
