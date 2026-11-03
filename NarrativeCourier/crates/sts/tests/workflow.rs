#[path = "../../../test_support/sts_workflow_fixture.rs"]
mod sts_workflow_fixture;

use std::fs;
use std::path::{Path, PathBuf};

use narrative_courier_runtime::token::hash_jcs;
use serde_json::json;
use sts_workflow::{Engine, NewProject, NextPacket};
use tempfile::TempDir;

struct Harness {
    _tmp: TempDir,
    engine: Engine,
    name: String,
}

impl Harness {
    fn new() -> Self {
        let source = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");
        let tmp = TempDir::new().unwrap();
        copy_tree(&source.join("skills"), &tmp.path().join("skills"));
        let engine = Engine::new(tmp.path()).expect("fixture roots resolve");
        let name = "demo".to_string();
        engine
            .create_project(&NewProject {
                project_name: name.clone(),
                title: "테스트".into(),
                synopsis: sts_workflow_fixture::SYNOPSIS.into(),
                source_format: "original".into(),
                target_duration: 100,
                genre: vec!["드라마".into()],
                tone: "절제".into(),
                theme_direction: "오해와 책임".into(),
                references: vec![],
            })
            .unwrap();
        Self {
            _tmp: tmp,
            engine,
            name,
        }
    }

    fn project(&self) -> PathBuf {
        self.engine.project_dir(&self.name).unwrap()
    }

    fn next_id(&self) -> Option<String> {
        match self.engine.next(&self.name).unwrap() {
            NextPacket::AgentTask { node, .. } | NextPacket::AwaitingHuman { node, .. } => {
                Some(node)
            }
            NextPacket::Paused | NextPacket::Completed { .. } => None,
        }
    }

    fn submit_next(&self) -> String {
        let (node, token) = match self.engine.next(&self.name).unwrap() {
            NextPacket::AgentTask {
                node, task_token, ..
            } => (node, task_token),
            other => panic!("expected agent, got {other:?}"),
        };
        self.engine
            .submit(
                &self.name,
                sts_workflow_fixture::artifact_for(&node),
                &token,
            )
            .unwrap();
        node
    }

    fn approve(&self) {
        let token = match self.engine.next(&self.name).unwrap() {
            NextPacket::AwaitingHuman { gate_token, .. } => gate_token,
            other => panic!("expected human, got {other:?}"),
        };
        self.engine
            .decide(&self.name, "approve", None, None, &token)
            .unwrap();
    }

    fn advance_to(&self, target: &str) {
        for _ in 0..80 {
            match self.engine.next(&self.name).unwrap() {
                NextPacket::AgentTask {
                    node, task_token, ..
                } => {
                    if node == target {
                        return;
                    }
                    self.engine
                        .submit(
                            &self.name,
                            sts_workflow_fixture::artifact_for(&node),
                            &task_token,
                        )
                        .unwrap();
                }
                NextPacket::AwaitingHuman {
                    node, gate_token, ..
                } => {
                    if node == target {
                        return;
                    }
                    self.engine
                        .decide(&self.name, "approve", None, None, &gate_token)
                        .unwrap();
                }
                other => panic!("target {target} not reached: {other:?}"),
            }
        }
        panic!("target {target} not reached");
    }
}

fn assert_public_release_load_rejects_bundle_tamper<F>(mutate: F, expected: &str)
where
    F: FnOnce(&mut serde_json::Value),
{
    let h = Harness::new();
    h.advance_to("final_release");
    h.approve();
    let path = h.project().join("release-bundle.json");
    let mut value: serde_json::Value = serde_json::from_slice(&fs::read(&path).unwrap()).unwrap();
    mutate(&mut value);
    fs::write(&path, serde_json::to_vec_pretty(&value).unwrap()).unwrap();
    let error = h.engine.release_bundle(&h.name).unwrap_err();
    assert!(matches!(
        &error,
        narrative_courier_runtime::Error::Contract(_)
    ));
    let error = error.to_string();
    assert!(
        error.contains(expected),
        "expected release tamper error containing {expected:?}, got {error}"
    );
}

