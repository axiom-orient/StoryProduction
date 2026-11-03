use narrative_courier_runtime::paths::{
    validate_project_name, validate_relative_path, RuntimePaths,
};
use narrative_courier_runtime::store::{atomic_write_text, sha256_bytes, sha256_file};
use narrative_courier_runtime::token::hash_jcs;
use std::collections::BTreeMap;
use tempfile::TempDir;

#[test]
fn path_boundaries_reject_escape() {
    assert!(validate_project_name("project-01").is_ok());
    assert!(validate_project_name("../escape").is_err());
    assert!(validate_relative_path("output/file.json").is_ok());
    assert!(validate_relative_path("../outside").is_err());
    assert!(RuntimePaths::new("/tmp/example", "skills/example").is_ok());
}

#[test]
fn atomic_publish_and_hash_are_stable() {
    let tmp = TempDir::new().unwrap();
    let path = tmp.path().join("state.json");
    atomic_write_text(&path, "one").unwrap();
    let first = sha256_file(&path).unwrap();
    assert_eq!(first, sha256_bytes(b"one"));
    atomic_write_text(&path, "two").unwrap();
    assert_eq!(sha256_file(&path).unwrap(), sha256_bytes(b"two"));
}

#[test]
fn causal_hash_is_order_stable_for_btree_payloads() {
    let mut a = BTreeMap::new();
    a.insert("a", 1_u32);
    a.insert("b", 2_u32);
    let mut b = BTreeMap::new();
    b.insert("b", 2_u32);
    b.insert("a", 1_u32);
    assert_eq!(hash_jcs(&a).unwrap(), hash_jcs(&b).unwrap());
}

#[test]
fn jcs_canonicalizes_object_order() {
    use narrative_courier_runtime::token::{hash_jcs, jcs_bytes};
    use serde_json::json;
    let a = json!({"b":2,"a":1,"nested":{"z":false,"x":"ok"}});
    let b = json!({"nested":{"x":"ok","z":false},"a":1,"b":2});
    assert_eq!(
        jcs_bytes(&a).unwrap(),
        br#"{"a":1,"b":2,"nested":{"x":"ok","z":false}}"#
    );
    assert_eq!(hash_jcs(&a).unwrap(), hash_jcs(&b).unwrap());
}

#[test]
fn jcs_rejects_non_integer_numbers_in_contract_domain() {
    use narrative_courier_runtime::token::jcs_bytes;
    use serde_json::json;
    assert!(jcs_bytes(&json!({"x":1.5})).is_err());
}

#[cfg(unix)]
#[test]
fn piped_command_timeout_covers_blocked_stdin_delivery() {
    use narrative_courier_runtime::process::run_piped_command;
    use std::process::Command;
    use std::time::{Duration, Instant};

    let tmp = TempDir::new().unwrap();
    let cancel = tmp.path().join("cancel");
    let mut command = Command::new("sh");
    command.args(["-c", "sleep 5"]);
    let payload = vec![b'x'; 4 * 1024 * 1024];
    let started = Instant::now();
    let error = run_piped_command(
        &mut command,
        &payload,
        Duration::from_millis(200),
        &cancel,
        "test-run",
    )
    .expect_err("child that never reads stdin must time out");
    assert!(error.to_string().contains("timed out"));
    assert!(started.elapsed() < Duration::from_secs(3));
}
