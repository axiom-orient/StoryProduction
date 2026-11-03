use std::fs;
use std::path::PathBuf;
use sts_workflow::{Engine, ImportScreenplayRequest};
use tempfile::TempDir;

const FOUNTAIN_FIXTURE: &str = "\
Title: 갈등의 방
Author: 테스트 작가
Contact: test@example.com

INT. 연구실 - 낮

[[내부 메모는 산출물에서 제외된다]]

/* 뼈대 노트
여러 줄이라도
통째로 제외 */
민준은 파일을 뒤진다.

/* 이전 판 */
민준이 화이트보드를 지운다.

@지우
(문을 열며)
민준, 여기 있었구나.

EXT. 옥상 - 밤

두 사람이 마주 선다.

> CUT TO:

FADE OUT.
";

const FDX_FIXTURE: &str = r#"<?xml version="1.0" encoding="UTF-8"?>
<FinalDraft DocumentType="Script" Template="No" Version="1">
  <Content>
    <Paragraph Type="Scene Heading">
      <Text>INT. 연구실 - 낮</Text>
    </Paragraph>
    <Paragraph Type="Action">
      <Text>민준은 파일을 뒤진다.</Text>
    </Paragraph>
    <Paragraph Type="Action">
      <Text>민준이 화이트보드를 지운다.</Text>
    </Paragraph>
    <Paragraph Type="Character">
      <Text>지우</Text>
    </Paragraph>
    <Paragraph Type="Parenthetical">
      <Text>문을 열며</Text>
    </Paragraph>
    <Paragraph Type="Dialogue">
      <Text>민준, 여기 있었구나.</Text>
    </Paragraph>
    <Paragraph Type="Scene Heading">
      <Text>EXT. 옥상 - 밤</Text>
    </Paragraph>
    <Paragraph Type="Action">
      <Text>두 사람이 마주 선다.</Text>
    </Paragraph>
    <Paragraph Type="Transition">
      <Text>CUT TO:</Text>
    </Paragraph>
  </Content>
</FinalDraft>
"#;

#[test]
fn fountain_fixture_parses_deterministically_with_expected_structure() {
    let first = sts_workflow::fountain::normalize_screenplay(
        sts_workflow::fountain::ScriptFormat::Fountain,
        FOUNTAIN_FIXTURE,
    )
    .unwrap();
    let second = sts_workflow::fountain::normalize_screenplay(
        sts_workflow::fountain::ScriptFormat::Fountain,
        FOUNTAIN_FIXTURE,
    )
    .unwrap();
    assert_eq!(first, second, "parsing must be deterministic");

    let summary = &first.summary;
    assert_eq!(summary.scene_count, 2);
    assert_eq!(summary.interior_count, 1);
    assert_eq!(summary.exterior_count, 1);
    assert_eq!(summary.cast, vec!["지우".to_string()]);
    assert_eq!(summary.skipped_lines, 0);

    assert!(first.text.starts_with("갈등의 방\n\n"), "{}", first.text);
    assert!(first.text.contains("INT. 연구실 - 낮\n"), "{}", first.text);
    assert!(!first.text.contains("내부 메모"), "{}", first.text);
    assert!(!first.text.contains("뼈대 노트"), "{}", first.text);
    assert!(!first.text.contains("이전 판"), "{}", first.text);
    assert!(
        first.text.contains("@지우\n".replace('@', "").as_str())
            || first.text.contains("지우\n(문을 열며)\n"),
        "cue with parenthetical and dialogue must be preserved: {}",
        first.text
    );
    assert!(first.text.contains("> CUT TO:"), "{}", first.text);
    assert!(first.text.ends_with("FADE OUT.\n"), "{}", first.text);
}

#[test]
fn fdx_fixture_normalizes_to_same_structure_as_fountain() {
    let fountain = sts_workflow::fountain::normalize_screenplay(
        sts_workflow::fountain::ScriptFormat::Fountain,
        FOUNTAIN_FIXTURE,
    )
    .unwrap();
    let fdx = sts_workflow::fountain::normalize_screenplay(
        sts_workflow::fountain::ScriptFormat::Fdx,
        FDX_FIXTURE,
    )
    .unwrap();

    assert_eq!(fountain.summary.scene_count, fdx.summary.scene_count);
    assert_eq!(fountain.summary.cast, fdx.summary.cast);
    for heading in ["INT. 연구실 - 낮", "EXT. 옥상 - 밤"] {
        assert!(fdx.text.contains(heading), "{}", fdx.text);
    }
    assert!(fdx.text.contains("민준은 파일을 뒤진다."), "{}", fdx.text);
    assert!(fdx.text.contains("(문을 열며)"), "{}", fdx.text);
    assert!(fdx.text.contains("민준, 여기 있었구나."), "{}", fdx.text);
    assert!(fdx.text.contains("> CUT TO:"), "{}", fdx.text);
}

