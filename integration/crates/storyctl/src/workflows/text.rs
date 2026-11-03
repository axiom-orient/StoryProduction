//! User text workflows; source and approval authority remain at public owners.
use crate::{
    adapters::owners::{
        path_string, validate_geultu_preparation, validate_geultu_submission, validate_geultu_task,
        validate_geultu_task_binding, validate_narrative_artifact, validate_narrative_submission,
        validate_storyboard_submission, validate_storyboard_task, BoundNarrativeOwner,
        BoundWritingOwner, NarrativeDirectoryBinding,
    },
    effects::{files, hash, json, workspace},
    error::{Error, Result},
    ports::{NarrativeOwner, WritingOwner},
};
use serde_json::{json, Value};
use std::{
    path::{Path, PathBuf},
    sync::{Arc, OnceLock},
};
use story_integration_core::{Outcome, ProjectId};

pub struct TextContext {
    pub repository: PathBuf,
    pub workspace: PathBuf,
    pub writing: Box<dyn WritingOwner>,
    pub narrative: Box<dyn NarrativeOwner>,
    pub narrative_project: Option<ProjectId>,
    binding: Arc<workspace::WorkspaceBinding>,
    project_identity: OnceLock<ProjectId>,
    narrative_binding: Arc<NarrativeDirectoryBinding>,
}
impl TextContext {
    pub fn new(
        repository: PathBuf,
        workspace_path: PathBuf,
        geultu: PathBuf,
        nvc: PathBuf,
        project: Option<String>,
    ) -> Result<Self> {
        let binding = Arc::new(workspace::WorkspaceBinding::new(&workspace_path)?);
        let root = binding.display_path().to_path_buf();
        let writing = Box::new(BoundWritingOwner {
            executable: geultu,
            workspace: binding.clone(),
            repository: repository.clone(),
            directory: OnceLock::new(),
        });
        let narrative_binding = Arc::new(NarrativeDirectoryBinding::new(binding.clone()));
        let narrative = Box::new(BoundNarrativeOwner {
            executable: nvc,
            repository: repository.clone(),
            binding: narrative_binding.clone(),
        });
        let narrative_project = project
            .map(|v| ProjectId::new(&v).map_err(Error::not_applied))
            .transpose()?;
        Ok(Self {
            repository,
            workspace: root,
            writing,
            narrative,
            narrative_project,
            binding,
            project_identity: OnceLock::new(),
            narrative_binding,
        })
    }
    pub fn with_owners(
        repository: PathBuf,
        workspace_path: PathBuf,
        writing: Box<dyn WritingOwner>,
        narrative: Box<dyn NarrativeOwner>,
        project: Option<String>,
    ) -> Result<Self> {
        let binding = Arc::new(workspace::WorkspaceBinding::new(&workspace_path)?);
        let narrative_project = project
            .map(|value| ProjectId::new(&value).map_err(Error::not_applied))
            .transpose()?;
        let narrative_binding = Arc::new(NarrativeDirectoryBinding::new(binding.clone()));
        Ok(Self {
            repository,
            workspace: binding.display_path().to_path_buf(),
            writing,
            narrative,
            narrative_project,
            binding,
            project_identity: OnceLock::new(),
            narrative_binding,
        })
    }
    pub(crate) fn workspace_directory(&self) -> Result<Arc<files::Directory>> {
        self.binding.directory()
    }
    pub(crate) fn ensure_workspace_current(&self) -> Result<()> {
        self.binding.ensure_current()
    }
    pub fn descriptor(&self) -> Result<Value> {
        let bytes = self
            .workspace_directory()?
            .read(Path::new("story-project.json"))?;
        let value = json::parse_object(&bytes)?;
        if value["schemaVersion"] != "story.text-project/1" {
            return Err(Error::not_applied("unsupported Story text project"));
        }
        let identity = ProjectId::new(string(&value, "projectId")?).map_err(Error::not_applied)?;
        let expected = self.project_identity.get_or_init(|| identity.clone());
        if *expected != identity {
            return Err(project_changed(
                "Story project identity changed during this invocation",
            ));
        }
        Ok(value)
    }
    pub fn project_id(&self) -> Result<String> {
        Ok(string(&self.descriptor()?, "projectId")?.into())
    }
    pub fn nvc_id(&self) -> Result<String> {
        self.narrative_project
            .as_ref()
            .map(|v| Ok(v.as_str().into()))
            .unwrap_or_else(|| self.project_id())
    }
    pub(crate) fn nvc_directory(&self) -> Result<Arc<files::Directory>> {
        self.narrative_binding.project(&self.nvc_id()?)
    }
    pub(crate) fn nvc_directory_optional(&self) -> Result<Option<Arc<files::Directory>>> {
        self.narrative_binding.project_optional(&self.nvc_id()?)
    }
    pub fn nvc_root(&self) -> Result<PathBuf> {
        Ok(self.workspace.join("narrative").join(self.nvc_id()?))
    }
    pub fn author(&self, name: &str, mut arguments: Value) -> Result<Value> {
        if !name.starts_with("authoring.")
            && name != "voice.profile.create"
            && name != "voice.profile.get"
        {
            return Err(Error::not_applied(
                "use an authoring or voice profile operation",
            ));
        }
        let project = self.project_id()?;
        let object = arguments
            .as_object_mut()
            .ok_or_else(|| Error::not_applied("author input must be an object"))?;
        if let Some(expected) = object.remove("expected_project_id") {
            if !expected.is_null() && expected != project {
                return Err(Error::not_applied(
                    "StoryProduction project identity changed before owner operation",
                ));
            }
        }
        object.insert("project_id".into(), Value::String(project));
        let result = self.writing.call(name, &arguments)?;
        self.project_id()?;
        Ok(result)
    }
    pub fn narrative_call(&self, name: &str, extra: &[String]) -> Result<Value> {
        self.project_id()?;
        let raw = self.narrative.text(name, &self.nvc_id()?, extra)?;
        self.project_id()?;
        if name == "cancel" {
            return Ok(json!({"status":"cancel_requested","message":raw.trim()}));
        }
        json::parse_object(raw.as_bytes()).map_err(|_| {
            Error::unknown("NarrativeCourier returned invalid response; inspect owner state")
                .with_stdout(raw.into_bytes())
        })
    }
    pub fn source_current(&self) -> Result<bool> {
        match self.source_current_observation() {
            Err(error) if error.outcome == Outcome::SourceChanged => Ok(false),
            result => result,
        }
    }
    fn source_current_observation(&self) -> Result<bool> {
        let expected = self.author("authoring.export", json!({"kind":"synopsis"}))?;
        let expected_hash = string(&expected, "markdown_sha256")
            .map_err(|_| Error::unknown("writing owner did not return a synopsis hash"))?;
        if !hash::is_sha256(&format!("sha256:{expected_hash}")) {
            return Err(Error::unknown(
                "writing owner returned an invalid synopsis hash",
            ));
        }
        let binding = self.narrative_call("source-binding", &[])?;
        let reviewed = binding
            .get("reviewed_synopsis")
            .ok_or_else(|| Error::unknown("NVC did not return a reviewed synopsis observation"))?;
        if reviewed.is_null() {
            return Ok(false);
        }
        if !reviewed.is_object() {
            return Err(Error::unknown(
                "NVC reviewed synopsis observation is invalid",
            ));
        }
        let bound_hash = string(reviewed, "synopsis_sha256")
            .map_err(|_| Error::unknown("NVC source binding lacks a synopsis hash"))?;
        let bound_project = string(reviewed, "source_project_id")
            .map_err(|_| Error::unknown("NVC source binding lacks a project identity"))?;
        if !hash::is_sha256(&format!("sha256:{bound_hash}")) {
            return Err(Error::unknown(
                "NVC source binding has an invalid synopsis hash",
            ));
        }
        Ok(bound_project == self.project_id()? && bound_hash == expected_hash)
    }
    pub fn board(&self, options: &[String]) -> Result<Value> {
        if options.first().map(String::as_str) != Some("cancel") && !self.source_current()? {
            return Err(Error::not_applied("writing synopsis differs from selected STS source; choose current narrative project"));
        }
        let result = self
            .narrative
            .tool("storyboard", options, &self.nvc_root()?)?;
        self.project_id()?;
        Ok(result)
    }
}
pub(crate) fn project_changed(message: impl Into<String>) -> Error {
    let mut error = Error::not_applied(message);
    error.outcome = Outcome::SourceChanged;
    error
}
pub fn string<'a>(value: &'a Value, key: &str) -> Result<&'a str> {
    value
        .get(key)
        .and_then(Value::as_str)
        .ok_or_else(|| Error::not_applied(format!("missing or invalid {key}")))
}
fn array<'a>(value: &'a Value, key: &str) -> Result<&'a Vec<Value>> {
    value
        .get(key)
        .and_then(Value::as_array)
        .ok_or_else(|| Error::unknown(format!("owner response lacks {key}")))
}
pub fn create(
    ctx: &TextContext,
    project: &str,
    title: &str,
    source_path: &Path,
    actor: &str,
    language: &str,
) -> Result<Value> {
    ProjectId::new(project).map_err(Error::not_applied)?;
    let source = files::read_text(source_path)?;
    if [source.as_str(), title, actor, language]
        .iter()
        .any(|v| v.trim().is_empty())
    {
        return Err(Error::not_applied(
            "source, title, actor and language must not be empty",
        ));
    }
    let request = json!({"project_id":project,"command_id":"story-text-source","source":source,"actor":actor});
    if json::stable_bytes(&request)?.len() > crate::effects::MAX_BYTES {
        return Err(Error::not_applied(
            "encoded source request exceeds 16 MiB; no project initialized",
        ));
    }
    let descriptor = json!({"schemaVersion":"story.text-project/1","projectId":project,"title":title,"language":language,"intakeActor":actor,"intakeSourceSha256":hash::sha256_hex_bytes(source.as_bytes()),"owners":{"writing":"geultu","scenes":"NarrativeCourier","storyboard":"NarrativeCourier"}});
    ctx.binding.create()?.immutable(
        Path::new("story-project.json"),
        &json::stable_bytes(&descriptor)?,
    )?;
    ctx.writing.initialize()?;
    ctx.writing.call(
        "project.create",
        &json!({"project_id":project,"title":title,"command_id":"story-text-create"}),
    )?;
    ctx.author("authoring.start",json!({"command_id":"story-text-source","source":source,"actor":actor,"expected_project_id":project}))?;
    let current=ctx.author("authoring.document",json!({"kind":"source","expected_project_id":project})).map_err(|_|Error::unknown("initialization returned but current source could not be confirmed; inspect workspace"))?;
    if !current["document"].is_object()
        || current["document"]["kind"] != "source"
        || !current["document"]["blocks"].is_array()
    {
        return Err(Error::unknown(
            "writing owner did not confirm source document",
        ));
    }
    let status = if current["document"]["blocks"]
        == json!([{"id":"source","heading":"원문","text":source}])
    {
        "source_saved"
    } else {
        "source_changed"
    };
    Ok(
        json!({"status":status,"project":path_string(&ctx.workspace.join("story-project.json"))?,"next":"storyctl text --workspace PATH next"}),
    )
}
pub fn approved_board(ctx: &TextContext) -> Result<Option<Value>> {
    let project = ctx.nvc_directory()?;
    let Some(boards) = project.child_optional("storyboards/boards")? else {
        return Ok(None);
    };
    let Some(reviews) = project.child_optional("storyboards/reviews")? else {
        return Ok(None);
    };
    let mut board_paths = std::collections::BTreeMap::new();
    for path in boards.list_json_files()? {
        if let Ok(board) = boards.read(&path).and_then(|b| json::parse_object(&b)) {
            if let Some(h) = board["contentHash"].as_str() {
                board_paths
                    .entry(h.to_owned())
                    .or_insert(Path::new("storyboards/boards").join(path));
            }
        }
    }
    for path in reviews.list_json_files()? {
        let receipt = match reviews.read(&path).and_then(|b| json::parse_object(&b)) {
            Ok(v) => v,
            Err(_) => continue,
        };
        if receipt["decision"] != "approved" {
            continue;
        }
        let Some(board) = receipt["boardHash"]
            .as_str()
            .and_then(|h| board_paths.get(h))
        else {
            continue;
        };
        let options = vec![
            "bundle".into(),
            "--board".into(),
            path_string(board)?,
            "--review".into(),
            path_string(&Path::new("storyboards/reviews").join(path))?,
        ];
        let release = match ctx.board(&options) {
            Ok(v) => v,
            Err(e) if e.outcome == Outcome::NotApplied => continue,
            Err(e) => return Err(e),
        };
        if release["status"] == "approved_text_package" {
            super::publication::validate_narrative_release(ctx, &release)?;
            return Ok(Some(
                json!({"status":"approved","boardHash":release["boardHash"],"source":release["source"],"boardPath":options[2],"reviewPath":options[4]}),
            ));
        }
    }
    Ok(None)
}
pub fn status(ctx: &TextContext) -> Result<Value> {
    let author = ctx.author("authoring.status", json!({}))?;
    let started = ctx.nvc_directory_optional()?.is_some();
    let narrative = if started {
        ctx.narrative_call("agent-task", &[])?
    } else {
        json!({"status":"not_started"})
    };
    let current = if started {
        Some(
            array(&author, "documents")?
                .iter()
                .all(|d| d["status"] == "current")
                && ctx.source_current()?,
        )
    } else {
        None
    };
    let review = if narrative["status"] == "completed" && current == Some(true) {
        approved_board(ctx)?
    } else {
        None
    };
    let completion = if current == Some(false) {
        "source_changed"
    } else if review.is_some() {
        "complete"
    } else if narrative["status"] == "completed" {
        "requires_storyboard_review"
    } else {
        "in_progress"
    };
    Ok(
        json!({"project":ctx.descriptor()?,"authoring":author,"narrative":narrative,"narrativeSourceCurrent":current,"storyboardReview":review,"completion":completion}),
    )
}
pub fn next(ctx: &TextContext, instruction: &str) -> Result<Value> {
    let state = ctx.author("authoring.status", json!({}))?;
    for candidate in array(&state, "pending_candidates")? {
        if candidate["adoption_blockers"]
            .as_array()
            .is_some_and(Vec::is_empty)
        {
            return Ok(
                json!({"owner":"geultu","status":"awaiting_human","candidate":ctx.author("authoring.candidate",json!({"id":candidate["candidate_id"]}))?}),
            );
        }
    }
    if let Some(target) = state["next"]["kind"].as_str().filter(|s| !s.is_empty()) {
        if target == "source" {
            return Err(Error::not_applied(
                "project intake incomplete; resume original create command",
            ));
        }
        for task in array(&state, "tasks")?.iter().rev() {
            if task["target"] != target
                || task["cancelled"] == true
                || task["has_candidates"] == true
            {
                continue;
            }
            let packet = ctx.author("authoring.task", json!({"id":task["id"]}))?;
            validate_geultu_task(&packet, string(task, "id")?)?;
            if packet["task"]["target"] != target {
                return Err(Error::unknown(
                    "Geultu task packet target differs from its status observation",
                ));
            }
            if packet["task"]["cancelled"] == false
                && packet["adoption_blockers"]
                    .as_array()
                    .is_some_and(Vec::is_empty)
                && (instruction.is_empty() || packet["task"]["instruction"] == instruction)
            {
                return Ok(
                    json!({"owner":"geultu","status":"agent_task","task_id":task["id"],"packet":packet}),
                );
            }
        }
        let revision = state["project_revision"]
            .as_u64()
            .ok_or_else(|| Error::unknown("owner project_revision invalid"))?;
        let id = format!("{target}-{revision}");
        let brief = if !instruction.is_empty() {
            instruction
        } else if target == "story" {
            "원문의 의미와 작가의 문체를 보존해 실제 읽히는 완결된 스토리를 작성한다."
        } else {
            "채택한 스토리의 인과·인물·반전·결말을 보존한 제작용 시놉시스를 작성한다."
        };
        let project = ctx.project_id()?;
        let input = json!({"project_id":project,"expected_project_id":project,
            "command_id":format!("prepare-{id}"),"task_id":id,"target":target,
            "instruction":brief,"expected_project_revision":revision});
        let result = ctx.author("authoring.prepare", input.clone())?;
        validate_geultu_preparation(&result, &input)?;
        if result["result"]["task"]["cancelled"] != false
            || !result["result"]["adoption_blockers"]
                .as_array()
                .is_some_and(Vec::is_empty)
        {
            return Err(Error::not_applied(
                "prepared Geultu task is cancelled or blocked; inspect current owner state",
            ));
        }
        return Ok(
            json!({"owner":"geultu","status":"agent_task","task_id":id,"packet":result["result"]}),
        );
    }
    if ctx.nvc_directory_optional()?.is_none() {
        return Ok(
            json!({"owner":"geultu","status":"awaiting_handoff","synopsis":ctx.author("authoring.export",json!({"kind":"synopsis"}))?,"nextAction":"human authoring.handoff with the shown revision and markdown_sha256, then import-handoff"}),
        );
    }
    if !ctx.source_current()? {
        return Ok(
            json!({"owner":"NarrativeCourier","status":"source_changed","nextAction":"handoff current synopsis into new --narrative-project; previous project is preserved"}),
        );
    }
    let mut packet = ctx.narrative_call("agent-task", &[])?;
    packet
        .as_object_mut()
        .ok_or_else(|| Error::unknown("NVC packet is not object"))?
        .insert("owner".into(), json!("NarrativeCourier"));
    if packet["status"] == "agent_task" {
        let text = ctx.narrative.text("next", &ctx.nvc_id()?, &[])?;
        if ctx.narrative_call("agent-task", &[])?["task_token"] != packet["task_token"] {
            return Err(Error::not_applied(
                "STS task changed while preparing text; read next again",
            ));
        }
        packet["task_text"] = json!(text);
    }
    if packet["status"] == "completed" {
        if let Some(review) = approved_board(ctx)? {
            packet["nextAction"] = json!(format!(
                "package --board {} --review {} --output PATH",
                string(&review, "boardPath")?,
                string(&review, "reviewPath")?
            ));
            packet["storyboardReview"] = review;
        } else {
            packet["nextAction"] =
                json!("storyboard prepare; screenplay completion is not storyboard completion");
        }
    }
    Ok(packet)
}
pub fn submit(ctx: &TextContext, packet: &Value, content: &str, attempt: &str) -> Result<Value> {
    if packet["status"] != "agent_task" {
        return Err(Error::not_applied("packet is not writing task"));
    }
    match string(packet, "owner")? {
        "geultu" => {
            let current = ctx.author("authoring.task", json!({"id":packet["task_id"]}))?;
            validate_geultu_task_binding(&packet["packet"], &current, string(packet, "task_id")?)?;
            let project = ctx.project_id()?;
            let input = json!({"project_id":project,"expected_project_id":project,
                "command_id":format!("submit-{attempt}"),"candidate_id":format!("draft-{attempt}"),
                "task_id":packet["task_id"],"draft":json::parse_object(content.as_bytes())?});
            let receipt = ctx.author("authoring.submit", input.clone())?;
            validate_geultu_submission(&receipt, &input)?;
            Ok(receipt)
        }
        "NarrativeCourierStoryboard" => {
            if !ctx.source_current()? {
                return Err(Error::not_applied(
                    "writing synopsis changed before storyboard submission",
                ));
            }
            json::parse_object(content.as_bytes())?;
            let task = string(packet, "task_path")?;
            let observed_task = ctx.board(&["task".into(), "--task".into(), task.into()])?;
            validate_storyboard_task(&observed_task, &packet["packet"]["task"])?;
            let path = format!("storyboards/drafts/{attempt}.json");
            let project = ctx.nvc_directory()?;
            project.immutable(&path, content.as_bytes())?;
            ctx.nvc_directory().map_err(|error| {
                Error::unknown(format!(
                    "NVC project changed after draft publication: {error}"
                ))
            })?;
            let output = format!("storyboards/boards/{attempt}.json");
            let receipt = ctx.board(&[
                "submit".into(),
                "--task".into(),
                task.into(),
                "--draft".into(),
                path,
                "--output".into(),
                output.clone(),
            ])?;
            validate_storyboard_submission(&receipt, &output)?;
            Ok(receipt)
        }
        "NarrativeCourier" => {
            if !ctx.source_current()? {
                return Err(Error::not_applied(
                    "writing synopsis changed before scene submission",
                ));
            }
            let path = format!("input/story-agent/{attempt}.md");
            let project = ctx.nvc_directory()?;
            project.immutable(&path, content.as_bytes())?;
            ctx.nvc_directory().map_err(|error| {
                Error::unknown(format!(
                    "NVC project changed after draft publication: {error}"
                ))
            })?;
            let receipt = ctx.narrative_call(
                "submit",
                &[
                    path,
                    "--task-token".into(),
                    string(packet, "task_token")?.into(),
                ],
            )?;
            validate_narrative_submission(&receipt, packet)?;
            let observed = ctx
                .narrative_call("artifact", &[string(packet, "node")?.into()])
                .map_err(|error| {
                    Error::unknown(format!(
                        "NVC submission was acknowledged but artifact observation failed: {error}"
                    ))
                })?;
            validate_narrative_artifact(&observed, packet, content, Some(&receipt))?;
            Ok(receipt)
        }
        _ => Err(Error::not_applied("unsupported writing task owner")),
    }
}
pub fn handoff(ctx: &TextContext, root: &Path, duration: u32) -> Result<Value> {
    if !(1..=1440).contains(&duration) {
        return Err(Error::not_applied("duration must be in 1..1440 minutes"));
    }
    let project = ctx.project_id()?;
    let source = ctx.author(
        "authoring.export",
        json!({"kind":"synopsis","expected_project_id":project}),
    )?;
    let root = files::non_symlink_path(root)?;
    let bytes = files::read_bytes(&root.join("reviewed-synopsis-handoff.v1.json"))?;
    let handoff = json::parse_object(&bytes)?;
    if handoff["project_id"] != project
        || handoff["source_revision"] != source["project_revision"]
        || handoff["synopsis_sha256"] != source["markdown_sha256"]
        || handoff["synopsis_path"] != "synopsis.md"
    {
        return Err(Error::not_applied(
            "handoff does not belong to current reviewed synopsis",
        ));
    }
    let synopsis = files::read_bytes(&root.join("synopsis.md"))?;
    if synopsis != string(&source, "markdown")?.as_bytes()
        || handoff["synopsis_sha256"] != hash::sha256_hex_bytes(&synopsis)
    {
        return Err(Error::not_applied(
            "handoff synopsis bytes differ from current writing authority",
        ));
    }
    let handoff_hash = hash::sha256_hex_bytes(&bytes);
    let temp = tempfile::Builder::new()
        .prefix("story-handoff-")
        .tempdir()?;
    // The OS temp root may be a known alias (/var on Darwin). Resolve only our
    // newly created private directory, never an untrusted producer input.
    let captured = temp.path().canonicalize()?;
    files::immutable(&captured.join("synopsis.md"), &synopsis)?;
    files::immutable(&captured.join("reviewed-synopsis-handoff.v1.json"), &bytes)?;
    if ctx.author(
        "authoring.export",
        json!({"kind":"synopsis","expected_project_id":project}),
    )? != source
    {
        return Err(Error::not_applied(
            "writing changed while capturing handoff",
        ));
    }
    let result = ctx
        .narrative_call(
            "import-reviewed-synopsis",
            &[
                "--handoff-root".into(),
                path_string(&captured)?,
                "--handoff".into(),
                "reviewed-synopsis-handoff.v1.json".into(),
                "--duration".into(),
                duration.to_string(),
            ],
        )
        .map_err(|mut error| {
            error.message = format!(
                "inspect selected narrative project for handoff {handoff_hash} before retrying: {}",
                error.message
            );
            error
        })?;
    let observed = (|| -> Result<Value> {
        let binding = ctx.narrative_call("source-binding", &[])?;
        let b = &binding["reviewed_synopsis"];
        if !b.is_object()
            || b["handoff_sha256"] != handoff_hash
            || b["source_project_id"] != project
            || b["source_revision"] != handoff["source_revision"]
            || b["synopsis_sha256"] != handoff["synopsis_sha256"]
        {
            return Err(Error::unknown(
                "NVC did not confirm captured handoff binding",
            ));
        }
        ctx.author(
            "authoring.export",
            json!({"kind":"synopsis","expected_project_id":project}),
        )
    })();
    let current = observed.map_err(|error| Error::unknown(format!("NVC import was confirmed but its binding/current-source observation failed; inspect handoff {handoff_hash}: {error}")))?;
    temp.close().map_err(|e| {
        Error::unknown(format!(
            "NVC import dispatched but captured cleanup not confirmed: {e}"
        ))
    })?;
    if current != source {
        Ok(
            json!({"status":"source_changed","result":result,"nextAction":"imported project preserved; review current synopsis and import into new narrative project"}),
        )
    } else {
        Ok(result)
    }
}
