use std::env;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use base64::Engine;
use serde_json::{Map, Value};

use super::Session;
use crate::filesystem::publication::{MeasurementPublishTestFault, PublishTestFault};
use crate::lock_release::release_lock;
use crate::process_identity::{probe_owner, OwnerProbe};
use crate::protocol::{canonical_json, json_object, sha256_prefixed};
use crate::startup::{Config, SessionPurpose};

fn test_root(label: &str) -> PathBuf {
    let stamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("clock after epoch")
        .as_nanos();
    let root = fs::canonicalize(env::temp_dir())
        .expect("canonical temporary directory")
        .join(format!(
            "smc-trackb-custody-unit-{label}-{}-{stamp}",
            std::process::id()
        ));
    fs::create_dir_all(&root).expect("unit root");
    root
}

fn session(root: &Path, operation_id: &str) -> Session {
    let mut session = Session::new(Config {
        project_root: root.to_path_buf(),
        purpose: SessionPurpose::Narrative,
    })
    .expect("unit project root descriptor");
    let init = json_object([
        ("id", Value::String("init".into())),
        ("op", Value::String("initialize".into())),
        ("mode", Value::String("mutable".into())),
        ("narrativeId", Value::String("narrative.unit".into())),
        ("operationId", Value::String(operation_id.into())),
    ]);
    session
        .initialize(init.as_object().expect("init object"))
        .expect("initialize");
    let acquire = json_object([
        ("id", Value::String("acquire".into())),
        ("op", Value::String("acquire".into())),
        ("narrativeId", Value::String("narrative.unit".into())),
        ("operationId", Value::String(operation_id.into())),
    ]);
    session
        .acquire(acquire.as_object().expect("acquire object"))
        .expect("acquire");
    session
}

fn measurement_session(root: &Path) -> Session {
    Session::new(Config {
        project_root: root.to_path_buf(),
        purpose: SessionPurpose::Measurement,
    })
    .expect("unit project root descriptor")
}

fn measurement_artifact_request(id: &str, kind: &str, path: &str) -> Value {
    json_object([
        ("id", Value::String(id.into())),
        ("op", Value::String("readMeasurementArtifact".into())),
        ("kind", Value::String(kind.into())),
        ("path", Value::String(path.into())),
    ])
}

#[test]
fn process_start_identity_distinguishes_reuse_from_dead_pid() {
    let current = std::process::id();
    assert!(matches!(
        probe_owner(current, "definitely-different-process-start"),
        OwnerProbe::Reused
    ));
    assert!(matches!(
        probe_owner(current, &format!("unavailable:{current}")),
        OwnerProbe::Unknown
    ));
    let dead = (current.saturating_add(10_000)).max(10_000);
    let probe = unsafe { libc::kill(dead as libc::pid_t, 0) };
    if probe != 0 && io::Error::last_os_error().raw_os_error() == Some(libc::ESRCH) {
        assert!(matches!(probe_owner(dead, "dead-owner"), OwnerProbe::Dead));
    }
}

#[test]
fn measurement_parent_fsync_failure_preserves_published_target_and_cleans_own_temp() {
    for (label, fault) in [
        (
            "measurement-fsync-failure",
            MeasurementPublishTestFault::FailParentSync,
        ),
        (
            "measurement-reopen-failure",
            MeasurementPublishTestFault::FailBeforeReopen,
        ),
    ] {
        let root = test_root(label);
        fs::write(root.join("authority.json"), b"authority").expect("authority");
        fs::write(root.join("scene.html"), b"html").expect("html");
        fs::write(root.join("provenance.json"), b"provenance").expect("provenance");
        let mut session = measurement_session(&root);
        for (id, kind, path) in [
            ("authority", "narrativeAuthority", "authority.json"),
            ("html", "html", "scene.html"),
            ("provenance", "provenance", "provenance.json"),
        ] {
            let request = measurement_artifact_request(id, kind, path);
            session
                .read_measurement_artifact(request.as_object().expect("artifact request"))
                .expect("measurement artifact read");
        }
        let receipt = b"receipt";
        let request = json_object([
            ("id", Value::String("publish".into())),
            ("op", Value::String("publishMeasurementReceipt".into())),
            ("path", Value::String("out/receipt.json".into())),
            (
                "bytesBase64",
                Value::String(URL_SAFE_NO_PAD.encode(receipt)),
            ),
        ]);
        let error = session
            .publish_measurement_receipt_with_test_fault(
                request.as_object().expect("publish request"),
                fault,
            )
            .expect_err("measurement publication fault seam must fail");
        assert_eq!(error.code, "trackb-storage-integrity");
        assert_eq!(fs::read(root.join("out/receipt.json")).unwrap(), receipt);
        assert!(fs::read_dir(root.join("out"))
            .unwrap()
            .filter_map(std::result::Result::ok)
            .all(|entry| !entry
                .file_name()
                .to_string_lossy()
                .starts_with(".smc-trackb-measurement-")));
        fs::remove_dir_all(root).expect("remove unit root");
    }
}

