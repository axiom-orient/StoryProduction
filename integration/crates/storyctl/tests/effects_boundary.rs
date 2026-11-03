use serde_json::json;
use std::fs::{self, File};
use std::os::fd::AsRawFd;
use std::os::unix::fs::symlink;
use std::os::unix::process::CommandExt;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};
use story_integration_core::Outcome;
use storyctl::contracts::{verify_contract_registry, SchemaRegistry};
use storyctl::effects::{files, hash, json as codec, process, workspace};

#[test]
fn held_directory_survives_root_and_child_replacement_without_redirecting_records() {
    let (_temp, root) = temporary();
    let selected = root.join("selected");
    let directory = files::Directory::open(&selected, true).unwrap();
    directory
        .immutable("attempts/original.json", b"original")
        .unwrap();
    let attempts = directory.child("attempts", false).unwrap();
    fs::rename(&selected, root.join("moved")).unwrap();
    fs::create_dir(&selected).unwrap();
    directory.immutable("root.json", b"held root").unwrap();
    assert_eq!(
        directory.read("attempts/original.json").unwrap(),
        b"original"
    );
    assert!(directory.verify_identity().is_err());
    assert!(!selected.join("root.json").exists());
    assert!(root.join("moved/root.json").is_file());
    fs::rename(root.join("moved/attempts"), root.join("moved/old-attempts")).unwrap();
    fs::create_dir(root.join("moved/attempts")).unwrap();
    assert!(directory.verify_child("attempts", &attempts).is_err());
    attempts
        .atomic_write_json("result.json", &json!({"status":"outcome_unknown"}))
        .unwrap();
    assert!(root.join("moved/old-attempts/result.json").is_file());
    assert!(!root.join("moved/attempts/result.json").exists());
}

#[test]
fn held_claim_cleanup_checks_inode_and_child_symlink_never_escapes() {
    let (_temp, root) = temporary();
    let directory = files::Directory::open(&root, false).unwrap();
    let claim = directory.exclusive("claim", b"owned").unwrap();
    fs::rename(root.join("claim"), root.join("old-claim")).unwrap();
    fs::write(root.join("claim"), b"replacement").unwrap();
    assert!(directory.remove_file("claim", &claim).is_err());
    assert_eq!(fs::read(root.join("claim")).unwrap(), b"replacement");
    let held = directory.reserve_child("attempt").unwrap().unwrap();
    assert!(directory.reserve_child("attempt").unwrap().is_none());
    fs::rename(root.join("attempt"), root.join("old-attempt")).unwrap();
    let outside = root.join("outside");
    fs::create_dir(&outside).unwrap();
    symlink(&outside, root.join("attempt")).unwrap();
    assert!(directory.remove_empty_child("attempt", &held).is_err());
    assert!(directory.read("attempt/escaped.json").is_err());
    held.immutable("saved.json", b"original namespace").unwrap();
    assert!(fs::read_dir(&outside).unwrap().next().is_none());
    assert!(!directory.is_empty().unwrap());
    assert!(!directory.is_empty().unwrap());
}

#[test]
fn dropping_submission_lock_releases_it_even_while_a_fork_retains_the_descriptor() {
    use std::io::{Read, Write};
    use std::os::unix::net::UnixStream;
    let (_temp, root) = temporary();
    let directory = files::Directory::open(&root, false).unwrap();
    let lock = directory.submission_lock().unwrap();
    let (mut ready, child_ready) = UnixStream::pair().unwrap();
    let (mut release, child_release) = UnixStream::pair().unwrap();
    ready
        .set_read_timeout(Some(Duration::from_secs(5)))
        .unwrap();
    let child = std::thread::spawn(move || {
        let ready_fd = child_ready.as_raw_fd();
        let release_fd = child_release.as_raw_fd();
        let mut command = Command::new("/usr/bin/true");
        unsafe {
            command.pre_exec(move || {
                if libc::write(ready_fd, b"r".as_ptr().cast(), 1) != 1 {
                    return Err(std::io::Error::last_os_error());
                }
                let mut byte = [0u8];
                if libc::read(release_fd, byte.as_mut_ptr().cast(), 1) != 1 {
                    return Err(std::io::Error::last_os_error());
                }
                Ok(())
            });
        }
        command.status()
    });
    ready.read_exact(&mut [0u8]).unwrap();
    drop(lock);
    let reacquired = directory.submission_lock();
    release.write_all(b"x").unwrap();
    assert!(child.join().unwrap().unwrap().success());
    assert!(
        reacquired.is_ok(),
        "fork's pre-exec descriptor must not retain a released lock"
    );
}

