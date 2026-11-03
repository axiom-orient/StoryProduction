//! Mechanical producer seams; no actual author mutation or provider is executed.
use serde_json::json;
use std::fs;
use std::os::unix::fs::PermissionsExt;
use storyctl::adapters::owners::{Geultu, NarrativeCourier};
use storyctl::ports::{NarrativeOwner, WritingOwner};
use storyctl::workflows::text::TextContext;

fn fake_owner(script: &str) -> (tempfile::TempDir, Geultu) {
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path().canonicalize().unwrap();
    let binary = root.join("fake-geultu");
    fs::write(
        &binary,
        format!("#!/bin/sh\n/bin/cat > /dev/null\n{script}\n"),
    )
    .unwrap();
    fs::set_permissions(&binary, fs::Permissions::from_mode(0o700)).unwrap();
    let owner = Geultu {
        executable: binary,
        workspace: root.join("writing"),
        repository: root,
        pass_fds: vec![],
    };
    (temp, owner)
}

#[test]
fn successful_exit_without_an_object_result_never_confirms_or_refuses_a_mutation() {
    for operation in [
        "authoring.submit",
        "authoring.start",
        "authoring.review",
        "authoring.prepare",
    ] {
        for raw in [
            r#"{"ok":true,"result":[]}"#,
            r#"{"ok":true,"result":null}"#,
            r#"{"ok":true}"#,
        ] {
            let (_temp, owner) = fake_owner(&format!("printf '%s' '{raw}'"));
            let error = owner
                .call(operation, &json!({"project_id":"test"}))
                .unwrap_err();
            assert_eq!(
                error.outcome.as_str(),
                "outcome_unknown",
                "{operation}: {raw}"
            );
            assert_eq!(error.stdout, raw.as_bytes());
        }
    }
}

#[test]
fn malformed_json_remains_unknown_and_explicit_owner_refusal_remains_not_applied() {
    let (_temp, owner) = fake_owner("printf '{'");
    let error = owner.call("authoring.submit", &json!({})).unwrap_err();
    assert_eq!(error.outcome.as_str(), "outcome_unknown");
    assert_eq!(error.stdout, b"{");
    let(_temp,owner)=fake_owner("printf '%s' '{\"status\":\"not_applied\",\"error\":\"fixture refusal before commit\"}' >&2; exit 2");
    assert_eq!(
        owner
            .call("authoring.submit", &json!({}))
            .unwrap_err()
            .outcome
            .as_str(),
        "not_applied"
    );
}

#[test]
fn story_geultu_calls_bind_assets_to_selected_story_root() {
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path().canonicalize().unwrap();
    let capture = root.join("geultu-environment.txt");
    let binary = root.join("fake-geultu");
    let script = format!(
        "#!/bin/sh\n/bin/cat > /dev/null\nprintf '%s\\n%s\\n' \"$GEULTU_ASSETS_ROOT\" \"$GEULTU_CRAFT_PACK_ROOT\" > '{}'\nprintf '%s' '{{\"ok\":true,\"result\":{{}}}}'\n",
        capture.display()
    );
    fs::write(&binary, script).unwrap();
    fs::set_permissions(&binary, fs::Permissions::from_mode(0o700)).unwrap();
    let owner = Geultu {
        executable: binary,
        workspace: root.join("writing"),
        repository: root.clone(),
        pass_fds: vec![],
    };

    owner
        .call("project.get", &json!({"project_id":"synthetic"}))
        .unwrap();
    let configured: Vec<_> = fs::read_to_string(capture)
        .unwrap()
        .lines()
        .map(str::to_owned)
        .collect();
    assert_eq!(configured.len(), 2);
    assert_eq!(
        configured[0],
        root.join("geultu/assets").display().to_string()
    );
    assert_eq!(
        configured[1],
        root.join("geultu/assets/craft-packs-story-baseline-v1")
            .display()
            .to_string()
    );
}

#[test]
fn successful_exit_with_missing_or_false_ok_is_unknown_and_keeps_raw_stdout() {
    for raw in [
        "{}",
        r#"{"ok":false,"error":{"code":"conflict"}}"#,
        r#"{"ok":null,"result":{}}"#,
        r#"{"ok":"true","result":{}}"#,
    ] {
        for operation in [
            "authoring.submit",
            "authoring.start",
            "authoring.review",
            "authoring.prepare",
        ] {
            let (_temp, owner) = fake_owner(&format!("printf '%s' '{raw}'"));
            let error = owner.call(operation, &json!({})).unwrap_err();
            assert_eq!(
                error.outcome.as_str(),
                "outcome_unknown",
                "{operation}: {raw}"
            );
            assert_eq!(error.stdout, raw.as_bytes());
        }
    }
}

