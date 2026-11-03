//! Producer response and refusal interpretation belongs to the producer adapter.
use crate::effects::workspace::WorkspaceBinding;
use crate::{
    effects::{
        json,
        process::{invoke, ProcessSpec},
    },
    error::{Error, Result},
    ports::{NarrativeOwner, WritingOwner},
};
use serde_json::Value;
use std::{
    collections::BTreeMap,
    path::{Path, PathBuf},
    sync::{Arc, OnceLock},
    time::Duration,
};

pub(crate) struct BoundWritingOwner {
    pub(crate) executable: PathBuf,
    pub(crate) repository: PathBuf,
    pub(crate) workspace: Arc<WorkspaceBinding>,
    pub(crate) directory: OnceLock<Arc<crate::effects::files::Directory>>,
}
impl BoundWritingOwner {
    fn namespace(&self, create: bool) -> Result<Arc<crate::effects::files::Directory>> {
        self.workspace.ensure_current()?;
        let root = self.workspace.directory()?;
        if self.directory.get().is_none() {
            let _ = self.directory.set(Arc::new(root.child("writing", create)?));
        }
        let directory = self.directory.get().unwrap().clone();
        root.verify_child("writing", &directory)?;
        Ok(directory)
    }
    fn adapter(&self, create: bool) -> Result<Geultu> {
        let fd = self.namespace(create)?.raw_fd();
        Ok(Geultu {
            executable: self.executable.clone(),
            repository: self.repository.clone(),
            workspace: PathBuf::from("."),
            pass_fds: vec![fd],
        })
    }
}
impl WritingOwner for BoundWritingOwner {
    fn initialize(&self) -> Result<()> {
        self.adapter(true)?.initialize()?;
        self.namespace(false).map_err(|error| {
            Error::unknown(format!("Geultu root changed after initialization: {error}"))
        })?;
        self.workspace.ensure_current().map_err(|error| {
            Error::unknown(format!(
                "owner initialization returned but selected workspace changed: {error}"
            ))
        })
    }
    fn call(&self, operation: &str, input: &Value) -> Result<Value> {
        let result = self.adapter(false)?.call(operation, input)?;
        self.namespace(false).map_err(|error| {
            Error::unknown(format!("Geultu root changed after owner call: {error}"))
        })?;
        self.workspace.ensure_current().map_err(|error| {
            Error::unknown(format!(
                "owner call returned but selected workspace changed: {error}"
            ))
        })?;
        Ok(result)
    }
}

/// One invocation's NVC namespace and selected project. Display paths are
/// never reopened after selection; Story and owner calls share these handles.
pub(crate) struct NarrativeDirectoryBinding {
    pub(crate) workspace: Arc<WorkspaceBinding>,
    namespace: OnceLock<Arc<crate::effects::files::Directory>>,
    project: OnceLock<(String, Arc<crate::effects::files::Directory>)>,
}
impl NarrativeDirectoryBinding {
    pub(crate) fn new(workspace: Arc<WorkspaceBinding>) -> Self {
        Self {
            workspace,
            namespace: OnceLock::new(),
            project: OnceLock::new(),
        }
    }
    fn namespace(&self, create: bool) -> Result<Arc<crate::effects::files::Directory>> {
        self.workspace.ensure_current()?;
        let root = self.workspace.directory()?;
        if self.namespace.get().is_none() {
            let _ = self
                .namespace
                .set(Arc::new(root.child("narrative", create)?));
        }
        let directory = self.namespace.get().unwrap().clone();
        root.verify_child("narrative", &directory)?;
        Ok(directory)
    }
    pub(crate) fn project_optional(
        &self,
        name: &str,
    ) -> Result<Option<Arc<crate::effects::files::Directory>>> {
        story_integration_core::ProjectId::new(name).map_err(Error::not_applied)?;
        let root = self.workspace.directory()?;
        if self.namespace.get().is_none() && root.child_optional("narrative")?.is_none() {
            self.workspace.ensure_current()?;
            return Ok(None);
        }
        let namespace = self.namespace(false)?;
        if let Some((expected, project)) = self.project.get() {
            if expected != name {
                return Err(Error::not_applied("selected NVC project identity changed"));
            }
            namespace.verify_child(name, project)?;
            return Ok(Some(project.clone()));
        }
        let Some(project) = namespace.child_optional(name)? else {
            return Ok(None);
        };
        let _ = self.project.set((name.to_owned(), Arc::new(project)));
        let (expected, project) = self.project.get().unwrap();
        if expected != name {
            return Err(Error::not_applied("selected NVC project identity changed"));
        }
        namespace.verify_child(name, project)?;
        Ok(Some(project.clone()))
    }
    pub(crate) fn project(&self, name: &str) -> Result<Arc<crate::effects::files::Directory>> {
        self.project_optional(name)?
            .ok_or_else(|| Error::not_applied("selected NVC project is missing"))
    }
}

