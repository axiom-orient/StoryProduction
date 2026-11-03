//! Story exchange envelopes and review eligibility. No owner approval is granted here.
use crate::effects::{files, hash};
use crate::error::{Error, Result};
use crate::roots::{CapabilityRoot, StoryResourceRoot};
use chrono::{SecondsFormat, Utc};
use serde_json::{json, Value};
use std::path::{Component, Path, PathBuf};
use story_integration_core::{CapabilityState, JobStatus, Sha256};

pub(crate) const REQUEST_SCHEMA: &str =
    "https://story-production.local/contracts/capability-request/v1";
pub(crate) const JOB_SCHEMA: &str = "https://story-production.local/contracts/capability-job/v1";
pub(crate) const RESULT_SCHEMA: &str =
    "https://story-production.local/contracts/capability-result/v1";

pub(crate) fn now() -> String {
    Utc::now().to_rfc3339_opts(SecondsFormat::Micros, true)
}

pub(crate) fn safe_relative(root: &Path, relative: &str) -> Result<PathBuf> {
    if relative.is_empty()
        || relative.contains(['\\', ':'])
        || !Path::new(relative)
            .components()
            .all(|c| matches!(c, Component::Normal(_)))
        || relative
            .split('/')
            .any(|part| part.is_empty() || part == "." || part == "..")
    {
        return Err(Error::not_applied(
            "artifact path must be a safe relative path",
        ));
    }
    files::non_symlink_path(&root.join(relative))
}

pub(crate) fn verify_artifact(root: &Path, value: &Value) -> Result<PathBuf> {
    let relative = value["path"]
        .as_str()
        .ok_or_else(|| Error::not_applied("artifact path is missing"))?;
    let expected_hash = value["sha256"]
        .as_str()
        .ok_or_else(|| Error::not_applied("artifact hash is missing"))?;
    let expected_length = value["byteLength"]
        .as_u64()
        .ok_or_else(|| Error::not_applied("artifact byteLength is invalid"))?;
    let path = safe_relative(root, relative)?;
    let metadata =
        std::fs::symlink_metadata(&path).map_err(|e| Error::not_applied(e.to_string()))?;
    if Sha256::new(expected_hash).is_err()
        || !metadata.is_file()
        || metadata.len() != expected_length
        || hash::sha256_file(&path)? != expected_hash
    {
        return Err(Error::not_applied(format!(
            "artifact hash/length/type mismatch: {relative}"
        )));
    }
    Ok(path)
}

pub fn validate_candidate(
    root: &Path,
    current_source: &Value,
    result: &Value,
    expected_request_hash: Option<&str>,
) -> Result<Value> {
    let resource_root = StoryResourceRoot::select(root)?;
    let capability_root = resource_root.capability_root(None)?;
    validate_candidate_in_roots(
        &resource_root,
        &capability_root,
        current_source,
        result,
        expected_request_hash,
    )
}

pub(crate) fn validate_candidate_in_roots(
    resource_root: &StoryResourceRoot,
    capability_root: &CapabilityRoot,
    current_source: &Value,
    result: &Value,
    expected_request_hash: Option<&str>,
) -> Result<Value> {
    crate::contracts::SchemaRegistry::new(&resource_root.path().join("integration/contracts"))?
        .validate(RESULT_SCHEMA, result)?;
    if result["status"] != "succeeded" || result["candidateOnly"] != true {
        return Err(Error::not_applied(
            "only succeeded candidateOnly results are eligible for review",
        ));
    }
    if !current_source.is_object() || result.get("source") != Some(current_source) {
        return Err(Error::not_applied(
            "candidate source is stale or does not match current authority",
        ));
    }
    let request_hash = result["requestHash"]
        .as_str()
        .ok_or_else(|| Error::not_applied("candidate requestHash is invalid"))?;
    if Sha256::new(request_hash).is_err()
        || expected_request_hash.is_some_and(|h| h != request_hash)
    {
        return Err(Error::not_applied(
            "candidate requestHash does not match the requested execution",
        ));
    }
    let artifacts = result["artifacts"]
        .as_array()
        .filter(|v| !v.is_empty())
        .ok_or_else(|| Error::not_applied("candidate must contain at least one artifact"))?;
    let verified = artifacts
        .iter()
        .map(|a| {
            verify_artifact(capability_root.path(), a).map(|p| p.to_string_lossy().into_owned())
        })
        .collect::<Result<Vec<_>>>()?;
    if let Some(receipt) = result.get("receipt") {
        verify_artifact(capability_root.path(), receipt)?;
    }
    Ok(json!({"status":"eligible_for_review", "candidateOnly":true, "artifacts":verified}))
}

