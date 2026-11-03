//! Story source observation around immutable producer packets and receipts.
use super::text::{string, TextContext};
use crate::runtime::RootMode;
use crate::{
    adapters::owners::{observed_owner, path_string},
    contracts::SchemaRegistry,
    effects::{files, json, process::ProcessSpec},
    error::{Error, Result},
    roots::StoryResourceRoot,
};
use serde_json::{json, Value};
use std::{
    collections::BTreeMap,
    os::unix::fs::PermissionsExt,
    path::{Path, PathBuf},
    time::Duration,
};

const PACKET_SCHEMA: &str = "https://story-production.local/contracts/animatic-packet-v1";
const RESULT_SCHEMA: &str = "https://story-production.local/contracts/animatic-result-v1";
const ASSET_REVIEW_SCHEMA: &str =
    "https://story-production.local/contracts/animatic-assets-review-v1";

fn producer_error(value: &Value, message: &str) -> Error {
    Error::unknown(message).with_stdout(json::stable_bytes(value).unwrap_or_default())
}

fn executable(repository: &Path, selected: &Path) -> Result<PathBuf> {
    let absolute = if selected.is_absolute() {
        selected.to_owned()
    } else {
        repository.join(selected)
    };
    let canonical = absolute
        .canonicalize()
        .map_err(|_| Error::not_started("selected animatic producer executable is unavailable"))?;
    if !canonical
        .metadata()
        .is_ok_and(|metadata| metadata.is_file() && metadata.permissions().mode() & 0o111 != 0)
    {
        return Err(Error::not_started(
            "selected animatic producer must be a regular executable file",
        ));
    }
    Ok(canonical)
}

fn producer_root_changed(value: &Value) -> Error {
    producer_error(value, "animatic asset root identity changed after producer dispatch; inspect the original selected root")
}

fn validate_smc(value: &Value, attempt: &str, schemas: &SchemaRegistry) -> Result<()> {
    if value.get("attemptId").is_some_and(|id| id != attempt) {
        return Err(producer_error(
            value,
            "SMC returned another attempt identity",
        ));
    }
    if value["schemaVersion"] == "smc.animatic-result/1" {
        return schemas.validate(RESULT_SCHEMA, value).map_err(|_| {
            producer_error(value, "SMC returned a malformed public terminal receipt")
        });
    }
    let keys = value
        .as_object()
        .ok_or_else(|| producer_error(value, "SMC response is not an object"))?;
    let detail = value["detail"]
        .as_str()
        .is_some_and(|v| !v.is_empty() && v.len() <= 16384);
    let hash = value["requestHash"]
        .as_str()
        .is_some_and(crate::effects::hash::is_sha256);
    let valid = match value["status"].as_str() {
        Some("not_started") => {
            keys.keys()
                .all(|key| ["status", "attemptId", "detail"].contains(&key.as_str()))
                && value["attemptId"] == attempt
                && detail
        }
        Some("running" | "outcome_unknown") => {
            value["attemptId"] == attempt
                && detail
                && keys.keys().all(|key| {
                    ["status", "attemptId", "requestHash", "detail"].contains(&key.as_str())
                })
                && (hash
                    || (value["status"] == "outcome_unknown" && !keys.contains_key("requestHash")))
        }
        _ => false,
    };
    if !valid {
        return Err(producer_error(
            value,
            "SMC did not confirm its public attempt observation",
        ));
    }
    Ok(())
}

fn validate_packet(value: &Value, schemas: &SchemaRegistry) -> Result<()> {
    schemas
        .validate(PACKET_SCHEMA, value)
        .map_err(|_| producer_error(value, "NVC returned a malformed animatic packet"))?;
    if value["review"]["decision"] != "approved"
        || value["review"]["planHash"] != value["plan"]["contentHash"]
        || value["review"]["source"] != value["plan"]["source"]
    {
        return Err(producer_error(
            value,
            "NVC packet does not confirm its public approved plan binding",
        ));
    }
    Ok(())
}

fn owner_option<'a>(args: &'a [String], key: &str) -> Option<&'a str> {
    let flag = format!("--{key}");
    args.iter().enumerate().find_map(|(index, argument)| {
        argument.strip_prefix(&format!("{flag}=")).or_else(|| {
            (argument == &flag)
                .then(|| args.get(index + 1).map(String::as_str))
                .flatten()
        })
    })
}

