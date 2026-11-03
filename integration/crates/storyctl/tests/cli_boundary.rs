//! Syntax and help must stop before an integration-owned command dispatches.
use std::{fs, os::unix::fs::PermissionsExt, process::Command};

#[test]
fn missing_option_value_never_becomes_a_mutating_title() {
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path().canonicalize().unwrap();
    let marker = root.join("owner-called");
    let owner = root.join("owner");
    fs::write(
        &owner,
        format!("#!/bin/sh\n/bin/touch '{}'\nexit 2\n", marker.display()),
    )
    .unwrap();
    fs::set_permissions(&owner, fs::Permissions::from_mode(0o700)).unwrap();
    let source = root.join("source.md");
    fs::write(&source, b"source\n").unwrap();
    let workspace = root.join("workspace");
    let result = Command::new(env!("CARGO_BIN_EXE_storyctl"))
        .args(["text", "--workspace"])
        .arg(&workspace)
        .arg("--geultu-bin")
        .arg(&owner)
        .args(["create", "--project", "test", "--source"])
        .arg(&source)
        .args(["--actor", "fixture", "--title", "--source"])
        .output()
        .unwrap();
    assert_eq!(result.status.code(), Some(2));
    assert!(!marker.exists());
    assert!(!workspace.exists());
    assert!(String::from_utf8(result.stderr)
        .unwrap()
        .contains("missing value for --title"));
}

#[test]
fn nested_help_does_not_require_resources_or_contact_owners() {
    for args in [
        vec!["--story-root", "/absent-story-root", "text", "--help"],
        vec![
            "text",
            "--workspace",
            "/absent-workspace",
            "storyboard",
            "bundle",
            "--help",
        ],
        vec!["text", "animatic", "direction", "review", "-h"],
    ] {
        let result = Command::new(env!("CARGO_BIN_EXE_storyctl"))
            .args(args)
            .output()
            .unwrap();
        assert_eq!(result.status.code(), Some(0));
        assert!(result.stderr.is_empty());
        assert!(String::from_utf8(result.stdout)
            .unwrap()
            .contains("Native Story orchestration"));
    }
}

#[test]
fn flag_shaped_values_use_explicit_equals_and_root_accepts_equals() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .ancestors()
        .nth(3)
        .unwrap();
    let result = Command::new(env!("CARGO_BIN_EXE_storyctl"))
        .arg(format!("--story-root={}", root.display()))
        .args(["doctor", "--scope=--help"])
        .output()
        .unwrap();
    assert_eq!(result.status.code(), Some(2));
    assert!(String::from_utf8(result.stderr)
        .unwrap()
        .contains("doctor scope must be"));
}

#[test]
fn unknown_owner_observation_is_preserved_in_the_public_error_without_success() {
    use std::io::Write;
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path().canonicalize().unwrap();
    fs::create_dir(root.join("writing")).unwrap();
    fs::write(
        root.join("story-project.json"),
        br#"{"schemaVersion":"story.text-project/1","projectId":"test"}"#,
    )
    .unwrap();
    let owner = root.join("owner");
    let observed = serde_json::json!({"ok":true,"result":{"unexpected":"owner observation"}});
    fs::write(
        &owner,
        format!("#!/bin/sh\n/bin/cat >/dev/null\nprintf '%s' '{observed}'\n"),
    )
    .unwrap();
    fs::set_permissions(&owner, fs::Permissions::from_mode(0o700)).unwrap();
    let mut child = Command::new(env!("CARGO_BIN_EXE_storyctl"))
        .args(["text", "--workspace"])
        .arg(&root)
        .arg("--geultu-bin")
        .arg(&owner)
        .args(["author", "authoring.review", "--input", "-"])
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .unwrap();
    child.stdin.take().unwrap().write_all(b"{}").unwrap();
    let result = child.wait_with_output().unwrap();
    assert_eq!(result.status.code(), Some(2));
    assert!(result.stdout.is_empty());
    let error: serde_json::Value = serde_json::from_slice(&result.stderr).unwrap();
    assert_eq!(error["status"], "outcome_unknown");
    assert_eq!(error["observedOutput"], observed);
}