fn copy_tree(src: &Path, dst: &Path) {
    fs::create_dir_all(dst).unwrap();
    for entry in fs::read_dir(src).unwrap() {
        let entry = entry.unwrap();
        let target = dst.join(entry.file_name());
        if entry.file_type().unwrap().is_dir() {
            copy_tree(&entry.path(), &target);
        } else {
            fs::copy(entry.path(), target).unwrap();
        }
    }
}

#[test]
fn manifest_loads_and_domain_is_sts() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");
    let manifest = Engine::new(root)
        .expect("fixture roots resolve")
        .load_manifest()
        .unwrap();
    assert_eq!(manifest.domain_id, "sts");
    assert_eq!(manifest.schema_version, 3);
}

#[test]
fn project_dir_rejects_traversal_before_creating_the_projects_root() {
    let tmp = TempDir::new().unwrap();
    let projects = tmp.path().join("projects");
    let engine = Engine::with_roots(tmp.path().join("assets"), projects.clone())
        .expect("fixture roots resolve");
    assert!(engine.project_dir("../escape").is_err());
    assert!(engine.project_dir("/tmp/escape").is_err());
    assert!(!projects.exists());
}

#[test]
fn source_is_frozen_and_requires_human_approval() {
    let h = Harness::new();
    assert_eq!(h.submit_next(), "step_00");
    assert_eq!(h.next_id().as_deref(), Some("source_approval"));
    fs::write(h.project().join("input/synopsis.md"), "changed").unwrap();
    assert!(h.engine.next(&h.name).is_err());
}

#[test]
fn stale_human_gate_token_is_rejected() {
    let h = Harness::new();
    h.submit_next();
    let token = match h.engine.next(&h.name).unwrap() {
        NextPacket::AwaitingHuman { gate_token, .. } => gate_token,
        _ => panic!(),
    };
    h.engine
        .decide(&h.name, "approve", None, None, &token)
        .unwrap();
    assert!(h
        .engine
        .decide(&h.name, "approve", None, None, &token)
        .is_err());
}

#[test]
fn artifact_review_revision_returns_to_declared_agent() {
    let h = Harness::new();
    h.submit_next();
    let token = match h.engine.next(&h.name).unwrap() {
        NextPacket::AwaitingHuman { gate_token, .. } => gate_token,
        _ => panic!(),
    };
    h.engine
        .decide(&h.name, "revise", None, Some("REL-001을 더 명확히"), &token)
        .unwrap();
    assert_eq!(h.next_id().as_deref(), Some("step_00"));
}

#[test]
fn part_two_receives_only_part_one_transfer_capsule() {
    let h = Harness::new();
    h.advance_to("step_07_part2");
    let (_, task) = h.engine.task_text(&h.name).unwrap();
    assert!(task.contains("[PROGRAM CONTEXT REDUCTION"));
    assert!(task.contains("## [TRANSFER CAPSULE]"));
}

#[test]
fn source_blind_reverse_task_does_not_receive_source_or_blueprint() {
    let h = Harness::new();
    h.advance_to("step_08_reverse");
    let (_, task) = h.engine.task_text(&h.name).unwrap();
    assert!(task.contains("output/final_screenplay.md"));
    assert!(!task.contains("output/step_00_source_ir.md"));
    assert!(!task.contains("output/step_06_scene_blueprint.md"));
}

#[test]
fn step_08_ready_requires_recovered_alignment_and_scene_evidence() {
    let h = Harness::new();
    h.advance_to("step_08_verify");
    let token = match h.engine.next(&h.name).unwrap() {
        NextPacket::AgentTask { task_token, .. } => task_token,
        _ => panic!(),
    };
    let error = h
        .engine
        .submit(
            &h.name,
            "# Verification\n- SRC-001: PRESERVED\n- REL-001: PRESERVED\n\nSTS_STATUS: READY\n",
            &token,
        )
        .unwrap_err()
        .to_string();
    assert!(error.contains("no alignment table"));
}