pub(crate) struct BoundNarrativeOwner {
    pub(crate) executable: PathBuf,
    pub(crate) repository: PathBuf,
    pub(crate) binding: Arc<NarrativeDirectoryBinding>,
}
impl BoundNarrativeOwner {
    fn namespace_adapter(&self, create: bool) -> Result<NarrativeCourier> {
        let fd = self.binding.namespace(create)?.raw_fd();
        Ok(NarrativeCourier {
            executable: self.executable.clone(),
            repository: self.repository.clone(),
            projects: PathBuf::from("."),
            pass_fds: vec![fd],
        })
    }
    fn project_adapter(&self, project: &str) -> Result<NarrativeCourier> {
        let fd = self.binding.project(project)?.raw_fd();
        Ok(NarrativeCourier {
            executable: self.executable.clone(),
            repository: self.repository.clone(),
            projects: PathBuf::from("."),
            pass_fds: vec![fd],
        })
    }
    fn after_call(&self, project: &str) -> Result<()> {
        self.binding.project(project).map_err(|error| {
            Error::unknown(format!("NVC project changed after owner call: {error}"))
        })?;
        self.binding.workspace.ensure_current().map_err(|error| {
            Error::unknown(format!(
                "owner call returned but selected workspace changed: {error}"
            ))
        })
    }
}
impl NarrativeOwner for BoundNarrativeOwner {
    fn text(&self, operation: &str, project: &str, arguments: &[String]) -> Result<String> {
        let result = if operation == "import-reviewed-synopsis" {
            self.binding.project_optional(project)?;
            self.namespace_adapter(true)?
                .text(operation, project, arguments)?
        } else {
            let mut arguments = arguments.to_vec();
            arguments.extend(["--project-root".into(), ".".into()]);
            self.project_adapter(project)?
                .text(operation, project, &arguments)?
        };
        self.after_call(project)
            .map_err(|error| error.with_stdout(result.as_bytes().to_vec()))?;
        Ok(result)
    }
    fn tool(&self, lane: &str, arguments: &[String], root: &Path) -> Result<Value> {
        let relative = root
            .strip_prefix(self.binding.workspace.display_path())
            .map_err(|_| {
                Error::not_applied("narrative tool root escaped the selected workspace")
            })?;
        let project = relative
            .strip_prefix("narrative")
            .map_err(|_| Error::not_applied("narrative tool root escaped its owner namespace"))?;
        if project.components().count() != 1 {
            return Err(Error::not_applied(
                "narrative tool root must select one project",
            ));
        }
        let project = path_string(project)?;
        let result = self
            .project_adapter(&project)?
            .tool(lane, arguments, Path::new("."))?;
        self.after_call(&project)?;
        Ok(result)
    }
}

