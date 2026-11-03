//! Isolated executable seams. These fixtures never call a model/image provider.
use serde_json::{json, Value};
use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::{
    path::{Path, PathBuf},
    process::Command,
};
use storyctl::adapters::promptgen::{run_promptgen, PromptgenOptions};
use storyctl::effects::hash;
use storyctl::workflows::capability::{
    build_scene_asset_request, validate_candidate, SceneAssetOptions,
};

struct Fixture {
    _temp: tempfile::TempDir,
    root: PathBuf,
    request: Value,
    options: PromptgenOptions,
}
impl Fixture {
    fn new() -> Self {
        let temp = tempfile::tempdir().unwrap();
        let root = temp.path().canonicalize().unwrap();
        let integration = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");
        fs::create_dir_all(root.join("integration/contracts")).unwrap();
        for entry in fs::read_dir(integration.join("contracts")).unwrap() {
            let entry = entry.unwrap();
            if entry.path().is_file() {
                fs::copy(
                    entry.path(),
                    root.join("integration/contracts").join(entry.file_name()),
                )
                .unwrap();
            }
        }
        let mut payload: Value = serde_json::from_slice(
            &fs::read(
                integration.join("../ImagePromptCompiler/examples/image-cinematic-storyboard.json"),
            )
            .unwrap(),
        )
        .unwrap();
        payload["output"]["width"] = json!(16);
        payload["output"]["height"] = json!(16);
        let raw = serde_json::to_vec(&payload).unwrap();
        fs::write(root.join("payload.json"), &raw).unwrap();
        let mut request: Value = serde_json::from_slice(
            &fs::read(
                integration.join("contracts/examples/capability-request.prompt-compile.json"),
            )
            .unwrap(),
        )
        .unwrap();
        request["payload"] = json!({"kind":"promptgen.image_request","identity":"payload.test","path":"payload.json",
            "sha256":hash::sha256_bytes(&raw),"mediaType":"application/json","byteLength":raw.len()});
        let options = PromptgenOptions {
            request: root.join("request.json"),
            output_dir: root.join("results/run"),
            promptgen_bin: root.join("test-promptgen"),
            timeout_seconds: 1,
            artifact_wait_seconds: 0,
        };
        Self {
            _temp: temp,
            root,
            request,
            options,
        }
    }
    fn provider(&self, body: &str) {
        let script=format!("#!/bin/sh\nresult=''\nimage=''\nwhile [ \"$#\" -gt 0 ]; do\ncase \"$1\" in\n--result) result=\"$2\"; shift 2;;\n--image-output) image=\"$2\"; shift 2;;\n*) shift;;\nesac\ndone\n/bin/cat > /dev/null\n{body}\n");
        let binary = &self.options.promptgen_bin;
        fs::write(binary, script).unwrap();
        fs::set_permissions(binary, fs::Permissions::from_mode(0o700)).unwrap();
    }
    fn invoke(&self) -> storyctl::error::Result<Value> {
        fs::write(
            &self.options.request,
            serde_json::to_vec(&self.request).unwrap(),
        )
        .unwrap();
        run_promptgen(&self.root, &self.options)
    }
    fn compile(&self) {
        self.provider("/bin/cat > \"$result\" <<'JSON'\n{\"kind\":\"image\",\"status\":\"valid\",\"prompt\":\"fixture prompt\",\"diagnostics\":[],\"metadata\":{}}\nJSON");
    }
    fn external(&mut self, edit: bool) {
        if edit {
            let source = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
                .join("../../../ImagePromptCompiler/examples/image-edit-product.json");
            let mut payload: Value = serde_json::from_slice(&fs::read(source).unwrap()).unwrap();
            payload["output"]["width"] = json!(16);
            payload["output"]["height"] = json!(16);
            let bytes = serde_json::to_vec(&payload).unwrap();
            fs::write(self.root.join("payload.json"), &bytes).unwrap();
            self.request["payload"]["sha256"] = json!(hash::sha256_bytes(&bytes));
            self.request["payload"]["byteLength"] = json!(bytes.len());
        }
        self.request["capability"] = json!(if edit { "image.edit" } else { "image.generate" });
        self.request["provider"]["kind"] = json!(if edit { "openai" } else { "codex" });
        self.request["provider"]["networkPolicy"] = json!("required");
        self.request["rights"]["providerTransferAllowed"] = json!(true);
        self.request["expectedOutputs"] =
            json!([{"kind":"image_candidate","mediaType":"image/png"}]);
    }
    fn receipt_provider(&mut self, edit: bool, drift: bool) {
        let png: &[u8] = &[
            137, 80, 78, 71, 13, 10, 26, 10, 0, 0, 0, 13, 73, 72, 68, 82, 0, 0, 0, 16, 0, 0, 0, 16,
            8, 6, 0, 0, 0, 31, 243, 255, 97, 0, 0, 0, 24, 73, 68, 65, 84, 120, 156, 99, 96, 96, 96,
            248, 79, 33, 30, 53, 96, 212, 128, 81, 3, 134, 135, 1, 0, 155, 204, 255, 1, 162, 232,
            111, 166, 0, 0, 0, 0, 73, 69, 78, 68, 174, 66, 96, 130,
        ];
        fs::write(self.root.join("fixture.png"), png).unwrap();
        self.external(edit);
        if edit {
            self.request["references"] = json!([{"kind":"image_reference","identity":"reference.test","path":"fixture.png",
            "sha256":hash::sha256_bytes(png),"mediaType":"image/png","byteLength":png.len(),"width":16,"height":16}]);
        }
        let mut receipt = json!({"backend":if edit{"openai-images"}else{"codex-imagegen"},"provider":"openai","http_status":200,
            "width":16,"height":16,"thread_id":"test.thread","image_call_id":"test.call","process_exit_code":0,
            "compiled_prompt_sha256":hash::sha256_hex_bytes(b"fixture prompt"),"reference_sha256":hash::sha256_hex_bytes(png),"reference_bytes":png.len()});
        let digest = if drift {
            "0".repeat(64)
        } else {
            hash::sha256_hex_bytes(png)
        };
        receipt[if edit { "output_sha256" } else { "sha256" }] = json!(digest);
        receipt[if edit {
            "output_path"
        } else {
            "output_artifact"
        }] = json!(self.options.output_dir.join("image.png"));
        let value = if edit {
            receipt
        } else {
            json!({"compilation":{"kind":"image","status":"valid","prompt":"fixture prompt","diagnostics":[],"metadata":{}},"execution":receipt})
        };
        let source = self
            .root
            .join("fixture.png")
            .to_string_lossy()
            .replace('\'', "'\\''");
        self.provider(&format!(
            "/bin/cp '{source}' \"$image\"\n/bin/cat > \"$result\" <<'JSON'\n{value}\nJSON"
        ));
    }
    fn assert_terminal(&self, result: &Value, status: &str) {
        assert_eq!(result["result"]["status"], status);
        assert_eq!(result["job"]["status"], status);
        assert_eq!(result["result"]["candidateOnly"], true);
        for name in ["job", "result"] {
            let saved: Value = serde_json::from_slice(
                &fs::read(
                    self.options
                        .output_dir
                        .join(format!("capability-{name}.json")),
                )
                .unwrap(),
            )
            .unwrap();
            assert_eq!(saved, result[name]);
        }
        if status != "succeeded" {
            assert_eq!(result["result"]["artifacts"], json!([]));
            assert_eq!(result["result"]["failure"]["retryable"], false);
        }
    }
}