#[test]
fn final_release_requires_ready_marker() {
    let h = Harness::new();
    h.advance_to("step_08_verify");
    let token = match h.engine.next(&h.name).unwrap() {
        NextPacket::AgentTask { task_token, .. } => task_token,
        _ => panic!(),
    };
    h.engine
        .submit(
            &h.name,
            concat!(
                "# Verification\n",
                "| Source ID | State | Recovered Screenplay IR IDs | Screenplay scene evidence | Rationale |\n",
                "|---|---|---|---|---|\n",
                "| SRC-001 | DRIFTED | OBS-001 | SCN-001: 기록이 주장을 반박한다 | 보호된 근거와 충돌한다. |\n",
                "| SRC-002 | PRESERVED | OBS-002 | SCN-001: 지우가 민준을 의심한다 | 의심의 주체와 대상을 보존한다. |\n",
                "| SRC-003 | PRESERVED | OBS-003 | SCN-001: 기록으로 진실을 확인한다 | 확인 행위와 결과를 보존한다. |\n",
                "| SRC-004 | PRESERVED | OBS-004 | SCN-001: 지우가 혼자 떠난다 | 결말의 주체를 보존한다. |\n",
                "| REL-001 | UNCERTAIN | OREL-001 | SCN-001: 전이 순서가 불명확하다 | 판정할 근거가 부족하다. |\n",
                "\nSTS_STATUS: BLOCKED\n"
            ),
            &token,
        )
        .unwrap();
    let gate = match h.engine.next(&h.name).unwrap() {
        NextPacket::AwaitingHuman { gate_token, .. } => gate_token,
        _ => panic!(),
    };
    assert!(h
        .engine
        .decide(&h.name, "approve", None, None, &gate)
        .is_err());
}

#[test]
fn final_release_publishes_hash_bound_narrative_package() {
    let h = Harness::new();
    h.advance_to("final_release");
    h.approve();
    let package = h.engine.narrative_package(&h.name).unwrap();
    assert_eq!(package.narrative_id, h.name);
    assert_eq!(package.schema_version, 2);
    assert_eq!(
        package.canonical_narrative.artifact.path,
        "output/final_screenplay.md"
    );
    assert_eq!(package.scene_map.path, "output/scene-map.json");
    assert_eq!(package.scene_map.kind, "narrative_scene_map_v2");
    let scene_map: serde_json::Value =
        serde_json::from_slice(&fs::read(h.project().join("output/scene-map.json")).unwrap())
            .unwrap();
    assert_eq!(scene_map["schema_version"], 2);
    assert!(scene_map["map_id"].as_str().unwrap().starts_with("nsm2_"));
    assert!(scene_map["semantic_manifest"].is_object());
    assert_eq!(scene_map["scenes"].as_array().unwrap().len(), 1);
    assert_eq!(scene_map["scenes"][0]["scene_id"], "SCN-001");
    assert_eq!(scene_map["scenes"][0]["time_of_day_label"], "밤");
    let bundle = h.engine.release_bundle(&h.name).unwrap();
    assert_eq!(bundle.status, "COMPLETED");
    assert_eq!(bundle.package_id, package.package_id);
    assert!(bundle.bundle_id.starts_with("nvcb_"));
    assert_eq!(bundle.narrative_package.kind, "narrative_package_v2");
    assert_eq!(bundle.narrative_package.path, "narrative-package.json");
    assert_eq!(bundle.scene_map.kind, "narrative_scene_map_v2");
    assert_eq!(bundle.scene_map.path, "output/scene-map.json");
    assert_eq!(h.engine.release_bundle(&h.name).unwrap(), bundle);
    assert!(h.project().join("release-bundle.json").is_file());
    assert!(h.project().join("narrative-package.json").is_file());
    assert!(h.engine.doctor(&h.name).unwrap().ok);
}