pub struct Geultu {
    pub executable: PathBuf,
    pub workspace: PathBuf,
    pub repository: PathBuf,
    pub pass_fds: Vec<i32>,
}
pub struct NarrativeCourier {
    pub executable: PathBuf,
    pub projects: PathBuf,
    pub repository: PathBuf,
    pub pass_fds: Vec<i32>,
}
pub fn path_string(path: &Path) -> Result<String> {
    path.to_str()
        .map(str::to_owned)
        .ok_or_else(|| Error::not_applied("path is not UTF-8"))
}
fn bound_path(path: &Path, fds: &[i32]) -> Result<String> {
    for fd in fds {
        let root = PathBuf::from(format!("/dev/fd/{fd}"));
        if let Ok(relative) = path.strip_prefix(&root) {
            return path_string(relative);
        }
    }
    path_string(path)
}
fn owner_cwd(repository: &Path, fds: &[i32]) -> PathBuf {
    fds.first()
        .map(|fd| PathBuf::from(format!("/dev/fd/{fd}")))
        .unwrap_or_else(|| repository.to_path_buf())
}
fn command(
    argv: Vec<String>,
    input: Vec<u8>,
    cwd: PathBuf,
    env: BTreeMap<String, String>,
    pass_fds: Vec<i32>,
) -> Result<crate::effects::process::ProcessOutput> {
    invoke(&ProcessSpec {
        argv,
        input,
        cwd,
        env,
        pass_fds,
        timeout: Duration::from_secs(crate::effects::process::DEFAULT_OWNER_TIMEOUT_SECONDS),
        max_output_bytes: crate::effects::MAX_BYTES,
    })
}
fn failure(stdout: Vec<u8>, stderr: Vec<u8>) -> Error {
    let message = String::from_utf8_lossy(&stderr).into_owned();
    let message = if message.is_empty() {
        "owner process did not confirm operation".into()
    } else {
        message
    };
    let mut error = if json::parse_object(message.as_bytes())
        .ok()
        .is_some_and(|v| v["status"] == "not_applied")
    {
        Error::not_applied(message).with_stdout(stdout)
    } else {
        Error::unknown(message).with_stdout(stdout)
    };
    error.spawned = true;
    error.stderr = stderr;
    error
}
impl Geultu {
    fn craftpack_environment(&self) -> Result<BTreeMap<String, String>> {
        let assets_root = self.repository.join(crate::runtime::GEULTU_ASSETS_ROOT);
        Ok(BTreeMap::from([
            ("GEULTU_ASSETS_ROOT".into(), path_string(&assets_root)?),
            (
                "GEULTU_CRAFT_PACK_ROOT".into(),
                path_string(&self.repository.join(crate::runtime::BASELINE_PACK_ROOT))?,
            ),
        ]))
    }
}