fn temporary() -> (tempfile::TempDir, PathBuf) {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().canonicalize().unwrap();
    (directory, path)
}

fn shell(script: &str) -> process::ProcessSpec {
    process::ProcessSpec {
        argv: vec!["/bin/sh".into(), "-c".into(), script.into()],
        cwd: "/".into(),
        ..Default::default()
    }
}

#[test]
fn strict_json_rejects_nested_escaped_duplicate_and_non_finite() {
    for input in [
        br#"{"x":{"a":1,"\u0061":2}}"#.as_slice(),
        br#"{"a":NaN}"#,
        br#"{"a":Infinity}"#,
        br#"{"a":1e999}"#,
        br#"{"a":1,}"#,
        br#"{"a":1} trailing"#,
    ] {
        assert!(
            codec::parse_object(input).is_err(),
            "accepted {:?}",
            String::from_utf8_lossy(input)
        );
    }
    for input in [b"[]".as_slice(), b"null", b"1", &[0xff]] {
        assert!(codec::parse_object(input).is_err());
    }
    let too_large = vec![b' '; storyctl::effects::MAX_BYTES + 1];
    assert!(codec::parse_object(&too_large).is_err());
}

#[test]
fn persisted_bytes_match_python_unicode_big_integer_and_float_form() {
    let value = codec::parse_value(br#"{"z":"\ud55c\uae00","integer":184467440737095516160000,"floats":[1.0,-0.0,0.00001,0.0001,1000000000000000.0,10000000000000000.0,1e20,1e-7],"a":[],"negativeZero":-0}"#).unwrap();
    let expected = "{\n  \"a\": [],\n  \"floats\": [\n    1.0,\n    -0.0,\n    1e-05,\n    0.0001,\n    1000000000000000.0,\n    1e+16,\n    1e+20,\n    1e-07\n  ],\n  \"integer\": 184467440737095516160000,\n  \"negativeZero\": 0,\n  \"z\": \"한글\"\n}\n";
    assert_eq!(codec::stable_bytes(&value).unwrap(), expected.as_bytes());
}

#[test]
fn immutable_equal_replay_differs_from_exclusive_execution_claim() {
    let (_directory, root) = temporary();
    let path = root.join("nested/artifact.json");
    files::immutable(&path, b"original\n").unwrap();
    files::immutable(&path, b"original\n").unwrap();
    assert!(files::immutable(&path, b"changed\n").is_err());
    assert_eq!(files::read_bytes(&path).unwrap(), b"original\n");
    let claim = root.join(".execution-owner");
    files::exclusive(&claim, b"request").unwrap();
    assert!(files::exclusive(&claim, b"request").is_err());
    assert!(files::reserve_directory(&root.join("fresh")).unwrap());
    assert!(!files::reserve_directory(&root.join("fresh")).unwrap());
}

#[test]
fn concurrent_immutable_publication_cannot_overwrite_winner() {
    let (_directory, root) = temporary();
    let output = root.join("output.bin");
    let first = output.clone();
    let second = output.clone();
    let left = std::thread::spawn(move || files::immutable(&first, b"left"));
    let right = std::thread::spawn(move || files::immutable(&second, b"right"));
    let results = [left.join().unwrap(), right.join().unwrap()];
    assert_eq!(results.iter().filter(|result| result.is_ok()).count(), 1);
    assert!(matches!(
        files::read_bytes(&output).unwrap().as_slice(),
        b"left" | b"right"
    ));
    assert!(fs::read_dir(root).unwrap().all(|entry| {
        !entry
            .unwrap()
            .file_name()
            .to_string_lossy()
            .ends_with(".tmp")
    }));
}

#[test]
fn symlink_fifo_and_oversized_file_are_rejected_without_blocking() {
    let (_directory, root) = temporary();
    let ordinary = root.join("input");
    fs::write(&ordinary, b"abc").unwrap();
    symlink(&ordinary, root.join("link")).unwrap();
    assert!(files::read_bytes(&root.join("link")).is_err());
    let fifo = std::ffi::CString::new(root.join("fifo").to_str().unwrap()).unwrap();
    assert_eq!(unsafe { libc::mkfifo(fifo.as_ptr(), 0o600) }, 0);
    let started = Instant::now();
    assert!(files::read_bytes(&root.join("fifo")).is_err());
    assert!(started.elapsed() < Duration::from_secs(1));
    assert!(files::read_bytes_limited(&ordinary, 2).is_err());
    symlink(&root, root.join("alias")).unwrap();
    assert!(files::create_directories(&root.join("alias/new")).is_err());
    assert!(!root.join("new").exists());
}

#[test]
fn descriptor_relative_read_survives_path_rename_and_rejects_traversal() {
    let (_directory, root) = temporary();
    let selected = root.join("selected");
    fs::create_dir(&selected).unwrap();
    fs::write(selected.join("story-project.json"), b"original").unwrap();
    let descriptor = File::open(&selected).unwrap();
    fs::rename(&selected, root.join("moved")).unwrap();
    fs::create_dir(&selected).unwrap();
    fs::write(selected.join("story-project.json"), b"replacement").unwrap();
    assert_eq!(
        files::read_at(descriptor.as_raw_fd(), "story-project.json").unwrap(),
        b"original"
    );
    assert!(files::read_at(descriptor.as_raw_fd(), "../selected/story-project.json").is_err());
}

#[test]
fn mutable_json_and_submission_lock_preserve_distinct_policies() {
    let (_directory, root) = temporary();
    let job = root.join("job.json");
    files::atomic_write_json(&job, &json!({"status":"running"})).unwrap();
    files::atomic_write_json(&job, &json!({"status":"failed"})).unwrap();
    assert_eq!(
        codec::parse_object(&files::read_bytes(&job).unwrap()).unwrap()["status"],
        "failed"
    );
    let lock = files::submission_lock(&root).unwrap();
    assert!(files::submission_lock(&root).is_err());
    drop(lock);
    assert!(files::submission_lock(&root).is_ok());
    assert_eq!(
        hash::sha256_file(&job).unwrap(),
        hash::sha256_bytes(&files::read_bytes(&job).unwrap())
    );
}

#[test]
fn process_drains_input_and_both_outputs_and_leaves_nonzero_to_adapter() {
    let mut spec = shell("/bin/cat; printf diagnostic >&2; exit 73");
    spec.input = vec![b'x'; 1024 * 1024];
    let result = process::invoke(&spec).unwrap();
    assert_eq!(result.stdout, spec.input);
    assert_eq!(result.stderr, b"diagnostic");
    assert_eq!(result.status.code(), Some(73));
}

#[test]
fn spawn_failures_differ_from_timeout_output_limit_and_invalid_utf8() {
    let mut missing = shell("");
    missing.argv = vec!["/story-missing-executable".into()];
    let error = process::invoke(&missing).err().unwrap();
    assert_eq!(error.outcome, Outcome::NotStarted);
    assert!(!error.spawned);
    let mut timeout = shell("printf '{\"receipt\":' ; printf before-timeout >&2; /bin/sleep 10");
    timeout.timeout = Duration::from_secs(1);
    let error = process::invoke(&timeout).err().unwrap();
    assert_eq!(error.outcome, Outcome::Unknown);
    assert!(error.spawned);
    assert_eq!(error.stdout, b"{\"receipt\":");
    assert_eq!(error.stderr, b"before-timeout");
    let mut limit = shell("while :; do printf '0123456789' >&2; done");
    limit.max_output_bytes = 64;
    let error = process::invoke(&limit).err().unwrap();
    assert_eq!(error.outcome, Outcome::Unknown);
    assert!(error.message.contains("output limit"));
    assert!(error.stdout.len() + error.stderr.len() <= 64);
    let error = process::invoke(&shell("printf '\\377'")).err().unwrap();
    assert_eq!(error.outcome, Outcome::Unknown);
    assert!(error.message.contains("UTF-8"));
}

#[test]
fn surviving_descendant_is_unknown_even_when_parent_exits_successfully() {
    let spec = shell("/bin/sleep 10 </dev/null >/dev/null 2>&1 & printf '{}'");
    let error = process::invoke(&spec).err().unwrap();
    assert_eq!(error.outcome, Outcome::Unknown);
    assert!(error.message.contains("descendants"));
    assert_eq!(error.stdout, b"{}");
}

#[test]
fn signal_helper() {
    let Ok(ready) = std::env::var("STORY_TEST_SIGNAL_READY") else {
        return;
    };
    process::install_signal_handlers();
    let mut spec =
        shell("printf ready > \"$STORY_TEST_SIGNAL_READY\"; printf partial; /bin/sleep 10");
    spec.env.insert("STORY_TEST_SIGNAL_READY".into(), ready);
    let error = process::invoke(&spec).err().unwrap();
    assert_eq!(error.outcome, Outcome::Unknown);
    assert!(error.message.contains("cancelled"));
    assert!(process::cancellation_requested());
}

#[test]
fn sigterm_after_dispatch_is_unknown_and_cleans_local_process_group() {
    let (_directory, root) = temporary();
    let ready = root.join("ready");
    let mut child = Command::new(std::env::current_exe().unwrap())
        .args(["--exact", "signal_helper", "--nocapture"])
        .env("STORY_TEST_SIGNAL_READY", &ready)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    let deadline = Instant::now() + Duration::from_secs(5);
    while !ready.exists() && Instant::now() < deadline {
        std::thread::sleep(Duration::from_millis(10));
    }
    assert!(ready.exists(), "child did not dispatch");
    assert_eq!(unsafe { libc::kill(child.id() as i32, libc::SIGTERM) }, 0);
    assert!(child.wait().unwrap().success());
}

#[test]
fn descriptor_helper() {
    let Ok(raw) = std::env::var("STORY_TEST_DESCRIPTOR") else {
        return;
    };
    let descriptor: i32 = raw.parse().unwrap();
    let root = PathBuf::from(format!("/dev/fd/{descriptor}"));
    assert_eq!(
        workspace::workspace_descriptor_for_argument(&root).unwrap(),
        Some(descriptor)
    );
    assert!(workspace::workspace_descriptor_for_argument(&root.join("child")).is_err());
    assert!(workspace::workspace_descriptor_for_argument(Path::new("/")).is_err());
    assert_eq!(
        files::read_bytes(&root.join("source")).unwrap(),
        b"selected"
    );
    assert!(files::directory_exists(&root).unwrap());
    assert!(!files::directory_exists(&root.join("missing/deep")).unwrap());
    files::create_directories(&root.join("boards")).unwrap();
    files::immutable(&root.join("boards/a.json"), b"{}\n").unwrap();
    assert_eq!(
        files::list_json_files(&root.join("boards")).unwrap(),
        vec![root.join("boards/a.json")]
    );
    files::create_directories(&root.join("empty")).unwrap();
    files::remove_empty_directory(&root.join("empty")).unwrap();
    assert!(!files::directory_exists(&root.join("empty")).unwrap());
    let mut spec = shell("/bin/cat source");
    spec.cwd = root;
    spec.pass_fds.push(descriptor);
    assert_eq!(process::invoke(&spec).unwrap().stdout, b"selected");
}

#[test]
fn semi_directory_descriptor_is_validated_and_inherited_by_owner() {
    let (_directory, root) = temporary();
    fs::write(root.join("source"), b"selected").unwrap();
    let anchor = File::open(&root).unwrap();
    let descriptor = anchor.as_raw_fd();
    let mut command = Command::new(std::env::current_exe().unwrap());
    command
        .args(["--exact", "descriptor_helper", "--nocapture"])
        .env("STORY_TEST_DESCRIPTOR", descriptor.to_string())
        .env(workspace::WORKSPACE_FD_ENV, descriptor.to_string());
    unsafe {
        command.pre_exec(move || {
            let flags = libc::fcntl(descriptor, libc::F_GETFD);
            if libc::fcntl(descriptor, libc::F_SETFD, flags & !libc::FD_CLOEXEC) < 0 {
                return Err(std::io::Error::last_os_error());
            }
            Ok(())
        });
    }
    assert!(command.status().unwrap().success());
}

#[test]
fn explicit_held_directory_cwd_needs_no_global_workspace_descriptor_binding() {
    let (_temp, root) = temporary();
    let selected = root.join("selected");
    fs::create_dir(&selected).unwrap();
    fs::write(selected.join("source"), b"selected bytes").unwrap();
    let anchor = File::open(&selected).unwrap();
    let descriptor = anchor.as_raw_fd();
    fs::rename(&selected, root.join("moved")).unwrap();
    fs::create_dir(&selected).unwrap();
    fs::write(selected.join("source"), b"replacement bytes").unwrap();
    let mut spec = shell("/bin/cat source");
    spec.cwd = PathBuf::from(format!("/dev/fd/{descriptor}"));
    spec.pass_fds.push(descriptor);
    assert_eq!(process::invoke(&spec).unwrap().stdout, b"selected bytes");
}

#[test]
fn schema_validation_checks_format_refs_and_rejects_unregistered_external_ref() {
    let (_directory, root) = temporary();
    fs::write(
        root.join("base.schema.json"),
        br#"{"$id":"https://story.test/base","type":"string","format":"date-time"}"#,
    )
    .unwrap();
    fs::write(
        root.join("child.schema.json"),
        br#"{"$id":"https://story.test/child","$ref":"https://story.test/base"}"#,
    )
    .unwrap();
    fs::write(root.join("producer.schema.json"), br#"{"type":"integer"}"#).unwrap();
    let registry = SchemaRegistry::new(&root).unwrap();
    registry
        .validate("https://story.test/child", &json!("2026-10-03T00:00:00Z"))
        .unwrap();
    assert!(registry
        .validate("https://story.test/child", &json!("invalid timestamp"))
        .is_err());
    registry
        .validate("producer.schema.json", &json!(1))
        .unwrap();
    fs::write(
        root.join("remote.schema.json"),
        br#"{"$id":"https://story.test/remote","$ref":"file:///etc/passwd"}"#,
    )
    .unwrap();
    assert!(SchemaRegistry::new(&root).is_err());
}

#[test]
fn current_producer_contract_pins_and_story_schemas_compile() {
    let integration = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .canonicalize()
        .unwrap();
    let story = integration.parent().unwrap();
    let result =
        verify_contract_registry(story, &integration.join("contracts/registry.json")).unwrap();
    assert_eq!(result["status"], "PASS");
    assert_eq!(result["contracts"].as_array().unwrap().len(), 33);
    SchemaRegistry::new(&integration.join("contracts")).unwrap();
}