#[test]
fn release_refuses_replaced_lock_path_and_preserves_replacement() {
    let root = test_root("release-replacement");
    let mut session = session(&root, "op.release-replacement");
    let narrative = session.narrative_dir.clone().expect("narrative");
    let lock = narrative.join(".release.lock");
    let replacement_target = root.join("replacement-target");
    fs::write(&replacement_target, b"preserve").expect("replacement target");
    let moved = narrative.join(".release.lock.old");
    fs::rename(&lock, &moved).expect("move owned lock");
    std::os::unix::fs::symlink(&replacement_target, &lock).expect("replacement symlink");
    let held = session.lock.take().expect("held lock");
    let error = release_lock(held).expect_err("replacement must require recovery");
    assert_eq!(error.code, "trackb-lock-recovery-required");
    assert_eq!(
        fs::read(&replacement_target).expect("target bytes"),
        b"preserve"
    );
    assert!(lock.exists());
    fs::remove_dir_all(root).expect("remove unit root");
}

fn strict_fixture() -> (Vec<u8>, String, Vec<u8>, String) {
    let mut delivery_set = serde_json::json!({
        "schemaVersion": 2,
        "deliverySetId": "delivery.unit.v1",
        "narrativeId": "narrative.unit",
        "candidateHash": "sha256:aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
        "authorityHash": "sha256:bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb",
        "meaningQualificationHash": "sha256:9999999999999999999999999999999999999999999999999999999999999999",
        "baseDeliverySetHash": "NONE",
        "approvalHash": "sha256:cccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccc",
        "artifacts": {
            "projectionReceiptHash": "sha256:dddddddddddddddddddddddddddddddddddddddddddddddddddddddddddddddd",
            "sceneGraphHash": "sha256:eeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeee",
            "assetHandoffHash": "sha256:ffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffff",
            "assetHandoffSceneGraphHash": "sha256:eeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeee",
            "htmlHash": "sha256:aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
            "provenanceHash": "sha256:bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb"
        },
        "evidenceCellHashes": ["sha256:1111111111111111111111111111111111111111111111111111111111111111"],
        "revision": 1,
        "state": "PROMOTED",
        "deliverySetHash": ""
    });
    let mut delivery_without_hash = delivery_set.as_object().unwrap().clone();
    delivery_without_hash.remove("deliverySetHash");
    let delivery_hash =
        sha256_prefixed(canonical_json(&Value::Object(delivery_without_hash)).as_bytes());
    delivery_set["deliverySetHash"] = Value::String(delivery_hash.clone());
    let delivery_bytes = serde_json::to_vec(&delivery_set).unwrap();
    let path = format!(
        "scenes/trackb/narrative.unit/delivery-sets/{}/trackb-delivery-set-v2.json",
        delivery_hash.strip_prefix("sha256:").unwrap()
    );
    let mut pointer = serde_json::json!({
        "schemaVersion": 2,
        "pointerKey": "pointer.narrative.unit",
        "narrativeId": "narrative.unit",
        "revision": 1,
        "state": "PROMOTED",
        "deliverySetHash": delivery_hash,
        "candidateHash": "sha256:aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
        "approvalHash": "sha256:cccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccc",
        "meaningQualificationHash": "sha256:9999999999999999999999999999999999999999999999999999999999999999",
        "baseDeliverySetHash": "NONE",
        "deliverySetPath": path,
        "pointerHash": ""
    });
    let mut pointer_without_hash = pointer.as_object().unwrap().clone();
    pointer_without_hash.remove("pointerHash");
    let pointer_hash =
        sha256_prefixed(canonical_json(&Value::Object(pointer_without_hash)).as_bytes());
    pointer["pointerHash"] = Value::String(pointer_hash);
    (
        serde_json::to_vec(&pointer).unwrap(),
        delivery_hash,
        delivery_bytes,
        path,
    )
}

