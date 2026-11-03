use narrative_courier_contracts::{
    ArtifactRef, AssetIndex, AssetKind, AssetSource, CanonicalNarrative, Coverage,
    DeliveryAssetInput, DeliveryIntent, DeliveryRequest, DeliveryStatus, DirectorPlan,
    GenerationMode, NarrativeAuthority, NarrativePackage, OutputFormat, OutputRequest,
    StsDomainRef, VerificationStatus,
};
use narrative_courier_delivery::DeliveryEngine;
use narrative_courier_runtime::store::{sha256_bytes, sha256_file};
use narrative_courier_runtime::token::hash_jcs;
use std::fs;
use tempfile::TempDir;

fn artifact(project: &std::path::Path, relative: &str, kind: &str, body: &str) -> ArtifactRef {
    let path = project.join(relative);
    fs::create_dir_all(path.parent().unwrap()).unwrap();
    fs::write(&path, body).unwrap();
    ArtifactRef {
        kind: kind.into(),
        path: relative.into(),
        sha256: sha256_file(&path).unwrap(),
        mime: Some("image/png".into()),
    }
}

fn package(project: &std::path::Path) -> NarrativePackage {
    let screenplay = artifact(
        project,
        "output/final_screenplay.md",
        "screenplay",
        "SCN-001. INT. ROOM - DAY\n\nA verified scene.\n",
    );
    let source_ir = artifact(project, "output/source_ir.json", "source_ir", "{}\n");
    let narrative_ir = artifact(project, "output/narrative_ir.json", "narrative_ir", "{}\n");
    let constraints = artifact(
        project,
        "output/execution_constraints.json",
        "execution_constraints",
        "{}\n",
    );
    let states = artifact(
        project,
        "output/character_states.json",
        "character_states",
        "{}\n",
    );
    let scenes = artifact(project, "output/scene_ir.json", "scene_ir", "{}\n");
    let scene_map = artifact(
        project,
        "output/scene-map.json",
        "narrative_scene_map_v2",
        "{}\n",
    );
    let verification = artifact(
        project,
        "output/verification.md",
        "verification",
        "STS_STATUS: READY\n",
    );
    let mut package = NarrativePackage {
        schema_version: 2,
        package_id: String::new(),
        narrative_id: "story-test".into(),
        sts_domain: StsDomainRef {
            id: "sts".into(),
            version: "4.2.0".into(),
            bundle_sha256: sha256_bytes(b"sts-bundle"),
        },
        source_snapshot_sha256: hash_jcs(
            &serde_json::json!({"fixture":"simulated-source-snapshot"}),
        )
        .unwrap(),
        canonical_narrative: CanonicalNarrative {
            kind: "SCREENPLAY".into(),
            artifact: screenplay,
        },
        authority: NarrativeAuthority {
            source_ir,
            narrative_ir,
            execution_constraints: constraints,
            character_states: states,
            scene_ir: scenes,
            verification,
        },
        scene_map,
    };
    package.package_id = package.derived_id().unwrap();
    package
}

fn write_package(project: &std::path::Path, package: &NarrativePackage) {
    let mut bytes = serde_json::to_vec_pretty(package).unwrap();
    bytes.push(b'\n');
    fs::write(project.join("narrative-package.json"), &bytes).unwrap();
    let state = serde_json::json!({
        "project_name": package.narrative_id,
        "status": "completed",
        "source_snapshot": {"fixture":"simulated-source-snapshot"},
        "nodes": {"final_release":{"status":"completed","decision":"approve"}}
    });
    let state_bytes = serde_json::to_vec_pretty(&state).unwrap();
    fs::write(project.join("state.json"), &state_bytes).unwrap();
    let package_hash = sha256_bytes(&bytes);
    let mut bundle = narrative_courier_contracts::NvcReleaseBundle {
        schema_version: 1,
        bundle_id: String::new(),
        project_name: package.narrative_id.clone(),
        status: "COMPLETED".into(),
        completed_state: ArtifactRef {
            kind: "completed_state".into(),
            path: "state.json".into(),
            sha256: sha256_bytes(&state_bytes),
            mime: Some("application/json".into()),
        },
        narrative_package: ArtifactRef {
            kind: "narrative_package_v2".into(),
            path: "narrative-package.json".into(),
            sha256: package_hash.clone(),
            mime: Some("application/json".into()),
        },
        scene_map: package.scene_map.clone(),
        package_id: package.package_id.clone(),
        package_sha256: package_hash,
        source_snapshot_sha256: package.source_snapshot_sha256.clone(),
    };
    bundle.bundle_id = bundle.derived_id().unwrap();
    fs::write(
        project.join("release-bundle.json"),
        serde_json::to_vec_pretty(&bundle).unwrap(),
    )
    .unwrap();
}