#[test]
fn prepare_conflict_uses_the_actual_error_body_and_preserves_diagnostics() {
    for (diagnostic, expected) in [
        (
            json!({"ok":false,"error":{"code":"conflict","message":"project revision changed"}}),
            "not_applied",
        ),
        (
            json!({"ok":false,"error":{"code":"conflict","message":"project revision changed","committed_revision":null}}),
            "not_applied",
        ),
        (
            json!({"ok":false,"error":{"code":"conflict"}}),
            "outcome_unknown",
        ),
        (
            json!({"ok":false,"error":{"code":"conflict","message":"snapshot failed","committed_revision":3}}),
            "outcome_unknown",
        ),
    ] {
        let raw = serde_json::to_string(&diagnostic).unwrap();
        let (_temp, owner) = fake_owner(&format!(
            "printf diagnostic; printf '%s' '{raw}' >&2; exit 1"
        ));
        let error = owner.call("authoring.prepare", &json!({})).unwrap_err();
        assert_eq!(error.outcome.as_str(), expected);
        assert_eq!(error.stdout, b"diagnostic");
        assert_eq!(error.stderr, raw.as_bytes());
    }
}

#[test]
fn known_authoring_mutations_require_a_matching_public_commit_envelope() {
    for operation in [
        "authoring.start",
        "authoring.brief",
        "authoring.edit",
        "authoring.review",
        "authoring.cancel",
        "authoring.restore",
    ] {
        for raw in [
            r#"{"ok":true,"result":{}}"#,
            r#"{"ok":true,"result":{"commit":{"project_id":"other","revision":1,"event_count":1,"idempotent":false},"result":{}}}"#,
        ] {
            let (_temp, owner) = fake_owner(&format!("printf '%s' '{raw}'"));
            let error = owner
                .call(operation, &json!({"project_id":"probe"}))
                .unwrap_err();
            assert_eq!(error.outcome.as_str(), "outcome_unknown", "{operation}");
            assert_eq!(error.stdout, raw.as_bytes());
        }
        let receipt = json!({"ok":true,"result":{"commit":{"project_id":"probe","revision":1,"event_count":1,"idempotent":false},"result":{}}});
        let (_temp, owner) = fake_owner(&format!("printf '%s' '{receipt}'"));
        assert_eq!(
            owner
                .call(operation, &json!({"project_id":"probe"}))
                .unwrap(),
            receipt["result"]
        );
    }
}

#[test]
fn direct_nvc_submit_surfaces_reject_malformed_success_and_keep_raw_stdout() {
    let (_temp, owner) = fake_owner("printf '%s' '{}'");
    let nvc = NarrativeCourier {
        executable: owner.executable,
        projects: owner.workspace,
        repository: owner.repository,
        pass_fds: vec![],
    };
    let error = nvc
        .text(
            "submit",
            "probe",
            &[
                "input/draft.md".into(),
                "--task-token".into(),
                "token".into(),
            ],
        )
        .unwrap_err();
    assert_eq!(error.outcome.as_str(), "outcome_unknown");
    assert_eq!(error.stdout, b"{}");
    let error = nvc
        .tool(
            "storyboard",
            &[
                "submit".into(),
                "--output".into(),
                "storyboards/boards/one.json".into(),
            ],
            &nvc.repository,
        )
        .unwrap_err();
    assert_eq!(error.outcome.as_str(), "outcome_unknown");
    assert_eq!(error.stdout, b"{}");
}

#[test]
fn direct_storyboard_submit_accepts_the_owners_two_output_option_forms() {
    let receipt = json!({"status":"candidate","path":"storyboards/boards/one.json","contentHash":format!("sha256:{}","a".repeat(64)),"sceneCount":1,"markdown":"candidate","diff":"","openQuestions":[]});
    let (_temp, owner) = fake_owner(&format!("printf '%s' '{receipt}'"));
    let nvc = NarrativeCourier {
        executable: owner.executable,
        projects: owner.workspace,
        repository: owner.repository,
        pass_fds: vec![],
    };
    for args in [
        vec![
            "submit".into(),
            "--output".into(),
            "storyboards/boards/one.json".into(),
        ],
        vec![
            "submit".into(),
            "--output=storyboards/boards/one.json".into(),
        ],
    ] {
        assert_eq!(
            nvc.tool("storyboard", &args, &nvc.repository).unwrap(),
            receipt
        );
    }
}

#[test]
fn missing_owner_hashes_never_establish_current_source_authority() {
    let hash = "1".repeat(64);
    for (export, binding, outcome) in [
        (
            json!({}),
            json!({"reviewed_synopsis":{"source_project_id":"test"}}),
            None,
        ),
        (
            json!({"markdown_sha256":hash}),
            json!({"reviewed_synopsis":{"source_project_id":"test"}}),
            None,
        ),
        (
            json!({"markdown_sha256":hash}),
            json!({"reviewed_synopsis":{"source_project_id":"test","synopsis_sha256":"bad"}}),
            None,
        ),
        (
            json!({"markdown_sha256":hash}),
            json!({"reviewed_synopsis":{"source_project_id":"test","synopsis_sha256":"2".repeat(64)}}),
            Some(false),
        ),
        (
            json!({"markdown_sha256":hash}),
            json!({"reviewed_synopsis":{"source_project_id":"test","synopsis_sha256":hash}}),
            Some(true),
        ),
    ] {
        let (_writing_temp, writing) = fake_owner(&format!(
            "printf '%s' '{}'",
            json!({"ok":true,"result":export})
        ));
        let (_narrative_temp, narrative) = fake_owner(&format!("printf '%s' '{binding}'"));
        let workspace = tempfile::tempdir().unwrap();
        let root = workspace.path().canonicalize().unwrap();
        fs::create_dir(root.join("writing")).unwrap();
        fs::create_dir_all(root.join("narrative/test")).unwrap();
        fs::write(
            root.join("story-project.json"),
            serde_json::to_vec(&json!({"schemaVersion":"story.text-project/1","projectId":"test"}))
                .unwrap(),
        )
        .unwrap();
        let context = TextContext::new(
            root.clone(),
            root,
            writing.executable,
            narrative.executable,
            None,
        )
        .unwrap();
        let observed = context.source_current();
        match outcome {
            Some(expected) => assert_eq!(observed.unwrap(), expected),
            None => assert_eq!(observed.unwrap_err().outcome.as_str(), "outcome_unknown"),
        }
    }
}

