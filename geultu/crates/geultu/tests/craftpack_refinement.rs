//! M3 integration tests: craft-pack loading with SHA256SUMS integrity (H4)
//! and the diagnose→propose→approve refinement patch flow (H5).

mod support;

use geultu::craftpack::{load_craft_pack, CraftpackPaths};
use geultu::generation::{
    GenerationInput, GenerationPlan, OutputFormat, WritingAdapter, WritingType,
};
use geultu::refinement::{diagnose_text, load_refinement_pack, repair_decision};
use geultu::{Application, Error, Workspace};
use serde_json::{json, Value};
use sha2::Digest;
use std::fs;
use std::path::{Path, PathBuf};
use support::unique_temp_id;

fn repo_assets() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../assets")
}

fn real_craft_pack_dir() -> PathBuf {
    repo_assets().join("craft-packs-scriptforge-v4")
}

fn story_baseline_pack_dir() -> PathBuf {
    repo_assets().join("craft-packs-story-baseline-v1")
}

fn fixture_pack(label: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures/refinement")
        .join(label)
}

/// Recursively copies a pack directory into a fresh temp directory so
/// corruption scenarios never touch `assets/`.
fn copied_pack(source: &Path, label: &str) -> PathBuf {
    let target = std::env::temp_dir().join(format!(
        "geultu-m3-{label}-{}-{}",
        std::process::id(),
        unique_temp_id()
    ));
    let _ = fs::remove_dir_all(&target);
    fn copy_dir(from: &Path, to: &Path) {
        fs::create_dir_all(to).unwrap();
        for entry in fs::read_dir(from).unwrap() {
            let entry = entry.unwrap();
            let destination = to.join(entry.file_name());
            if entry.file_type().unwrap().is_dir() {
                copy_dir(&entry.path(), &destination);
            } else {
                fs::copy(entry.path(), destination).unwrap();
            }
        }
    }
    copy_dir(source, &target);
    target
}

fn workspace_app(label: &str) -> (std::path::PathBuf, Application) {
    let root = std::env::temp_dir().join(format!(
        "geultu-m3-ws-{label}-{}-{}",
        std::process::id(),
        unique_temp_id()
    ));
    let _ = fs::remove_dir_all(&root);
    let workspace = Workspace::init(&root).unwrap();
    (
        root.clone(),
        Application::from_workspace(workspace)
            .with_craftpack_paths(CraftpackPaths::from_assets_root(repo_assets())),
    )
}

#[derive(Debug)]
struct FixtureWritingAdapter;

impl WritingAdapter for FixtureWritingAdapter {
    fn id(&self) -> &str {
        "fixture-fiction"
    }

    fn writing_type(&self) -> &WritingType {
        static FICTION: std::sync::OnceLock<WritingType> = std::sync::OnceLock::new();
        FICTION.get_or_init(|| WritingType::new("fiction").unwrap())
    }

    fn prepare(&self, input: &GenerationInput) -> geultu::Result<GenerationPlan> {
        Ok(GenerationPlan {
            project_id: input.request.project_id.clone(),
            writing_type: self.writing_type().clone(),
            stage: input
                .request
                .stage
                .clone()
                .unwrap_or_else(|| geultu::WritingStage::new("draft").unwrap()),
            adapter_id: self.id().into(),
            source_content_revision: input.context.source_content_revision,
            prompt: format!("fixture: {}", input.request.brief),
            output_format: OutputFormat::Markdown,
            input_hash: "fixture-hash".into(),
            craftpack: None,
        })
    }
}

// ---------------------------------------------------------------- H4 loader