pub struct SceneAssetOptions {
    pub project_id: String,
    pub smc_project_root: PathBuf,
    pub projection_receipt_path: String,
    pub projection_receipt_hash: String,
    pub outputs: Vec<String>,
    pub human_final_review: bool,
}

pub fn build_scene_asset_request(story_root: &Path, options: &SceneAssetOptions) -> Result<Value> {
    if Sha256::new(&options.projection_receipt_hash).is_err() {
        return Err(Error::not_applied(
            "projection_receipt_hash must be sha256:<64 lowercase hex>",
        ));
    }
    if options.project_id.is_empty() || options.outputs.iter().any(String::is_empty) {
        return Err(Error::not_applied(
            "project_id and outputs must be non-empty",
        ));
    }
    let root = files::non_symlink_path(&options.smc_project_root)?;
    if !root.is_dir() {
        return Err(Error::not_applied(
            "SMC project root must be an existing directory",
        ));
    }
    let receipt = safe_relative(&root, &options.projection_receipt_path)?;
    let raw = files::read_bytes(&receipt)?;
    crate::effects::json::parse_object(&raw)?;
    if hash::sha256_bytes(&raw) != options.projection_receipt_hash {
        return Err(Error::not_applied(
            "projection receipt bytes do not match projection_receipt_hash",
        ));
    }
    let outputs = if options.outputs.is_empty() {
        vec!["RUNTIME_ASSETS".to_owned()]
    } else {
        options.outputs.clone()
    };
    let request = json!({"schema_version":1, "project_id":options.project_id,
        "goal":"release scene assets for the exact SMC narrative projection",
        "source":{"kind":"SMC_NARRATIVE_SCENE_PROJECTION", "path":options.projection_receipt_path,
            "project_root":root, "sha256":&options.projection_receipt_hash[7..]},
        "outputs":outputs,
        "review_policy":{"codex_visual_review":true,"human_final_review":options.human_final_review}});
    crate::contracts::SchemaRegistry::new(&story_root.join("integration/contracts"))?.validate(
        "https://scene-asset-compiler.local/run-request.schema.json",
        &request,
    )?;
    Ok(request)
}

pub(crate) fn create_job(request: &Value, request_hash: &str) -> Result<Value> {
    if Sha256::new(request_hash).is_err() {
        return Err(Error::not_applied("request hash is invalid"));
    }
    let initial = CapabilityState::queued();
    let at = now();
    Ok(
        json!({"schemaVersion":"story.capability-job/1", "jobId":format!("job.{}",&request_hash[7..31]),
        "requestId":request["requestId"],"requestHash":request_hash,"capability":request["capability"],
        "source":request["source"],"provider":request["provider"],"status":initial.status().as_str(),"stage":"queued",
        "attempt":initial.attempt(),"cancelRequested":initial.cancel_requested(),"createdAt":at,"updatedAt":at}),
    )
}

fn job_status(status: &str) -> Result<JobStatus> {
    match status {
        "queued" => Ok(JobStatus::Queued),
        "running" => Ok(JobStatus::Running),
        "succeeded" => Ok(JobStatus::Succeeded),
        "failed" => Ok(JobStatus::Failed),
        "cancelled" => Ok(JobStatus::Cancelled),
        "outcome_unknown" => Ok(JobStatus::Unknown),
        _ => Err(Error::not_applied("invalid capability job status")),
    }
}

fn state(job: &Value) -> Result<CapabilityState> {
    if job.get("resultId").is_some() || job.get("failure").is_some() {
        return Err(Error::not_applied(
            "active job cannot carry terminal-only fields",
        ));
    }
    let status = job_status(job["status"].as_str().unwrap_or(""))?;
    let attempt = job["attempt"]
        .as_u64()
        .and_then(|v| u32::try_from(v).ok())
        .ok_or_else(|| Error::not_applied("job attempt has an invalid type"))?;
    let cancel_requested = job["cancelRequested"]
        .as_bool()
        .ok_or_else(|| Error::not_applied("job cancelRequested has an invalid type"))?;
    CapabilityState::from_active_snapshot(status, attempt, cancel_requested)
        .map_err(Error::not_applied)
}

pub(crate) fn request_cancellation(job: &Value) -> Result<Value> {
    let next = state(job)?
        .request_cancellation()
        .map_err(Error::not_applied)?;
    let mut value = job.clone();
    value["cancelRequested"] = json!(next.cancel_requested());
    value["updatedAt"] = json!(now());
    Ok(value)
}