#[test]
fn compilation_captures_exact_input_and_rejects_attempt_reuse() {
    let fixture = Fixture::new();
    fixture.compile();
    let result = fixture.invoke().unwrap();
    fixture.assert_terminal(&result, "succeeded");
    assert_eq!(
        fs::read(fixture.options.output_dir.join("input.json")).unwrap(),
        fs::read(fixture.root.join("payload.json")).unwrap()
    );
    let old = fs::read(fixture.options.output_dir.join("capability-result.json")).unwrap();
    assert!(fixture.invoke().is_err());
    assert_eq!(
        fs::read(fixture.options.output_dir.join("capability-result.json")).unwrap(),
        old
    );
}

#[test]
fn dispatched_snapshots_cannot_change_without_an_uncertain_terminal_result() {
    for edit in [false, true] {
        let mut fixture = Fixture::new();
        fixture.receipt_provider(edit, false);
        let binary = &fixture.options.promptgen_bin;
        let original = fs::read_to_string(binary).unwrap();
        let original_payload = fs::read(fixture.root.join("payload.json")).unwrap();
        fs::write(
            binary,
            format!("{original}\n/bin/printf '\\n' >> input.json\n"),
        )
        .unwrap();
        let result = fixture.invoke().unwrap();
        fixture.assert_terminal(&result, "outcome_unknown");
        assert_eq!(
            fs::read(fixture.root.join("payload.json")).unwrap(),
            original_payload
        );
        assert_eq!(
            hash::sha256_bytes(
                &fs::read(fixture.options.output_dir.join("capability-request.json")).unwrap()
            ),
            result["result"]["requestHash"]
        );
    }
    let mut fixture = Fixture::new();
    fixture.receipt_provider(true, false);
    let binary = &fixture.options.promptgen_bin;
    let original = fs::read_to_string(binary).unwrap();
    fs::write(
        binary,
        format!("{original}\n/bin/printf '\\n' >> reference.png\n"),
    )
    .unwrap();
    fixture.assert_terminal(&fixture.invoke().unwrap(), "outcome_unknown");
}

