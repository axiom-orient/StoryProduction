use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use base64::Engine;
use serde_json::{json, Map, Value};
use sha2::{Digest, Sha256};
use std::fs;
use std::io::{BufRead, BufReader, Write};
use std::path::{Path, PathBuf};
use std::process::{Child, ChildStdin, ChildStdout, Command, Stdio};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

const NARRATIVE: &str = "narrative.test";
const HASH_A: &str = "sha256:aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";
const HASH_B: &str = "sha256:bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb";
const HASH_C: &str = "sha256:cccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccc";
const HASH_D: &str = "sha256:dddddddddddddddddddddddddddddddddddddddddddddddddddddddddddddddd";
const HASH_E: &str = "sha256:eeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeee";
const HASH_F: &str = "sha256:ffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffff";
const HASH_1: &str = "sha256:1111111111111111111111111111111111111111111111111111111111111111";

struct Harness {
    child: Child,
    stdin: ChildStdin,
    stdout: BufReader<ChildStdout>,
    operation: String,
}

impl Harness {
    fn start(root: &Path, operation: &str) -> Self {
        Self::start_mode_with_purpose(root, operation, "mutable", "narrative")
    }

    fn start_read_only(root: &Path, operation: &str) -> Self {
        Self::start_mode_with_purpose(root, operation, "readOnly", "narrative")
    }

    fn start_project(root: &Path, operation: &str) -> Self {
        Self::start_purpose(root, operation, "project-ingress")
    }

    fn start_measurement(root: &Path, operation: &str) -> Self {
        Self::start_purpose(root, operation, "measurement")
    }

    fn start_purpose(root: &Path, operation: &str, purpose: &str) -> Self {
        let mut child = Command::new(env!("CARGO_BIN_EXE_smc-trackb-custody"))
            .arg("--project-root")
            .arg(root)
            .arg("--purpose")
            .arg(purpose)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .expect("spawn custody helper");
        let stdin = child.stdin.take().expect("child stdin");
        let stdout = BufReader::new(child.stdout.take().expect("child stdout"));
        Self {
            child,
            stdin,
            stdout,
            operation: operation.to_owned(),
        }
    }

    fn start_mode_with_purpose(root: &Path, operation: &str, mode: &str, purpose: &str) -> Self {
        let mut harness = Self::start_purpose(root, operation, purpose);
        let initialized = harness.request(json!({
            "id": "init",
            "op": "initialize",
            "mode": mode,
            "narrativeId": NARRATIVE,
            "operationId": operation,
        }));
        assert!(initialized["ok"].as_bool().unwrap(), "{initialized:?}");
        assert_eq!(initialized["result"]["mode"], mode);
        harness
    }

    fn acquire(&mut self) -> Value {
        self.request(json!({
            "id": "acquire",
            "op": "acquire",
            "narrativeId": NARRATIVE,
            "operationId": self.operation.clone(),
        }))
    }

    fn request(&mut self, request: Value) -> Value {
        let line = serde_json::to_vec(&request).unwrap();
        self.stdin.write_all(&line).unwrap();
        self.stdin.write_all(b"\n").unwrap();
        self.stdin.flush().unwrap();
        let mut response = String::new();
        self.stdout.read_line(&mut response).unwrap();
        serde_json::from_str(response.trim()).unwrap()
    }

    fn close(mut self) {
        let response = self.request(json!({ "id": "close", "op": "close" }));
        assert!(response["ok"].as_bool().unwrap(), "{response:?}");
        self.child.wait().unwrap();
    }
}

impl Drop for Harness {
    fn drop(&mut self) {
        if self.child.try_wait().unwrap().is_none() {
            let _ = self.child.kill();
            let _ = self.child.wait();
        }
    }
}

fn project_root(label: &str) -> PathBuf {
    let stamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let path = fs::canonicalize(std::env::temp_dir())
        .unwrap()
        .join(format!(
            "smc-trackb-custody-{label}-{}-{stamp}",
            std::process::id()
        ));
    fs::create_dir_all(&path).unwrap();
    path
}

fn pointer_path(root: &Path) -> PathBuf {
    root.join("scenes/trackb/narrative.test/delivery-pointer-v2.json")
}

fn project_artifact_request(
    id: &str,
    kind: &str,
    path: &str,
    projection_receipt_hash: Option<&str>,
) -> Value {
    let mut request = json!({
        "id": id,
        "op": "readProjectArtifact",
        "kind": kind,
        "path": path
    });
    if let Some(hash) = projection_receipt_hash {
        request["projectionReceiptHash"] = Value::String(hash.to_owned());
    }
    request
}

fn measurement_artifact_request(id: &str, kind: &str, path: &str) -> Value {
    json!({
        "id": id,
        "op": "readMeasurementArtifact",
        "kind": kind,
        "path": path
    })
}

fn raw_hash(bytes: &[u8]) -> String {
    let digest = Sha256::digest(bytes);
    format!("sha256:{digest:x}")
}

fn assert_project_artifact(response: &Value, kind: &str, path: &str, bytes: &[u8]) {
    assert!(response["ok"].as_bool().unwrap(), "{response:?}");
    assert_eq!(response["result"]["kind"], kind);
    assert_eq!(response["result"]["path"], path);
    assert_eq!(response["result"]["exists"], true);
    assert_eq!(
        response["result"]["bytesBase64"],
        URL_SAFE_NO_PAD.encode(bytes)
    );
    assert_eq!(response["result"]["byteLength"], bytes.len());
    assert_eq!(response["result"]["sha256"], raw_hash(bytes));
}

fn assert_measurement_artifact(response: &Value, kind: &str, path: &str, bytes: &[u8]) {
    assert!(response["ok"].as_bool().unwrap(), "{response:?}");
    assert_eq!(response["result"]["kind"], kind);
    assert_eq!(response["result"]["path"], path);
    assert_eq!(response["result"]["exists"], true);
    assert_eq!(
        response["result"]["bytesBase64"],
        URL_SAFE_NO_PAD.encode(bytes)
    );
    assert_eq!(response["result"]["byteLength"], bytes.len());
    assert_eq!(response["result"]["sha256"], raw_hash(bytes));
}

fn delivery_set_relative(hash: &str) -> String {
    format!(
        "delivery-sets/{}/trackb-delivery-set-v2.json",
        hash.strip_prefix("sha256:").unwrap()
    )
}

fn delivery_set_project_path(hash: &str) -> String {
    format!(
        "scenes/trackb/{NARRATIVE}/delivery-sets/{}/trackb-delivery-set-v2.json",
        hash.strip_prefix("sha256:").unwrap()
    )
}

fn canonical_json(value: &Value) -> String {
    match value {
        Value::Null => "null".to_owned(),
        Value::Bool(value) => value.to_string(),
        Value::Number(value) => value.to_string(),
        Value::String(value) => serde_json::to_string(value).unwrap(),
        Value::Array(values) => format!(
            "[{}]",
            values
                .iter()
                .map(canonical_json)
                .collect::<Vec<_>>()
                .join(",")
        ),
        Value::Object(values) => {
            let mut keys = values.keys().collect::<Vec<_>>();
            keys.sort();
            format!(
                "{{{}}}",
                keys.into_iter()
                    .map(|key| format!(
                        "{}:{}",
                        serde_json::to_string(key).unwrap(),
                        canonical_json(&values[key])
                    ))
                    .collect::<Vec<_>>()
                    .join(",")
            )
        }
    }
}

fn self_hash(value: &Value, field: &str) -> String {
    let mut object = value.as_object().unwrap().clone();
    object.remove(field);
    let digest = Sha256::digest(canonical_json(&Value::Object(object)).as_bytes());
    format!("sha256:{digest:x}")
}