#[test]
fn story_baseline_pack_loads_with_one_draft_and_one_review_gate() {
    let dir = story_baseline_pack_dir();
    let pack = load_craft_pack(&dir).unwrap();
    assert_eq!(pack.manifest().pack_id, "story-baseline");
    assert_eq!(pack.manifest().version, "1.0.0");
    assert_eq!(pack.manifest().license, "MIT");
    assert_eq!(
        pack.manifest().redistribution_policy,
        "mit_licensed_story_owned_baseline"
    );
    assert_eq!(pack.manifest().prompt_asset_count, 2);
    assert_eq!(pack.workflow().nodes.len(), 2);
    assert_eq!(pack.workflow().agent_count(), 1);
    assert_eq!(pack.workflow().human_count(), 1);
    assert!(pack.files().contains_key("source/story-baseline-v1.zip"));
    assert_eq!(
        pack.manifest().source_archive_sha256,
        format!(
            "{:x}",
            sha2::Sha256::digest(fs::read(dir.join("source/story-baseline-v1.zip")).unwrap())
        )
    );
    let agent = pack
        .workflow()
        .nodes
        .iter()
        .find_map(|node| match node {
            geultu::craftpack::WorkflowNode::Agent(agent) => Some(agent),
            geultu::craftpack::WorkflowNode::Human(_) => None,
        })
        .unwrap();
    assert_eq!(agent.id, "draft-scene");
    assert_eq!(agent.step, Some(1));
    assert_eq!(agent.phase.as_deref(), Some("draft"));
    assert_eq!(agent.role, "scene-drafter");
    assert_eq!(agent.inputs, ["source_scene", "author_intent"]);
    assert_eq!(agent.output, "scene_candidate");
    let human = pack
        .workflow()
        .nodes
        .iter()
        .find_map(|node| match node {
            geultu::craftpack::WorkflowNode::Human(human) => Some(human),
            geultu::craftpack::WorkflowNode::Agent(_) => None,
        })
        .unwrap();
    assert_eq!(human.id, "review-scene");
    assert_eq!(human.gate_type, "review");
    assert_eq!(human.label, "Review scene candidate");
    assert_eq!(human.source, "scene_candidate");
    assert_eq!(human.checkpoint.as_deref(), Some("author-review"));
}

#[test]
fn story_baseline_pack_corruption_fails_closed_without_touching_source_assets() {
    let dir = copied_pack(&story_baseline_pack_dir(), "story-baseline-byteflip");
    let victim = dir.join("prompts/system.md");
    let mut body = fs::read_to_string(&victim).unwrap();
    body.push('x');
    fs::write(&victim, body).unwrap();
    let error = load_craft_pack(&dir).unwrap_err();
    assert!(
        matches!(&error, Error::Corrupt(message)
            if message.contains("integrity mismatch") && message.contains("prompts/system.md")),
        "unexpected error: {error:?}"
    );
    let _ = fs::remove_dir_all(dir);
}

#[test]
fn craft_pack_loads_and_surfaces_manifest_counts() {
    let pack = load_craft_pack(&real_craft_pack_dir()).unwrap();
    assert_eq!(pack.manifest().pack_id, "scriptforge-v4");
    assert_eq!(pack.manifest().version, "4.0.0");
    assert_eq!(pack.manifest().prompt_asset_count, 15);
    assert_eq!(pack.workflow().nodes.len(), 26);
    assert_eq!(pack.workflow().agent_count(), 18);
    assert_eq!(pack.workflow().human_count(), 8);
    assert_eq!(pack.files().len(), 20);
    assert!(
        pack.files().contains_key("prompts/workflow_manifest.json"),
        "manifest must be covered by SHA256SUMS"
    );
    let steps: Vec<_> = pack.workflow().agent_nodes();
    assert_eq!(steps.first().unwrap().id, "step_00");
    assert!(steps.iter().any(|node| node.role == "writer"));
}