#[test]
fn output_path_replacement_never_receives_the_claimed_job_or_terminal_result() {
    let fixture = Fixture::new();
    fixture.provider("directory=$(/bin/pwd)\n/bin/mv \"$directory\" \"$directory-moved\"\n/bin/mkdir \"$directory\"\n/bin/cat > \"$result\" <<'JSON'\n{\"kind\":\"image\",\"status\":\"valid\",\"prompt\":\"fixture prompt\",\"diagnostics\":[],\"metadata\":{}}\nJSON");
    let result = fixture.invoke().unwrap();
    assert_eq!(result["result"]["status"], "outcome_unknown");
    assert_eq!(result["job"]["status"], "outcome_unknown");
    assert!(fs::read_dir(&fixture.options.output_dir)
        .unwrap()
        .next()
        .is_none());
    let moved = fixture.root.join("results/run-moved");
    for name in ["job", "result"] {
        let saved: Value = serde_json::from_slice(
            &fs::read(moved.join(format!("capability-{name}.json"))).unwrap(),
        )
        .unwrap();
        assert_eq!(saved, result[name]);
    }
    assert!(moved.join(".execution-owner").is_file());
}

#[test]
fn producer_payload_and_compilation_schemas_are_checked_at_the_boundary() {
    let mut fixture = Fixture::new();
    fixture.compile();
    let raw = br#"{"output":{"width":1,"height":1}}"#;
    fs::write(fixture.root.join("payload.json"), raw).unwrap();
    fixture.request["payload"]["sha256"] = json!(hash::sha256_bytes(raw));
    fixture.request["payload"]["byteLength"] = json!(raw.len());
    assert!(fixture.invoke().is_err());
    assert!(!fixture.options.output_dir.exists());
    let fixture = Fixture::new();
    fixture.provider("/bin/cat > \"$result\" <<'JSON'\n{\"kind\":\"image\",\"status\":\"valid\",\"prompt\":\"fixture prompt\",\"diagnostics\":[]}\nJSON");
    fixture.assert_terminal(&fixture.invoke().unwrap(), "failed");
}