fn validate_assets(value: &Value, args: &[String], schemas: &SchemaRegistry) -> Result<()> {
    if matches!(
        value["status"].as_str(),
        Some(
            "failed"
                | "cancelled"
                | "outcome_unknown"
                | "not_started"
                | "not_applied"
                | "source_stale"
        )
    ) {
        return Ok(());
    }
    let valid = match args.first().map(String::as_str) {
        Some("submit") => {
            value["status"] == "candidate"
                && value["path"]
                    .as_str()
                    .is_some_and(|path| Some(path) == owner_option(args, "output"))
                && value["contentHash"]
                    .as_str()
                    .is_some_and(crate::effects::hash::is_sha256)
        }
        Some("inspect") => {
            value["status"] == "inspected"
                && ["packHash", "packetHash"].iter().all(|key| {
                    value[key]
                        .as_str()
                        .is_some_and(crate::effects::hash::is_sha256)
                })
        }
        Some("review") => {
            schemas.validate(ASSET_REVIEW_SCHEMA, value).is_ok()
                && value["decision"]
                    .as_str()
                    .is_some_and(|decision| Some(decision) == owner_option(args, "decision"))
        }
        _ => false,
    };
    if !valid {
        return Err(producer_error(
            value,
            "SAC did not confirm the requested public asset operation",
        ));
    }
    Ok(())
}
fn take(args: &mut Vec<String>, key: &str) -> Result<Option<String>> {
    let flag = format!("--{key}");
    let inline = format!("{flag}=");
    let Some(i) = args
        .iter()
        .position(|v| v == &flag || v.starts_with(&inline))
    else {
        return Ok(None);
    };
    let raw = args.remove(i);
    let value = if let Some(value) = raw.strip_prefix(&inline) {
        value.to_owned()
    } else {
        if args
            .get(i)
            .is_none_or(|value| value.starts_with("--") || value == "-h")
        {
            return Err(Error::not_applied(format!("missing --{key} value")));
        }
        args.remove(i)
    };
    if args.iter().any(|v| v == &flag || v.starts_with(&inline)) {
        return Err(Error::not_applied(format!("duplicate --{key}")));
    }
    Ok(Some(value))
}
fn required(args: &mut Vec<String>, key: &str) -> Result<String> {
    take(args, key)?
        .filter(|v| !v.is_empty())
        .ok_or_else(|| Error::not_applied(format!("required --{key}")))
}
pub fn observe(result: Value, status: &str, detail: &str) -> Result<Value> {
    if result["status"] != "candidate" || !["current", "changed", "unobserved"].contains(&status) {
        return Err(Error::not_applied(
            "only observed render candidate can receive source observation",
        ));
    }
    let mut value = json!({"schemaVersion":"story.animatic-observation/1","status":match status{"current"=>"candidate","changed"=>"source_stale",_=>"outcome_unknown"},"sourceStatus":status,"render":result});
    if status != "current" {
        let bounded = if detail.chars().count() > 16384 {
            format!(
                "{}\n[Observation detail truncated; inspect the owner.]",
                detail.chars().take(16000).collect::<String>()
            )
        } else {
            detail.to_owned()
        };
        value["detail"] = json!(bounded);
    }
    Ok(value)
}
pub fn run(ctx: &TextContext, arguments: &[String]) -> Result<Value> {
    run_inner(ctx, arguments, None)
}

pub(crate) fn run_with_resource_root(
    ctx: &TextContext,
    arguments: &[String],
    resource_root: &StoryResourceRoot,
) -> Result<Value> {
    run_inner(ctx, arguments, Some(resource_root))
}

fn output_options(args: &[String]) -> Result<Vec<String>> {
    let flag = "--output";
    let inline = "--output=";
    let mut values = Vec::new();
    let mut index = 0;
    while index < args.len() {
        let argument = &args[index];
        if let Some(value) = argument.strip_prefix(inline) {
            values.push(value.to_owned());
        } else if argument == flag {
            let value = args
                .get(index + 1)
                .filter(|value| !value.starts_with("--") && value.as_str() != "-h")
                .ok_or_else(|| Error::not_applied("missing --output value"))?;
            values.push(value.clone());
            index += 1;
        }
        index += 1;
    }
    Ok(values)
}

