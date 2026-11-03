//! One provider attempt; recovery never dispatches the provider again.
use super::text::{self, string, TextContext};
use crate::{
    adapters::owners::{
        path_string, validate_geultu_submission, validate_narrative_artifact,
        validate_narrative_submission, validate_storyboard_submission,
    },
    contracts::SchemaRegistry,
    effects::{
        files, json,
        process::{invoke, ProcessSpec},
    },
    error::{Error, Result},
};
use serde_json::{json, Value};
use std::{
    collections::BTreeMap,
    path::{Path, PathBuf},
    sync::Arc,
    time::Duration,
};
use story_integration_core::AttemptId;
const REQUEST: &str = "https://story-production.local/contracts/text-writer-request/v1";
const RESPONSE: &str = "https://story-production.local/contracts/text-writer-result/v1";
pub struct RunOptions {
    pub attempt: String,
    pub writer: PathBuf,
    pub arguments: Vec<String>,
    pub timeout: u64,
    pub instruction: String,
    pub storyboard_task: Option<String>,
}
fn display_directory(ctx: &TextContext, attempt: &str) -> Result<PathBuf> {
    AttemptId::new(attempt).map_err(Error::not_applied)?;
    Ok(ctx.workspace.join("attempts").join(attempt))
}
struct AttemptDirectory {
    root: Arc<files::Directory>,
    parent: files::Directory,
    directory: files::Directory,
    attempt: String,
    display: PathBuf,
}
impl AttemptDirectory {
    fn current(&self, ctx: &TextContext) -> Result<()> {
        ctx.ensure_workspace_current()?;
        ctx.project_id()?;
        self.root.verify_child("attempts", &self.parent)?;
        self.parent.verify_child(&self.attempt, &self.directory)
    }
    fn phase(&self, ctx: &TextContext) -> Result<()> {
        self.current(ctx)?;
        if crate::effects::process::cancellation_requested() {
            return Err(Error::unknown(
                "attempt cancellation was observed; inspect existing effects before recovery",
            ));
        }
        Ok(())
    }
}
fn existing_directory(ctx: &TextContext, attempt: &str) -> Result<Option<AttemptDirectory>> {
    let display = display_directory(ctx, attempt)?;
    let root = ctx.workspace_directory()?;
    let Some(parent) = root.child_optional("attempts")? else {
        return Ok(None);
    };
    let Some(directory) = parent.child_optional(attempt)? else {
        return Ok(None);
    };
    let selected = AttemptDirectory {
        root,
        parent,
        directory,
        attempt: attempt.into(),
        display,
    };
    selected.current(ctx)?;
    Ok(Some(selected))
}
fn claim_name(attempt: &str) -> String {
    format!(".story-attempt-{attempt}.claim")
}
fn submit_lock_name(attempt: &str) -> String {
    format!(".story-attempt-{attempt}.submit.lock")
}
fn bind_request(
    ctx: &TextContext,
    req: &Value,
    dir: &files::Directory,
    attempt: &str,
    schemas: &SchemaRegistry,
) -> Result<()> {
    let packet = &req["task"];
    let captured = if let Some(identity) = packet.get("storyProjectId") {
        identity.as_str().map(str::to_owned).ok_or_else(|| {
            Error::unknown("writer request Story project routing identity is invalid")
        })?
    } else {
        let mut identity = None;
        if packet["owner"] == "geultu" {
            for name in ["recovery.json", "result.json"] {
                if !dir.exists(name)? {
                    continue;
                }
                let stored = json::parse_object(&dir.read(name)?)?;
                if stored["attempt"] == attempt && stored["status"] == "candidate_stored" {
                    if let Some(project) = stored["result"]["commit"]["project_id"].as_str() {
                        let provider = response(&dir.read("response.json")?, attempt, schemas)?;
                        if provider["status"] != "succeeded" {
                            return Err(Error::unknown(
                                "legacy commit has no matching successful provider response",
                            ));
                        }
                        validate_geultu_submission(
                            &stored["result"],
                            &json!({
                            "project_id":project,"candidate_id":format!("draft-{attempt}"),
                            "task_id":packet["task_id"],"draft":json::parse_object(string(&provider,"content")?.as_bytes())?}),
                        )?;
                        identity = Some(project.to_owned());
                        break;
                    }
                }
            }
        }
        identity.ok_or_else(|| Error::unknown("legacy writer request has no proven project routing identity; inspect its saved task/response and choose an explicit author submission; provider is not retried"))?
    };
    story_integration_core::ProjectId::new(&captured)
        .map_err(|_| Error::unknown("writer request project routing identity is invalid"))?;
    if captured != ctx.project_id()? {
        return Err(text::project_changed(
            "writer request belongs to another Story project",
        ));
    }
    if matches!(
        packet["owner"].as_str(),
        Some("NarrativeCourier" | "NarrativeCourierStoryboard")
    ) {
        let captured = packet["narrativeProjectId"].as_str().ok_or_else(|| Error::unknown("legacy writer request has no proven narrative project routing identity; inspect saved response without automatic submission"))?;
        story_integration_core::ProjectId::new(captured).map_err(|_| {
            Error::unknown("writer request narrative project routing identity is invalid")
        })?;
        if captured != ctx.nvc_id()? {
            return Err(text::project_changed(
                "writer request belongs to another narrative project",
            ));
        }
    }
    Ok(())
}
fn contracts(ctx: &TextContext) -> Result<SchemaRegistry> {
    SchemaRegistry::new(&ctx.repository.join("integration/contracts"))
}
fn request(value: Value, attempt: &str, schemas: &SchemaRegistry) -> Result<Value> {
    schemas.validate(REQUEST, &value)?;
    if value["attemptId"] != attempt {
        return Err(Error::not_applied("writer request attempt id mismatch"));
    }
    let packet = &value["task"];
    let fields: &[&str] = match string(packet, "owner")? {
        "geultu" => &["task_id"],
        "NarrativeCourier" => &["node", "task_token", "task_text"],
        "NarrativeCourierStoryboard" => &["task_path"],
        _ => return Err(Error::not_applied("unsupported writer owner")),
    };
    if packet["status"] != "agent_task"
        || fields
            .iter()
            .any(|k| packet[*k].as_str().is_none_or(|s| s.trim().is_empty()))
    {
        return Err(Error::not_applied(
            "writer request does not contain supported writing task",
        ));
    }
    Ok(value)
}
fn response(raw: &[u8], attempt: &str, schemas: &SchemaRegistry) -> Result<Value> {
    let value = json::parse_object(raw)?;
    schemas.validate(RESPONSE, &value)?;
    if value["attemptId"] != attempt {
        return Err(Error::not_applied("writer response attempt id mismatch"));
    }
    if value["status"] == "succeeded" {
        if string(&value, "content")?.trim().is_empty() {
            return Err(Error::not_applied(
                "successful writer response content empty",
            ));
        }
    } else if string(&value, "error")?.trim().is_empty() {
        return Err(Error::not_applied("non-success response error empty"));
    }
    Ok(value)
}
fn saved_evidence(
    ctx: &TextContext,
    dir: &files::Directory,
    attempt: &str,
    req: &Value,
    value: &Value,
    schemas: &SchemaRegistry,
) -> Result<()> {
    let provider = response(&dir.read("response.json")?, attempt, schemas)?;
    if provider["status"] != "succeeded" {
        return Err(Error::unknown(
            "saved candidate acknowledgment has no successful provider response",
        ));
    }
    let packet = &req["task"];
    let content = string(&provider, "content")?;
    if value["status"] == "matching_artifact_observed" {
        if packet["owner"] != "NarrativeCourier" {
            return Err(Error::unknown(
                "saved matching artifact belongs to an unsupported writer owner",
            ));
        }
        return validate_narrative_artifact(
            &json!({"item":value["artifact"],"content":content}),
            packet,
            content,
            None,
        );
    }
    match string(packet, "owner")? {
        "geultu" => validate_geultu_submission(
            &value["result"],
            &json!({
            "project_id":ctx.project_id()?,"candidate_id":format!("draft-{attempt}"),
            "task_id":packet["task_id"],"draft":json::parse_object(content.as_bytes())?}),
        ),
        "NarrativeCourier" => validate_narrative_submission(&value["result"], packet),
        "NarrativeCourierStoryboard" => validate_storyboard_submission(
            &value["result"],
            &format!("storyboards/boards/{attempt}.json"),
        ),
        _ => Err(Error::unknown(
            "saved candidate belongs to an unsupported writer owner",
        )),
    }
}
fn saved(
    ctx: &TextContext,
    dir: &files::Directory,
    display: &Path,
    attempt: &str,
    req: &Value,
    schemas: &SchemaRegistry,
) -> Result<Option<Value>> {
    for name in ["recovery.json", "result.json"] {
        if !dir.exists(name)? {
            continue;
        }
        let value = json::parse_object(&dir.read(name)?)?;
        if value["attempt"] != attempt
            || !matches!(
                value["status"].as_str(),
                Some(
                    "candidate_stored"
                        | "matching_artifact_observed"
                        | "failed"
                        | "cancelled"
                        | "outcome_unknown"
                        | "not_started"
                        | "not_applied"
                        | "source_changed"
                        | "source_stale"
                )
            )
        {
            return Err(Error::not_applied(
                "saved attempt identity or status invalid",
            ));
        }
        if matches!(
            value["status"].as_str(),
            Some("candidate_stored" | "matching_artifact_observed")
        ) {
            if let Err(error) = saved_evidence(ctx, dir, attempt, req, &value, schemas) {
                // Preserve the original immutable record. A caller can inspect
                // it, and recover may still use the saved response/owner state.
                return Ok(Some(json!({"status":"outcome_unknown","attempt":attempt,
                    "path":path_string(display)?,"error":format!("saved owner acknowledgment is invalid: {error}"),
                    "nextAction":"inspect saved response and current owner state before recovery"})));
            }
        }
        return Ok(Some(value));
    }
    Ok(None)
}
pub fn run(ctx: &TextContext, options: &RunOptions) -> Result<Value> {
    let display = display_directory(ctx, &options.attempt)?;
    let schemas = contracts(ctx)?;
    if options.writer.as_os_str().is_empty()
        || !(1..=crate::effects::process::MAX_PROCESS_TIMEOUT_SECONDS).contains(&options.timeout)
    {
        return Err(Error::not_applied(format!(
            "writer program must be explicit; timeout must be 1..={} seconds",
            crate::effects::process::MAX_PROCESS_TIMEOUT_SECONDS
        )));
    }
    let writer = std::path::absolute(&options.writer)?;
    let program = path_string(&writer)?;
    ctx.ensure_workspace_current()?;
    if let Some(selected) = existing_directory(ctx, &options.attempt)? {
        if selected.directory.exists("request.json")? {
            let prior = request(
                json::parse_object(&selected.directory.read("request.json")?)?,
                &options.attempt,
                &schemas,
            )?;
            bind_request(ctx, &prior, &selected.directory, &options.attempt, &schemas)?;
            if prior["providerProgram"] != program
                || prior["providerArguments"] != json!(options.arguments)
            {
                return Err(Error::not_applied(
                    "attempt belongs to another writer invocation",
                ));
            }
            if let Some(v) = saved(
                ctx,
                &selected.directory,
                &display,
                &options.attempt,
                &prior,
                &schemas,
            )? {
                selected.current(ctx)?;
                return Ok(v);
            }
        }
        return Ok(
            json!({"status":"outcome_unknown","attempt":options.attempt,"path":path_string(&display)?,"nextAction":"inspect saved request, response and owner state; do not start replacement attempt"}),
        );
    }
    let root = ctx.workspace_directory()?;
    let claim_path = claim_name(&options.attempt);
    let claim = match root.exclusive(&claim_path, options.attempt.as_bytes()) {
        Ok(claim) => claim,
        Err(error) => {
            return Ok(
                json!({"status":"outcome_unknown","attempt":options.attempt,"path":path_string(&display)?,"error":format!("attempt execution claim was not acquired: {error}"),"nextAction":"inspect existing attempt; provider is not retried"}),
            )
        }
    };
    let parent = root.child("attempts", true)?;
    let Some(directory) = parent.reserve_child(&options.attempt)? else {
        return Ok(
            json!({"status":"outcome_unknown","attempt":options.attempt,"nextAction":"inspect existing attempt"}),
        );
    };
    let selected = AttemptDirectory {
        root,
        parent,
        directory,
        attempt: options.attempt.clone(),
        display: display.clone(),
    };
    selected.current(ctx)?;
    let prepared = (|| -> Result<Value> {
        if let Some(task) = &options.storyboard_task {
            if !ctx.source_current()? {
                return Err(Error::not_applied(
                    "writing synopsis differs from STS project",
                ));
            }
            Ok(
                json!({"owner":"NarrativeCourierStoryboard","status":"agent_task","task_path":task,"packet":ctx.board(&["task".into(),"--task".into(),task.clone()])?}),
            )
        } else {
            text::next(ctx, &options.instruction)
        }
    })();
    let mut packet = match prepared {
        Ok(packet) => packet,
        Err(error)
            if error.outcome == story_integration_core::Outcome::NotStarted && !error.spawned =>
        {
            selected.current(ctx)?;
            selected
                .parent
                .remove_empty_child(&options.attempt, &selected.directory)?;
            selected.root.remove_file(&claim_path, &claim)?;
            return Err(error);
        }
        Err(error) => return Err(error),
    };
    if packet["status"] != "agent_task" {
        selected.current(ctx)?;
        selected
            .parent
            .remove_empty_child(&options.attempt, &selected.directory)?;
        selected.root.remove_file(&claim_path, &claim)?;
        return Ok(packet);
    }
    packet["storyProjectId"] = json!(ctx.project_id()?);
    if matches!(
        packet["owner"].as_str(),
        Some("NarrativeCourier" | "NarrativeCourierStoryboard")
    ) {
        packet["narrativeProjectId"] = json!(ctx.nvc_id()?);
    }
    let req = request(
        json!({"schemaVersion":"story.text-writer-request/1","attemptId":options.attempt,"task":packet,"allowedEffect":"return_one_text_candidate","providerProgram":program,"providerArguments":options.arguments}),
        &options.attempt,
        &schemas,
    )?;
    let bytes = json::stable_bytes(&req)?;
    selected.current(ctx)?;
    selected.directory.immutable("request.json", &bytes)?;
    let mut result =
        json!({"status":"outcome_unknown","attempt":options.attempt,"path":path_string(&display)?});
    let mut provider_dispatched = false;
    let execution = (|| -> Result<Value> {
        let mut argv = vec![program];
        argv.extend(options.arguments.clone());
        selected.current(ctx)?;
        let workspace = ctx.workspace_directory()?;
        let execution = invoke(&ProcessSpec {
            argv,
            input: bytes,
            cwd: PathBuf::from(format!("/dev/fd/{}", workspace.raw_fd())),
            timeout: Duration::from_secs(options.timeout),
            env: BTreeMap::new(),
            pass_fds: vec![workspace.raw_fd()],
            max_output_bytes: crate::effects::MAX_BYTES,
        });
        let output = match execution {
            Ok(output) => {
                provider_dispatched = true;
                output
            }
            Err(error) => {
                provider_dispatched = error.spawned;
                return Err(error);
            }
        };
        if !output.status.success() {
            return Err(
                Error::unknown(String::from_utf8_lossy(&output.stderr)).with_stdout(output.stdout)
            );
        }
        selected
            .directory
            .immutable("response.json", &output.stdout)?;
        selected.phase(ctx)?;
        let response = response(&output.stdout, &options.attempt, &schemas)?;
        if response["status"] != "succeeded" {
            result["status"] = response["status"].clone();
            result["error"] = response["error"].clone();
            return Ok(result.clone());
        }
        result["providerStatus"] = json!("succeeded");
        let _flat_lock = selected.root.lock(&submit_lock_name(&options.attempt))?;
        let _lock = selected.directory.submission_lock()?;
        selected.phase(ctx)?;
        if let Some(v) = saved(
            ctx,
            &selected.directory,
            &display,
            &options.attempt,
            &req,
            &schemas,
        )? {
            return Ok(v);
        }
        let stored = text::submit(
            ctx,
            &packet,
            string(&response, "content")?,
            &options.attempt,
        )?;
        selected.phase(ctx)?;
        let submitted = json!({"status":"candidate_stored","attempt":options.attempt,"path":path_string(&display)?,"providerStatus":"succeeded","result":stored,"nextAction":"next"});
        // Keep run/recover serialization through the durable acknowledgment.
        selected
            .directory
            .immutable("result.json", &json::stable_bytes(&submitted)?)?;
        Ok(submitted)
    })();
    match execution {
        Ok(value) => result = value,
        Err(error) => {
            if provider_dispatched && error.outcome == story_integration_core::Outcome::NotStarted {
                result["status"] = json!("outcome_unknown");
            } else if error.outcome == story_integration_core::Outcome::SourceChanged {
                result["status"] = json!("source_changed");
            } else if error.spawned {
                result["status"] = json!(error.outcome.as_str());
            } else if error.outcome == story_integration_core::Outcome::NotStarted {
                result["status"] = json!("not_started");
            }
            result["error"] = json!(error.message);
            result["nextAction"] =
                json!("inspect response.json and current owner state before recovery");
            if !error.stdout.is_empty() && !selected.directory.exists("response.json")? {
                selected
                    .directory
                    .immutable("response.json", &error.stdout)?;
            }
        }
    }
    selected.directory.immutable("result.json", &json::stable_bytes(&result)?).map_err(|error| {
        Error::unknown(format!("attempt result persistence was not confirmed; inspect {} and saved response before recovery: {error}",display.join("result.json").display()))
    })?;
    Ok(result)
}
pub fn recover(ctx: &TextContext, attempt: &str) -> Result<Value> {
    let selected = existing_directory(ctx, attempt)?
        .ok_or_else(|| Error::not_applied("attempt directory is unavailable"))?;
    let schemas = contracts(ctx)?;
    // Before request publication a run may still return a non-writing packet and
    // remove its empty reservation. Recovery must not poison that reservation.
    let req = request(
        json::parse_object(&selected.directory.read("request.json")?)?,
        attempt,
        &schemas,
    )?;
    bind_request(ctx, &req, &selected.directory, attempt, &schemas)?;
    let _flat_lock = selected.root.lock(&submit_lock_name(attempt))?;
    let _lock = selected.directory.submission_lock()?;
    selected.phase(ctx)?;
    if let Some(value) = saved(
        ctx,
        &selected.directory,
        &selected.display,
        attempt,
        &req,
        &schemas,
    )? {
        if matches!(
            value["status"].as_str(),
            Some("candidate_stored" | "matching_artifact_observed" | "failed" | "cancelled")
        ) || !selected.directory.exists("response.json")?
        {
            selected.phase(ctx)?;
            return Ok(value);
        }
    }
    let response = response(
        &selected.directory.read("response.json")?,
        attempt,
        &schemas,
    )?;
    if response["status"] != "succeeded" {
        return Err(Error::not_applied(
            "only existing successful response can be recovered; provider is not retried",
        ));
    }
    let recovery = (|| -> Result<Value> {
        selected.phase(ctx)?;
        let packet = &req["task"];
        if packet["owner"] == "NarrativeCourier" {
            if !ctx.source_current()? {
                return Err(Error::not_applied(
                    "writing synopsis changed; saved STS response not current",
                ));
            }
            if ctx.narrative_call("agent-task", &[])?["task_token"] != packet["task_token"] {
                let observed = ctx.narrative_call("artifact", &[string(packet, "node")?.into()])?;
                if observed["content"] != response["content"] {
                    return Ok(
                        json!({"status":"outcome_unknown","attempt":attempt,"nextAction":"STS token changed and artifact differs; inspect saved response"}),
                    );
                }
                validate_narrative_artifact(
                    &observed,
                    packet,
                    string(&response, "content")?,
                    None,
                )?;
                selected.phase(ctx)?;
                let result = json!({"status":"matching_artifact_observed","attempt":attempt,"artifact":observed["item"]});
                selected.directory.immutable("recovery.json", &json::stable_bytes(&result)?).map_err(|error| Error::unknown(format!("matching owner artifact observed but recovery receipt persistence failed; inspect {}: {error}",selected.display.display())))?;
                return Ok(result);
            }
        }
        let result = json!({"status":"candidate_stored","attempt":attempt,"result":text::submit(ctx,packet,string(&response,"content")?,attempt)?});
        selected.phase(ctx)?;
        selected.directory.immutable("recovery.json", &json::stable_bytes(&result)?).map_err(|error| Error::unknown(format!("owner submission returned but recovery receipt persistence failed; inspect {}: {error}",selected.display.display())))?;
        Ok(result)
    })();
    recovery.map_err(|error| {
        if error.outcome == story_integration_core::Outcome::NotStarted {
            Error::unknown(format!("saved provider success exists but recovery could not observe or submit to its owner: {error}"))
        } else {
            error
        }
    })
}