#[test]
fn craftpack_status_reports_identity_integrity_and_counts() {
    let (_root, app) = workspace_app("status");
    let status = app.call("craftpack.status", json!({})).unwrap();
    assert_eq!(status["pack_id"], "scriptforge-v4");
    assert_eq!(status["version"], "4.0.0");
    assert_eq!(status["api_version"], "geultu.dev/craft-pack/v1");
    assert_eq!(status["license"], "CC-BY-NC-SA-4.0");
    assert_eq!(status["integrity"]["verified"], true);
    assert_eq!(status["integrity"]["checked_files"], 20);
    assert_eq!(status["prompt_asset_count"], 15);
    assert_eq!(status["workflow"]["node_count"], 26);
    assert_eq!(status["workflow"]["agent_nodes"], 18);
    assert_eq!(status["workflow"]["human_gates"], 8);
    assert_eq!(
        status["workflow"]["manifest_sha256"],
        "36766d7eeb5a131d8af8cd47cdb723ae0db4c77ca225c3072d274a5d737181a9"
    );
    assert_eq!(status["files"].as_array().unwrap().len(), 20);
}

#[test]
fn craft_pack_byte_flip_is_rejected_naming_the_file() {
    let dir = copied_pack(&real_craft_pack_dir(), "byteflip");
    let victim = dir.join("prompts/writer/system.md");
    let mut body = fs::read_to_string(&victim).unwrap();
    body.push('x');
    fs::write(&victim, body).unwrap();
    let error = load_craft_pack(&dir).unwrap_err();
    assert!(
        matches!(&error, Error::Corrupt(message)
            if message.contains("integrity mismatch") && message.contains("prompts/writer/system.md")),
        "unexpected error: {error:?}"
    );
    let _ = fs::remove_dir_all(dir);
}

#[test]
fn craft_pack_missing_file_is_rejected_naming_the_file() {
    let dir = copied_pack(&real_craft_pack_dir(), "missing");
    fs::remove_file(dir.join("LICENSE")).unwrap();
    let error = load_craft_pack(&dir).unwrap_err();
    assert!(
        matches!(&error, Error::NotFound(message) if message.contains("LICENSE")),
        "unexpected error: {error:?}"
    );
    let _ = fs::remove_dir_all(dir);
}

#[test]
fn sums_path_traversal_entry_is_rejected_before_any_read() {
    let dir = copied_pack(&real_craft_pack_dir(), "traversal");
    let sums_path = dir.join("SHA256SUMS");
    let mut sums = fs::read_to_string(&sums_path).unwrap();
    sums.push_str(&format!("{}  ../escape.md\n", "ab".repeat(32)));
    fs::write(&sums_path, sums).unwrap();
    let error = load_craft_pack(&dir).unwrap_err();
    assert!(
        matches!(&error, Error::InvalidInput(message) if message.contains("..")),
        "unexpected error: {error:?}"
    );
    let _ = fs::remove_dir_all(dir);
}

#[test]
fn generation_prepare_references_verified_craftpack_nodes() {
    let root = std::env::temp_dir().join(format!(
        "geultu-m3-plan-{}-{}",
        std::process::id(),
        unique_temp_id()
    ));
    let _ = fs::remove_dir_all(&root);
    let mut registry = geultu::GenerationRegistry::default();
    registry.add_writing_adapter(FixtureWritingAdapter);
    let app = Application::from_workspace(Workspace::init(&root).unwrap())
        .with_generation_registry(registry)
        .with_craftpack_paths(CraftpackPaths::from_assets_root(repo_assets()));
    app.call(
        "project.create",
        json!({"project_id": "book", "title": "Book", "command_id": "create"}),
    )
    .unwrap();

    let plan = app
        .call(
            "generation.prepare",
            json!({
                "project_id": "book",
                "writing_type": "fiction",
                "brief": "opening scene",
                "expected_content_revision": 0
            }),
        )
        .unwrap();
    let craftpack = &plan["craftpack"];
    assert_eq!(craftpack["pack_id"], "scriptforge-v4");
    assert_eq!(craftpack["version"], "4.0.0");
    assert_eq!(craftpack["workflow_node_count"], 26);
    assert_eq!(
        craftpack["workflow_manifest_sha256"],
        "36766d7eeb5a131d8af8cd47cdb723ae0db4c77ca225c3072d274a5d737181a9"
    );
    let nodes = craftpack["nodes"].as_array().unwrap();
    assert_eq!(nodes.len(), 18, "all agent nodes are cited");
    assert!(
        nodes.iter().all(|node| node["task_prompt"]
            .as_str()
            .unwrap()
            .starts_with("prompts/")),
        "cited task prompts must be real pack paths"
    );
    let _ = fs::remove_dir_all(root);
}