fn fixture() -> (Value, Vec<u8>, String, Vec<u8>, String) {
    let mut delivery_set = json!({
        "schemaVersion": 2,
        "deliverySetId": "delivery.test.v1",
        "narrativeId": NARRATIVE,
        "candidateHash": HASH_A,
        "authorityHash": HASH_B,
        "meaningQualificationHash": HASH_1,
        "baseDeliverySetHash": "NONE",
        "approvalHash": HASH_C,
        "artifacts": {
            "projectionReceiptHash": HASH_D,
            "sceneGraphHash": HASH_E,
            "assetHandoffHash": HASH_F,
            "assetHandoffSceneGraphHash": HASH_E,
            "htmlHash": HASH_A,
            "provenanceHash": HASH_B
        },
        "evidenceCellHashes": [HASH_1],
        "revision": 1,
        "state": "PROMOTED",
        "deliverySetHash": ""
    });
    let delivery_hash = self_hash(&delivery_set, "deliverySetHash");
    delivery_set["deliverySetHash"] = Value::String(delivery_hash.clone());
    let delivery_bytes = serde_json::to_vec(&delivery_set).unwrap();
    let delivery_path = delivery_set_project_path(&delivery_hash);

    let mut pointer = json!({
        "schemaVersion": 2,
        "pointerKey": "pointer.narrative.test",
        "narrativeId": NARRATIVE,
        "revision": 1,
        "state": "PROMOTED",
        "deliverySetHash": delivery_hash,
        "candidateHash": HASH_A,
        "approvalHash": HASH_C,
        "meaningQualificationHash": HASH_1,
        "baseDeliverySetHash": "NONE",
        "deliverySetPath": delivery_path,
        "pointerHash": ""
    });
    let pointer_hash = self_hash(&pointer, "pointerHash");
    pointer["pointerHash"] = Value::String(pointer_hash);
    let pointer_bytes = serde_json::to_vec(&pointer).unwrap();
    (
        pointer,
        pointer_bytes,
        delivery_hash,
        delivery_bytes,
        delivery_path,
    )
}

fn publish_request(
    id: &str,
    expected_base: &str,
    expected_pointer_token: &str,
    pointer_bytes: &[u8],
    delivery_path: &str,
    delivery_bytes: &[u8],
    delivery_token: &str,
) -> Value {
    json!({
        "id": id,
        "op": "publishPointer",
        "path": "delivery-pointer-v2.json",
        "expectedBaseDeliverySetHash": expected_base,
        "expectedPointerSnapshotToken": expected_pointer_token,
        "pointerBytesBase64": URL_SAFE_NO_PAD.encode(pointer_bytes),
        "deliverySetPath": delivery_path,
        "deliverySetBytesBase64": URL_SAFE_NO_PAD.encode(delivery_bytes),
        "deliverySetSnapshotToken": delivery_token
    })
}

fn write_delivery_set(helper: &mut Harness, delivery_hash: &str, bytes: &[u8]) -> String {
    let response = helper.request(json!({
        "id": "write-delivery-set",
        "op": "writeImmutable",
        "path": delivery_set_relative(delivery_hash),
        "bytesBase64": URL_SAFE_NO_PAD.encode(bytes),
    }));
    assert!(response["ok"].as_bool().unwrap(), "{response:?}");
    let snapshot = helper.request(json!({
        "id": "snapshot-delivery-set",
        "op": "snapshot",
        "path": delivery_set_relative(delivery_hash),
    }));
    snapshot["result"]["snapshotToken"]
        .as_str()
        .unwrap()
        .to_owned()
}

