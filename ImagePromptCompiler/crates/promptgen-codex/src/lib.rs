#![forbid(unsafe_code)]

mod artifact;
mod candidate;
mod event_stream;
mod fidelity;
mod fidelity_gate;
mod instruction;
mod model;
mod preflight;
mod process;
use artifact::discovery::discover_artifact;
use artifact::publication::{lexical_absolute, validate_copy_publish};
use candidate::{prepare_candidate, sha256_file};
use fidelity_gate::{run_fidelity_validation, run_repair_generation};
use instruction::{PromptIdentity, build_instruction};
pub use model::{
    CodexExecutionError, CodexExecutionReceipt, CodexRunConfig, FidelityCheck, SubjectCountCheck,
    TextFidelityCheck, VisibleInstance,
};
use preflight::{ensure_runtime_paths, validate_backend_contract, validate_compilation};
use process::{
    OwnedChild, WorkingDirectory, create_new_file, exit_code, isolated_codex_exec,
    probe_codex_version, read_log_snippet,
};
use promptgen_core::image_artifact::sha256::digest_hex;

use std::io;
use std::path::Path;
use std::process::Stdio;
use std::sync::atomic::AtomicU64;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use promptgen_core::compile_image_prompt;
use promptgen_core::diagnostic::CompilationOutcome;
use promptgen_core::image::ImagePromptRequest;

static NEXT_NONCE: AtomicU64 = AtomicU64::new(1);