#[test]
fn a_confirmed_story_project_switch_is_changed_rather_than_unobserved_source() {
    let (_temp, owner) = fake_owner("printf '%s' '{}'");
    let workspace = tempfile::tempdir().unwrap();
    let root = workspace.path().canonicalize().unwrap();
    fs::write(
        root.join("story-project.json"),
        br#"{"schemaVersion":"story.text-project/1","projectId":"before"}"#,
    )
    .unwrap();
    let context = TextContext::new(
        root.clone(),
        root.clone(),
        owner.executable.clone(),
        owner.executable,
        None,
    )
    .unwrap();
    assert_eq!(context.project_id().unwrap(), "before");
    fs::write(
        root.join("story-project.json"),
        br#"{"schemaVersion":"story.text-project/1","projectId":"after"}"#,
    )
    .unwrap();
    assert_eq!(
        context.project_id().unwrap_err().outcome.as_str(),
        "source_changed"
    );
    assert!(!context.source_current().unwrap());
}

#[test]
fn a_selected_nvc_project_leaf_cannot_be_reopened_as_a_replacement() {
    let digest = "a".repeat(64);
    let (_temp, writing) = fake_owner(&format!(
        "printf '%s' '{}'",
        json!({"ok":true,"result":{"markdown_sha256":digest}})
    ));
    let (_nvc_temp, nvc) = fake_owner(&format!(
        "printf '%s' '{}'",
        json!({"reviewed_synopsis":{"source_project_id":"test","synopsis_sha256":digest}})
    ));
    let workspace = tempfile::tempdir().unwrap();
    let root = workspace.path().canonicalize().unwrap();
    fs::create_dir(root.join("writing")).unwrap();
    fs::create_dir_all(root.join("narrative/test")).unwrap();
    fs::write(
        root.join("story-project.json"),
        br#"{"schemaVersion":"story.text-project/1","projectId":"test"}"#,
    )
    .unwrap();
    let context = TextContext::new(
        root.clone(),
        root.clone(),
        writing.executable,
        nvc.executable,
        None,
    )
    .unwrap();
    assert!(context.source_current().unwrap());
    fs::rename(root.join("narrative/test"), root.join("narrative/original")).unwrap();
    fs::create_dir(root.join("narrative/test")).unwrap();
    let packet = json!({"owner":"NarrativeCourier","status":"agent_task","task_token":"token","node":"step_00"});
    let error =
        storyctl::workflows::text::submit(&context, &packet, "fixture only", "one").unwrap_err();
    assert_eq!(error.outcome.as_str(), "not_applied");
    assert!(!root.join("narrative/test/input").exists());
    assert!(!root.join("narrative/original/input").exists());
}

#[test]
fn nvc_owner_call_uses_the_selected_project_cwd_and_marks_a_mid_call_leaf_swap_unknown() {
    let (_temp, writing) = fake_owner(r#"printf '%s' '{"ok":true,"result":{}}'"#);
    let workspace = tempfile::tempdir().unwrap();
    let root = workspace.path().canonicalize().unwrap();
    let selected = root.join("narrative/test");
    let moved = root.join("narrative/original");
    fs::create_dir_all(&selected).unwrap();
    fs::write(
        root.join("story-project.json"),
        br#"{"schemaVersion":"story.text-project/1","projectId":"test"}"#,
    )
    .unwrap();
    let nvc = root.join("nvc");
    fs::write(&nvc, format!("#!/bin/sh\n/bin/mv '{}' '{}'\n/bin/mkdir '{}'\nprintf captured > received.txt\nprintf '%s' '{{\"status\":\"fixture\"}}'\n", selected.display(), moved.display(), selected.display())).unwrap();
    fs::set_permissions(&nvc, fs::Permissions::from_mode(0o700)).unwrap();
    let context = TextContext::new(root.clone(), root, writing.executable, nvc, None).unwrap();
    let error = context.narrative_call("agent-task", &[]).unwrap_err();
    assert_eq!(error.outcome.as_str(), "outcome_unknown");
    assert_eq!(error.stdout, br#"{"status":"fixture"}"#);
    assert_eq!(fs::read(moved.join("received.txt")).unwrap(), b"captured");
    assert!(!selected.join("received.txt").exists());
}