// ------------------------------------------------------- H5 refinement flow

const DIAGNOSE_TEXT: &str = "매우 매우 조용했다!! 그리고 그리고 문을 닫았다   계속......";

fn expected_codes(findings: &[Value]) -> Vec<String> {
    findings
        .iter()
        .map(|finding| finding["code"].as_str().unwrap().to_string())
        .collect()
}

#[test]
fn diagnose_rules_produce_expected_upper_snake_codes() {
    let findings = diagnose_text("scene_1", DIAGNOSE_TEXT);
    let codes = expected_codes(
        &serde_json::to_value(&findings)
            .unwrap()
            .as_array()
            .unwrap()
            .clone(),
    );
    for code in [
        "REPEATED_WORD",
        "PUNCTUATION_STACK",
        "WHITESPACE_RUN",
        "ELLIPSIS_RUN",
    ] {
        assert!(codes.contains(&code.into()), "missing {code}: {codes:?}");
    }
    // Spans are character offsets over Korean text and round-trip to the
    // matched substring.
    for finding in &findings {
        let chars: Vec<char> = DIAGNOSE_TEXT.chars().collect();
        assert_eq!(
            finding.found,
            chars[finding.span_start..finding.span_end]
                .iter()
                .collect::<String>()
        );
    }
    let repeated = findings
        .iter()
        .find(|finding| finding.code == "REPEATED_WORD")
        .unwrap();
    assert_eq!(
        repeated.replacement, None,
        "meaning-sensitive findings stay human-owned"
    );
    let stack = findings
        .iter()
        .find(|finding| finding.code == "PUNCTUATION_STACK")
        .unwrap();
    assert_eq!(stack.replacement.as_deref(), Some("!"));
}

#[test]
fn refinement_fixture_packs_load_and_fail_closed_on_policy_violations() {
    let pack = load_refinement_pack(&fixture_pack("pack-ko")).unwrap();
    assert_eq!(pack.catalog().catalog_id, "language-refinement-fixture");
    assert!(pack.catalog().policy.translation_forbidden);
    assert!(pack.files().contains_key("archives/koreanize-fixture.zip"));

    let error = load_refinement_pack(&fixture_pack("pack-bad-policy")).unwrap_err();
    assert!(
        matches!(&error, Error::InvalidInput(message) if message.contains("translation_forbidden")),
        "unexpected error: {error:?}"
    );

    let error = load_refinement_pack(&fixture_pack("pack-unlisted-archive")).unwrap_err();
    assert!(
        matches!(&error, Error::Corrupt(message) if message.contains("not covered by SHA256SUMS")),
        "unexpected error: {error:?}"
    );
}

#[test]
fn refinement_diagnose_command_reports_policy_and_findings() {
    let (_root, app) = workspace_app("diagnose-cmd");
    app.call(
        "project.create",
        json!({"project_id": "book", "title": "Book", "command_id": "create"}),
    )
    .unwrap();
    app.call(
        "scene.add",
        json!({
            "project_id": "book",
            "scene_id":"scene_1",
            "title": "장면",
            "text": DIAGNOSE_TEXT,
            "command_id": "scene"
        }),
    )
    .unwrap();

    let result = app
        .call("refinement.diagnose", json!({"project_id": "book"}))
        .unwrap();
    assert_eq!(result["catalog_id"], "language-refinement-v1");
    assert_eq!(result["policy"]["translation_forbidden"], true);
    assert_eq!(
        result["policy"]["operations"],
        json!(["diagnose", "plan_patch"])
    );
    assert_eq!(result["policy"]["approval_owned_by"], "geultu");
    assert!(
        result["locales"].as_array().unwrap().contains(&json!("ko")),
        "the ko locale archive must be listed"
    );
    let codes = expected_codes(result["findings"].as_array().unwrap());
    assert!(codes.contains(&"REPEATED_WORD".to_string()), "{codes:?}");
    assert_eq!(
        result["finding_count"],
        result["findings"].as_array().unwrap().len()
    );

    let scoped = app
        .call(
            "refinement.diagnose",
            json!({"project_id": "book", "scene_id":"scene_1"}),
        )
        .unwrap();
    assert_eq!(scoped["findings"], result["findings"]);

    let error = app
        .call(
            "refinement.diagnose",
            json!({"project_id": "book", "scene_id": "ghost"}),
        )
        .unwrap_err();
    assert!(matches!(error, Error::NotFound(_)));
}