pub fn execute_image_generation(
    compilation: &CompilationOutcome,
    request: &ImagePromptRequest,
    config: &CodexRunConfig,
) -> Result<CodexExecutionReceipt, CodexExecutionError> {
    validate_compilation(compilation)?;
    let rebound = compile_image_prompt(request);
    if !rebound.is_valid() || rebound.prompt != compilation.prompt {
        return Err(CodexExecutionError::new(
            "CODEX_COMPILATION_MISMATCH",
            "compiled prompt does not match the supplied typed image request",
        ));
    }
    let prompt = compilation.prompt.as_deref().ok_or_else(|| {
        CodexExecutionError::new("CODEX_MISSING_PROMPT", "valid compilation has no prompt")
    })?;
    validate_backend_contract(request)?;
    if prompt.trim().is_empty() {
        return Err(CodexExecutionError::new(
            "CODEX_EMPTY_PROMPT",
            "compiled image prompt must not be blank",
        ));
    }
    let compiled_prompt_sha256 = digest_hex(prompt.as_bytes());
    let compiled_prompt_chars = prompt.chars().count() as u64;
    ensure_runtime_paths(config)?;

    let working = WorkingDirectory::create()?;
    // A sub-second image execution timeout is useful for tests, but it is not
    // enough headroom to spawn even a trivial shell reliably on a busy host.
    // Keep the version probe bounded while giving it a small independent floor.
    let probe_timeout = config
        .timeout
        .max(Duration::from_secs(1))
        .min(Duration::from_secs(15));
    let codex_version = probe_codex_version(&config.codex_binary, &working.path, probe_timeout)?;
    let stdout_path = working.path.join("events.jsonl");
    let stderr_path = working.path.join("stderr.log");
    let stdout_file = create_new_file(&stdout_path, "create Codex event log")?;
    let stderr_file = create_new_file(&stderr_path, "create Codex error log")?;
    let instruction = build_instruction(
        prompt,
        &request.output,
        PromptIdentity {
            sha256: &compiled_prompt_sha256,
            chars: compiled_prompt_chars,
        },
    );
    let started_at = SystemTime::now();
    let started_unix_ms = millis(started_at);

    let mut command = isolated_codex_exec(&config.codex_binary);
    let mut child = OwnedChild::spawn(
        command
            .arg("--json")
            .arg("--enable")
            .arg("image_generation")
            .arg(instruction)
            .env("CODEX_HOME", &config.codex_home)
            .current_dir(&working.path)
            .stdin(Stdio::null())
            .stdout(Stdio::from(stdout_file))
            .stderr(Stdio::from(stderr_file)),
        "CODEX_SPAWN",
        &config.codex_binary,
    )?;

    let status = child.wait(config.timeout)?;
    let stdout = read_log_snippet(&stdout_path);
    let stderr = read_log_snippet(&stderr_path);
    if !status.success() {
        return Err(CodexExecutionError::new(
            "CODEX_PROCESS_FAILED",
            format!(
                "exit_code={}; stdout={stdout:?}; stderr={stderr:?}",
                exit_code(status)
            ),
        ));
    }

    let turn = event_stream::parse_and_validate(
        &stdout_path,
        Some(&config.codex_home.join("skills/.system/imagegen/SKILL.md")),
    )?;
    let (mut thread_id, mut event_contract) = (turn.thread_id, turn.event_contract);
    let (mut call_id, mut source_artifact) = discover_artifact(
        &config.generated_root(),
        &thread_id,
        started_at,
        config.artifact_wait_timeout,
    )?;
    let mut final_status = status;
    let mut fidelity_checks = Vec::new();
    let mut final_candidate = None;
    for attempt in 1..=config.max_fidelity_attempts {
        let candidate = prepare_candidate(
            &source_artifact,
            (request.output.width, request.output.height),
            &working.path,
            attempt,
        )?;
        let candidate_sha256 = sha256_file(&candidate.path)?;
        let check = run_fidelity_validation(
            prompt,
            request,
            &candidate.path,
            candidate_sha256,
            attempt,
            &working.path,
            config,
        )?;
        let passed = check.pass;
        let correction = check.repair_instruction.clone();
        fidelity_checks.push(check);
        if passed {
            final_candidate = Some(candidate);
            break;
        }
        if attempt == config.max_fidelity_attempts {
            return Err(CodexExecutionError::new(
                "CODEX_FIDELITY_FAILED",
                format!(
                    "no candidate passed the strict image contract after {} attempt(s): {}",
                    fidelity_checks.len(),
                    fidelity_checks
                        .last()
                        .map(|check| check.summary.as_str())
                        .unwrap_or("unspecified fidelity failure")
                ),
            ));
        }
        if correction.trim().is_empty() {
            return Err(CodexExecutionError::new(
                "CODEX_FIDELITY_RESULT",
                "failed fidelity result did not provide one repair instruction",
            ));
        }
        let repaired = run_repair_generation(
            prompt,
            &correction,
            &candidate.path,
            attempt + 1,
            &working.path,
            config,
        )?;
        thread_id = repaired.turn.thread_id;
        event_contract = repaired.turn.event_contract;
        call_id = repaired.call_id;
        source_artifact = repaired.source_artifact;
        final_status = repaired.status;
    }
    let final_candidate = final_candidate.ok_or_else(|| {
        CodexExecutionError::new(
            "CODEX_FIDELITY_FAILED",
            "no generated candidate reached publication",
        )
    })?;
    let (sha256, info) = validate_copy_publish(
        &final_candidate.path,
        &config.output_path,
        config.overwrite,
        (request.output.width, request.output.height),
    )?;
    let finished_unix_ms = millis(SystemTime::now());

    Ok(CodexExecutionReceipt {
        backend: "codex-imagegen".to_owned(),
        codex_version,
        event_contract,
        thread_id,
        image_call_id: call_id,
        source_artifact,
        output_artifact: lexical_absolute(&config.output_path).map_err(|error| {
            io_error("resolve output path", &config.output_path, error)
        })?,
        sha256,
        width: info.width,
        height: info.height,
        source_width: final_candidate.source_width,
        source_height: final_candidate.source_height,
        dimensions_normalized: final_candidate.dimensions_normalized,
        started_unix_ms,
        finished_unix_ms,
        process_exit_code: exit_code(final_status),
        compiled_prompt_sha256,
        compiled_prompt_chars,
        fidelity_checks,
        control_notes: vec![
            format!(
                "the turn completed {} allowlisted read(s) of the exact built-in image skill; every other shell command, file change, MCP call, and web tool remains rejected",
                turn.image_skill_reads
            ),
            "this adapter is verified for text-to-image generation only and rejects unbound reference contracts".to_owned(),
            "artifact identity is enforced by the exact thread-owned directory".to_owned(),
            if final_candidate.dimensions_normalized {
                format!(
                    "the provider returned {}x{} at the exact requested aspect ratio; the opaque PNG was downscaled locally to {}x{} before fidelity validation and publication",
                    final_candidate.source_width,
                    final_candidate.source_height,
                    info.width,
                    info.height
                )
            } else {
                "requested dimensions are verified against the fully inflated PNG".to_owned()
            },
            "the submitted compiled-prompt SHA-256 and character count are recorded for transport provenance".to_owned(),
            format!(
                "every reported subject center must remain inside its typed placement region, with a {}-percentage-point visual measurement tolerance",
                fidelity::PLACEMENT_MEASUREMENT_TOLERANCE_PERCENT
            ),
            "quality and opaque-background intent are prompt-level controls and are not inferable from PNG bytes".to_owned(),
            format!(
                "the published candidate passed a strict visual gate after at most {} generated candidate(s); failed candidates were never published",
                config.max_fidelity_attempts
            ),
            "the validated copy is published without using a global newest-file heuristic".to_owned(),
        ],
    })
}

fn io_error(operation: &'static str, path: &Path, error: io::Error) -> CodexExecutionError {
    CodexExecutionError::new(
        "CODEX_IO",
        format!(
            "operation={operation}, path={}, error={error}",
            path.display()
        ),
    )
}

fn millis(time: SystemTime) -> u64 {
    time.duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis()
        .try_into()
        .unwrap_or(u64::MAX)
}

#[cfg(test)]
#[path = "tests/lib.rs"]
mod tests;