fn strict_publish_request(
    expected_base: &str,
    expected_token: &str,
    pointer_bytes: &[u8],
    delivery_hash: &str,
    delivery_bytes: &[u8],
    delivery_token: &str,
) -> Map<String, Value> {
    json_object([
        ("id", Value::String("publish-strict".into())),
        ("op", Value::String("publishPointer".into())),
        ("path", Value::String("delivery-pointer-v2.json".into())),
        (
            "expectedBaseDeliverySetHash",
            Value::String(expected_base.into()),
        ),
        (
            "expectedPointerSnapshotToken",
            Value::String(expected_token.into()),
        ),
        (
            "pointerBytesBase64",
            Value::String(URL_SAFE_NO_PAD.encode(pointer_bytes)),
        ),
        (
            "deliverySetPath",
            Value::String(format!(
                "scenes/trackb/narrative.unit/delivery-sets/{}/trackb-delivery-set-v2.json",
                delivery_hash.strip_prefix("sha256:").unwrap()
            )),
        ),
        (
            "deliverySetBytesBase64",
            Value::String(URL_SAFE_NO_PAD.encode(delivery_bytes)),
        ),
        (
            "deliverySetSnapshotToken",
            Value::String(delivery_token.into()),
        ),
    ])
    .as_object()
    .expect("publish object")
    .clone()
}

fn install_delivery_set(session: &mut Session, hash: &str, bytes: &[u8]) -> String {
    let write = json_object([
        ("id", Value::String("write-ds".into())),
        ("op", Value::String("writeImmutable".into())),
        (
            "path",
            Value::String(format!(
                "delivery-sets/{}/trackb-delivery-set-v2.json",
                hash.strip_prefix("sha256:").unwrap()
            )),
        ),
        ("bytesBase64", Value::String(URL_SAFE_NO_PAD.encode(bytes))),
    ]);
    session
        .write_immutable(write.as_object().unwrap())
        .expect("delivery set write");
    let snapshot = json_object([
        ("id", Value::String("snapshot-ds".into())),
        ("op", Value::String("snapshot".into())),
        (
            "path",
            Value::String(format!(
                "delivery-sets/{}/trackb-delivery-set-v2.json",
                hash.strip_prefix("sha256:").unwrap()
            )),
        ),
    ]);
    session.snapshot(snapshot.as_object().unwrap()).unwrap()["snapshotToken"]
        .as_str()
        .unwrap()
        .to_owned()
}