#[test]
fn missing_or_malformed_candidate_and_spawn_failures_are_terminal() {
    for body in [
        "exit 0",
        "printf '{' > \"$result\"",
        "printf '%s' '{\"kind\":\"image\",\"status\":\"invalid\"}' > \"$result\"",
    ] {
        let fixture = Fixture::new();
        fixture.provider(body);
        fixture.assert_terminal(&fixture.invoke().unwrap(), "failed");
    }
    let fixture = Fixture::new();
    fixture.provider("exit 0");
    fs::write(&fixture.options.promptgen_bin, "bad executable format").unwrap();
    fixture.assert_terminal(&fixture.invoke().unwrap(), "failed");
}

#[test]
fn explicit_missing_binary_and_unsafe_output_start_nothing() {
    let mut fixture = Fixture::new();
    fixture.options.promptgen_bin = fixture.root.join("missing");
    assert!(fixture.invoke().is_err());
    assert!(!fixture.options.output_dir.exists());
    fixture.options.promptgen_bin = fixture.root.join("test-promptgen");
    fixture.compile();
    fixture.options.output_dir = fixture.root.parent().unwrap().join(format!(
        "{}-outside",
        fixture.root.file_name().unwrap().to_string_lossy()
    ));
    assert!(fixture.invoke().is_err());
    assert!(!fixture.options.output_dir.exists());
}

fn marker_executable(path: &Path, marker: &Path) {
    fs::create_dir_all(path.parent().unwrap()).unwrap();
    fs::write(
        path,
        format!("#!/bin/sh\n: > '{}'\nexit 1\n", marker.display()),
    )
    .unwrap();
    fs::set_permissions(path, fs::Permissions::from_mode(0o700)).unwrap();
}

fn run_promptgen_cli(
    fixture: &Fixture,
    output_dir: &Path,
    path: &Path,
    promptgen_bin: Option<&Path>,
) -> std::process::Output {
    let mut command = Command::new(env!("CARGO_BIN_EXE_storyctl"));
    command
        .arg("--story-root")
        .arg(&fixture.root)
        .args(["run-promptgen", "--request"])
        .arg(&fixture.options.request)
        .arg("--output-dir")
        .arg(output_dir);
    if let Some(binary) = promptgen_bin {
        command.arg("--promptgen-bin").arg(binary);
    }
    command.env("PATH", path).output().unwrap()
}

#[test]
fn promptgen_cli_requires_explicit_binary_before_search_or_claim() {
    for fallback in ["release", "debug", "PATH"] {
        let fixture = Fixture::new();
        fs::write(
            &fixture.options.request,
            serde_json::to_vec(&fixture.request).unwrap(),
        )
        .unwrap();
        let marker = fixture.root.join(format!("{fallback}-binary-was-run"));
        let path = fixture.root.join("only-selected-path");
        fs::create_dir_all(&path).unwrap();
        let binary = match fallback {
            "release" | "debug" => fixture
                .root
                .join(format!("ImagePromptCompiler/target/{fallback}/promptgen")),
            _ => path.join("promptgen"),
        };
        marker_executable(&binary, &marker);
        let output_dir = fixture.root.join(format!("results/{fallback}"));
        let result = run_promptgen_cli(&fixture, &output_dir, &path, None);
        assert_eq!(result.status.code(), Some(2), "{fallback}");
        let error: Value = serde_json::from_slice(&result.stderr).unwrap();
        assert_eq!(error["status"], "not_applied", "{fallback}");
        assert!(error["error"]
            .as_str()
            .unwrap()
            .contains("required option --promptgen-bin"));
        assert!(!marker.exists(), "{fallback} fallback ran");
        assert!(
            !output_dir.exists(),
            "{fallback} claim directory was created"
        );
    }
}

