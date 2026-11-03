//! CLI syntax and wire output; no owner state is interpreted here.
use crate::{
    adapters::{
        owners::path_string,
        promptgen::{self, PromptgenOptions},
    },
    contracts::{self, SchemaRegistry},
    effects::{files, json},
    error::{Error, Result},
    roots::StoryResourceRoot,
    runtime,
    workflows::{
        animatic,
        attempts::{self, RunOptions},
        capability, publication,
        text::{self, TextContext},
    },
};
use serde_json::{json, Value};
use std::{
    collections::BTreeMap,
    io::Read,
    path::{Path, PathBuf},
};
const HELP:&str="storyctl [--story-root PATH] <doctor|verify-contracts|runtime|text|run-promptgen|capability|scene-asset>\n\nruntime assemble --output PATH [--include-promptgen true|false]\nruntime verify\ntext --workspace PATH [--geultu-bin PATH] [--nvc-bin PATH] [--narrative-project ID]\n  create|status|next|author|import-handoff|scene|storyboard|animatic|run|recover|export|package\nrun-promptgen [--capability-root PATH] --request PATH --output-dir PATH --promptgen-bin PATH\ncapability validate-candidate [--capability-root PATH] --current-source PATH --result PATH\nInstalled runtimes require an external --capability-root for capability files. Native Story orchestration; owners retain source and approval authority.";
#[derive(Default)]
struct Options {
    values: BTreeMap<String, Vec<String>>,
}
impl Options {
    fn get(&self, k: &str) -> Option<&str> {
        self.values
            .get(k)
            .and_then(|v| v.first())
            .map(String::as_str)
    }
    fn required(&self, k: &str) -> Result<&str> {
        self.get(k)
            .filter(|v| !v.is_empty())
            .ok_or_else(|| Error::not_applied(format!("required option --{k}")))
    }
    fn all(&self, k: &str) -> Vec<String> {
        self.values.get(k).cloned().unwrap_or_default()
    }
    fn integer(&self, k: &str, default: u64) -> Result<u64> {
        self.get(k)
            .map(|v| {
                v.parse::<u64>()
                    .map_err(|_| Error::not_applied(format!("invalid --{k}")))
            })
            .unwrap_or(Ok(default))
    }
}
fn options(args: &[String], allowed: &[&str], repeat: &[&str]) -> Result<Options> {
    let mut result = Options::default();
    let mut i = 0;
    while i < args.len() {
        let raw = &args[i];
        let key = raw
            .strip_prefix("--")
            .ok_or_else(|| Error::not_applied(format!("unexpected argument {raw}")))?;
        let (key, inline) = key
            .split_once('=')
            .map(|(k, v)| (k, Some(v)))
            .unwrap_or((key, None));
        if !allowed.contains(&key) {
            return Err(Error::not_applied(format!("unsupported option --{key}")));
        }
        let value = if let Some(v) = inline {
            v.to_string()
        } else {
            i += 1;
            args.get(i)
                .filter(|value| !value.starts_with("--") && value.as_str() != "-h")
                .ok_or_else(|| Error::not_applied(format!("missing value for --{key}")))?
                .clone()
        };
        let values = result.values.entry(key.into()).or_default();
        if !values.is_empty() && !repeat.contains(&key) {
            return Err(Error::not_applied(format!("duplicate option --{key}")));
        }
        values.push(value);
        i += 1;
    }
    Ok(result)
}
fn repository(explicit: Option<&str>) -> Result<StoryResourceRoot> {
    if let Some(root) = explicit
        .map(str::to_owned)
        .or_else(|| std::env::var("STORY_ROOT").ok())
    {
        return StoryResourceRoot::select(Path::new(&root));
    }
    let exe = std::env::current_exe()?.canonicalize()?;
    let executable_parent = exe
        .parent()
        .ok_or_else(|| Error::not_started("storyctl executable has no parent directory"))?;
    let working_directory = std::env::current_dir()?;
    for root in executable_parent
        .ancestors()
        .chain(working_directory.ancestors())
    {
        // A manifest makes the candidate an install root even if malformed;
        // check it before considering any source-tree registry fallback.
        if files::path_exists(&root.join(runtime::RUNTIME_MANIFEST))? {
            return StoryResourceRoot::select(root);
        }
        let installed_entry = root.join(runtime::STORYCTL_ENTRY);
        if std::fs::canonicalize(&installed_entry).is_ok_and(|entry| entry == exe) {
            return Err(Error::not_started(
                "installed Story runtime is incomplete because runtime-manifest.json is missing",
            ));
        }
        if files::path_exists(&root.join("integration/contracts/registry.json"))? {
            return StoryResourceRoot::select(root);
        }
    }
    Err(Error::not_started(
        "Story resources unavailable; pass --story-root PATH or set STORY_ROOT",
    ))
}
fn input(value: &str) -> Result<Value> {
    if value != "-" {
        return json::parse_object(&files::read_bytes(Path::new(value))?);
    }
    let mut bytes = vec![];
    std::io::stdin()
        .take((crate::effects::MAX_BYTES + 1) as u64)
        .read_to_end(&mut bytes)?;
    if bytes.len() > crate::effects::MAX_BYTES {
        return Err(Error::not_applied("input exceeds 16 MiB"));
    }
    json::parse_object(&bytes)
}
pub fn main(mut args: Vec<String>) -> i32 {
    if args.is_empty() || args.iter().any(|v| v == "--help" || v == "-h") {
        println!("{HELP}");
        return 0;
    }
    if args == ["--version"] {
        println!("storyctl {}", env!("CARGO_PKG_VERSION"));
        return 0;
    }
    let mut root_arg = None;
    if args.first().is_some_and(|v| v == "--story-root") {
        if args.len() < 3 || args[1].starts_with("--") {
            eprintln!("{HELP}");
            return 2;
        }
        root_arg = Some(args.remove(1));
        args.remove(0);
    } else if let Some(root) = args[0].strip_prefix("--story-root=") {
        root_arg = Some(root.to_owned());
        args.remove(0);
        if args.is_empty() {
            eprintln!("{HELP}");
            return 2;
        }
    }
    let command = args.remove(0);
    let result = repository(root_arg.as_deref()).and_then(|root| dispatch(&root, &command, &args));
    match result {
        Ok(value) => {
            let encoded = match serde_json::to_string_pretty(&value) {
                Ok(encoded) => encoded,
                Err(error) => {
                    eprintln!(
                        "{}",
                        json!({"status":"outcome_unknown","error":format!("result encoding failed: {error}")})
                    );
                    return 2;
                }
            };
            println!("{encoded}");
            if command == "run-promptgen" {
                if value["result"]["status"] == "succeeded" {
                    0
                } else {
                    2
                }
            } else if value["status"] == "not_applied" {
                2
            } else if matches!(
                value["status"].as_str(),
                Some(
                    "failed"
                        | "cancelled"
                        | "outcome_unknown"
                        | "not_started"
                        | "source_changed"
                        | "source_stale"
                        | "PARTIAL"
                )
            ) {
                3
            } else {
                0
            }
        }
        Err(error) => {
            let mut payload = json!({"status":error.outcome.as_str(),"error":error.message});
            if error.outcome == story_integration_core::Outcome::Unknown {
                if let Ok(observed) = json::parse_object(&error.stdout) {
                    payload["observedOutput"] = observed;
                    if serde_json::to_vec(&payload)
                        .is_ok_and(|bytes| bytes.len() > crate::effects::MAX_BYTES)
                    {
                        payload.as_object_mut().unwrap().remove("observedOutput");
                        payload["observedOutputOmitted"] =
                            json!("response byte limit exceeded; inspect owner state");
                    }
                }
            }
            eprintln!("{payload}");
            if error.outcome == story_integration_core::Outcome::NotStarted {
                3
            } else {
                2
            }
        }
    }
}
fn dispatch(root: &StoryResourceRoot, command: &str, args: &[String]) -> Result<Value> {
    match command {
        "text" => text_command(root, args),
        "runtime" => runtime_command(root, args),
        "doctor" => {
            let opt = options(
                args,
                &[
                    "scope",
                    "geultu-bin",
                    "nvc-bin",
                    "promptgen-bin",
                    "sac-bin",
                    "smc-bin",
                ],
                &[],
            )?;
            doctor(root.path(), &opt)
        }
        "verify-contracts" => {
            if !args.is_empty() {
                return Err(Error::not_applied("verify-contracts takes no options"));
            }
            let registry = contracts::verify_contract_registry(
                root.path(),
                &root.path().join("integration/contracts/registry.json"),
            )?;
            let schemas = SchemaRegistry::new(&root.path().join("integration/contracts"))?;
            let mut examples = vec![];
            for entry in std::fs::read_dir(root.path().join("integration/contracts/examples"))? {
                let path = entry?.path();
                if path.extension().is_none_or(|v| v != "json") {
                    continue;
                }
                let value = json::parse_object(&files::read_bytes(&path)?)?;
                let id = match value["schemaVersion"].as_str() {
                    Some("story.capability-request/1") => {
                        "https://story-production.local/contracts/capability-request/v1"
                    }
                    Some("story.capability-job/1") => {
                        "https://story-production.local/contracts/capability-job/v1"
                    }
                    Some("story.capability-result/1") => {
                        "https://story-production.local/contracts/capability-result/v1"
                    }
                    _ => continue,
                };
                schemas.validate(id, &value)?;
                examples.push(path_string(
                    path.strip_prefix(root.path())
                        .map_err(|_| Error::not_applied("example outside root"))?,
                )?);
            }
            examples.sort();
            Ok(json!({"status":"PASS","registry":registry,"examples":examples}))
        }
        "run-promptgen" => {
            let opt = options(
                args,
                &[
                    "capability-root",
                    "request",
                    "output-dir",
                    "promptgen-bin",
                    "timeout-seconds",
                    "artifact-wait-seconds",
                ],
                &[],
            )?;
            let capability_root =
                root.capability_root(opt.get("capability-root").map(Path::new))?;
            let promptgen_bin = match (root.mode(), opt.get("promptgen-bin")) {
                (_, Some(path)) => PathBuf::from(path),
                (runtime::RootMode::Installed, None) => root.path().join(runtime::PROMPTGEN_BIN),
                (runtime::RootMode::Source, None) => {
                    return Err(Error::not_applied("required option --promptgen-bin"));
                }
            };
            promptgen::run_promptgen_in_roots(
                root,
                &capability_root,
                &PromptgenOptions {
                    request: PathBuf::from(opt.required("request")?),
                    output_dir: PathBuf::from(opt.required("output-dir")?),
                    promptgen_bin,
                    timeout_seconds: opt
                        .integer("timeout-seconds", promptgen::DEFAULT_TIMEOUT_SECONDS)?,
                    artifact_wait_seconds: opt.integer(
                        "artifact-wait-seconds",
                        promptgen::DEFAULT_ARTIFACT_WAIT_SECONDS,
                    )?,
                },
            )
        }
        "capability" => {
            if args.first().map(String::as_str) != Some("validate-candidate") {
                return Err(Error::not_applied("use capability validate-candidate"));
            }
            let opt = options(
                &args[1..],
                &[
                    "capability-root",
                    "current-source",
                    "result",
                    "expected-request-hash",
                ],
                &[],
            )?;
            let capability_root =
                root.capability_root(opt.get("capability-root").map(Path::new))?;
            capability::validate_candidate_in_roots(
                root,
                &capability_root,
                &input(opt.required("current-source")?)?,
                &input(opt.required("result")?)?,
                opt.get("expected-request-hash"),
            )
        }
        "scene-asset" => {
            if args.first().map(String::as_str) != Some("request") {
                return Err(Error::not_applied("use scene-asset request"));
            }
            let opt = options(
                &args[1..],
                &[
                    "project",
                    "smc-project-root",
                    "receipt",
                    "receipt-hash",
                    "output",
                    "human-final-review",
                ],
                &["output"],
            )?;
            let policy = match opt.get("human-final-review").unwrap_or("true") {
                "true" => true,
                "false" => false,
                _ => {
                    return Err(Error::not_applied(
                        "human-final-review must be true or false",
                    ))
                }
            };
            capability::build_scene_asset_request(
                root.path(),
                &capability::SceneAssetOptions {
                    project_id: opt.required("project")?.into(),
                    smc_project_root: PathBuf::from(opt.required("smc-project-root")?),
                    projection_receipt_path: opt.required("receipt")?.into(),
                    projection_receipt_hash: opt.required("receipt-hash")?.into(),
                    outputs: opt.all("output"),
                    human_final_review: policy,
                },
            )
        }
        _ => Err(Error::not_applied(format!("unknown command {command}"))),
    }
}
fn runtime_command(root: &StoryResourceRoot, args: &[String]) -> Result<Value> {
    let operation = args
        .first()
        .map(String::as_str)
        .ok_or_else(|| Error::not_applied("runtime operation required"))?;
    match operation {
        "verify" => {
            options(&args[1..], &[], &[])?;
            runtime::verify(root.path())
        }
        "assemble" => {
            let opt = options(&args[1..], &["output", "include-promptgen"], &[])?;
            let include_promptgen = match opt.get("include-promptgen").unwrap_or("false") {
                "true" => true,
                "false" => false,
                _ => {
                    return Err(Error::not_applied(
                        "include-promptgen must be true or false",
                    ))
                }
            };
            runtime::assemble(
                root.path(),
                Path::new(opt.required("output")?),
                include_promptgen,
            )
        }
        _ => Err(Error::not_applied("use runtime assemble or runtime verify")),
    }
}
fn text_command(root: &StoryResourceRoot, args: &[String]) -> Result<Value> {
    let mut index = 0;
    while index < args.len() && args[index].starts_with("--") {
        if args[index].contains('=') {
            index += 1
        } else {
            index += 2;
        }
    }
    if index >= args.len() {
        return Err(Error::not_applied("text subcommand required"));
    }
    let common = options(
        &args[..index],
        &[
            "workspace",
            "geultu-bin",
            "nvc-bin",
            "narrative-project",
            "story-root",
        ],
        &[],
    )?;
    let selected_root = common
        .get("story-root")
        .map(|p| repository(Some(p)))
        .transpose()?
        .unwrap_or_else(|| root.clone());
    let workspace = selected_root.workspace_path(Path::new(common.required("workspace")?))?;
    let ctx = TextContext::new(
        selected_root.path().to_path_buf(),
        workspace,
        std::path::absolute(
            common
                .get("geultu-bin")
                .map(PathBuf::from)
                .unwrap_or_else(|| selected_root.path().join(runtime::GEULTU_BIN)),
        )?,
        std::path::absolute(
            common
                .get("nvc-bin")
                .map(PathBuf::from)
                .unwrap_or_else(|| selected_root.path().join(runtime::NVC_BIN)),
        )?,
        common.get("narrative-project").map(str::to_owned),
    )?;
    let command = &args[index];
    let rest = &args[index + 1..];
    match command.as_str() {
        "create" => {
            let opt = options(
                rest,
                &["project", "title", "source", "actor", "language"],
                &[],
            )?;
            text::create(
                &ctx,
                opt.required("project")?,
                opt.required("title")?,
                Path::new(opt.required("source")?),
                opt.required("actor")?,
                opt.get("language").unwrap_or("ko"),
            )
        }
        "status" => {
            options(rest, &[], &[])?;
            text::status(&ctx)
        }
        "next" => {
            let opt = options(rest, &["instruction"], &[])?;
            text::next(&ctx, opt.get("instruction").unwrap_or(""))
        }
        "author" => {
            let name = rest
                .first()
                .ok_or_else(|| Error::not_applied("author operation required"))?;
            let opt = options(&rest[1..], &["input"], &[])?;
            ctx.author(name, input(opt.required("input")?)?)
        }
        "import-handoff" => {
            let opt = options(rest, &["handoff-root", "duration"], &[])?;
            let d = opt
                .required("duration")?
                .parse::<u32>()
                .map_err(|_| Error::not_applied("duration must be integer"))?;
            text::handoff(&ctx, Path::new(opt.required("handoff-root")?), d)
        }
        "scene" => {
            let op = rest
                .first()
                .ok_or_else(|| Error::not_applied("scene operation required"))?;
            if ![
                "agent-task",
                "submit",
                "decide",
                "resume",
                "status",
                "cancel",
            ]
            .contains(&op.as_str())
            {
                return Err(Error::not_applied("unsupported scene operation"));
            }
            if ["submit", "decide"].contains(&op.as_str()) && !ctx.source_current()? {
                return Err(Error::not_applied(
                    "writing synopsis changed; hand off current synopsis before advancing scenes",
                ));
            }
            ctx.narrative_call(op, &rest[1..])
        }
        "storyboard" => ctx.board(rest),
        "animatic" => animatic::run_with_resource_root(&ctx, rest, &selected_root),
        "run" => {
            let opt = options(
                rest,
                &[
                    "attempt",
                    "writer-program",
                    "writer-arg",
                    "timeout",
                    "instruction",
                    "storyboard-task",
                ],
                &["writer-arg"],
            )?;
            attempts::run(
                &ctx,
                &RunOptions {
                    attempt: opt.required("attempt")?.into(),
                    writer: PathBuf::from(opt.required("writer-program")?),
                    arguments: opt.all("writer-arg"),
                    timeout: opt.integer("timeout", 600)?,
                    instruction: opt.get("instruction").unwrap_or("").into(),
                    storyboard_task: opt.get("storyboard-task").map(str::to_owned),
                },
            )
        }
        "recover" => {
            let opt = options(rest, &["attempt"], &[])?;
            attempts::recover(&ctx, opt.required("attempt")?)
        }
        "export" => {
            let opt = options(rest, &["output"], &[])?;
            let output = selected_root.writable_path(Path::new(opt.required("output")?))?;
            publication::export(&ctx, &output)
        }
        "package" => {
            let opt = options(rest, &["board", "review", "output"], &[])?;
            let output = selected_root.writable_path(Path::new(opt.required("output")?))?;
            publication::package(
                &ctx,
                opt.required("board")?,
                opt.required("review")?,
                &output,
            )
        }
        _ => Err(Error::not_applied(format!(
            "unknown text subcommand {command}"
        ))),
    }
}
fn executable(path: &Path) -> bool {
    use std::os::unix::fs::PermissionsExt;
    std::fs::metadata(path).is_ok_and(|m| m.is_file() && m.permissions().mode() & 0o111 != 0)
}
fn doctor(root: &Path, opt: &Options) -> Result<Value> {
    let scope = opt.get("scope").unwrap_or("text");
    if !["authoring", "text", "promptgen", "animatic"].contains(&scope) {
        return Err(Error::not_applied(
            "doctor scope must be authoring, text, promptgen or animatic",
        ));
    }
    let geultu = opt
        .get("geultu-bin")
        .map(PathBuf::from)
        .unwrap_or_else(|| root.join(runtime::GEULTU_BIN));
    let nvc = opt
        .get("nvc-bin")
        .map(PathBuf::from)
        .unwrap_or_else(|| root.join(runtime::NVC_BIN));
    let promptgen = opt
        .get("promptgen-bin")
        .map(PathBuf::from)
        .unwrap_or_else(|| root.join(runtime::PROMPTGEN_BIN));
    let sac = opt
        .get("sac-bin")
        .map(PathBuf::from)
        .unwrap_or_else(|| root.join(runtime::SAC_BIN));
    let smc = opt
        .get("smc-bin")
        .map(PathBuf::from)
        .unwrap_or_else(|| root.join(runtime::SMC_BIN));
    let checks = BTreeMap::from([
        ("geultu", executable(&geultu)),
        ("nvc", executable(&nvc)),
        ("promptgen", executable(&promptgen)),
        ("sac_animatic", executable(&sac)),
        ("smc_animatic", executable(&smc)),
        (
            "nvc_resources",
            root.join("NarrativeCourier/skills/sts").is_dir(),
        ),
    ]);
    let required: &[&str] = match scope {
        "authoring" => &["geultu"],
        "text" => &["geultu", "nvc", "nvc_resources"],
        "promptgen" => &["promptgen"],
        _ => &[
            "geultu",
            "nvc",
            "nvc_resources",
            "sac_animatic",
            "smc_animatic",
        ],
    };
    let missing: Vec<_> = required.iter().filter(|k| !checks[**k]).collect();
    let optional: Vec<_> = checks
        .iter()
        .filter(|(k, v)| !**v && !required.contains(k))
        .map(|(k, _)| k)
        .collect();
    Ok(
        json!({"status":if missing.is_empty(){"PASS"}else{"PARTIAL"},"scope":scope,"checks":checks,"missingRequired":missing,"missingOptional":optional,"runtime":"native","qualification":"executable_and_resources_present; run native verification for workflow conformance"}),
    )
}