/// Reads the canonical candidate subject through the public review entrypoint.
fn stored_candidate_binding(app: &Application, candidate_id: &str) -> (u64, String) {
    let subject = app
        .call(
            "candidate.subject",
            json!({"project_id":"book","candidate_id":candidate_id}),
        )
        .unwrap();
    (
        subject["revision"].as_u64().unwrap(),
        subject["digest"].as_str().unwrap().to_string(),
    )
}

#[test]
fn propose_then_approved_apply_patches_scene_and_records_decision() {
    let (root, app) = seeded_refinement_flow("approve");
    let proposed = app
        .call(
            "refinement.propose_patch",
            json!({
                "project_id": "book",
                "scene_id":"scene_1",
                "candidate_id": "refine1",
                "changeset_id": "refine1-cs",
                "command_id": "propose-1"
            }),
        )
        .unwrap();
    assert_eq!(proposed["proposal"]["decision"], "REPAIR");
    let decision_id = proposed["proposal"]["decision_id"].as_str().unwrap();
    assert!(decision_id.starts_with("decision_") && decision_id.len() == 73);
    assert_eq!(proposed["candidate_id"], "refine1");
    assert_eq!(proposed["changeset_id"], "refine1-cs");
    let applied = proposed["applied_codes"].as_array().unwrap();
    assert!(!applied.is_empty(), "patchable findings must exist");

    // Nothing is applied yet: candidate and changeset are pending.
    let before = app
        .call("project.get", json!({"project_id": "book"}))
        .unwrap();
    assert_eq!(before["candidates"]["refine1"]["status"], "pending");
    assert_eq!(before["changesets"]["refine1-cs"]["status"], "pending");
    assert_eq!(before["scenes"]["scene_1"]["text"], DIAGNOSE_TEXT);

    let (artifact_revision, digest) = stored_candidate_binding(&app, "refine1");
    let approved = app
        .call(
            "changeset.approve",
            json!({
                "project_id": "book",
                "changeset_id": "refine1-cs",
                "expected_project_revision": before["revision"],
                "actor_id": "editor",
                "reason": "mechanical cleanup accepted",
                "expected_artifact_revision": artifact_revision,
                "expected_artifact_digest": digest,
                "command_id": "approve-1"
            }),
        )
        .unwrap();
    assert_eq!(approved["decision"]["decision"], "APPROVE");

    let after = app
        .call("project.get", json!({"project_id": "book"}))
        .unwrap();
    assert_eq!(after["changesets"]["refine1-cs"]["status"], "approved");
    let patched_text = after["scenes"]["scene_1"]["text"].as_str().unwrap();
    assert_ne!(patched_text, DIAGNOSE_TEXT);
    assert!(
        !patched_text.contains("!!") && !patched_text.contains("......"),
        "stacked punctuation must be collapsed: {patched_text}"
    );
    assert!(
        after["human_decisions"][approved["decision"]["decision_id"].as_str().unwrap()]["subject"]
            ["id"]
            == "candidate:refine1",
        "approval must bind to the refinement candidate"
    );

    // The recorded patch is retrievable as a first-class record.
    let candidates = app
        .call("candidate.list", json!({"project_id": "book"}))
        .unwrap();
    let record = candidates
        .as_array()
        .unwrap()
        .iter()
        .find(|candidate| candidate["id"] == "refine1")
        .unwrap();
    assert_eq!(record["kind"], "refinement_patch");
    assert_eq!(record["content"], json!(patched_text));
    let _ = fs::remove_dir_all(root);
}