#[test]
fn public_release_load_rejects_tampered_bundle_artifact_kinds_and_identity() {
    assert_public_release_load_rejects_bundle_tamper(
        |value| {
            value["narrative_package"]["kind"] = serde_json::Value::String("scene_map".into());
        },
        "narrative_package",
    );
    assert_public_release_load_rejects_bundle_tamper(
        |value| {
            value["scene_map"]["kind"] = serde_json::Value::String("scene_map".into());
        },
        "scene_map.kind",
    );
    assert_public_release_load_rejects_bundle_tamper(
        |value| {
            value["bundle_id"] = serde_json::Value::String(format!("nvcb_{}", "0".repeat(64)));
        },
        "bundle_id",
    );
}

#[test]
fn public_release_load_rejects_tampered_narrative_package_scene_map_kind() {
    let h = Harness::new();
    h.advance_to("final_release");
    h.approve();
    let path = h.project().join("narrative-package.json");
    let mut value: serde_json::Value = serde_json::from_slice(&fs::read(&path).unwrap()).unwrap();
    value["scene_map"]["kind"] = serde_json::Value::String("scene_map".into());
    fs::write(&path, serde_json::to_vec_pretty(&value).unwrap()).unwrap();
    let error = h.engine.release_bundle(&h.name).unwrap_err();
    assert!(matches!(
        &error,
        narrative_courier_runtime::Error::Contract(_)
    ));
    let error = error.to_string();
    assert!(error.contains("NarrativePackage scene_map.kind"), "{error}");
}

#[test]
fn reviewed_synopsis_handoff_is_verified_before_sts_project_creation() {
    let h = Harness::new();
    let handoff_root = h._tmp.path().join("reviewed-handoff");
    fs::create_dir_all(&handoff_root).unwrap();
    let synopsis = "# 검토 이야기  \n\n지우는 진실을 확인하고 떠난다.\n\n";
    fs::write(handoff_root.join("synopsis.md"), synopsis.as_bytes()).unwrap();
    let synopsis_sha256 = narrative_courier_runtime::store::sha256_bytes(synopsis.as_bytes());
    let mut handoff = json!({
        "schema_id": "geultu.dev/reviewed-synopsis-handoff/v1",
        "schema_version": 1,
        "handoff_id": "",
        "project_id": "geultu-book",
        "source_revision": 7,
        "synopsis_path": "synopsis.md",
        "synopsis_sha256": synopsis_sha256,
        "title": "검토 이야기",
        "language": "ko",
        "approval": {
            "actor_id": "human-author",
            "decision": "APPROVE",
            "synopsis_sha256": synopsis_sha256,
            "source_revision": 7,
            "approved_at_ms": 1
        }
    });
    let identity = handoff.as_object_mut().unwrap();
    identity.remove("handoff_id");
    let handoff_id = format!("gsh_{}", hash_jcs(&handoff).unwrap());
    handoff["handoff_id"] = handoff_id.into();
    fs::write(
        handoff_root.join("reviewed-synopsis-handoff.v1.json"),
        serde_json::to_vec_pretty(&handoff).unwrap(),
    )
    .unwrap();

    let view = h
        .engine
        .create_project_from_reviewed_synopsis(
            &sts_workflow::CreateReviewedSynopsisProjectRequest {
                project_name: "handoff-demo".into(),
                title: None,
                handoff_root: handoff_root.display().to_string(),
                handoff_path: "reviewed-synopsis-handoff.v1.json".into(),
                target_duration: 100,
                genre: vec!["드라마".into()],
                tone: "절제".into(),
                theme_direction: "책임".into(),
            },
        )
        .unwrap();
    assert_eq!(view.name, "handoff-demo");
    let config: sts_workflow::ProjectConfig = serde_yaml::from_slice(
        &fs::read(
            h.project()
                .parent()
                .unwrap()
                .join("handoff-demo/config.yaml"),
        )
        .unwrap(),
    )
    .unwrap();
    assert_eq!(config.title, "검토 이야기");
    assert_eq!(
        fs::read(
            h.engine
                .project_dir("handoff-demo")
                .unwrap()
                .join("input/synopsis.md")
        )
        .unwrap(),
        synopsis.as_bytes()
    );
    assert_eq!(
        config.reviewed_synopsis.as_ref().unwrap().handoff_id,
        handoff["handoff_id"].as_str().unwrap()
    );
    assert_eq!(
        config.reviewed_synopsis.as_ref().unwrap().synopsis_sha256,
        synopsis_sha256
    );
}