#[test]
fn promptgen_cli_runs_the_explicitly_selected_binary() {
    let fixture = Fixture::new();
    fixture.compile();
    fs::write(
        &fixture.options.request,
        serde_json::to_vec(&fixture.request).unwrap(),
    )
    .unwrap();
    let path = fixture.root.join("empty-path");
    fs::create_dir_all(&path).unwrap();
    let result = run_promptgen_cli(
        &fixture,
        &fixture.options.output_dir,
        &path,
        Some(&fixture.options.promptgen_bin),
    );
    assert_eq!(
        result.status.code(),
        Some(0),
        "{}",
        String::from_utf8_lossy(&result.stderr)
    );
    let value: Value = serde_json::from_slice(&result.stdout).unwrap();
    assert_eq!(value["result"]["status"], "succeeded");
    assert_eq!(value["job"]["status"], "succeeded");
    assert_eq!(
        fs::read(fixture.options.output_dir.join("input.json")).unwrap(),
        fs::read(fixture.root.join("payload.json")).unwrap()
    );
}

#[test]
fn cli_uses_story_resources_and_capability_files_from_separate_roots() {
    let fixture = Fixture::new();
    fixture.compile();
    let capability_temp = tempfile::tempdir().unwrap();
    let capability_root = capability_temp.path().canonicalize().unwrap();
    fs::copy(
        fixture.root.join("payload.json"),
        capability_root.join("payload.json"),
    )
    .unwrap();
    fs::write(
        &fixture.options.request,
        serde_json::to_vec(&fixture.request).unwrap(),
    )
    .unwrap();

    let execution = Command::new(env!("CARGO_BIN_EXE_storyctl"))
        .arg("--story-root")
        .arg(&fixture.root)
        .args([
            "run-promptgen",
            "--capability-root",
            capability_root.to_str().unwrap(),
            "--request",
            fixture.options.request.to_str().unwrap(),
            "--output-dir",
            "results/run",
            "--promptgen-bin",
        ])
        .arg(&fixture.options.promptgen_bin)
        .output()
        .unwrap();
    assert_eq!(
        execution.status.code(),
        Some(0),
        "{}",
        String::from_utf8_lossy(&execution.stderr)
    );
    let execution: Value = serde_json::from_slice(&execution.stdout).unwrap();
    assert_eq!(execution["result"]["status"], "succeeded");
    assert_eq!(
        execution["result"]["artifacts"][0]["path"],
        "results/run/promptgen-result.json"
    );
    assert!(capability_root
        .join("results/run/promptgen-result.json")
        .is_file());
    assert!(!fixture
        .root
        .join("results/run/promptgen-result.json")
        .exists());

    fs::write(
        fixture.root.join("current-source.json"),
        serde_json::to_vec(&fixture.request["source"]).unwrap(),
    )
    .unwrap();
    fs::write(
        fixture.root.join("candidate-result.json"),
        serde_json::to_vec(&execution["result"]).unwrap(),
    )
    .unwrap();
    let review = Command::new(env!("CARGO_BIN_EXE_storyctl"))
        .arg("--story-root")
        .arg(&fixture.root)
        .args([
            "capability",
            "validate-candidate",
            "--capability-root",
            capability_root.to_str().unwrap(),
            "--current-source",
            fixture.root.join("current-source.json").to_str().unwrap(),
            "--result",
            fixture.root.join("candidate-result.json").to_str().unwrap(),
        ])
        .output()
        .unwrap();
    assert_eq!(
        review.status.code(),
        Some(0),
        "{}",
        String::from_utf8_lossy(&review.stderr)
    );
    let review: Value = serde_json::from_slice(&review.stdout).unwrap();
    assert_eq!(review["status"], "eligible_for_review");
}