#[test]
fn unknown_format_is_rejected_explicitly() {
    let err = sts_workflow::fountain::detect_format("script.rtf", "hello").unwrap_err();
    assert!(err.to_string().contains("unsupported screenplay format"));
}

fn harness() -> (TempDir, Engine) {
    let source = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");
    let tmp = TempDir::new().unwrap();
    copy_tree(&source.join("skills"), &tmp.path().join("skills"));
    let engine = Engine::new(tmp.path()).expect("fixture roots resolve");
    (tmp, engine)
}

fn copy_tree(src: &std::path::Path, dst: &std::path::Path) {
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
fn import_creates_project_with_normalized_reference_and_empty_output_authority() {
    let (_tmp, engine) = harness();
    let workdir = TempDir::new().unwrap();
    let script_path = workdir.path().join("galoon.fountain");
    let synopsis_path = workdir.path().join("synopsis.md");
    fs::write(&script_path, FOUNTAIN_FIXTURE).unwrap();
    fs::write(
        &synopsis_path,
        "인간이 승인한 요약. 지우는 진실을 확인하고 혼자 떠난다.",
    )
    .unwrap();

    let report = engine
        .import_screenplay(&ImportScreenplayRequest {
            project_name: "imported".into(),
            title: "갈등의 방".into(),
            synopsis: fs::read_to_string(&synopsis_path).unwrap(),
            source_format: "original".into(),
            target_duration: 100,
            genre: vec![],
            tone: String::new(),
            theme_direction: String::new(),
            script_path: script_path.display().to_string(),
        })
        .unwrap();

    assert_eq!(report.project.name, "imported");
    assert_eq!(report.summary.scene_count, 2);
    assert_eq!(
        report.reference_files,
        vec![
            "imported.screenplay.txt".to_string(),
            "imported.source.txt".to_string()
        ]
    );

    let project_dir = engine.project_dir("imported").unwrap();
    let normalized = project_dir.join("input/references/imported.screenplay.txt");
    let raw_source = project_dir.join("input/references/imported.source.txt");
    assert!(normalized.is_file());
    assert!(raw_source.is_file());
    assert_eq!(
        fs::read_to_string(normalized).unwrap(),
        sts_workflow::fountain::normalize_screenplay(
            sts_workflow::fountain::ScriptFormat::Fountain,
            FOUNTAIN_FIXTURE
        )
        .unwrap()
        .text
    );
    assert_eq!(fs::read_to_string(raw_source).unwrap(), FOUNTAIN_FIXTURE);

    let config = fs::read_to_string(project_dir.join("config.yaml")).unwrap();
    assert!(config.contains("imported.screenplay.txt"));
    assert!(config.contains("synopsis.md"));

    let output_entries: Vec<_> = fs::read_dir(project_dir.join("output"))
        .unwrap()
        .collect::<Result<Vec<_>, _>>()
        .unwrap();
    assert!(
        output_entries.is_empty(),
        "import must not fabricate narrative authority artifacts under output/: {output_entries:?}"
    );

    let projects_root = engine
        .project_dir("imported")
        .unwrap()
        .parent()
        .unwrap()
        .to_path_buf();
    let staging_leftover: Vec<_> = fs::read_dir(&projects_root)
        .unwrap()
        .filter_map(Result::ok)
        .filter(|e| e.file_name().to_string_lossy().starts_with(".import-"))
        .collect();
    assert!(
        staging_leftover.is_empty(),
        "staging dir must be cleaned up: {staging_leftover:?}"
    );
}

#[test]
fn duplicate_import_is_rejected_and_missing_scenes_fail_fast() {
    let (_tmp, engine) = harness();
    let workdir = TempDir::new().unwrap();
    let script_path = workdir.path().join("galoon.fountain");
    let synopsis_path = workdir.path().join("synopsis.md");
    fs::write(&script_path, FOUNTAIN_FIXTURE).unwrap();
    fs::write(&synopsis_path, "승인된 시놉시스.").unwrap();
    let request = ImportScreenplayRequest {
        project_name: "dup".into(),
        title: "갈등의 방".into(),
        synopsis: fs::read_to_string(&synopsis_path).unwrap(),
        source_format: "original".into(),
        target_duration: 100,
        genre: vec![],
        tone: String::new(),
        theme_direction: String::new(),
        script_path: script_path.display().to_string(),
    };
    engine.import_screenplay(&request).unwrap();
    let second = engine.import_screenplay(&request).unwrap_err();
    assert!(second.to_string().contains("already exists"));

    let sceneless = workdir.path().join("sceneless.fountain");
    fs::write(&sceneless, "그저 문장들 뿐인 텍스트.").unwrap();
    let err = engine
        .import_screenplay(&ImportScreenplayRequest {
            script_path: sceneless.display().to_string(),
            ..request
        })
        .unwrap_err();
    assert!(err.to_string().contains("no scene headings"));
}

mod export_roundtrip {
    use super::*;
    use sts_workflow::NextPacket;

    fn artifact_for(node: &str) -> &'static str {
        match node {
            "step_00" => concat!(
                "# Source IR\n\n## SRC Assertions\n",
                "SRC-001 — 민준은 범인이 아니다.; polarity=NEGATE; epistemic=FACT; materiality=CORE; protected=yes; evidence=SPAN-001\n\n",
                "## REL Relations\n\n",
                "## Source Coverage Audit\n",
                "| Source span | Exact source text | SRC/REL ID | Unmapped reason |\n",
                "|---|---|---|---|\n",
                "| SPAN-001 | 민준은 범인이 아니다. | SRC-001 | — |\n\n",
                "## GAP Open Adaptation Space\nGAP-001: scene location\n\n",
                "## CON Authority Conflicts\n없음\n\n",
                "## Ending Contract\n민준은 범인이 아니다.\n\n",
                "## Adaptation Preflight\nTarget scale: SHORT-FORM. Coverage: FULL. Semantic risk: MODERATE.\nAdaptation Pressure: MODERATE.\n",
                "## Fidelity Risks\n민준이 범인이라는 내용으로 바뀌지 않는다.\n"
            ),
            "step_02" => "## Fabula\n사건\n\n## Syuzhet\n제시 순서\n\n## Information Ledger\n상태\n\n## Transformation Ledger\nPRESERVE\n",
            "step_06" => concat!(
                "SCN-001. INT. 방 - 밤\n",
                "- Necessary function: 기록을 확인하고 결말의 선택을 수행한다\n",
                "- Cinematic value: 기록을 읽는 행동이 확인으로 바뀐다\n",
                "- Required preconditions: 지우는 민준을 의심한다\n",
                "- Observable action: 지우가 기록을 확인한다\n",
                "- Dominant turn: 기록이 민준의 범행을 부정한다\n",
                "- Required postconditions: 지우는 진실을 확인한다\n",
                "- Required props: 기록\n",
                "- Downstream dependencies: none\n",
                "- Risk: 의심과 사실을 합치지 않는다\n"
            ),
            "step_07_part1" | "step_07_part2" | "step_07_part3" | "step_07_part4" => concat!(
                "SCN-001. INT. 방 - 밤\n지우가 기록을 확인한다.\n\n",
                "## [TRANSFER CAPSULE]\n- last SCN: SCN-001\n- knowledge: 지우는 진실을 안다\n"
            ),
            "step_07_full" | "step_08_apply" => {
                "SCN-001. INT. 방 - 밤\n지우가 기록을 확인한다. 그 기록에는 민준이 범인이 아니라고 적혀 있다.\n\n    지우   이제 알겠어.\n"
            }
            "step_08_reverse" => concat!(
                "## OBS Assertions\n",
                "- OBS-001 — 기록에는 민준이 범인이 아니라고 적혀 있다. evidence=SCN-001\n\n",
                "## OREL Relations\n",
                "- OREL-001 — 기록 확인 BEFORE 지우의 대사. evidence=SCN-001\n"
            ),
            "step_08_verify" => concat!(
                "# Verification\n",
                "| Source ID | State | Recovered Screenplay IR IDs | Screenplay scene evidence | Rationale |\n",
                "|---|---|---|---|---|\n",
                "| SRC-001 | DRAMATIZED | OBS-001 | SCN-001: 기록에 민준의 무죄가 적혀 있다 | 핵심 주장을 대본 장면에서 복원한다. |\n",
                "\nSTS_STATUS: READY\n"
            ),
            _ => "# Artifact\n\nverified\n",
        }
    }

    fn drive_to_completed(engine: &Engine, name: &str) {
        for _ in 0..200 {
            match engine.next(name).unwrap() {
                NextPacket::AgentTask {
                    node, task_token, ..
                } => {
                    engine
                        .submit(name, artifact_for(&node), &task_token)
                        .unwrap();
                }
                NextPacket::AwaitingHuman { gate_token, .. } => {
                    engine
                        .decide(name, "approve", None, None, &gate_token)
                        .unwrap();
                }
                NextPacket::Paused => panic!("paused unexpectedly"),
                NextPacket::Completed { .. } => return,
            }
        }
        panic!("workflow did not complete");
    }

    const CANONICAL_SCREENPLAY: &str = "SCN-001. INT. 방 - 밤\n지우가 기록을 확인한다.\n\n    지우   이제 알겠어.\n\nSCN-002. EXT. 옥상 - 새벽\n민준이 기다린다.\n";

    #[test]
    fn fountain_export_round_trips_structure_for_korean_cast() {
        let text =
            sts_workflow::fountain::to_fountain(CANONICAL_SCREENPLAY, Some("갈등의 방")).unwrap();
        assert!(text.starts_with("Title: 갈등의 방\n\n"), "{text}");
        assert!(text.contains(".SCN-001. INT. 방 - 밤"), "{text}");
        assert!(
            text.contains("@지우"),
            "Korean cue must be force-marked: {text}"
        );

        let reparsed = sts_workflow::fountain::normalize_screenplay(
            sts_workflow::fountain::ScriptFormat::Fountain,
            &text,
        )
        .unwrap();
        assert_eq!(reparsed.summary.scene_count, 2);
        assert_eq!(reparsed.summary.interior_count, 1);
        assert_eq!(reparsed.summary.exterior_count, 1);
        assert_eq!(reparsed.summary.cast, vec!["지우".to_string()]);
        assert!(reparsed.text.contains("이제 알겠어."), "{}", reparsed.text);
        assert!(
            reparsed.text.contains("민준이 기다린다."),
            "{}",
            reparsed.text
        );

        let again =
            sts_workflow::fountain::to_fountain(CANONICAL_SCREENPLAY, Some("갈등의 방")).unwrap();
        assert_eq!(text, again, "export must be deterministic");
    }

    #[test]
    fn fdx_export_matches_fountain_structure_and_is_valid_input() {
        let fountain =
            sts_workflow::fountain::to_fountain(CANONICAL_SCREENPLAY, Some("갈등의 방")).unwrap();
        let fdx = sts_workflow::fountain::to_fdx(CANONICAL_SCREENPLAY, Some("갈등의 방")).unwrap();
        assert!(fdx.contains("<FinalDraft DocumentType=\"Script\""));
        assert!(fdx.contains("Type=\"Scene Heading\""));

        let from_fdx = sts_workflow::fountain::normalize_screenplay(
            sts_workflow::fountain::ScriptFormat::Fdx,
            &fdx,
        )
        .unwrap();
        let from_fountain = sts_workflow::fountain::normalize_screenplay(
            sts_workflow::fountain::ScriptFormat::Fountain,
            &fountain,
        )
        .unwrap();
        assert_eq!(
            from_fdx.summary.scene_count,
            from_fountain.summary.scene_count
        );
        assert_eq!(from_fdx.summary.cast, from_fountain.summary.cast);
    }

    #[test]
    fn engine_exports_require_completed_release_and_write_files() {
        let (_tmp, engine) = harness();
        let workdir = TempDir::new().unwrap();
        let script_path = workdir.path().join("story.fountain");
        fs::write(&script_path, FOUNTAIN_FIXTURE).unwrap();

        engine
            .import_screenplay(&ImportScreenplayRequest {
                project_name: "rt".into(),
                title: "라운드트립".into(),
                synopsis: "민준은 범인이 아니다.".into(),
                source_format: "original".into(),
                target_duration: 100,
                genre: vec![],
                tone: String::new(),
                theme_direction: String::new(),
                script_path: script_path.display().to_string(),
            })
            .unwrap();

        let out = workdir.path().join("out.fountain");
        let blocked = engine.export_screenplay_fountain("rt", &out).unwrap_err();
        assert!(blocked.to_string().contains("workflow is not complete"));

        drive_to_completed(&engine, "rt");
        engine.export_screenplay_fountain("rt", &out).unwrap();
        let exported = fs::read_to_string(&out).unwrap();
        assert!(exported.contains("@지우"));

        let reparsed = sts_workflow::fountain::normalize_screenplay(
            sts_workflow::fountain::ScriptFormat::Fountain,
            &exported,
        )
        .unwrap();
        assert_eq!(
            reparsed.summary.scene_count, 1,
            "export must follow the COMPLETED canonical screenplay, not import references"
        );
        assert_eq!(reparsed.summary.cast, vec!["지우".to_string()]);

        let out_fdx = workdir.path().join("out.fdx");
        engine.export_screenplay_fdx("rt", &out_fdx).unwrap();
        assert!(fs::read_to_string(&out_fdx)
            .unwrap()
            .contains("<FinalDraft"));
    }
}
