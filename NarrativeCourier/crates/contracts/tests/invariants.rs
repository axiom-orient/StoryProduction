use narrative_courier_contracts::{
    validate_delivery_request, validate_director_plan, validate_narrative_package,
    validate_narrative_scene_map, validate_nvc_release_bundle,
    validate_reviewed_synopsis_handoff_identity, ArtifactRef, AssetRequest, AssetSource,
    CanonicalNarrative, Coverage, DeliveryIntent, DeliveryPlan, DeliveryRequest, DirectorPlan,
    GrammarAxis, NarrativeAuthority, NarrativePackage, NarrativeSceneMap, NvcReleaseBundle,
    OutputFormat, OutputRequest, PresentationStructure, ProjectionOperation, ProjectionUnit,
    ReviewedSynopsisApproval, ReviewedSynopsisHandoff, SourceLocator, StsDomainRef,
};
use narrative_courier_runtime::token::hash_jcs;
use serde_json::Value;

fn rebind_scene_map_hashes(value: &mut Value) {
    let entries = value["semantic_manifest"]["entries"]
        .as_array_mut()
        .unwrap();
    for entry in entries {
        let mut body = entry.clone();
        body.as_object_mut().unwrap().remove("entry_hash");
        entry["entry_hash"] = format!("sha256:{}", hash_jcs(&body).unwrap()).into();
    }

    let mut manifest = value["semantic_manifest"].clone();
    manifest
        .as_object_mut()
        .unwrap()
        .remove("semantic_manifest_hash");
    value["semantic_manifest"]["semantic_manifest_hash"] =
        format!("sha256:{}", hash_jcs(&manifest).unwrap()).into();

    let mut map = value.clone();
    map["map_id"] = "".into();
    value["map_id"] = format!("nsm2_{}", hash_jcs(&map).unwrap()).into();
}

fn scene_map_fixture() -> NarrativeSceneMap {
    let mut value = serde_json::json!({
        "schema_version": 2,
        "map_id": "",
        "narrative_id": "story",
        "screenplay_sha256": "a".repeat(64),
        "scene_ir_sha256": "b".repeat(64),
        "scenes": [{
            "scene_id": "SCN-001",
            "order": 1,
            "heading": "SCN-001. INT. 방 - 밤",
            "setting": "INT",
            "location": "방",
            "time_of_day": "night",
            "time_of_day_label": "밤",
            "silent": false,
            "characters": ["지우"],
            "action": ["지우가 문을 닫는다."],
            "dialogue": [
                {"character": "지우", "line": "같은 말"},
                {"character": "지우", "line": "같은 말"}
            ],
            "asset_requirements": []
        }],
        "semantic_manifest": {
            "schema_version": 1,
            "semantic_manifest_hash": "",
            "entries": [
                {
                    "semantic_id": "semantic.action.SCN-001.0000",
                    "entry_hash": "",
                    "scene_id": "SCN-001",
                    "kind": "action",
                    "source_text": "지우가 문을 닫는다.",
                    "source_locator": {
                        "scene_id": "SCN-001",
                        "field": "action",
                        "occurrence_index": 0
                    }
                },
                {
                    "semantic_id": "semantic.dialogue.SCN-001.0000",
                    "entry_hash": "",
                    "scene_id": "SCN-001",
                    "kind": "dialogue",
                    "source_text": "같은 말",
                    "source_locator": {
                        "scene_id": "SCN-001",
                        "field": "dialogue",
                        "occurrence_index": 0,
                        "character": "지우",
                        "dialogue_index": 0
                    }
                },
                {
                    "semantic_id": "semantic.dialogue.SCN-001.0001",
                    "entry_hash": "",
                    "scene_id": "SCN-001",
                    "kind": "dialogue",
                    "source_text": "같은 말",
                    "source_locator": {
                        "scene_id": "SCN-001",
                        "field": "dialogue",
                        "occurrence_index": 1,
                        "character": "지우",
                        "dialogue_index": 1
                    }
                }
            ]
        }
    });
    rebind_scene_map_hashes(&mut value);
    serde_json::from_value(value).unwrap()
}