#[test]
fn implicit_root_discovery_uses_executable_parent_and_fails_closed_for_install_markers() {
    let cwd = tempfile::tempdir().unwrap();
    let unrelated_registry = cwd.path().join("integration/contracts");
    fs::create_dir_all(&unrelated_registry).unwrap();
    fs::write(
        unrelated_registry.join("registry.json"),
        br#"{"schemaVersion":"story.contract-registry/1"}"#,
    )
    .unwrap();

    let source = Command::new(env!("CARGO_BIN_EXE_storyctl"))
        .args(["verify-contracts"])
        .current_dir(cwd.path())
        .env_remove("STORY_ROOT")
        .output()
        .unwrap();
    assert_eq!(
        source.status.code(),
        Some(0),
        "{}",
        String::from_utf8_lossy(&source.stderr)
    );
    let source: Value = serde_json::from_slice(&source.stdout).unwrap();
    assert_eq!(source["status"], "PASS");

    let installed = cwd.path().join("installed");
    let bin = installed.join("bin");
    fs::create_dir_all(&bin).unwrap();
    let installed_registry = installed.join("integration/contracts");
    fs::create_dir_all(&installed_registry).unwrap();
    fs::write(
        installed_registry.join("registry.json"),
        br#"{"schemaVersion":"story.contract-registry/1"}"#,
    )
    .unwrap();
    let installed_executable = bin.join("storyctl");
    fs::copy(env!("CARGO_BIN_EXE_storyctl"), &installed_executable).unwrap();
    fs::set_permissions(&installed_executable, fs::Permissions::from_mode(0o755)).unwrap();

    let missing_manifest = Command::new(&installed_executable)
        .args(["doctor", "--scope", "authoring"])
        .current_dir(cwd.path())
        .env_remove("STORY_ROOT")
        .output()
        .unwrap();
    assert_eq!(missing_manifest.status.code(), Some(3));
    assert!(missing_manifest.stdout.is_empty());
    assert!(String::from_utf8_lossy(&missing_manifest.stderr)
        .contains("installed Story runtime is incomplete"));

    fs::write(installed.join("runtime-manifest.json"), b"{}\n").unwrap();
    let corrupt_manifest = Command::new(&installed_executable)
        .args(["doctor", "--scope", "authoring"])
        .current_dir(cwd.path())
        .env_remove("STORY_ROOT")
        .output()
        .unwrap();
    assert_ne!(corrupt_manifest.status.code(), Some(0));
    assert!(corrupt_manifest.stdout.is_empty());
    assert!(
        !String::from_utf8_lossy(&corrupt_manifest.stderr).contains("Story resources unavailable")
    );
}

#[test]
fn symlink_candidate_and_payload_drift_never_succeed() {
    let fixture = Fixture::new();
    fixture.provider("/bin/ln -s ../input.json \"$result\"");
    fixture.assert_terminal(&fixture.invoke().unwrap(), "failed");
    let fixture = Fixture::new();
    fixture.compile();
    fs::write(fixture.root.join("payload.json"), "changed").unwrap();
    assert!(fixture.invoke().is_err());
    assert!(!fixture.options.output_dir.exists());
}

#[test]
fn dispatched_external_failure_is_unknown_and_diagnostics_are_bounded() {
    let mut fixture = Fixture::new();
    fixture.external(false);
    fixture.provider("exit 7");
    let result = fixture.invoke().unwrap();
    fixture.assert_terminal(&result, "outcome_unknown");
    assert_eq!(result["result"]["failure"]["code"], "PROMPTGEN_FAILED");
    let fixture = Fixture::new();
    fixture.provider("/usr/bin/head -c 262144 /dev/zero; /bin/sleep 10");
    let result = fixture.invoke().unwrap();
    fixture.assert_terminal(&result, "outcome_unknown");
    assert_eq!(
        result["result"]["failure"]["code"],
        "PROMPTGEN_OUTPUT_LIMIT"
    );
    assert!(result["stdout"].as_str().unwrap().len() <= 64 * 1024);
}