#[test]
fn measurement_purpose_enforces_exact_order_kind_bounds_and_immutable_publish() {
    let root = project_root("measurement-protocol");
    let authority = b"{\"authority\":true}";
    let html = b"<!doctype html><html><body>measurement</body></html>";
    let provenance = b"{\"schemaVersion\":2}";
    fs::create_dir_all(root.join("source")).unwrap();
    fs::write(root.join("source/authority.json"), authority).unwrap();
    fs::write(root.join("source/scene.html"), html).unwrap();
    fs::write(root.join("source/provenance.json"), provenance).unwrap();

    let mut helper = Harness::start_measurement(&root, "op.measurement-protocol");
    let wrong_order = helper.request(measurement_artifact_request(
        "wrong-order",
        "html",
        "source/scene.html",
    ));
    assert_eq!(wrong_order["error"]["code"], "trackb-input-invalid");

    assert_measurement_artifact(
        &helper.request(measurement_artifact_request(
            "authority",
            "narrativeAuthority",
            "source/authority.json",
        )),
        "narrativeAuthority",
        "source/authority.json",
        authority,
    );
    assert_measurement_artifact(
        &helper.request(measurement_artifact_request(
            "html",
            "html",
            "source/scene.html",
        )),
        "html",
        "source/scene.html",
        html,
    );
    assert_measurement_artifact(
        &helper.request(measurement_artifact_request(
            "provenance",
            "provenance",
            "source/provenance.json",
        )),
        "provenance",
        "source/provenance.json",
        provenance,
    );
    let duplicate = helper.request(measurement_artifact_request(
        "duplicate",
        "provenance",
        "source/provenance.json",
    ));
    assert_eq!(duplicate["error"]["code"], "trackb-input-invalid");

    let receipt = b"{\"schemaVersion\":1,\"receiptHash\":\"sha256:test\"}";
    let first = helper.request(json!({
        "id": "publish-first",
        "op": "publishMeasurementReceipt",
        "path": "out/scene.evidence-receipt.json",
        "bytesBase64": URL_SAFE_NO_PAD.encode(receipt)
    }));
    assert!(first["ok"].as_bool().unwrap(), "{first:?}");
    assert_eq!(first["result"]["reused"], false);
    assert_eq!(
        first["result"]["snapshot"]["bytesBase64"],
        URL_SAFE_NO_PAD.encode(receipt)
    );
    let reused = helper.request(json!({
        "id": "publish-reused",
        "op": "publishMeasurementReceipt",
        "path": "out/scene.evidence-receipt.json",
        "bytesBase64": URL_SAFE_NO_PAD.encode(receipt)
    }));
    assert!(reused["ok"].as_bool().unwrap(), "{reused:?}");
    assert_eq!(reused["result"]["reused"], true);
    let conflict = helper.request(json!({
        "id": "publish-conflict",
        "op": "publishMeasurementReceipt",
        "path": "out/scene.evidence-receipt.json",
        "bytesBase64": URL_SAFE_NO_PAD.encode(b"different")
    }));
    assert_eq!(conflict["error"]["code"], "trackb-storage-integrity");
    helper.close();

    assert_eq!(
        fs::read(root.join("out/scene.evidence-receipt.json")).unwrap(),
        receipt
    );
    assert!(fs::read_dir(root.join("out"))
        .unwrap()
        .filter_map(std::result::Result::ok)
        .all(|entry| !entry
            .file_name()
            .to_string_lossy()
            .starts_with(".smc-trackb-measurement-")));
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn measurement_purpose_rejects_unsafe_and_symlinked_artifact_paths() {
    let root = project_root("measurement-paths");
    let target = project_root("measurement-path-target");
    fs::write(target.join("authority.json"), b"target").unwrap();
    fs::create_dir_all(root.join("safe")).unwrap();
    fs::write(root.join("safe/authority.json"), b"safe").unwrap();
    std::os::unix::fs::symlink(
        target.join("authority.json"),
        root.join("terminal-link.json"),
    )
    .unwrap();
    std::os::unix::fs::symlink(&target, root.join("ancestor-link")).unwrap();

    let mut helper = Harness::start_measurement(&root, "op.measurement-paths");
    let traversal = helper.request(measurement_artifact_request(
        "traversal",
        "narrativeAuthority",
        "../authority.json",
    ));
    assert_eq!(traversal["error"]["code"], "trackb-input-invalid");
    for path in ["terminal-link.json", "ancestor-link/authority.json"] {
        let response = helper.request(measurement_artifact_request(
            "unsafe-path",
            "narrativeAuthority",
            path,
        ));
        assert_eq!(response["error"]["code"], "trackb-storage-integrity");
        // A failed first read does not advance the fixed sequence.
    }
    let valid = helper.request(measurement_artifact_request(
        "valid",
        "narrativeAuthority",
        "safe/authority.json",
    ));
    assert_measurement_artifact(&valid, "narrativeAuthority", "safe/authority.json", b"safe");
    helper.close();
    assert_eq!(fs::read(target.join("authority.json")).unwrap(), b"target");
    fs::remove_dir_all(root).unwrap();
    fs::remove_dir_all(target).unwrap();
}

#[test]
fn measurement_purpose_applies_kind_specific_bounds() {
    for (label, kind, path, size) in [
        (
            "authority",
            "narrativeAuthority",
            "authority.bin",
            8 * 1024 * 1024 + 1,
        ),
        ("html", "html", "html.bin", 16 * 1024 * 1024 + 1),
        (
            "provenance",
            "provenance",
            "provenance.bin",
            1024 * 1024 + 1,
        ),
    ] {
        let root = project_root(&format!("measurement-bound-{label}"));
        if kind != "narrativeAuthority" {
            fs::write(root.join("authority.json"), b"authority").unwrap();
        }
        if kind == "provenance" {
            fs::write(root.join("html.bin"), b"html").unwrap();
        }
        fs::write(root.join(path), vec![b'x'; size]).unwrap();
        let mut helper =
            Harness::start_measurement(&root, &format!("op.measurement-bound-{label}"));
        if kind != "narrativeAuthority" {
            let authority = helper.request(measurement_artifact_request(
                "authority",
                "narrativeAuthority",
                "authority.json",
            ));
            assert_measurement_artifact(
                &authority,
                "narrativeAuthority",
                "authority.json",
                b"authority",
            );
        }
        if kind == "provenance" {
            let html = helper.request(measurement_artifact_request("html", "html", "html.bin"));
            assert_measurement_artifact(&html, "html", "html.bin", b"html");
        }
        let response = helper.request(measurement_artifact_request("oversize", kind, path));
        assert_eq!(response["error"]["code"], "trackb-storage-integrity");
        helper.close();
        fs::remove_dir_all(root).unwrap();
    }
}

#[test]
fn project_artifact_reads_enforce_kind_order_once_and_initialize_boundary() {
    let root = project_root("project-artifact-order");
    let artifacts = [
        (
            "authority/narrative-authority.json",
            b"authority".as_slice(),
        ),
        (
            "projection/narrative-scene-projection.v3.json",
            b"projection".as_slice(),
        ),
        (
            "handoff/scene-motion-asset-handoff.v2.json",
            b"asset-handoff".as_slice(),
        ),
        ("out/scene.html", b"html".as_slice()),
        ("out/scene.provenance.json", b"provenance".as_slice()),
        (
            "projection/generations/generation.001/scene-graph.json",
            b"scene-graph".as_slice(),
        ),
        (
            "out/scene.evidence-receipt.json",
            b"evidence-receipt".as_slice(),
        ),
    ];
    for (path, bytes) in artifacts {
        let absolute = root.join(path);
        fs::create_dir_all(absolute.parent().unwrap()).unwrap();
        fs::write(absolute, bytes).unwrap();
    }

    let mut helper = Harness::start_project(&root, "op.project-artifact-order");
    let scene_before_receipt = helper.request(project_artifact_request(
        "scene-before-receipt",
        "sceneGraph",
        "projection/generations/generation.001/scene-graph.json",
        Some(HASH_A),
    ));
    assert_eq!(
        scene_before_receipt["error"]["code"],
        "trackb-input-invalid"
    );

    let out_of_order_projection = helper.request(project_artifact_request(
        "projection-before-authority",
        "projectionReceipt",
        "projection/narrative-scene-projection.v3.json",
        None,
    ));
    assert_eq!(
        out_of_order_projection["error"]["code"],
        "trackb-input-invalid"
    );

    for (id, kind, path, bytes) in [
        (
            "authority",
            "narrativeAuthority",
            "authority/narrative-authority.json",
            b"authority".as_slice(),
        ),
        (
            "projection",
            "projectionReceipt",
            "projection/narrative-scene-projection.v3.json",
            b"projection".as_slice(),
        ),
    ] {
        let response = helper.request(project_artifact_request(id, kind, path, None));
        assert_project_artifact(&response, kind, path, bytes);
    }
    let projection_hash = raw_hash(b"projection");
    let wrong_hash = helper.request(project_artifact_request(
        "scene-wrong-hash",
        "sceneGraph",
        "projection/generations/generation.001/scene-graph.json",
        Some(HASH_A),
    ));
    assert_eq!(wrong_hash["error"]["code"], "trackb-input-invalid");

    for (id, kind, path, bytes) in [
        (
            "handoff",
            "assetHandoff",
            "handoff/scene-motion-asset-handoff.v2.json",
            b"asset-handoff".as_slice(),
        ),
        ("html", "html", "out/scene.html", b"html".as_slice()),
        (
            "provenance",
            "provenance",
            "out/scene.provenance.json",
            b"provenance".as_slice(),
        ),
        (
            "evidence-receipt",
            "evidenceReceipt",
            "out/scene.evidence-receipt.json",
            b"evidence-receipt".as_slice(),
        ),
    ] {
        let response = helper.request(project_artifact_request(id, kind, path, None));
        assert_project_artifact(&response, kind, path, bytes);
    }
    let scene = helper.request(project_artifact_request(
        "scene",
        "sceneGraph",
        "projection/generations/generation.001/scene-graph.json",
        Some(&projection_hash),
    ));
    assert_project_artifact(
        &scene,
        "sceneGraph",
        "projection/generations/generation.001/scene-graph.json",
        b"scene-graph",
    );
    let duplicate = helper.request(project_artifact_request(
        "duplicate-projection",
        "projectionReceipt",
        "projection/narrative-scene-projection.v3.json",
        None,
    ));
    assert_eq!(duplicate["error"]["code"], "trackb-input-invalid");
    let unknown = helper.request(project_artifact_request(
        "unknown-kind",
        "unknown",
        "out/scene.html",
        None,
    ));
    assert_eq!(unknown["error"]["code"], "trackb-input-invalid");
    helper.close();

    let mut initialized = Harness::start_project(&root, "op.project-artifact-initialized");
    let init = initialized.request(json!({
        "id": "initialize",
        "op": "initialize",
        "mode": "mutable",
        "narrativeId": NARRATIVE,
        "operationId": "op.project-artifact-initialized"
    }));
    assert_eq!(init["error"]["code"], "trackb-input-invalid");
    let after_initialize = initialized.request(project_artifact_request(
        "after-initialize",
        "narrativeAuthority",
        "authority/narrative-authority.json",
        None,
    ));
    assert_project_artifact(
        &after_initialize,
        "narrativeAuthority",
        "authority/narrative-authority.json",
        b"authority",
    );
    initialized.close();

    let mut narrative = Harness::start(&root, "op.project-artifact-narrative");
    let forbidden = narrative.request(project_artifact_request(
        "narrative-purpose",
        "narrativeAuthority",
        "authority/narrative-authority.json",
        None,
    ));
    assert_eq!(forbidden["error"]["code"], "trackb-input-invalid");
    narrative.close();
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn project_ingress_requires_complete_order_and_present_artifacts_before_initialize() {
    let root = project_root("project-ingress-gate");
    fs::write(root.join("artifact.bin"), b"project-artifact").unwrap();
    let artifact_hash = raw_hash(b"project-artifact");

    let mut zero = Harness::start_project(&root, "op.project-ingress-zero");
    let zero_init = zero.request(json!({
        "id": "zero-init",
        "op": "initialize",
        "mode": "readOnly",
        "narrativeId": NARRATIVE,
        "operationId": "op.project-ingress-zero"
    }));
    assert_eq!(zero_init["error"]["code"], "trackb-input-invalid");
    zero.close();

    let mut partial = Harness::start_project(&root, "op.project-ingress-partial");
    let authority = partial.request(project_artifact_request(
        "partial-authority",
        "narrativeAuthority",
        "artifact.bin",
        None,
    ));
    assert_project_artifact(
        &authority,
        "narrativeAuthority",
        "artifact.bin",
        b"project-artifact",
    );
    let partial_init = partial.request(json!({
        "id": "partial-init",
        "op": "initialize",
        "mode": "readOnly",
        "narrativeId": NARRATIVE,
        "operationId": "op.project-ingress-partial"
    }));
    assert_eq!(partial_init["error"]["code"], "trackb-input-invalid");
    partial.close();

    let mut out_of_order = Harness::start_project(&root, "op.project-ingress-order");
    let out_of_order_response = out_of_order.request(project_artifact_request(
        "order-projection",
        "projectionReceipt",
        "artifact.bin",
        None,
    ));
    assert_eq!(
        out_of_order_response["error"]["code"],
        "trackb-input-invalid"
    );
    let order_init = out_of_order.request(json!({
        "id": "order-init",
        "op": "initialize",
        "mode": "readOnly",
        "narrativeId": NARRATIVE,
        "operationId": "op.project-ingress-order"
    }));
    assert_eq!(order_init["error"]["code"], "trackb-input-invalid");
    out_of_order.close();

    let mut absent = Harness::start_project(&root, "op.project-ingress-absent");
    let absent_authority = absent.request(project_artifact_request(
        "absent-authority",
        "narrativeAuthority",
        "missing/artifact.bin",
        None,
    ));
    assert!(
        absent_authority["ok"].as_bool().unwrap(),
        "{absent_authority:?}"
    );
    assert_eq!(absent_authority["result"]["exists"], false);
    for (id, kind) in [
        ("absent-projection", "projectionReceipt"),
        ("absent-handoff", "assetHandoff"),
        ("absent-html", "html"),
        ("absent-provenance", "provenance"),
        ("absent-evidence-receipt", "evidenceReceipt"),
    ] {
        let response = absent.request(project_artifact_request(id, kind, "artifact.bin", None));
        assert_project_artifact(&response, kind, "artifact.bin", b"project-artifact");
    }
    let absent_scene = absent.request(project_artifact_request(
        "absent-scene",
        "sceneGraph",
        "artifact.bin",
        Some(&artifact_hash),
    ));
    assert_project_artifact(
        &absent_scene,
        "sceneGraph",
        "artifact.bin",
        b"project-artifact",
    );
    let absent_init = absent.request(json!({
        "id": "absent-init",
        "op": "initialize",
        "mode": "readOnly",
        "narrativeId": NARRATIVE,
        "operationId": "op.project-ingress-absent"
    }));
    assert_eq!(absent_init["error"]["code"], "trackb-input-invalid");
    absent.close();

    let mut complete = Harness::start_project(&root, "op.project-ingress-complete");
    for (id, kind) in [
        ("complete-authority", "narrativeAuthority"),
        ("complete-projection", "projectionReceipt"),
        ("complete-handoff", "assetHandoff"),
        ("complete-html", "html"),
        ("complete-provenance", "provenance"),
        ("complete-evidence-receipt", "evidenceReceipt"),
    ] {
        let response = complete.request(project_artifact_request(id, kind, "artifact.bin", None));
        assert_project_artifact(&response, kind, "artifact.bin", b"project-artifact");
    }
    let complete_scene = complete.request(project_artifact_request(
        "complete-scene",
        "sceneGraph",
        "artifact.bin",
        Some(&artifact_hash),
    ));
    assert_project_artifact(
        &complete_scene,
        "sceneGraph",
        "artifact.bin",
        b"project-artifact",
    );
    let initialized = complete.request(json!({
        "id": "complete-init",
        "op": "initialize",
        "mode": "readOnly",
        "narrativeId": NARRATIVE,
        "operationId": "op.project-ingress-complete"
    }));
    assert!(initialized["ok"].as_bool().unwrap(), "{initialized:?}");
    let after_initialize = complete.request(project_artifact_request(
        "complete-after-init",
        "narrativeAuthority",
        "artifact.bin",
        None,
    ));
    assert_eq!(after_initialize["error"]["code"], "trackb-input-invalid");
    complete.close();
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn project_artifact_reads_fail_closed_for_absent_symlink_nonregular_and_oversize_entries() {
    let root = project_root("project-artifact-integrity");
    let target = project_root("project-artifact-integrity-target");
    fs::write(target.join("target.json"), b"target-bytes").unwrap();
    fs::create_dir_all(root.join("safe")).expect("safe directory");
    fs::write(root.join("safe/regular.json"), b"regular").unwrap();
    fs::create_dir_all(root.join("directory.json")).unwrap();
    fs::create_dir_all(root.join("large")).unwrap();
    fs::write(
        root.join("large/large.bin"),
        vec![b'x'; 8 * 1024 * 1024 + 1],
    )
    .unwrap();
    std::os::unix::fs::symlink(target.join("target.json"), root.join("terminal-link.json"))
        .unwrap();
    std::os::unix::fs::symlink(&target, root.join("ancestor-link")).unwrap();

    let mut absent_helper = Harness::start_project(&root, "op.project-artifact-absent");
    let absent_parent = absent_helper.request(project_artifact_request(
        "absent-parent",
        "narrativeAuthority",
        "missing/absent.json",
        None,
    ));
    assert!(absent_parent["ok"].as_bool().unwrap(), "{absent_parent:?}");
    assert_eq!(absent_parent["result"]["exists"], false);
    let absent_final = absent_helper.request(project_artifact_request(
        "absent-final",
        "projectionReceipt",
        "safe/missing.json",
        None,
    ));
    assert!(absent_final["ok"].as_bool().unwrap(), "{absent_final:?}");
    assert_eq!(absent_final["result"]["exists"], false);
    absent_helper.close();

    let mut terminal_helper = Harness::start_project(&root, "op.project-artifact-terminal-link");
    let terminal_link = terminal_helper.request(project_artifact_request(
        "terminal-link",
        "narrativeAuthority",
        "terminal-link.json",
        None,
    ));
    assert_eq!(terminal_link["error"]["code"], "trackb-storage-integrity");
    terminal_helper.close();

    let mut ancestor_helper = Harness::start_project(&root, "op.project-artifact-ancestor-link");
    let ancestor_link = ancestor_helper.request(project_artifact_request(
        "ancestor-link",
        "narrativeAuthority",
        "ancestor-link/target.json",
        None,
    ));
    assert_eq!(ancestor_link["error"]["code"], "trackb-storage-integrity");
    ancestor_helper.close();

    let mut nonregular_helper = Harness::start_project(&root, "op.project-artifact-nonregular");
    let nonregular = nonregular_helper.request(project_artifact_request(
        "nonregular",
        "narrativeAuthority",
        "directory.json",
        None,
    ));
    assert_eq!(nonregular["error"]["code"], "trackb-storage-integrity");
    nonregular_helper.close();

    let mut oversize_helper = Harness::start_project(&root, "op.project-artifact-oversize");
    let oversize = oversize_helper.request(project_artifact_request(
        "oversize",
        "narrativeAuthority",
        "large/large.bin",
        None,
    ));
    assert_eq!(oversize["error"]["code"], "trackb-storage-integrity");
    oversize_helper.close();

    assert_eq!(
        fs::read(target.join("target.json")).unwrap(),
        b"target-bytes"
    );
    fs::remove_dir_all(root).unwrap();
    fs::remove_dir_all(target).unwrap();
}

#[test]
fn project_artifact_reads_use_retained_project_fd_after_project_path_replacement() {
    let root = project_root("project-artifact-replacement");
    fs::write(root.join("artifact.bin"), b"original-project-bytes").unwrap();
    let moved = root.with_extension("original");
    let mut helper = Harness::start_project(&root, "op.project-artifact-replacement");
    // The first read is also the protocol handshake: it proves the helper has
    // completed startup and retained the original project descriptor before
    // the pathname is replaced.
    let preflight = helper.request(project_artifact_request(
        "preflight",
        "narrativeAuthority",
        "artifact.bin",
        None,
    ));
    assert_project_artifact(
        &preflight,
        "narrativeAuthority",
        "artifact.bin",
        b"original-project-bytes",
    );
    fs::rename(&root, &moved).unwrap();
    fs::create_dir_all(&root).unwrap();
    fs::write(root.join("artifact.bin"), b"replacement-project-bytes").unwrap();

    let projection = helper.request(project_artifact_request(
        "retained-project-projection",
        "projectionReceipt",
        "artifact.bin",
        None,
    ));
    assert_project_artifact(
        &projection,
        "projectionReceipt",
        "artifact.bin",
        b"original-project-bytes",
    );
    let handoff = helper.request(project_artifact_request(
        "retained-project-handoff",
        "assetHandoff",
        "artifact.bin",
        None,
    ));
    assert_project_artifact(
        &handoff,
        "assetHandoff",
        "artifact.bin",
        b"original-project-bytes",
    );
    let response = helper.request(project_artifact_request(
        "retained-project-html",
        "html",
        "artifact.bin",
        None,
    ));
    assert_project_artifact(&response, "html", "artifact.bin", b"original-project-bytes");
    helper.close();
    assert_eq!(
        fs::read(moved.join("artifact.bin")).unwrap(),
        b"original-project-bytes"
    );
    assert_eq!(
        fs::read(root.join("artifact.bin")).unwrap(),
        b"replacement-project-bytes"
    );
    fs::remove_dir_all(root).unwrap();
    fs::remove_dir_all(moved).unwrap();
}

#[test]
fn immutable_namespace_rejects_pointer_bypass_and_accepts_canonical_artifact() {
    let root = project_root("namespace");
    let mut helper = Harness::start(&root, "op.namespace");
    assert!(helper.acquire()["ok"].as_bool().unwrap());
    let forbidden = helper.request(json!({
        "id": "pointer-write",
        "op": "writeImmutable",
        "path": "delivery-pointer-v2.json",
        "bytesBase64": "eA"
    }));
    assert_eq!(forbidden["error"]["code"], "trackb-input-invalid");
    let accepted = helper.request(json!({
        "id": "candidate-write",
        "op": "writeImmutable",
        "path": format!("candidates/{}/trackb-release-candidate-v4.json", "a".repeat(64)),
        "bytesBase64": "eA"
    }));
    assert!(accepted["ok"].as_bool().unwrap(), "{accepted:?}");
    let rejected_legacy = helper.request(json!({
        "id": "candidate-write-v3",
        "op": "writeImmutable",
        "path": format!("candidates/{}/trackb-release-candidate-v3.json", "a".repeat(64)),
        "bytesBase64": "eA"
    }));
    assert_eq!(rejected_legacy["error"]["code"], "trackb-input-invalid");
    helper.close();
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn read_only_empty_project_returns_absent_without_creating_scenes_or_lock() {
    let root = project_root("read-only-empty");
    let before = fs::metadata(&root).unwrap().modified().unwrap();
    let mut helper = Harness::start_read_only(&root, "op.read-only-empty");

    let snapshot = helper.request(json!({
        "id": "empty-snapshot",
        "op": "snapshot",
        "path": "delivery-pointer-v2.json"
    }));
    assert!(snapshot["ok"].as_bool().unwrap(), "{snapshot:?}");
    assert_eq!(snapshot["result"]["exists"], false);
    let read = helper.request(json!({
        "id": "empty-read",
        "op": "read",
        "path": "delivery-pointer-v2.json"
    }));
    assert!(read["ok"].as_bool().unwrap(), "{read:?}");
    assert_eq!(read["result"]["exists"], false);
    let acquire = helper.acquire();
    assert_eq!(acquire["error"]["code"], "trackb-input-invalid");
    let write = helper.request(json!({
        "id": "empty-write",
        "op": "writeImmutable",
        "path": "delivery-pointer-v2.json",
        "bytesBase64": "eA"
    }));
    assert_eq!(write["error"]["code"], "trackb-input-invalid");
    let publish = helper.request(json!({
        "id": "empty-publish",
        "op": "publishPointer",
        "path": "delivery-pointer-v2.json",
        "expectedBaseDeliverySetHash": "NONE",
        "expectedPointerSnapshotToken": "NONE",
        "pointerBytesBase64": "eA",
        "deliverySetPath": "delivery-sets/a/trackb-delivery-set-v2.json",
        "deliverySetBytesBase64": "eA",
        "deliverySetSnapshotToken": "opaque"
    }));
    assert_eq!(publish["error"]["code"], "trackb-input-invalid");
    helper.close();

    assert!(!root.join("scenes").exists());
    assert_eq!(fs::metadata(&root).unwrap().modified().unwrap(), before);
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn read_only_existing_namespace_reads_bounded_files_without_lock_or_mtime_changes() {
    let root = project_root("read-only-existing");
    let mut writer = Harness::start(&root, "op.read-only-writer");
    assert!(writer.acquire()["ok"].as_bool().unwrap());
    let (_pointer, pointer_bytes, delivery_hash, delivery_bytes, delivery_path) = fixture();
    let delivery_token = write_delivery_set(&mut writer, &delivery_hash, &delivery_bytes);
    let absent = writer.request(json!({
        "id": "writer-absent",
        "op": "snapshot",
        "path": "delivery-pointer-v2.json"
    }));
    let published = writer.request(publish_request(
        "writer-publish",
        "NONE",
        absent["result"]["snapshotToken"].as_str().unwrap(),
        &pointer_bytes,
        &delivery_path,
        &delivery_bytes,
        &delivery_token,
    ));
    assert!(published["ok"].as_bool().unwrap(), "{published:?}");
    writer.close();

    let narrative = root.join("scenes/trackb").join(NARRATIVE);
    let pointer = pointer_path(&root);
    let lock = narrative.join(".release.lock");
    assert!(!lock.exists());
    let root_mtime = fs::metadata(&root).unwrap().modified().unwrap();
    let narrative_mtime = fs::metadata(&narrative).unwrap().modified().unwrap();
    let pointer_mtime = fs::metadata(&pointer).unwrap().modified().unwrap();

    let mut reader = Harness::start_read_only(&root, "op.read-only-reader");
    let snapshot = reader.request(json!({
        "id": "reader-snapshot",
        "op": "snapshot",
        "path": "delivery-pointer-v2.json"
    }));
    assert!(snapshot["ok"].as_bool().unwrap(), "{snapshot:?}");
    assert_eq!(snapshot["result"]["exists"], true);
    assert_eq!(
        snapshot["result"]["bytesBase64"],
        URL_SAFE_NO_PAD.encode(&pointer_bytes)
    );
    let read = reader.request(json!({
        "id": "reader-read",
        "op": "read",
        "path": "delivery-pointer-v2.json"
    }));
    assert!(read["ok"].as_bool().unwrap(), "{read:?}");
    assert_eq!(
        read["result"]["bytesBase64"],
        URL_SAFE_NO_PAD.encode(&pointer_bytes)
    );
    let missing = reader.request(json!({
        "id": "reader-missing",
        "op": "read",
        "path": format!("candidates/{}/trackb-release-candidate-v3.json", "f".repeat(64))
    }));
    assert!(missing["ok"].as_bool().unwrap(), "{missing:?}");
    assert_eq!(missing["result"]["exists"], false);
    assert_eq!(reader.acquire()["error"]["code"], "trackb-input-invalid");
    let write = reader.request(json!({
        "id": "reader-write",
        "op": "writeImmutable",
        "path": format!("candidates/{}/trackb-release-candidate-v3.json", "f".repeat(64)),
        "bytesBase64": "eA"
    }));
    assert_eq!(write["error"]["code"], "trackb-input-invalid");
    reader.close();

    assert!(!lock.exists());
    assert_eq!(fs::metadata(&root).unwrap().modified().unwrap(), root_mtime);
    assert_eq!(
        fs::metadata(&narrative).unwrap().modified().unwrap(),
        narrative_mtime
    );
    assert_eq!(
        fs::metadata(&pointer).unwrap().modified().unwrap(),
        pointer_mtime
    );
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn read_only_narrative_pointer_remains_visible_with_dangling_or_corrupt_bytes() {
    let root = project_root("read-only-pointer-visibility");
    let mut writer = Harness::start(&root, "op.read-only-pointer-visibility-writer");
    assert!(writer.acquire()["ok"].as_bool().unwrap());
    let (_pointer, pointer_bytes, delivery_hash, delivery_bytes, delivery_path) = fixture();
    let delivery_token = write_delivery_set(&mut writer, &delivery_hash, &delivery_bytes);
    let absent = writer.request(json!({
        "id": "visibility-absent",
        "op": "snapshot",
        "path": "delivery-pointer-v2.json"
    }));
    let published = writer.request(publish_request(
        "visibility-publish",
        "NONE",
        absent["result"]["snapshotToken"].as_str().unwrap(),
        &pointer_bytes,
        &delivery_path,
        &delivery_bytes,
        &delivery_token,
    ));
    assert!(published["ok"].as_bool().unwrap(), "{published:?}");
    writer.close();

    // Remove every unrelated artifact directory.  The pointer remains at
    // the narrative root, but its referenced DeliverySet is now dangling.
    let narrative = root.join("scenes/trackb").join(NARRATIVE);
    for artifact in ["authority", "candidates", "delivery-sets", "approvals"] {
        fs::remove_dir_all(narrative.join(artifact)).unwrap();
    }
    let pointer = pointer_path(&root);
    assert!(pointer.exists());

    let mut reader = Harness::start_read_only(&root, "op.read-only-pointer-visibility-reader");
    let dangling = reader.request(json!({
        "id": "dangling-pointer",
        "op": "snapshot",
        "path": "delivery-pointer-v2.json"
    }));
    assert!(dangling["ok"].as_bool().unwrap(), "{dangling:?}");
    assert_eq!(dangling["result"]["exists"], true);
    assert_eq!(
        dangling["result"]["bytesBase64"],
        URL_SAFE_NO_PAD.encode(&pointer_bytes)
    );

    // Corrupt bytes are still a visible bounded pointer observation.  The
    // read-only custody layer must not turn malformed content into ABSENT or
    // create the unrelated artifact directories while reporting it.
    fs::write(&pointer, b"{corrupt-pointer").unwrap();
    let corrupt = reader.request(json!({
        "id": "corrupt-pointer",
        "op": "read",
        "path": "delivery-pointer-v2.json"
    }));
    assert!(corrupt["ok"].as_bool().unwrap(), "{corrupt:?}");
    assert_eq!(corrupt["result"]["exists"], true);
    assert_eq!(
        corrupt["result"]["bytesBase64"],
        URL_SAFE_NO_PAD.encode(b"{corrupt-pointer")
    );
    assert_eq!(reader.acquire()["error"]["code"], "trackb-input-invalid");
    let forbidden_write = reader.request(json!({
        "id": "read-only-pointer-write",
        "op": "writeImmutable",
        "path": format!("candidates/{}/trackb-release-candidate-v3.json", "a".repeat(64)),
        "bytesBase64": "eA"
    }));
    assert_eq!(forbidden_write["error"]["code"], "trackb-input-invalid");
    reader.close();

    assert!(!narrative.join(".release.lock").exists());
    for artifact in ["authority", "candidates", "delivery-sets", "approvals"] {
        assert!(!narrative.join(artifact).exists());
    }
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn read_only_symlink_fails_closed_and_narrative_replacement_reads_original_fd() {
    let root = project_root("read-only-replacement");
    let mut writer = Harness::start(&root, "op.read-only-setup");
    assert!(writer.acquire()["ok"].as_bool().unwrap());
    let (_pointer, pointer_bytes, delivery_hash, delivery_bytes, delivery_path) = fixture();
    let delivery_token = write_delivery_set(&mut writer, &delivery_hash, &delivery_bytes);
    let absent = writer.request(json!({
        "id": "setup-absent",
        "op": "snapshot",
        "path": "delivery-pointer-v2.json"
    }));
    let published = writer.request(publish_request(
        "setup-publish",
        "NONE",
        absent["result"]["snapshotToken"].as_str().unwrap(),
        &pointer_bytes,
        &delivery_path,
        &delivery_bytes,
        &delivery_token,
    ));
    assert!(published["ok"].as_bool().unwrap(), "{published:?}");
    writer.close();

    let narrative = root.join("scenes/trackb").join(NARRATIVE);
    let delivery_sets = narrative.join("delivery-sets");
    let original_delivery_sets = narrative.join("delivery-sets.original");
    let redirect_target = root.join("read-only-redirect-target");
    fs::create_dir_all(&redirect_target).unwrap();
    let mut reader = Harness::start_read_only(&root, "op.read-only-replacement");
    fs::rename(&delivery_sets, &original_delivery_sets).unwrap();
    std::os::unix::fs::symlink(&redirect_target, &delivery_sets).unwrap();
    let symlinked = reader.request(json!({
        "id": "reader-symlink",
        "op": "snapshot",
        "path": delivery_set_relative(&delivery_hash)
    }));
    assert_eq!(symlinked["error"]["code"], "trackb-storage-integrity");
    assert!(fs::read_dir(&redirect_target).unwrap().next().is_none());
    fs::remove_file(&delivery_sets).unwrap();
    fs::rename(&original_delivery_sets, &delivery_sets).unwrap();

    let original_narrative = root.join("scenes/trackb/narrative.test.original");
    fs::rename(&narrative, &original_narrative).unwrap();
    fs::create_dir_all(&narrative).unwrap();
    fs::write(narrative.join("delivery-pointer-v2.json"), b"replacement").unwrap();
    let current = reader.request(json!({
        "id": "reader-original-pointer",
        "op": "snapshot",
        "path": "delivery-pointer-v2.json"
    }));
    assert!(current["ok"].as_bool().unwrap(), "{current:?}");
    assert_eq!(
        current["result"]["bytesBase64"],
        URL_SAFE_NO_PAD.encode(&pointer_bytes)
    );
    reader.close();
    assert_eq!(
        fs::read(original_narrative.join("delivery-pointer-v2.json")).unwrap(),
        pointer_bytes
    );
    assert_eq!(
        fs::read(narrative.join("delivery-pointer-v2.json")).unwrap(),
        b"replacement"
    );
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn fresh_read_only_session_observes_pointer_after_mutable_publish_rename() {
    let root = project_root("read-only-reconcile");
    let mut writer = Harness::start(&root, "op.read-only-reconcile-writer");
    assert!(writer.acquire()["ok"].as_bool().unwrap());
    let (_pointer, pointer_bytes, delivery_hash, delivery_bytes, delivery_path) = fixture();
    let delivery_token = write_delivery_set(&mut writer, &delivery_hash, &delivery_bytes);
    let absent = writer.request(json!({
        "id": "reconcile-absent",
        "op": "snapshot",
        "path": "delivery-pointer-v2.json"
    }));
    let published = writer.request(publish_request(
        "reconcile-publish",
        "NONE",
        absent["result"]["snapshotToken"].as_str().unwrap(),
        &pointer_bytes,
        &delivery_path,
        &delivery_bytes,
        &delivery_token,
    ));
    assert!(published["ok"].as_bool().unwrap(), "{published:?}");
    writer.close();

    let mut reader = Harness::start_read_only(&root, "op.read-only-reconcile-reader");
    let observed = reader.request(json!({
        "id": "reconcile-read",
        "op": "read",
        "path": "delivery-pointer-v2.json"
    }));
    assert!(observed["ok"].as_bool().unwrap(), "{observed:?}");
    assert_eq!(observed["result"]["exists"], true);
    assert_eq!(
        observed["result"]["bytesBase64"],
        URL_SAFE_NO_PAD.encode(&pointer_bytes)
    );
    reader.close();
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn project_root_intermediate_ancestor_symlink_fails_closed_without_target_side_effects() {
    let workspace = project_root("project-root-ancestor-link");
    let target = project_root("project-root-ancestor-target");
    let target_project = target.join("project");
    fs::create_dir_all(&target_project).unwrap();
    let linked_parent = workspace.join("linked-parent");
    std::os::unix::fs::symlink(&target, &linked_parent).unwrap();
    let requested = linked_parent.join("project");

    let output = Command::new(env!("CARGO_BIN_EXE_smc-trackb-custody"))
        .arg("--project-root")
        .arg(&requested)
        .arg("--purpose")
        .arg("narrative")
        .arg("--protocol-version")
        .arg("1")
        .output()
        .unwrap();
    assert!(
        !output.status.success(),
        "symlinked ancestor must be rejected"
    );
    assert!(
        String::from_utf8_lossy(&output.stderr).contains("project root"),
        "unexpected stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(fs::read_dir(&target_project).unwrap().next().is_none());
    assert!(!target_project.join("scenes").exists());
    fs::remove_dir_all(workspace).unwrap();
    fs::remove_dir_all(target).unwrap();
}

#[test]
fn intermediate_artifact_directory_symlink_fails_closed_without_following_target() {
    let root = project_root("artifact-symlink");
    let mut helper = Harness::start(&root, "op.artifact-symlink");
    assert!(helper.acquire()["ok"].as_bool().unwrap());

    let narrative = root.join("scenes/trackb").join(NARRATIVE);
    let delivery_sets = narrative.join("delivery-sets");
    let original = narrative.join("delivery-sets.original");
    let redirect_target = root.join("redirect-target");
    fs::create_dir_all(&redirect_target).unwrap();
    fs::rename(&delivery_sets, &original).unwrap();
    std::os::unix::fs::symlink(&redirect_target, &delivery_sets).unwrap();

    let response = helper.request(json!({
        "id": "symlink-write",
        "op": "writeImmutable",
        "path": format!(
            "delivery-sets/{}/trackb-delivery-set-v2.json",
            "a".repeat(64)
        ),
        "bytesBase64": "eA"
    }));
    assert_eq!(response["error"]["code"], "trackb-storage-integrity");
    assert!(fs::read_dir(&redirect_target).unwrap().next().is_none());
    assert!(original.exists());
    helper.close();
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn narrative_path_replacement_keeps_storage_on_retained_directory_fd() {
    let root = project_root("narrative-replacement");
    let mut helper = Harness::start(&root, "op.narrative-replacement");
    assert!(helper.acquire()["ok"].as_bool().unwrap());

    let narrative = root.join("scenes/trackb").join(NARRATIVE);
    let original = root.join("scenes/trackb/narrative.test.original");
    fs::rename(&narrative, &original).unwrap();
    fs::create_dir_all(narrative.join("candidates")).unwrap();
    let relative = format!(
        "candidates/{}/trackb-release-candidate-v4.json",
        "a".repeat(64)
    );
    let response = helper.request(json!({
        "id": "narrative-replacement-write",
        "op": "writeImmutable",
        "path": relative,
        "bytesBase64": "eA"
    }));
    assert!(response["ok"].as_bool().unwrap(), "{response:?}");
    assert!(original
        .join("candidates")
        .join("a".repeat(64))
        .join("trackb-release-candidate-v4.json")
        .is_file());
    assert!(!narrative
        .join("candidates")
        .join("a".repeat(64))
        .join("trackb-release-candidate-v4.json")
        .exists());
    helper.close();
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn ancestor_path_replacement_keeps_storage_on_retained_directory_fd() {
    let root = project_root("ancestor-replacement");
    let mut helper = Harness::start(&root, "op.ancestor-replacement");
    assert!(helper.acquire()["ok"].as_bool().unwrap());

    let scenes = root.join("scenes");
    let original = root.join("scenes.original");
    fs::rename(&scenes, &original).unwrap();
    let replacement_narrative = root.join("scenes/trackb").join(NARRATIVE);
    fs::create_dir_all(replacement_narrative.join("candidates")).unwrap();
    let relative = format!(
        "candidates/{}/trackb-release-candidate-v4.json",
        "b".repeat(64)
    );
    let response = helper.request(json!({
        "id": "ancestor-replacement-write",
        "op": "writeImmutable",
        "path": relative,
        "bytesBase64": "eA"
    }));
    assert!(response["ok"].as_bool().unwrap(), "{response:?}");
    assert!(original
        .join("trackb")
        .join(NARRATIVE)
        .join("candidates")
        .join("b".repeat(64))
        .join("trackb-release-candidate-v4.json")
        .is_file());
    assert!(!replacement_narrative
        .join("candidates")
        .join("b".repeat(64))
        .join("trackb-release-candidate-v4.json")
        .exists());
    helper.close();
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn pointer_cas_validates_strict_v2_bytes_and_delivery_binding() {
    let root = project_root("pointer");
    let mut helper = Harness::start(&root, "op.pointer");
    assert!(helper.acquire()["ok"].as_bool().unwrap());
    let (_pointer, pointer_bytes, delivery_hash, delivery_bytes, delivery_path) = fixture();
    let delivery_token = write_delivery_set(&mut helper, &delivery_hash, &delivery_bytes);
    let initial = helper.request(json!({
        "id": "initial",
        "op": "snapshot",
        "path": "delivery-pointer-v2.json"
    }));
    let published = helper.request(publish_request(
        "publish",
        "NONE",
        initial["result"]["snapshotToken"].as_str().unwrap(),
        &pointer_bytes,
        &delivery_path,
        &delivery_bytes,
        &delivery_token,
    ));
    assert!(published["ok"].as_bool().unwrap(), "{published:?}");
    assert_eq!(fs::read(pointer_path(&root)).unwrap(), pointer_bytes);
    let current = helper.request(json!({
        "id": "current",
        "op": "snapshot",
        "path": "delivery-pointer-v2.json"
    }));
    let stale = helper.request(publish_request(
        "stale",
        "NONE",
        initial["result"]["snapshotToken"].as_str().unwrap(),
        &pointer_bytes,
        &delivery_path,
        &delivery_bytes,
        &delivery_token,
    ));
    assert_eq!(stale["error"]["code"], "trackb-cas-conflict");
    assert_ne!(current["result"]["snapshotToken"], "NONE");
    helper.close();
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn pointer_rejects_invalid_hash_dangling_and_mismatched_delivery_sets() {
    let root = project_root("pointer-negative");
    let mut helper = Harness::start(&root, "op.pointer-negative");
    assert!(helper.acquire()["ok"].as_bool().unwrap());
    let (pointer, pointer_bytes, delivery_hash, delivery_bytes, delivery_path) = fixture();
    let delivery_token = write_delivery_set(&mut helper, &delivery_hash, &delivery_bytes);

    let malformed = helper.request(publish_request(
        "malformed",
        "NONE",
        "NONE",
        b"{}",
        &delivery_path,
        &delivery_bytes,
        &delivery_token,
    ));
    assert_eq!(malformed["error"]["code"], "trackb-storage-integrity");

    let mut bad_hash = pointer.clone();
    bad_hash["pointerHash"] = Value::String(HASH_F.to_owned());
    let bad_hash_bytes = serde_json::to_vec(&bad_hash).unwrap();
    let bad_hash_result = helper.request(publish_request(
        "bad-hash",
        "NONE",
        "NONE",
        &bad_hash_bytes,
        &delivery_path,
        &delivery_bytes,
        &delivery_token,
    ));
    assert_eq!(bad_hash_result["error"]["code"], "trackb-storage-integrity");

    let dangling = helper.request(publish_request(
        "dangling",
        "NONE",
        "NONE",
        &pointer_bytes,
        &delivery_path,
        &delivery_bytes,
        "opaque-token",
    ));
    assert_eq!(dangling["error"]["code"], "trackb-storage-integrity");

    let mut mismatched = pointer.clone();
    mismatched["candidateHash"] = Value::String(HASH_D.to_owned());
    let mismatched_hash = self_hash(&mismatched, "pointerHash");
    mismatched["pointerHash"] = Value::String(mismatched_hash);
    let mismatched_bytes = serde_json::to_vec(&mismatched).unwrap();
    let mismatch_result = helper.request(publish_request(
        "mismatch",
        "NONE",
        "NONE",
        &mismatched_bytes,
        &delivery_path,
        &delivery_bytes,
        &delivery_token,
    ));
    assert_eq!(mismatch_result["error"]["code"], "trackb-storage-integrity");

    let wrong_base = helper.request(publish_request(
        "wrong-base",
        "opaque-token",
        "NONE",
        &pointer_bytes,
        &delivery_path,
        &delivery_bytes,
        &delivery_token,
    ));
    assert_eq!(wrong_base["error"]["code"], "trackb-input-invalid");
    helper.close();
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn current_invalid_pointer_is_not_used_as_a_cas_token() {
    let root = project_root("pointer-current-invalid");
    let mut helper = Harness::start(&root, "op.pointer-current-invalid");
    assert!(helper.acquire()["ok"].as_bool().unwrap());
    let (_pointer, pointer_bytes, delivery_hash, delivery_bytes, delivery_path) = fixture();
    let delivery_token = write_delivery_set(&mut helper, &delivery_hash, &delivery_bytes);
    let published = helper.request(publish_request(
        "publish",
        "NONE",
        "NONE",
        &pointer_bytes,
        &delivery_path,
        &delivery_bytes,
        &delivery_token,
    ));
    assert!(published["ok"].as_bool().unwrap());
    fs::write(pointer_path(&root), b"{}").unwrap();
    let response = helper.request(publish_request(
        "current-invalid",
        "NONE",
        "opaque-token",
        &pointer_bytes,
        &delivery_path,
        &delivery_bytes,
        &delivery_token,
    ));
    assert_eq!(response["error"]["code"], "trackb-storage-integrity");
    helper.close();
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn canonical_lock_bytes_are_required_and_live_contention_uses_three_250ms_probes() {
    let root = project_root("lock-canonical");
    let lock = root.join("scenes/trackb/narrative.test/.release.lock");
    fs::create_dir_all(lock.parent().unwrap()).unwrap();
    let mut record = Map::new();
    record.insert("schemaVersion".into(), json!(1));
    record.insert("narrativeId".into(), json!(NARRATIVE));
    record.insert("operationId".into(), json!("op.pretty"));
    record.insert("ownerPid".into(), json!(10_000_u32));
    record.insert("processStartIdentity".into(), json!("dead-owner"));
    record.insert("createdAt".into(), json!("2026-08-28T00:00:00Z"));
    let mut with_hash = record.clone();
    let digest = Sha256::digest(canonical_json(&Value::Object(record)).as_bytes());
    with_hash.insert("lockHash".into(), json!(format!("sha256:{digest:x}")));
    fs::write(&lock, serde_json::to_vec_pretty(&with_hash).unwrap()).unwrap();
    let mut malformed = Harness::start(&root, "op.malformed-canonical");
    let result = malformed.acquire();
    assert_eq!(result["error"]["code"], "trackb-lock-recovery-required");
    malformed.close();
    assert!(lock.exists());
    fs::remove_file(&lock).unwrap();

    let mut owner = Harness::start(&root, "op.owner");
    assert!(owner.acquire()["ok"].as_bool().unwrap());
    let started = Instant::now();
    let mut contender = Harness::start(&root, "op.contender");
    let busy = contender.acquire();
    let elapsed = started.elapsed();
    assert_eq!(busy["error"]["code"], "trackb-lock-busy");
    assert!(
        elapsed >= Duration::from_millis(450),
        "probe elapsed: {elapsed:?}"
    );
    assert!(
        elapsed < Duration::from_millis(950),
        "probe elapsed: {elapsed:?}"
    );
    contender.close();
    owner.close();
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn replacement_lock_inode_is_never_deleted_on_release() {
    let root = project_root("release-replacement");
    let mut owner = Harness::start(&root, "op.owner");
    assert!(owner.acquire()["ok"].as_bool().unwrap());
    let path = root.join("scenes/trackb/narrative.test/.release.lock");
    let moved = path.with_extension("old");
    fs::rename(&path, &moved).unwrap();
    fs::write(&path, b"replacement-lock-entry").unwrap();
    let response = owner.request(json!({ "id": "close", "op": "close" }));
    assert_eq!(response["error"]["code"], "trackb-lock-recovery-required");
    assert_eq!(fs::read(&path).unwrap(), b"replacement-lock-entry");
    assert!(moved.exists());
    drop(owner);
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn crashed_owner_lock_is_reclaimed_by_a_new_session() {
    let root = project_root("crash");
    let mut owner = Harness::start(&root, "op.crashed");
    assert!(owner.acquire()["ok"].as_bool().unwrap());
    let _ = owner.child.kill();
    let _ = owner.child.wait();
    let mut replacement = Harness::start(&root, "op.replacement");
    assert!(replacement.acquire()["ok"].as_bool().unwrap());
    replacement.close();
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn live_pid_with_different_process_start_identity_is_reclaimed_as_pid_reuse() {
    let root = project_root("pid-reuse");
    let path = root.join("scenes/trackb/narrative.test/.release.lock");
    fs::create_dir_all(path.parent().unwrap()).unwrap();
    let mut record = Map::new();
    record.insert("schemaVersion".into(), json!(1));
    record.insert("narrativeId".into(), json!(NARRATIVE));
    record.insert("operationId".into(), json!("op.reused"));
    record.insert("ownerPid".into(), json!(std::process::id()));
    record.insert(
        "processStartIdentity".into(),
        json!("definitely-not-this-process-start"),
    );
    record.insert("createdAt".into(), json!("2026-08-28T00:00:00Z"));
    let digest = Sha256::digest(canonical_json(&Value::Object(record.clone())).as_bytes());
    record.insert("lockHash".into(), json!(format!("sha256:{digest:x}")));
    fs::write(&path, serde_json::to_vec(&record).unwrap()).unwrap();
    let mut replacement = Harness::start(&root, "op.replacement");
    assert!(replacement.acquire()["ok"].as_bool().unwrap());
    replacement.close();
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn malformed_and_symlink_locks_require_manual_recovery() {
    let root = project_root("lock-negative");
    let path = root.join("scenes/trackb/narrative.test/.release.lock");
    fs::create_dir_all(path.parent().unwrap()).unwrap();
    fs::write(&path, b"not-json").unwrap();
    let mut malformed = Harness::start(&root, "op.malformed");
    assert_eq!(
        malformed.acquire()["error"]["code"],
        "trackb-lock-recovery-required"
    );
    malformed.close();
    fs::remove_file(&path).unwrap();
    let target = root.join("outside-lock");
    fs::write(&target, b"keep").unwrap();
    std::os::unix::fs::symlink(&target, &path).unwrap();
    let mut symlink = Harness::start(&root, "op.symlink");
    assert_eq!(
        symlink.acquire()["error"]["code"],
        "trackb-lock-recovery-required"
    );
    symlink.close();
    assert_eq!(fs::read(&target).unwrap(), b"keep");
    fs::remove_dir_all(root).unwrap();
}
