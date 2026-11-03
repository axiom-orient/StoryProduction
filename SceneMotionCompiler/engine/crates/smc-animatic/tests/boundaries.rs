use serde_json::Value;
use std::fs;
use std::process::Command;

#[test]
fn unreadable_or_malformed_existing_claim_remains_unknown() {
    let temporary = tempfile::tempdir().unwrap();
    let root = temporary.path().canonicalize().unwrap();
    let attempt = root.join("animatic/renders/broken");
    fs::create_dir_all(&attempt).unwrap();
    fs::write(
        attempt.join("request.json"),
        b"{\"workerPid\":1,\"workerPid\":2}",
    )
    .unwrap();
    let output = Command::new(env!("CARGO_BIN_EXE_smc-animatic"))
        .args([
            "status",
            "--project",
            root.to_str().unwrap(),
            "--attempt",
            "broken",
        ])
        .output()
        .unwrap();
    assert_eq!(output.status.code(), Some(3));
    let value: Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(value["status"], "outcome_unknown");
    assert_eq!(value["attemptId"], "broken");
}

#[test]
fn missing_attempt_is_not_started_and_does_not_create_a_claim() {
    let temporary = tempfile::tempdir().unwrap();
    let root = temporary.path().canonicalize().unwrap();
    let output = Command::new(env!("CARGO_BIN_EXE_smc-animatic"))
        .args([
            "status",
            "--project",
            root.to_str().unwrap(),
            "--attempt",
            "absent",
        ])
        .output()
        .unwrap();
    let value: Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(value["status"], "not_started");
    assert!(!root.join("animatic").exists());
}

#[test]
fn explicit_owner_root_must_supply_the_pinned_resources() {
    let temporary = tempfile::tempdir().unwrap();
    let root = temporary.path().canonicalize().unwrap();
    let owner = root.join("owner");
    fs::create_dir(&owner).unwrap();
    let output = Command::new(env!("CARGO_BIN_EXE_smc-animatic"))
        .args([
            "status",
            "--project",
            root.to_str().unwrap(),
            "--attempt",
            "absent",
            "--owner-root",
            owner.to_str().unwrap(),
        ])
        .output()
        .unwrap();
    assert_eq!(output.status.code(), Some(2));
    let value: Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(value["status"], "not_started");
    assert!(value["detail"]
        .as_str()
        .unwrap()
        .contains("OWNER_RESOURCE_UNAVAILABLE"));
    assert!(!root.join("animatic").exists());
}

#[test]
fn approved_sac_integral_float_pack_keeps_its_producer_hash() {
    use serde_json::json;
    use sha2::{Digest, Sha256};
    fn seal(mut value: Value) -> Value {
        value.as_object_mut().unwrap().remove("contentHash");
        value["contentHash"] = format!(
            "sha256:{:x}",
            Sha256::digest(serde_json::to_vec(&value).unwrap())
        )
        .into();
        value
    }
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path().canonicalize().unwrap();
    let hash = format!("sha256:{}", "1".repeat(64));
    let source =
        json!({"narrativePackageSha256":hash,"sceneMapSha256":hash,"releaseBundleSha256":hash});
    let plan = seal(
        json!({"schemaVersion":"nvc.animatic-plan/1","source":source,"boardHash":hash,"boardReviewHash":hash,
        "fps":12,"canvas":{"width":64,"height":64,"background":"#000000"},
        "assets":[{"assetId":"actor","width":1,"height":1,"alpha":"opaque","brief":"MECHANICAL_TEST_ONLY"}],
        "panels":[{"sceneId":"scene.test","panelId":"panel.test","durationFrames":1,
            "camera":[{"frame":0,"x":0,"y":0,"scalePerMille":1000,"easing":"linear"}],
            "layers":[{"layerId":"actor","depthPerMille":1000,"poses":[{"frame":0,"assetId":"actor"}],
                "motion":[{"frame":0,"x":0,"y":0,"scalePerMille":1000,"easing":"linear","rotationMilliDeg":0,"opacityPerMille":1000}]}],
            "captions":[]}],"openQuestions":[]}),
    );
    let review = seal(
        json!({"schemaVersion":"nvc.animatic-review/1","source":source,"planHash":plan["contentHash"],
        "decision":"approved","reviewer":"SIMULATED_TEST_ONLY","note":"mechanical fixture","reviewedAt":"2026-10-03T00:00:00Z"}),
    );
    let packet = seal(
        json!({"schemaVersion":"nvc.animatic-packet/1","plan":plan,"review":review,
        "captions":[{"panelId":"panel.test","soundNotes":"no audio","lines":[]}]}),
    );
    let pixels: &[u8] = &[
        137, 80, 78, 71, 13, 10, 26, 10, 0, 0, 0, 13, 73, 72, 68, 82, 0, 0, 0, 1, 0, 0, 0, 1, 8, 6,
        0, 0, 0, 31, 21, 196, 137, 0, 0, 0, 13, 73, 68, 65, 84, 120, 156, 99, 96, 96, 96, 248, 15,
        0, 1, 4, 1, 0, 95, 229, 195, 75, 0, 0, 0, 0, 73, 69, 78, 68, 174, 66, 96, 130,
    ];
    fs::write(root.join("actor.png"), pixels).unwrap();
    let pack = seal(
        json!({"schemaVersion":"sac.animatic-assets/1","packetHash":packet["contentHash"],
        "assets":[{"assetId":"actor","path":"actor.png","sha256":format!("sha256:{:x}",Sha256::digest(pixels)),
            "width":1.0,"height":1.0,"alpha":"opaque","provenance":{"executor":"user-supplied","source":"MECHANICAL_TEST_ONLY"}}]}),
    );
    let review = seal(
        json!({"schemaVersion":"sac.animatic-assets-review/1","packHash":pack["contentHash"],"packetHash":packet["contentHash"],
        "decision":"approved","reviewer":"SIMULATED_TEST_ONLY","note":"mechanical fixture","reviewedAt":"2026-10-03T00:00:00Z"}),
    );
    for (name, value) in [
        ("packet.json", packet),
        ("assets.json", pack),
        ("review.json", review),
    ] {
        fs::write(root.join(name), serde_json::to_vec(&value).unwrap()).unwrap();
    }
    let output = Command::new(env!("CARGO_BIN_EXE_smc-animatic"))
        .args([
            "inspect",
            "--project",
            root.to_str().unwrap(),
            "--packet",
            "packet.json",
            "--assets",
            "assets.json",
            "--asset-review",
            "review.json",
        ])
        .output()
        .unwrap();
    assert_eq!(
        output.status.code(),
        Some(0),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(
        serde_json::from_slice::<Value>(&output.stdout).unwrap()["status"],
        "validated"
    );
}