#[test]
fn adoption_and_sac_request_verify_hash_without_owner_mutation() {
    let fixture = Fixture::new();
    fixture.compile();
    let response = fixture.invoke().unwrap();
    let result = &response["result"];
    assert_eq!(
        validate_candidate(&fixture.root, &fixture.request["source"], result, None).unwrap()
            ["status"],
        "eligible_for_review"
    );
    assert!(
        validate_candidate(&fixture.root, &json!({"identity":"changed"}), result, None).is_err()
    );
    let options = SceneAssetOptions {
        project_id: "project.test".into(),
        smc_project_root: fixture.root.clone(),
        projection_receipt_path: "payload.json".into(),
        projection_receipt_hash: fixture.request["payload"]["sha256"]
            .as_str()
            .unwrap()
            .into(),
        outputs: vec!["RUNTIME_ASSETS".into()],
        human_final_review: true,
    };
    let request = build_scene_asset_request(&fixture.root, &options).unwrap();
    assert_eq!(request["source"]["kind"], "SMC_NARRATIVE_SCENE_PROJECTION");
    assert_eq!(request["review_policy"]["human_final_review"], true);
    let bytes = fs::read(fixture.root.join("payload.json")).unwrap();
    assert!(!bytes.is_empty());
    assert!(validate_candidate(Path::new("/"), &fixture.request["source"], result, None).is_err());
}

#[test]
fn optional_receipt_must_exist_and_match_its_typed_artifact_reference() {
    let fixture = Fixture::new();
    fixture.compile();
    let mut result = fixture.invoke().unwrap()["result"].clone();
    result["receipt"] = result["artifacts"][0].clone();
    validate_candidate(&fixture.root, &fixture.request["source"], &result, None).unwrap();
    for (field, value) in [
        ("path", json!("missing-receipt.json")),
        ("sha256", json!(format!("sha256:{}", "0".repeat(64)))),
        ("byteLength", json!(0)),
    ] {
        let mut invalid = result.clone();
        invalid["receipt"][field] = value;
        assert!(
            validate_candidate(&fixture.root, &fixture.request["source"], &invalid, None).is_err()
        );
    }
}

#[test]
fn scene_asset_compose_obeys_the_pinned_sac_run_request_contract() {
    let fixture = Fixture::new();
    let mut options = SceneAssetOptions {
        project_id: "project.test".into(),
        smc_project_root: fixture.root.clone(),
        projection_receipt_path: "payload.json".into(),
        projection_receipt_hash: fixture.request["payload"]["sha256"]
            .as_str()
            .unwrap()
            .into(),
        outputs: vec![],
        human_final_review: true,
    };
    let valid = build_scene_asset_request(&fixture.root, &options).unwrap();
    assert_eq!(valid["outputs"], json!(["RUNTIME_ASSETS"]));
    options.project_id = "bad/id".into();
    assert!(build_scene_asset_request(&fixture.root, &options).is_err());
    options.project_id = "project.test".into();
    options.outputs = vec!["NOT_A_SAC_OUTPUT".into()];
    assert!(build_scene_asset_request(&fixture.root, &options).is_err());
    options.outputs = vec!["RUNTIME_ASSETS".into(), "RUNTIME_ASSETS".into()];
    assert!(build_scene_asset_request(&fixture.root, &options).is_err());
    options.outputs = vec!["RUNTIME_ASSETS".into()];
    fs::write(fixture.root.join("payload.json"), b"[]").unwrap();
    options.projection_receipt_hash = hash::sha256_bytes(b"[]");
    assert!(build_scene_asset_request(&fixture.root, &options).is_err());
}

#[test]
fn imagegen_and_edit_receipts_bind_paths_hashes_prompt_and_reference() {
    for edit in [false, true] {
        let mut fixture = Fixture::new();
        fixture.receipt_provider(edit, false);
        let result = fixture.invoke().unwrap();
        fixture.assert_terminal(&result, "succeeded");
        assert_eq!(result["result"]["artifacts"].as_array().unwrap().len(), 2);
        assert_eq!(
            result["result"]["providerReceipt"]["backend"],
            if edit {
                "openai-images"
            } else {
                "codex-imagegen"
            }
        );
        let mut fixture = Fixture::new();
        fixture.receipt_provider(edit, true);
        fixture.assert_terminal(&fixture.invoke().unwrap(), "outcome_unknown");
    }
}