fn release_bundle_fixture() -> NvcReleaseBundle {
    let mut value = serde_json::json!({
        "schema_version": 1,
        "bundle_id": "",
        "project_name": "story",
        "status": "COMPLETED",
        "completed_state": {
            "kind": "completed_state",
            "path": "state.json",
            "sha256": "a".repeat(64),
            "mime": "application/json"
        },
        "narrative_package": {
            "kind": "narrative_package_v2",
            "path": "narrative-package.json",
            "sha256": "b".repeat(64),
            "mime": "application/json"
        },
        "scene_map": {
            "kind": "narrative_scene_map_v2",
            "path": "output/scene-map.json",
            "sha256": "c".repeat(64),
            "mime": "application/json"
        },
        "package_id": format!("nvc2_{}", "d".repeat(64)),
        "package_sha256": "b".repeat(64),
        "source_snapshot_sha256": "f".repeat(64)
    });
    let identity = value.as_object_mut().unwrap();
    identity.remove("bundle_id");
    let bundle_id = format!("nvcb_{}", hash_jcs(&value).unwrap());
    value["bundle_id"] = bundle_id.into();
    serde_json::from_value(value).unwrap()
}

fn request() -> DeliveryRequest {
    DeliveryRequest {
        schema_version: 1,
        request_id: "REQ-1".into(),
        narrative_package_sha256: "1".repeat(64),
        intent: DeliveryIntent::Presentation,
        coverage: Coverage::Full,
        audience: None,
        locale: Some("ko-KR".into()),
        must_include_refs: vec![],
        constraints: vec![],
        asset_inputs: vec![],
        outputs: vec![OutputRequest {
            output_id: "web".into(),
            format: OutputFormat::Html,
            backend_id: None,
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
fn rejects_duplicate_output_ids() {
    let mut r = request();
    r.outputs.push(r.outputs[0].clone());
    assert!(validate_delivery_request(&r).is_err());
}

#[test]
fn rejects_dangling_asset_ref() {
    let r = request();
    let p = DirectorPlan {
        schema_version: 1,
        plan_id: "P".into(),
        narrative_package_sha256: "1".repeat(64),
        delivery_request_sha256: hash_jcs(&r).unwrap(),
        director_bundle_sha256: "3".repeat(64),
        visual_grammar: [(
            "composition".into(),
            GrammarAxis {
                must: vec!["preserve".into()],
                avoid: vec![],
            },
        )]
        .into(),
        asset_requests: vec![],
        deliveries: vec![DeliveryPlan {
            output_request_id: "web".into(),
            presentation_structure: PresentationStructure::Scroll,
            units: vec![ProjectionUnit {
                unit_id: "U1".into(),
                order: 0,
                narrative_refs: vec!["SCN-001".into()],
                operation: ProjectionOperation::Preserve,
                direction: "render".into(),
                asset_refs: vec!["IMG-1".into()],
                content: Some("x".into()),
                duration_frames: None,
            }],
        }],
    };
    assert!(validate_director_plan(&p, &r).is_err());
}

fn reviewed_handoff() -> ReviewedSynopsisHandoff {
    let synopsis_sha256 = "a".repeat(64);
    let mut value = serde_json::json!({
        "schema_id": "geultu.dev/reviewed-synopsis-handoff/v1",
        "schema_version": 1,
        "handoff_id": "",
        "project_id": "geultu-project",
        "source_revision": 3,
        "synopsis_path": "synopsis.md",
        "synopsis_sha256": synopsis_sha256,
        "title": "검토 이야기",
        "language": "ko",
        "approval": {
            "actor_id": "human-author",
            "decision": "APPROVE",
            "synopsis_sha256": synopsis_sha256,
            "source_revision": 3,
            "approved_at_ms": 1
        }
    });
    let body = value.as_object_mut().unwrap();
    body.remove("handoff_id");
    let handoff_id = format!("gsh_{}", hash_jcs(&value).unwrap());
    value["handoff_id"] = handoff_id.into();
    serde_json::from_value(value).unwrap()
}

#[test]
fn reviewed_synopsis_dto_round_trips_the_producer_boundary() {
    let handoff = reviewed_handoff();
    validate_reviewed_synopsis_handoff_identity(&handoff).unwrap();
    let encoded = serde_json::to_value(&handoff).unwrap();
    assert_eq!(
        encoded["schema_id"],
        "geultu.dev/reviewed-synopsis-handoff/v1"
    );
    assert_eq!(encoded["schema_version"], 1);
    assert_eq!(encoded["approval"]["decision"], "APPROVE");
    assert_eq!(
        serde_json::from_value::<ReviewedSynopsisHandoff>(encoded).unwrap(),
        handoff
    );
}

#[test]
fn reviewed_synopsis_dto_rejects_unknown_fields() {
    let mut encoded = serde_json::to_value(reviewed_handoff()).unwrap();
    encoded["unexpected"] = true.into();
    assert!(serde_json::from_value::<ReviewedSynopsisHandoff>(encoded).is_err());
}

#[test]
fn optional_contract_fields_reject_explicit_null() {
    assert!(serde_json::from_value::<ArtifactRef>(serde_json::json!({
        "kind": "artifact",
        "path": "output/artifact.json",
        "sha256": "a".repeat(64),
        "mime": null
    }))
    .is_err());

    let mut source = serde_json::json!({"path":"asset.png","sha256":"b".repeat(64)});
    source["mime"] = Value::Null;
    assert!(serde_json::from_value::<AssetSource>(source).is_err());

    let asset = serde_json::json!({
        "asset_id":"IMG-1", "kind":"IMAGE", "role":"reference",
        "narrative_refs":["SCN-001"], "generation_mode":"PROCEDURAL",
        "brief":"preserve", "hard_constraints":[]
    });
    for field in ["consistency_group", "source"] {
        let mut value = asset.clone();
        value[field] = Value::Null;
        assert!(
            serde_json::from_value::<AssetRequest>(value).is_err(),
            "{field}"
        );
    }

    let unit = serde_json::json!({
        "unit_id":"U1", "order":0, "narrative_refs":["SCN-001"],
        "operation":"PRESERVE", "direction":"retain", "asset_refs":[]
    });
    for field in ["content", "duration_frames"] {
        let mut value = unit.clone();
        value[field] = Value::Null;
        assert!(
            serde_json::from_value::<ProjectionUnit>(value).is_err(),
            "{field}"
        );
    }

    let approval = serde_json::json!({
        "actor_id":"reviewer", "decision":"APPROVE", "synopsis_sha256":"c".repeat(64),
        "source_revision":1, "approved_at_ms":1, "reason":null
    });
    assert!(serde_json::from_value::<ReviewedSynopsisApproval>(approval).is_err());

    let locator = serde_json::json!({
        "scene_id":"SCN-001", "field":"dialogue", "occurrence_index":0,
        "character":"지우", "dialogue_index":0
    });
    for field in ["character", "dialogue_index"] {
        let mut value = locator.clone();
        value[field] = Value::Null;
        assert!(
            serde_json::from_value::<SourceLocator>(value).is_err(),
            "{field}"
        );
    }

    let mut request_value = serde_json::to_value(request()).unwrap();
    request_value["outputs"][0]["width"] = Value::Null;
    assert!(serde_json::from_value::<DeliveryRequest>(request_value).is_err());
}

#[test]
fn scene_map_allows_multiple_lines_from_one_character() {
    let map = scene_map_fixture();
    assert_eq!(map.scenes[0].dialogue.len(), 2);
    validate_narrative_scene_map(&map).unwrap();
}

#[test]
fn static_v2_fixture_is_accepted_by_rust_validator() {
    let map: NarrativeSceneMap = serde_json::from_str(include_str!(
        "../../../contracts/examples/narrative-scene-map-v2.example.json"
    ))
    .unwrap();
    validate_narrative_scene_map(&map).unwrap();
}

#[test]
fn scene_map_v2_rejects_duplicate_ids_locators_hash_drift_and_missing_occurrences() {
    let mut duplicate_id = serde_json::to_value(scene_map_fixture()).unwrap();
    duplicate_id["semantic_manifest"]["entries"][1]["semantic_id"] =
        duplicate_id["semantic_manifest"]["entries"][0]["semantic_id"].clone();
    let duplicate_id: NarrativeSceneMap = serde_json::from_value(duplicate_id).unwrap();
    assert!(validate_narrative_scene_map(&duplicate_id)
        .unwrap_err()
        .to_string()
        .contains("duplicate semantic_id"));

    let mut duplicate_locator = serde_json::to_value(scene_map_fixture()).unwrap();
    duplicate_locator["semantic_manifest"]["entries"][2]["source_locator"] =
        duplicate_locator["semantic_manifest"]["entries"][1]["source_locator"].clone();
    let duplicate_locator: NarrativeSceneMap = serde_json::from_value(duplicate_locator).unwrap();
    assert!(validate_narrative_scene_map(&duplicate_locator)
        .unwrap_err()
        .to_string()
        .contains("duplicate source_locator"));

    let mut hash_drift = serde_json::to_value(scene_map_fixture()).unwrap();
    hash_drift["semantic_manifest"]["entries"][0]["entry_hash"] =
        format!("sha256:{}", "0".repeat(64)).into();
    let hash_drift: NarrativeSceneMap = serde_json::from_value(hash_drift).unwrap();
    assert!(validate_narrative_scene_map(&hash_drift)
        .unwrap_err()
        .to_string()
        .contains("entry_hash mismatch"));

    for (field, index, expected) in [
        ("action", 0, "action occurrence_index"),
        ("dialogue", 1, "dialogue_index"),
    ] {
        let mut missing = serde_json::to_value(scene_map_fixture()).unwrap();
        let entry_index = if field == "action" { 0 } else { 2 };
        missing["semantic_manifest"]["entries"][entry_index]["source_locator"]
            ["occurrence_index"] = 99.into();
        if field == "dialogue" {
            missing["semantic_manifest"]["entries"][entry_index]["source_locator"]
                ["dialogue_index"] = 99.into();
        }
        rebind_scene_map_hashes(&mut missing);
        let missing: NarrativeSceneMap = serde_json::from_value(missing).unwrap();
        assert!(
            validate_narrative_scene_map(&missing)
                .unwrap_err()
                .to_string()
                .contains(expected),
            "missing {field} occurrence {index}"
        );
    }
}

#[test]
fn package_and_bundle_require_v2_scene_map_artifact_kind() {
    let artifact = ArtifactRef {
        kind: "scene_map".into(),
        path: "output/scene-map.json".into(),
        sha256: "a".repeat(64),
        mime: Some("application/json".into()),
    };
    let package = NarrativePackage {
        schema_version: 2,
        package_id: format!("nvc2_{}", "a".repeat(64)),
        narrative_id: "story".into(),
        sts_domain: StsDomainRef {
            id: "sts".into(),
            version: "4.2.0".into(),
            bundle_sha256: "b".repeat(64),
        },
        source_snapshot_sha256: "c".repeat(64),
        canonical_narrative: CanonicalNarrative {
            kind: "SCREENPLAY".into(),
            artifact: artifact.clone(),
        },
        authority: NarrativeAuthority {
            source_ir: artifact.clone(),
            narrative_ir: artifact.clone(),
            execution_constraints: artifact.clone(),
            character_states: artifact.clone(),
            scene_ir: artifact.clone(),
            verification: artifact.clone(),
        },
        scene_map: artifact.clone(),
    };
    assert!(validate_narrative_package(&package)
        .unwrap_err()
        .to_string()
        .contains("narrative_scene_map_v2"));
}

#[test]
fn release_bundle_requires_v2_artifact_kinds_and_hash_bound_identity() {
    let bundle = release_bundle_fixture();
    validate_nvc_release_bundle(&bundle).unwrap();

    let mut package_kind = serde_json::to_value(&bundle).unwrap();
    package_kind["narrative_package"]["kind"] = "scene_map".into();
    let package_kind: NvcReleaseBundle = serde_json::from_value(package_kind).unwrap();
    let error = validate_nvc_release_bundle(&package_kind).unwrap_err();
    assert!(matches!(
        &error,
        narrative_courier_runtime::Error::Contract(_)
    ));
    assert!(error.to_string().contains("narrative_package"));

    let mut scene_map_kind = serde_json::to_value(&bundle).unwrap();
    scene_map_kind["scene_map"]["kind"] = "scene_map".into();
    let scene_map_kind: NvcReleaseBundle = serde_json::from_value(scene_map_kind).unwrap();
    let error = validate_nvc_release_bundle(&scene_map_kind).unwrap_err();
    assert!(matches!(
        &error,
        narrative_courier_runtime::Error::Contract(_)
    ));
    assert!(error.to_string().contains("scene_map.kind"));

    let mut bundle_id = serde_json::to_value(&bundle).unwrap();
    bundle_id["bundle_id"] = format!("nvcb_{}", "0".repeat(64)).into();
    let bundle_id: NvcReleaseBundle = serde_json::from_value(bundle_id).unwrap();
    let error = validate_nvc_release_bundle(&bundle_id).unwrap_err();
    assert!(matches!(
        &error,
        narrative_courier_runtime::Error::Contract(_)
    ));
    assert!(error.to_string().contains("bundle_id"));
}