#[test]
fn korean_scene_map_extracts_dialogue_and_preserves_silent_scenes() {
    let screenplay = concat!(
        "SCN-001. INT. 방 - 밤\n",
        "지우\n",
        "(조용히)\n",
        "    문을 닫아.\n",
        "지우: 이제 알겠어.\n\n",
        "SCN-002. EXT. 옥상 - 새벽\n",
        "바람이 분다.\n",
    );
    let blueprint = concat!(
        "SCN-001. INT. 방 - 밤\n",
        "- Required props: 낡은 기록\n\n",
        "SCN-002. EXT. 옥상 - 새벽\n",
        "- Observable action: 바람이 분다\n",
    );

    let map = sts_workflow::scene_map::build_scene_map(
        "korean-dialogue",
        screenplay.as_bytes(),
        blueprint.as_bytes(),
    )
    .unwrap();

    assert_eq!(map.scenes.len(), 2);
    let speaking = &map.scenes[0];
    assert!(!speaking.silent);
    assert!(speaking.characters.iter().any(|value| value == "지우"));
    assert_eq!(speaking.dialogue.len(), 2);
    assert_eq!(speaking.dialogue[0].character, "지우");
    assert_eq!(speaking.dialogue[0].line, "문을 닫아.");
    assert_eq!(speaking.dialogue[1].character, "지우");
    assert_eq!(speaking.dialogue[1].line, "이제 알겠어.");
    assert_eq!(speaking.asset_requirements.len(), 1);

    let silent = &map.scenes[1];
    assert!(silent.silent);
    assert!(silent.dialogue.is_empty());
    assert!(silent.characters.is_empty());
}

#[test]
fn scene_map_v2_emits_all_declared_semantic_kinds_and_distinguishes_repeated_dialogue() {
    let screenplay = concat!(
        "SCN-001. INT. 기록실 - 밤\n",
        "지우가 낡은 문서를 펼친다.\n",
        "A Unicode 별빛 appears.\n",
        "지우\n",
        "    같은 말이다.\n",
        "지우: 같은 말이다.\n",
    );
    let blueprint = concat!(
        "SCN-001. INT. 기록실 - 밤\n",
        "- Emotion: 지우는 망설인다.\n",
        "- Lighting transition: 푸른 밤이 밝아진다.\n",
        "- Camera movement: 카메라가 문서로 이동한다.\n",
    );

    let map = sts_workflow::scene_map::build_scene_map(
        "semantic-v2-runtime",
        screenplay.as_bytes(),
        blueprint.as_bytes(),
    )
    .unwrap();

    assert_eq!(map.schema_version, 2);
    assert!(map.map_id.starts_with("nsm2_"));
    let entries = &map.semantic_manifest.entries;
    assert_eq!(entries.len(), 7);
    assert_eq!(
        entries
            .iter()
            .map(|entry| entry.kind.as_str())
            .collect::<Vec<_>>(),
        vec![
            "action",
            "action",
            "dialogue",
            "dialogue",
            "emotion",
            "lightingTransition",
            "cameraMovement"
        ]
    );
    let dialogue: Vec<_> = entries
        .iter()
        .filter(|entry| entry.kind == "dialogue")
        .collect();
    assert_eq!(dialogue[0].source_text, "같은 말이다.");
    assert_eq!(dialogue[1].source_text, "같은 말이다.");
    assert_ne!(dialogue[0].semantic_id, dialogue[1].semantic_id);
    assert_eq!(dialogue[0].source_locator.occurrence_index, 0);
    assert_eq!(dialogue[0].source_locator.dialogue_index, Some(0));
    assert_eq!(dialogue[1].source_locator.occurrence_index, 1);
    assert_eq!(dialogue[1].source_locator.dialogue_index, Some(1));
    assert!(entries
        .iter()
        .all(|entry| entry.entry_hash.starts_with("sha256:")
            && entry.semantic_id.starts_with("semantic.")));
    assert!(map
        .semantic_manifest
        .semantic_manifest_hash
        .starts_with("sha256:"));
    narrative_courier_contracts::validate_narrative_scene_map(&map).unwrap();
}