impl WritingOwner for Geultu {
    fn initialize(&self) -> Result<()> {
        let output = command(
            vec![
                path_string(&self.executable)?,
                "--workspace".into(),
                bound_path(&self.workspace, &self.pass_fds)?,
                "init".into(),
            ],
            vec![],
            owner_cwd(&self.repository, &self.pass_fds),
            BTreeMap::new(),
            self.pass_fds.clone(),
        )?;
        if !output.status.success() {
            return Err(failure(output.stdout, output.stderr));
        }
        Ok(())
    }
    fn call(&self, operation: &str, input: &Value) -> Result<Value> {
        let output = command(
            vec![
                path_string(&self.executable)?,
                "--workspace".into(),
                bound_path(&self.workspace, &self.pass_fds)?,
                "call".into(),
                operation.into(),
                "-".into(),
            ],
            json::stable_bytes(input)?,
            owner_cwd(&self.repository, &self.pass_fds),
            self.craftpack_environment()?,
            self.pass_fds.clone(),
        )?;
        if !output.status.success() {
            let diagnostic = json::parse_object(&output.stderr).ok();
            if operation == "authoring.prepare"
                && diagnostic.as_ref().is_some_and(|v| {
                    v["ok"] == false
                        && v["error"]["code"] == "conflict"
                        && v["error"]["message"]
                            .as_str()
                            .is_some_and(|message| !message.is_empty())
                        && v["error"]["committed_revision"].is_null()
                })
            {
                let mut error = Error::not_applied("authoring.prepare was not applied because project revision or task target changed; read current status")
                    .with_stdout(output.stdout).with_stderr(output.stderr);
                error.spawned = true;
                return Err(error);
            }
            return Err(failure(output.stdout, output.stderr));
        }
        let response = json::parse_object(&output.stdout).map_err(|_| {
            Error::unknown(
                "geultu returned an invalid response; inspect owner state before retrying",
            )
            .with_stdout(output.stdout.clone())
        })?;
        if response["ok"] == true && !response["result"].is_object() {
            return Err(Error::unknown(
                "geultu returned a malformed success response; inspect owner state before retrying",
            )
            .with_stdout(output.stdout));
        }
        if response["ok"] != true {
            return Err(Error::unknown(format!(
                "geultu returned an invalid success response; inspect owner state: {response}"
            ))
            .with_stdout(output.stdout));
        }
        if operation == "authoring.submit" {
            validate_geultu_submission(&response["result"], input)
                .map_err(|error| error.with_stdout(output.stdout.clone()))?;
        } else if operation == "authoring.prepare" {
            validate_geultu_preparation(&response["result"], input)
                .map_err(|error| error.with_stdout(output.stdout.clone()))?;
        } else if operation == "authoring.task" {
            validate_geultu_task(
                &response["result"],
                input["id"].as_str().unwrap_or_default(),
            )
            .map_err(|error| error.with_stdout(output.stdout.clone()))?;
        } else if matches!(
            operation,
            "authoring.start"
                | "authoring.brief"
                | "authoring.edit"
                | "authoring.review"
                | "authoring.cancel"
                | "authoring.restore"
        ) {
            validate_geultu_commit(&response["result"]["commit"], &input["project_id"])
                .map_err(|error| error.with_stdout(output.stdout.clone()))?;
            if !response["result"]["result"].is_object() {
                return Err(Error::unknown(
                    "Geultu mutation acknowledgment lacks its public result object",
                )
                .with_stdout(output.stdout));
            }
        }
        Ok(response["result"].clone())
    }
}
impl NarrativeOwner for NarrativeCourier {
    fn text(&self, command_name: &str, project: &str, arguments: &[String]) -> Result<String> {
        let mut argv = vec![
            path_string(&self.executable)?,
            command_name.into(),
            project.into(),
        ];
        argv.extend_from_slice(arguments);
        let env = BTreeMap::from([
            (
                "STS_ASSETS_ROOT".into(),
                path_string(&self.repository.join("NarrativeCourier"))?,
            ),
            (
                "STS_PROJECTS_ROOT".into(),
                bound_path(&self.projects, &self.pass_fds)?,
            ),
        ]);
        let output = command(
            argv,
            vec![],
            owner_cwd(&self.repository, &self.pass_fds),
            env,
            self.pass_fds.clone(),
        )?;
        if !output.status.success() {
            return Err(failure(output.stdout, output.stderr));
        }
        if command_name == "submit" {
            let receipt = json::parse_object(&output.stdout).map_err(|_| {
                Error::unknown("NarrativeCourier submit returned an invalid response")
                    .with_stdout(output.stdout.clone())
            })?;
            validate_narrative_receipt_shape(&receipt)
                .map_err(|error| error.with_stdout(output.stdout.clone()))?;
        }
        String::from_utf8(output.stdout)
            .map_err(|_| Error::unknown("NarrativeCourier did not return UTF-8"))
    }
    fn tool(&self, lane: &str, arguments: &[String], root: &Path) -> Result<Value> {
        let mut argv = vec![path_string(&self.executable)?, lane.into()];
        argv.extend_from_slice(arguments);
        argv.extend(["--project-root".into(), bound_path(root, &self.pass_fds)?]);
        let output = command(
            argv,
            vec![],
            owner_cwd(&self.repository, &self.pass_fds),
            BTreeMap::new(),
            self.pass_fds.clone(),
        )?;
        if !output.status.success() {
            return Err(failure(output.stdout, output.stderr));
        }
        let receipt = json::parse_object(&output.stdout).map_err(|_| {
            Error::unknown("NarrativeCourier tool returned an invalid response")
                .with_stdout(output.stdout.clone())
        })?;
        if lane == "storyboard" && arguments.first().map(String::as_str) == Some("submit") {
            let expected = arguments
                .iter()
                .enumerate()
                .find_map(|(index, argument)| {
                    argument.strip_prefix("--output=").or_else(|| {
                        (argument == "--output")
                            .then(|| arguments.get(index + 1).map(String::as_str))
                            .flatten()
                    })
                })
                .ok_or_else(|| {
                    Error::unknown("storyboard submit acknowledgment has no requested output path")
                        .with_stdout(output.stdout.clone())
                })?;
            validate_storyboard_submission(&receipt, expected)
                .map_err(|error| error.with_stdout(output.stdout.clone()))?;
        }
        Ok(receipt)
    }
}