#[test]
fn changeset_approval_requires_a_bound_human_decision() {
    let (root, app) = seeded_refinement_flow("unapproved");
    let proposed = app
        .call(
            "refinement.propose_patch",
            json!({
                "project_id": "book",
                "scene_id":"scene_1",
                "command_id": "propose-1"
            }),
        )
        .unwrap();
    let changeset_id = proposed["changeset_id"].as_str().unwrap().to_string();
    let project = app
        .call("project.get", json!({"project_id": "book"}))
        .unwrap();

    let journal_path = root.join(".geultu/projects/book/events.ndjson");
    let journal_before = fs::read_to_string(&journal_path).unwrap();

    // A changeset cannot be applied without a reviewer and exact candidate
    // subject claims, regardless of candidate kind.
    let error = app
        .call(
            "changeset.approve",
            json!({
                "project_id": "book",
                "changeset_id": changeset_id,
                "expected_project_revision": project["revision"]
            }),
        )
        .expect_err("claim-less changeset approval must fail");
    assert!(
        matches!(&error, Error::InvalidInput(message)
            if message.contains("actor_id") || message.contains("expected_artifact")),
        "unexpected error: {error:?}"
    );

    assert_eq!(
        fs::read_to_string(&journal_path).unwrap(),
        journal_before,
        "rejected approval must leave the journal untouched"
    );
    let after = app
        .call("project.get", json!({"project_id": "book"}))
        .unwrap();
    assert_eq!(after["scenes"]["scene_1"]["text"], DIAGNOSE_TEXT);
    assert_eq!(after["changesets"][changeset_id]["status"], "pending");
    let _ = fs::remove_dir_all(root);
}

#[test]
fn proposal_ids_are_deterministic_for_identical_input() {
    let build = |label: &str| {
        let (root, app) = seeded_refinement_flow(label);
        let proposed = app
            .call(
                "refinement.propose_patch",
                json!({
                    "project_id": "book",
                    "scene_id":"scene_1",
                    "command_id": "propose-det"
                }),
            )
            .unwrap();
        (root, app, proposed)
    };
    let (root_a, _app_a, proposal_a) = build("det-a");
    let (root_b, _app_b, proposal_b) = build("det-b");

    assert_eq!(
        proposal_a["proposal"]["decision_id"], proposal_b["proposal"]["decision_id"],
        "same input must derive the same decision_ identity"
    );
    assert_eq!(
        proposal_a["candidate_id"], proposal_b["candidate_id"],
        "content-derived candidate ids must match across workspaces"
    );
    assert_eq!(proposal_a["applied_codes"], proposal_b["applied_codes"]);

    // The pure decision builder agrees with the command-surface derivation
    // (reason codes are stored in sorted unique order).
    let rebuilt = repair_decision(
        "c28c27cc987a8a25742c5d961e42c1916551f0a784a3e20b135e16e7f5f0ed3e",
        2,
        &[
            "ELLIPSIS_RUN".into(),
            "PUNCTUATION_STACK".into(),
            "WHITESPACE_RUN".into(),
        ],
    )
    .unwrap();
    assert_eq!(
        rebuilt.decision_id,
        proposal_a["proposal"]["decision_id"].as_str().unwrap()
    );

    let _ = fs::remove_dir_all(root_a);
    let _ = fs::remove_dir_all(root_b);
}

/// Seeds a project whose single scene carries diagnosable text.
fn seeded_refinement_flow(label: &str) -> (std::path::PathBuf, Application) {
    let (root, app) = workspace_app(label);
    app.call(
        "project.create",
        json!({"project_id": "book", "title": "Book", "command_id": "create"}),
    )
    .unwrap();
    app.call(
        "scene.add",
        json!({
            "project_id": "book",
            "scene_id":"scene_1",
            "title": "장면",
            "text": DIAGNOSE_TEXT,
            "command_id": "scene"
        }),
    )
    .unwrap();
    (root, app)
}