#[test]
fn scene_map_v2_does_not_invent_optional_directives() {
    let screenplay = "SCN-001. INT. 방 - 밤\n민준이 창문을 연다.\n";
    let blueprint = concat!(
        "SCN-001. INT. 방 - 밤\n",
        "- Observable action: 바람이 분다\n",
        "- Emotional direction: 긴장\n",
        "- Lighting: 어두움\n",
        "- Camera: 천천히 이동\n",
    );
    let map = sts_workflow::scene_map::build_scene_map(
        "semantic-v2-no-invention",
        screenplay.as_bytes(),
        blueprint.as_bytes(),
    )
    .unwrap();
    assert_eq!(
        map.semantic_manifest
            .entries
            .iter()
            .map(|entry| entry.kind.as_str())
            .collect::<Vec<_>>(),
        vec!["action"]
    );
}

#[test]
fn doctor_rejects_tampered_narrative_package_after_release() {
    let h = Harness::new();
    h.advance_to("final_release");
    h.approve();
    let path = h.project().join("narrative-package.json");
    let mut value: serde_json::Value = serde_json::from_slice(&fs::read(&path).unwrap()).unwrap();
    value["package_id"] = serde_json::Value::String("tampered".into());
    fs::write(&path, serde_json::to_vec_pretty(&value).unwrap()).unwrap();
    let report = h.engine.doctor(&h.name).unwrap();
    assert!(!report.ok);
    assert!(report.errors.iter().any(|e| e.contains("NarrativePackage")));
}

#[test]
fn completed_resume_preserves_release_proof_and_repairs_only_missing_bundle() {
    let h = Harness::new();
    h.advance_to("final_release");
    h.approve();
    let state_path = h.project().join("state.json");
    let bundle_path = h.project().join("release-bundle.json");
    let state_bytes = fs::read(&state_path).unwrap();
    let bundle_bytes = fs::read(&bundle_path).unwrap();
    h.engine.resume(&h.name).unwrap();
    assert_eq!(fs::read(&state_path).unwrap(), state_bytes);
    assert_eq!(fs::read(&bundle_path).unwrap(), bundle_bytes);
    fs::remove_file(&bundle_path).unwrap();
    assert!(h.engine.release_bundle(&h.name).is_err());
    h.engine.resume(&h.name).unwrap();
    assert_eq!(fs::read(&state_path).unwrap(), state_bytes);
    assert_eq!(fs::read(&bundle_path).unwrap(), bundle_bytes);
    assert!(h.engine.release_bundle(&h.name).is_ok());
}

#[test]
fn completed_resume_does_not_replace_a_corrupt_release_bundle() {
    let h = Harness::new();
    h.advance_to("final_release");
    h.approve();
    let path = h.project().join("release-bundle.json");
    fs::write(&path, b"corrupt").unwrap();
    assert!(h.engine.resume(&h.name).is_err());
    assert_eq!(fs::read(&path).unwrap(), b"corrupt");
}