fn run_inner(
    ctx: &TextContext,
    arguments: &[String],
    resource_root: Option<&StoryResourceRoot>,
) -> Result<Value> {
    let mut args = arguments.to_vec();
    if args.is_empty() {
        return Err(Error::not_applied("animatic operation required"));
    }
    let operation = args.remove(0);
    if operation == "direction" {
        if !ctx.source_current()? {
            return Err(Error::not_applied(
                "writing synopsis changed before direction",
            ));
        }
        return ctx.narrative.tool("animatic", &args, &ctx.nvc_root()?);
    }
    let selected_asset_root = PathBuf::from(required(&mut args, "asset-root")?);
    let asset_root = match resource_root {
        Some(resource_root) => resource_root.writable_path(&selected_asset_root)?,
        None => selected_asset_root,
    };
    if let Some(resource_root) = resource_root.filter(|root| root.mode() == RootMode::Installed) {
        for output in output_options(&args)? {
            let output = PathBuf::from(output);
            let output = if output.is_absolute() {
                output
            } else {
                asset_root.join(output)
            };
            resource_root.writable_path(&output)?;
        }
    }
    let root = files::Directory::open(&asset_root, false)?;
    let sac = take(&mut args, "sac-bin")?
        .map(PathBuf::from)
        .unwrap_or_else(|| ctx.repository.join(crate::runtime::SAC_BIN));
    let smc = take(&mut args, "smc-bin")?
        .map(PathBuf::from)
        .unwrap_or_else(|| ctx.repository.join(crate::runtime::SMC_BIN));
    let schemas = SchemaRegistry::new(
        &ctx.repository
            .join("SceneMotionCompiler/contracts/schemas/animatic"),
    )?;
    let call = |argv: Vec<String>, timeout: u64| {
        root.verify_identity()?;
        observed_owner(&ProcessSpec {
            argv,
            input: vec![],
            cwd: PathBuf::from(format!("/dev/fd/{}", root.raw_fd())),
            timeout: Duration::from_secs(timeout),
            env: BTreeMap::new(),
            // The producer's '.' root is selected from this held asset directory, not a path lookup.
            pass_fds: vec![root.raw_fd()],
            max_output_bytes: crate::effects::MAX_BYTES,
        })
    };
    if operation == "assets" {
        let sac = executable(&ctx.repository, &sac)?;
        if !ctx.source_current()? {
            return Err(Error::not_applied(
                "writing synopsis changed before asset operation",
            ));
        }
        let mut argv = vec![path_string(&sac)?];
        argv.extend(args.clone());
        argv.extend([
            "--project-root".into(),
            ".".into(),
            "--owner-root".into(),
            path_string(&ctx.repository.join("SceneAssetCompiler"))?,
        ]);
        let result = call(argv, 120)?;
        if root.verify_identity().is_err() {
            return Err(producer_root_changed(&result));
        }
        validate_assets(&result, &args, &schemas)?;
        return Ok(result);
    }
    if ["status", "cancel", "recover"].contains(&operation.as_str()) {
        let attempt = required(&mut args, "attempt")?;
        if !args.is_empty() {
            return Err(Error::not_applied("unsupported animatic options"));
        }
        let smc = executable(&ctx.repository, &smc)?;
        let result = call(
            vec![
                path_string(&smc)?,
                operation,
                "--project".into(),
                ".".into(),
                "--attempt".into(),
                attempt.clone(),
                "--owner-root".into(),
                path_string(&ctx.repository.join("SceneMotionCompiler"))?,
            ],
            120,
        )?;
        if root.verify_identity().is_err() {
            return Err(producer_root_changed(&result));
        }
        validate_smc(&result, &attempt, &schemas)?;
        return Ok(result);
    }
    if !["packet", "render", "asset-draft"].contains(&operation.as_str()) {
        return Err(Error::not_applied("unsupported animatic operation"));
    }
    let mut options = vec!["export".into()];
    for (story, owner) in [
        ("board", "board"),
        ("board-review", "board-review"),
        ("plan", "plan"),
        ("review", "review"),
    ] {
        let value = required(&mut args, story)?;
        options.extend([format!("--{owner}"), value]);
    }
    let asset_draft = if operation == "asset-draft" {
        let bindings = required(&mut args, "bindings")?;
        let output = required(&mut args, "output")?;
        if !output.starts_with("animatic/") {
            return Err(Error::not_applied(
                "asset draft output must be under animatic/",
            ));
        }
        Some((bindings, output))
    } else {
        None
    };
    let render = if operation == "render" {
        let assets = required(&mut args, "assets")?;
        let review = required(&mut args, "asset-review")?;
        let attempt = required(&mut args, "attempt")?;
        let motion = take(&mut args, "motion")?.unwrap_or_else(|| "full".into());
        if !["full", "reduced"].contains(&motion.as_str()) || !args.is_empty() {
            return Err(Error::not_applied("unsupported render options or motion"));
        }
        Some((
            assets,
            review,
            attempt,
            motion,
            executable(&ctx.repository, &smc)?,
        ))
    } else {
        if !args.is_empty() {
            return Err(Error::not_applied("unsupported packet options"));
        }
        None
    };
    let export_current = || -> Result<Value> {
        root.verify_identity()?;
        if !ctx.source_current()? {
            return Err(Error::not_applied(
                "writing synopsis changed before animatic export",
            ));
        }
        ctx.narrative.tool("animatic", &options, &ctx.nvc_root()?)
    };
    let packet = export_current()?;
    root.verify_identity()?;
    validate_packet(&packet, &schemas)?;
    let hash = string(&packet, "contentHash")?;
    if !crate::effects::hash::is_sha256(hash) {
        return Err(Error::not_applied("NVC packet hash invalid"));
    }
    if let Some((bindings, output)) = asset_draft {
        let integration_schemas =
            SchemaRegistry::new(&ctx.repository.join("integration/contracts"))?;
        let prepared =
            super::animatic_assets::prepare(&root, &packet, &bindings, &integration_schemas)?;
        let current = export_current()?;
        validate_packet(&current, &schemas)?;
        if current != packet {
            return Err(Error::not_applied(
                "NVC packet changed before asset draft publication",
            ));
        }
        prepared.revalidate(&root)?;
        root.immutable(&output, &json::stable_bytes(&prepared.draft)?)?;
        return Ok(
            json!({"status":"draft_prepared", "candidateOnly":true, "path":output,
            "packetHash":hash, "next":"assets submit; actual PNG validation and human asset review remain with SAC"}),
        );
    }
    let relative = format!("animatic/packets/{}.json", &hash[7..]);
    root.immutable(&relative, &json::stable_bytes(&packet)?)?;
    let exported = json!({"status":"exported","packet":relative,"packetHash":hash,"source":packet["plan"]["source"]});
    if root.verify_identity().is_err() {
        return Err(producer_error(&exported, "asset root identity changed after packet publication; inspect the original selected root"));
    }
    let Some((assets, review, attempt, motion, smc)) = render else {
        return Ok(exported);
    };
    let result = call(
        vec![
            path_string(&smc)?,
            "render".into(),
            "--project".into(),
            ".".into(),
            "--packet".into(),
            relative,
            "--assets".into(),
            assets,
            "--asset-review".into(),
            review,
            "--attempt".into(),
            attempt.clone(),
            "--motion".into(),
            motion,
            "--owner-root".into(),
            path_string(&ctx.repository.join("SceneMotionCompiler"))?,
        ],
        1200,
    )?;
    validate_smc(&result, &attempt, &schemas)?;
    if root.verify_identity().is_err() {
        if result["status"] == "candidate" {
            return observe(result, "unobserved", "Animatic asset root identity changed after rendering; inspect the original selected root");
        }
        return Err(producer_root_changed(&result));
    }
    if result["status"] != "candidate" {
        return Ok(result);
    }
    let confirmed = call(
        vec![
            path_string(&smc)?,
            "status".into(),
            "--project".into(),
            ".".into(),
            "--attempt".into(),
            attempt,
            "--owner-root".into(),
            path_string(&ctx.repository.join("SceneMotionCompiler"))?,
        ],
        120,
    );
    if root.verify_identity().is_err() {
        return observe(
            result,
            "unobserved",
            "Animatic asset root identity changed during the producer status observation",
        );
    }
    match confirmed {
        Ok(ref current)
            if validate_smc(current, string(&result, "attemptId")?, &schemas).is_ok()
                && current == &result => {}
        Ok(_) => {
            return observe(
                result,
                "unobserved",
                "SMC public status did not confirm the captured render/request receipt",
            )
        }
        Err(error) => {
            return observe(
                result,
                "unobserved",
                &format!(
                    "SMC render receipt could not be confirmed: {}",
                    error.message
                ),
            )
        }
    }
    let current_source = ctx.source_current();
    if root.verify_identity().is_err() {
        return observe(
            result,
            "unobserved",
            "Animatic asset root identity changed during the writing source observation",
        );
    }
    match current_source {
        Ok(false) => {
            return observe(
                result,
                "changed",
                "Writing synopsis changed during rendering",
            )
        }
        Err(error) => return observe(result, "unobserved", &error.message),
        Ok(true) => {}
    }
    let latest = ctx
        .nvc_root()
        .and_then(|path| ctx.narrative.tool("animatic", &options, &path));
    if root.verify_identity().is_err() {
        return observe(
            result,
            "unobserved",
            "Animatic asset root identity changed during the final packet observation",
        );
    }
    match latest {
        Err(error) => observe(result, "unobserved", &error.message),
        Ok(latest) => match validate_packet(&latest, &schemas) {
            Err(error) => observe(result, "unobserved", &error.message),
            Ok(()) if latest != packet => {
                observe(result, "changed", "NVC packet changed during rendering")
            }
            Ok(()) => observe(result, "current", ""),
        },
    }
}