fn request(
    project: &std::path::Path,
    package: &NarrativePackage,
    request_id: &str,
) -> DeliveryRequest {
    let source_path = project.join("input/reference.png");
    fs::create_dir_all(source_path.parent().unwrap()).unwrap();
    fs::write(&source_path, include_bytes!("fixtures/pixel.png")).unwrap();
    DeliveryRequest {
        schema_version: 1,
        request_id: request_id.into(),
        narrative_package_sha256: hash_jcs(package).unwrap(),
        intent: DeliveryIntent::Presentation,
        coverage: Coverage::Full,
        audience: None,
        locale: None,
        must_include_refs: vec![],
        constraints: vec![],
        asset_inputs: vec![DeliveryAssetInput {
            asset_id: "ASSET-REF".into(),
            kind: AssetKind::Image,
            role: "reference".into(),
            narrative_refs: vec!["SCN-001".into()],
            generation_mode: GenerationMode::User,
            source: AssetSource {
                path: "input/reference.png".into(),
                sha256: sha256_file(&source_path).unwrap(),
                mime: Some("image/png".into()),
            },
        }],
        outputs: vec![OutputRequest {
            output_id: "html".into(),
            format: OutputFormat::Html,
            backend_id: Some("internal-html".into()),
            width: None,
            height: None,
            aspect_ratio: None,
            fps: None,
            duration_frames: None,
            constraints: vec![],
        }],
    }
}

#[test]
fn asset_materialization_review_and_output_identity_form_one_vertical_slice() {
    let tmp = TempDir::new().unwrap();
    let project = tmp.path();
    let package = package(project);
    write_package(project, &package);

    let first =
        DeliveryEngine::run(project, &package, request(project, &package, "REQ-1")).unwrap();
    assert_eq!(first.status, DeliveryStatus::NeedsReview);
    let first_artifact = first.deliveries[0].artifact.path.clone();

    let plan: DirectorPlan = serde_json::from_slice(
        &fs::read(project.join("visual/requests/REQ-1/director-plan.json")).unwrap(),
    )
    .unwrap();
    assert_eq!(plan.asset_requests.len(), 1);
    assert_eq!(plan.deliveries[0].units[0].asset_refs, vec!["ASSET-REF"]);
    let index: AssetIndex = serde_json::from_slice(
        &fs::read(project.join("visual/requests/REQ-1/asset-index.json")).unwrap(),
    )
    .unwrap();
    assert_eq!(index.assets.len(), 1);
    assert!(project.join(&index.assets[0].path).is_file());
    let html = fs::read_to_string(project.join(&first_artifact)).unwrap();
    assert!(html.contains("data-asset=\"ASSET-REF\""));
    assert!(html.contains("data:image/png;base64,"));

    let approved = DeliveryEngine::review(project, &package, "REQ-1", "html", true, None).unwrap();
    assert_eq!(approved.status, DeliveryStatus::Pass);
    assert_eq!(
        approved.deliveries[0].verification.visual.status,
        VerificationStatus::Pass
    );

    let second =
        DeliveryEngine::run(project, &package, request(project, &package, "REQ-2")).unwrap();
    assert_eq!(second.deliveries[0].artifact.path, first_artifact);
    assert_eq!(second.status, DeliveryStatus::NeedsReview);
    let rejected = DeliveryEngine::review(
        project,
        &package,
        "REQ-2",
        "html",
        false,
        Some("layout is not acceptable"),
    )
    .unwrap();
    assert_eq!(rejected.status, DeliveryStatus::Blocked);
    assert_eq!(
        rejected.deliveries[0].verification.visual.status,
        VerificationStatus::Fail
    );
}