/// Preserve an observed producer terminal receipt even when its CLI exits nonzero.
pub fn observed_owner(spec: &ProcessSpec) -> Result<Value> {
    let output = invoke(spec)?;
    let value = json::parse_object(&output.stdout);
    if output.status.success() {
        return value
            .map_err(|_| Error::unknown("owner response is invalid").with_stdout(output.stdout));
    }
    if let Ok(ref result) = value {
        if matches!(
            result["status"].as_str(),
            Some("failed" | "cancelled" | "outcome_unknown" | "not_started" | "source_stale")
        ) {
            return Ok(result.clone());
        }
    }
    Err(failure(output.stdout, output.stderr))
}
/// Check the public Geultu commit/candidate acknowledgment without interpreting
/// its draft digest or deciding whether the candidate may be adopted.
pub fn validate_geultu_submission(receipt: &Value, input: &Value) -> Result<()> {
    validate_geultu_commit(&receipt["commit"], &input["project_id"])?;
    let candidate = &receipt["result"]["candidate"];
    let digest = receipt["result"]["candidate_hash"].as_str();
    if input["project_id"].as_str().is_none_or(str::is_empty)
        || input["candidate_id"].as_str().is_none_or(str::is_empty)
        || input["task_id"].as_str().is_none_or(str::is_empty)
        || !input["draft"].is_object()
        || candidate["id"] != input["candidate_id"]
        || candidate["task_id"] != input["task_id"]
        || candidate["draft"] != input["draft"]
        || !digest
            .is_some_and(|digest| crate::effects::hash::is_sha256(&format!("sha256:{digest}")))
    {
        return Err(Error::unknown(
            "Geultu submission lacks the matching public commit and candidate acknowledgment",
        ));
    }
    Ok(())
}

fn validate_geultu_commit(commit: &Value, project: &Value) -> Result<()> {
    if project.as_str().is_none_or(str::is_empty)
        || commit["project_id"] != *project
        || commit["revision"].as_u64().is_none()
        || commit["event_count"].as_u64().is_none()
        || commit["idempotent"].as_bool().is_none()
    {
        return Err(Error::unknown(
            "Geultu did not confirm the matching public project commit",
        ));
    }
    Ok(())
}

/// Validate public task packet identity/shape. Cancellation and blockers are
/// owner observations; a read remains available even for a cancelled task.
pub fn validate_geultu_task(packet: &Value, task_id: &str) -> Result<()> {
    let task = &packet["task"];
    if task_id.is_empty()
        || task["id"] != task_id
        || !matches!(task["target"].as_str(), Some("story" | "synopsis"))
        || task["instruction"].as_str().is_none()
        || task["cancelled"].as_bool().is_none()
        || !task["inputs"].is_object()
        || !task["inputs"]["source"].is_object()
        || task["selection"].as_array().is_none()
        || !task
            .get("base")
            .is_some_and(|base| base.is_null() || base.is_object())
        || !task["brief"].is_object()
        || !task["guidance_hash"]
            .as_str()
            .is_some_and(|hash| crate::effects::hash::is_sha256(&format!("sha256:{hash}")))
        || task["voice_guidelines"].as_array().is_none()
        || packet["adoption_blockers"].as_array().is_none()
        || !packet["output_schema"].is_object()
    {
        return Err(Error::unknown(
            "Geultu did not confirm the public task packet identity and shape",
        ));
    }
    Ok(())
}

/// Compare the immutable public task snapshots. Cancellation and current
/// adoption blockers remain producer-owned mutable observations.
pub fn validate_geultu_task_binding(
    captured: &Value,
    current: &Value,
    task_id: &str,
) -> Result<()> {
    validate_geultu_task(captured, task_id)?;
    validate_geultu_task(current, task_id)?;
    let mut expected = captured["task"].as_object().unwrap().clone();
    let mut observed = current["task"].as_object().unwrap().clone();
    expected.remove("cancelled");
    observed.remove("cancelled");
    if expected != observed {
        return Err(Error::unknown("Geultu task immutable source/base/brief/instruction snapshot differs from the saved writer request"));
    }
    Ok(())
}