#[test]
fn actual_signal_after_dispatch_preserves_unknown_for_local_and_external_execution() {
    for external in [false, true] {
        let mut fixture = Fixture::new();
        if external {
            fixture.external(false);
        }
        fixture.provider("/usr/bin/touch provider-dispatched\n/bin/sleep 10");
        fs::write(
            &fixture.options.request,
            serde_json::to_vec(&fixture.request).unwrap(),
        )
        .unwrap();
        let mut child = std::process::Command::new(env!("CARGO_BIN_EXE_storyctl"))
            .args([
                "--story-root",
                fixture.root.to_str().unwrap(),
                "run-promptgen",
                "--request",
                fixture.options.request.to_str().unwrap(),
                "--output-dir",
                fixture.options.output_dir.to_str().unwrap(),
                "--promptgen-bin",
                fixture.options.promptgen_bin.to_str().unwrap(),
            ])
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::piped())
            .spawn()
            .unwrap();
        let start = std::time::Instant::now();
        loop {
            let job = fs::read(fixture.options.output_dir.join("capability-job.json"))
                .ok()
                .and_then(|raw| serde_json::from_slice::<Value>(&raw).ok());
            if job.is_some_and(|job| job["status"] == "running")
                && fixture
                    .options
                    .output_dir
                    .join("provider-dispatched")
                    .is_file()
            {
                break;
            }
            if start.elapsed() > std::time::Duration::from_secs(5) {
                let _ = child.kill();
                panic!("provider did not start");
            }
            std::thread::sleep(std::time::Duration::from_millis(10));
        }
        assert!(std::process::Command::new("/bin/kill")
            .args(["-INT", &child.id().to_string()])
            .status()
            .unwrap()
            .success());
        let output = child.wait_with_output().unwrap();
        assert_eq!(output.status.code(), Some(2));
        let result: Value = serde_json::from_slice(&output.stdout).unwrap();
        fixture.assert_terminal(&result, "outcome_unknown");
        assert_eq!(result["job"]["cancelRequested"], true);
    }
}

#[test]
fn local_compile_transport_timeout_is_unknown_but_invalid_receipt_is_failed() {
    let fixture = Fixture::new();
    fixture.provider("/usr/bin/touch provider-dispatched\n/bin/sleep 10");
    let result = fixture.invoke().unwrap();
    assert!(fixture
        .options
        .output_dir
        .join("provider-dispatched")
        .is_file());
    fixture.assert_terminal(&result, "outcome_unknown");
    assert_eq!(result["result"]["failure"]["code"], "PROMPTGEN_TIMEOUT");
    let fixture = Fixture::new();
    fixture.provider("printf '{' > \"$result\"");
    fixture.assert_terminal(&fixture.invoke().unwrap(), "failed");
}

#[test]
fn result_is_durable_before_terminal_job_and_persistence_failure_never_retries() {
    let fixture = Fixture::new();
    fixture.provider("/bin/cat > \"$result\" <<'JSON'\n{\"kind\":\"image\",\"status\":\"valid\",\"prompt\":\"fixture prompt\",\"diagnostics\":[],\"metadata\":{}}\nJSON\n/bin/rm capability-job.json\n/bin/mkdir capability-job.json");
    let error = fixture.invoke().unwrap_err();
    assert_eq!(error.outcome.as_str(), "outcome_unknown");
    assert!(error.message.contains("terminal persistence incomplete"));
    let result: Value = serde_json::from_slice(
        &fs::read(fixture.options.output_dir.join("capability-result.json")).unwrap(),
    )
    .unwrap();
    assert_eq!(result["status"], "succeeded");
    assert!(fixture
        .options
        .output_dir
        .join("capability-job.json")
        .is_dir());
    assert!(fixture.invoke().is_err());
}