pub(crate) fn transition_job(
    job: &Value,
    status: &str,
    stage: &str,
    result_id: Option<&str>,
    failure: Option<&Value>,
) -> Result<Value> {
    if stage.trim().is_empty()
        || stage.chars().count() > 160
        || result_id.is_some_and(|s| s.is_empty())
    {
        return Err(Error::not_applied(
            "invalid capability job stage/result identity",
        ));
    }
    let next = state(job)?
        .transition(job_status(status)?, result_id.is_some(), failure.is_some())
        .map_err(Error::not_applied)?;
    let mut value = job.clone();
    value["status"] = json!(status);
    value["stage"] = json!(stage);
    value["attempt"] = json!(next.attempt());
    value["cancelRequested"] = json!(next.cancel_requested());
    value["updatedAt"] = json!(now());
    if let Some(id) = result_id {
        value["resultId"] = json!(id);
    }
    if let Some(failure) = failure {
        value["failure"] = failure.clone();
    }
    Ok(value)
}

pub(crate) fn terminal_result(
    request: &Value,
    request_hash: &str,
    status: &str,
    failure: Option<Value>,
) -> Value {
    let mut value = json!({"schemaVersion":"story.capability-result/1", "resultId":format!("result.{}",&request_hash[7..31]),
        "requestId":request["requestId"],"requestHash":request_hash,"capability":request["capability"],
        "source":request["source"],"provider":request["provider"],"status":status,
        "artifacts":[],"candidateOnly":true,"createdAt":now()});
    if let Some(failure) = failure {
        value["failure"] = failure;
    }
    value
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn cancellation_never_becomes_success() {
        let request = json!({"requestId":"request.test","capability":"image.prompt.compile","source":{},"provider":{}});
        let job = create_job(&request, &format!("sha256:{}", "a".repeat(64))).unwrap();
        assert_eq!(job["status"], "queued");
        assert_eq!(job["attempt"], 0);
        assert_eq!(job["cancelRequested"], false);
        let job = transition_job(&job, "running", "run", None, None).unwrap();
        let job = request_cancellation(&job).unwrap();
        assert!(transition_job(&job, "succeeded", "done", Some("result.test"), None).is_err());
        assert_eq!(
            transition_job(&job, "outcome_unknown", "cancelled", None, None).unwrap()
                ["cancelRequested"],
            true
        );
    }

    #[test]
    fn active_snapshot_parser_rejects_invalid_attempts_and_terminal_states() {
        for (status, attempt) in [("queued", 1), ("running", 0)] {
            let job = json!({"status":status,"attempt":attempt,"cancelRequested":false});
            assert!(state(&job).is_err());
        }
        for status in ["succeeded", "failed", "cancelled", "outcome_unknown"] {
            let job = json!({"status":status,"attempt":0,"cancelRequested":false});
            assert!(state(&job).is_err());
        }

        let queued = json!({"status":"queued","attempt":0,"cancelRequested":true});
        assert!(state(&queued).unwrap().cancel_requested());
        let running = json!({"status":"running","attempt":1,"cancelRequested":false});
        let state = state(&running).unwrap();
        assert_eq!(state.status(), JobStatus::Running);
        assert_eq!(state.attempt(), 1);
    }

    #[test]
    fn transition_job_preserves_pre_dispatch_terminal_attempt() {
        let request = json!({"requestId":"request.test","capability":"image.prompt.compile","source":{},"provider":{}});
        let job = create_job(&request, &format!("sha256:{}", "b".repeat(64))).unwrap();
        let failed = transition_job(
            &job,
            "failed",
            "preflight",
            None,
            Some(&json!({"code":"invalid_request"})),
        )
        .unwrap();
        assert_eq!(failed["attempt"], 0);
        assert_eq!(failed["status"], "failed");
        assert!(state(&failed).is_err());
    }

    #[test]
    fn source_match_does_not_grant_approval() {
        let source = json!({"identity":"source.test"});
        let result = json!({"status":"succeeded","candidateOnly":true,"source":source,
            "requestHash":format!("sha256:{}","a".repeat(64)),"artifacts":[]});
        assert!(validate_candidate(Path::new("/"), &source, &result, None).is_err());
        let stale = json!({"identity":"source.changed"});
        assert!(validate_candidate(Path::new("/"), &stale, &result, None).is_err());
    }
}