pub fn validate_geultu_preparation(receipt: &Value, input: &Value) -> Result<()> {
    validate_geultu_commit(&receipt["commit"], &input["project_id"])?;
    validate_geultu_task(
        &receipt["result"],
        input["task_id"].as_str().unwrap_or_default(),
    )?;
    if receipt["result"]["task"]["target"] != input["target"]
        || receipt["result"]["task"]["instruction"] != input["instruction"]
    {
        return Err(Error::unknown(
            "Geultu prepared task does not match the requested target and instruction",
        ));
    }
    Ok(())
}

/// STS submit receipts carry a node, output and version, rather than a digest or
/// task token. The corresponding public artifact observation confirms bytes.
pub fn validate_narrative_submission(receipt: &Value, packet: &Value) -> Result<()> {
    validate_narrative_receipt_shape(receipt)?;
    if receipt["node"] != packet["node"]
        || packet
            .get("output")
            .is_some_and(|output| *output != receipt["output"])
    {
        return Err(Error::unknown(
            "NVC submission lacks the matching public node/output/version acknowledgment",
        ));
    }
    Ok(())
}

fn validate_narrative_receipt_shape(receipt: &Value) -> Result<()> {
    if receipt["node"].as_str().is_none_or(str::is_empty)
        || receipt["output"].as_str().is_none_or(str::is_empty)
        || receipt["version"]
            .as_u64()
            .is_none_or(|version| version == 0)
        || !matches!(
            receipt["status"].as_str(),
            Some("initialized" | "in_progress" | "awaiting_human" | "paused" | "completed")
        )
    {
        return Err(Error::unknown(
            "NVC submit returned an invalid public node/output/version acknowledgment",
        ));
    }
    Ok(())
}

pub fn validate_narrative_artifact(
    observed: &Value,
    packet: &Value,
    content: &str,
    receipt: Option<&Value>,
) -> Result<()> {
    let item = &observed["item"];
    if observed["content"] != content
        || item["node"] != packet["node"]
        || item["node"].as_str().is_none_or(str::is_empty)
        || item["path"].as_str().is_none_or(str::is_empty)
        || item["version"].as_u64().is_none_or(|version| version == 0)
        || packet
            .get("output")
            .is_some_and(|output| *output != item["path"])
        || receipt.is_some_and(|receipt| {
            item["path"] != receipt["output"] || item["version"] != receipt["version"]
        })
    {
        return Err(Error::unknown(
            "NVC public artifact observation does not confirm the submitted node/version/bytes",
        ));
    }
    Ok(())
}

pub fn validate_storyboard_submission(receipt: &Value, output: &str) -> Result<()> {
    if receipt["status"] != "candidate"
        || receipt["path"] != output
        || !receipt["contentHash"]
            .as_str()
            .is_some_and(crate::effects::hash::is_sha256)
        || receipt["sceneCount"]
            .as_u64()
            .is_none_or(|count| count == 0)
        || receipt["markdown"].as_str().is_none_or(str::is_empty)
        || receipt["diff"].as_str().is_none()
        || receipt["openQuestions"].as_array().is_none()
    {
        return Err(Error::unknown(
            "NVC storyboard submission lacks its public candidate path/hash acknowledgment",
        ));
    }
    Ok(())
}

pub fn validate_storyboard_task(observed: &Value, saved: &Value) -> Result<()> {
    let hash = &observed["task"]["contentHash"];
    if observed["status"] != "ready_for_writing"
        || !hash.as_str().is_some_and(crate::effects::hash::is_sha256)
    {
        return Err(Error::unknown(
            "NVC did not confirm the public storyboard task identity",
        ));
    }
    // Old request records can lack the embedded task packet. When one exists,
    // its owner identity must still match; do not reconstruct a producer hash.
    if saved
        .get("contentHash")
        .is_some_and(|expected| expected != hash)
    {
        return Err(Error::not_applied(
            "storyboard task changed since the saved writer request",
        ));
    }
    Ok(())
}