#[test]
fn strict_pointer_pre_rename_failure_preserves_absence_and_temp_cleanup() {
    let root = test_root("strict-pointer-pre");
    let mut session = session(&root, "op.strict-pointer-pre");
    let (pointer_bytes, delivery_hash, delivery_bytes, _) = strict_fixture();
    let delivery_token = install_delivery_set(&mut session, &delivery_hash, &delivery_bytes);
    let request = strict_publish_request(
        "NONE",
        "NONE",
        &pointer_bytes,
        &delivery_hash,
        &delivery_bytes,
        &delivery_token,
    );
    let error = session
        .publish_pointer_with_test_fault(&request, PublishTestFault::FailBeforeRename)
        .expect_err("pre-rename seam must fail");
    assert_eq!(error.code, "trackb-storage-integrity");
    let narrative = session.narrative_dir.clone().unwrap();
    assert!(!narrative.join("delivery-pointer-v2.json").exists());
    assert!(fs::read_dir(&narrative)
        .unwrap()
        .filter_map(std::result::Result::ok)
        .all(|entry| !entry
            .file_name()
            .to_string_lossy()
            .starts_with(".delivery-pointer-v2.")));
    let _ = session.lock.take();
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn strict_pointer_temp_replacement_is_not_deleted() {
    for fault in [
        PublishTestFault::ReplaceTempRegular,
        PublishTestFault::ReplaceTempSymlink,
    ] {
        let root = test_root("strict-pointer-temp");
        let mut session = session(&root, "op.strict-pointer-temp");
        let (pointer_bytes, delivery_hash, delivery_bytes, _) = strict_fixture();
        let delivery_token = install_delivery_set(&mut session, &delivery_hash, &delivery_bytes);
        let request = strict_publish_request(
            "NONE",
            "NONE",
            &pointer_bytes,
            &delivery_hash,
            &delivery_bytes,
            &delivery_token,
        );
        let error = session
            .publish_pointer_with_test_fault(&request, fault)
            .expect_err("replacement must fail closed");
        assert_eq!(error.code, "trackb-storage-integrity");
        let narrative = session.narrative_dir.clone().unwrap();
        assert!(!narrative.join("delivery-pointer-v2.json").exists());
        assert!(fs::read_dir(&narrative)
            .unwrap()
            .filter_map(std::result::Result::ok)
            .any(|entry| entry
                .file_name()
                .to_string_lossy()
                .starts_with(".delivery-pointer-v2.")));
        let _ = session.lock.take();
        fs::remove_dir_all(root).unwrap();
    }
}

#[test]
fn strict_pointer_cleanup_replacement_between_identity_capture_and_cleanup_is_preserved() {
    // The test seam swaps the temporary entry after cleanup's first
    // fstatat(AT_SYMLINK_NOFOLLOW) and before unlink_exact's recheck.
    // Both a regular replacement and a symlink replacement must remain.
    for fault in [
        PublishTestFault::ReplaceTempAfterCleanupIdentity,
        PublishTestFault::ReplaceTempSymlinkAfterCleanupIdentity,
    ] {
        let root = test_root("strict-pointer-cleanup-race");
        let mut session = session(&root, "op.strict-pointer-cleanup-race");
        let (pointer_bytes, delivery_hash, delivery_bytes, _) = strict_fixture();
        let delivery_token = install_delivery_set(&mut session, &delivery_hash, &delivery_bytes);
        let request = strict_publish_request(
            "NONE",
            "NONE",
            &pointer_bytes,
            &delivery_hash,
            &delivery_bytes,
            &delivery_token,
        );
        let error = session
            .publish_pointer_with_test_fault(&request, fault)
            .expect_err("cleanup replacement seam must fail closed");
        assert_eq!(error.code, "trackb-storage-integrity");
        let narrative = session.narrative_dir.clone().unwrap();
        assert!(!narrative.join("delivery-pointer-v2.json").exists());
        let temporary = fs::read_dir(&narrative)
            .unwrap()
            .filter_map(std::result::Result::ok)
            .find(|entry| {
                entry
                    .file_name()
                    .to_string_lossy()
                    .starts_with(".delivery-pointer-v2.")
            })
            .expect("replacement residue must be retained");
        let temporary_path = temporary.path();
        let metadata = fs::symlink_metadata(&temporary_path).unwrap();
        if matches!(fault, PublishTestFault::ReplaceTempAfterCleanupIdentity) {
            assert!(metadata.is_file());
            assert_eq!(fs::read(&temporary_path).unwrap(), b"cleanup-replacement");
        } else {
            assert!(metadata.file_type().is_symlink());
            assert_eq!(
                fs::read_link(&temporary_path).unwrap(),
                PathBuf::from("test-cleanup-temporary-target")
            );
        }
        let _ = session.lock.take();
        fs::remove_dir_all(root).unwrap();
    }
}

#[test]
fn strict_pointer_post_rename_uncertainty_keeps_new_bytes_without_rollback() {
    for fault in [
        PublishTestFault::FailAfterRenameBeforeSync,
        PublishTestFault::FailAfterSyncBeforeReopen,
    ] {
        let root = test_root("strict-pointer-post");
        let mut session = session(&root, "op.strict-pointer-post");
        let (pointer_bytes, delivery_hash, delivery_bytes, _) = strict_fixture();
        let delivery_token = install_delivery_set(&mut session, &delivery_hash, &delivery_bytes);
        let request = strict_publish_request(
            "NONE",
            "NONE",
            &pointer_bytes,
            &delivery_hash,
            &delivery_bytes,
            &delivery_token,
        );
        let error = session
            .publish_pointer_with_test_fault(&request, fault)
            .expect_err("post-rename seam must require reconcile");
        assert_eq!(error.code, "trackb-pointer-reconcile-required");
        let narrative = session.narrative_dir.clone().unwrap();
        assert_eq!(
            fs::read(narrative.join("delivery-pointer-v2.json")).unwrap(),
            pointer_bytes
        );
        let _ = session.lock.take();
        fs::remove_dir_all(root).unwrap();
    }
}
