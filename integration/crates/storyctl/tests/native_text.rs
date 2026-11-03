//! Real public-owner synthetic authoring. No model/provider or real human approval.
use serde_json::{json, Value};
use std::{
    io::Write,
    path::{Path, PathBuf},
    process::Command,
};
use storyctl::effects::{hash, json as codec};
#[path = "support/animatic.rs"]
mod animatic;
fn storyctl_binary() -> PathBuf {
    std::env::var_os("STORY_TEST_STORYCTL")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(env!("CARGO_BIN_EXE_storyctl")))
}
const STORY:&str="폐역 기록실에서 지우는 원본 근무표를 찾는다. 원본에는 민준이 9시 10분에 역을 떠났다고 적혀 있고, 화재 신고 기록에는 9시 40분이 신고 시각으로 남아 있다. 지우는 근무표를 원본대로 바로잡고 봉투와 기록을 식탁에 둔다. 어머니가 깨어나기 전에 역 문을 잠그고 첫 열차가 오기 전에 혼자 떠난다.";
fn root() -> PathBuf {
    if let Some(selected) = std::env::var_os("STORY_TEST_RESOURCE_ROOT") {
        let selected = PathBuf::from(selected);
        assert!(
            selected.is_absolute(),
            "STORY_TEST_RESOURCE_ROOT must be absolute"
        );
        let selected = selected.canonicalize().unwrap();
        assert!(
            selected.is_dir(),
            "STORY_TEST_RESOURCE_ROOT must be a directory"
        );
        return selected;
    }
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .ancestors()
        .nth(3)
        .unwrap()
        .to_path_buf()
}
struct Flow {
    root: PathBuf,
    workspace: PathBuf,
    geultu: PathBuf,
    nvc: PathBuf,
    calls: usize,
}
impl Flow {
    fn command(&self) -> Command {
        let mut command = Command::new(storyctl_binary());
        command.args([
            "--story-root",
            self.root.to_str().unwrap(),
            "text",
            "--workspace",
            self.workspace.to_str().unwrap(),
            "--geultu-bin",
            self.geultu.to_str().unwrap(),
            "--nvc-bin",
            self.nvc.to_str().unwrap(),
        ]);
        command.env("PATH", "/story-test-no-interpreters");
        command
    }
    fn call(&mut self, args: &[&str]) -> Value {
        self.calls += 1;
        let output = self.command().args(args).output().unwrap();
        assert!(
            output.status.success(),
            "{args:?}: {}",
            String::from_utf8_lossy(&output.stderr)
        );
        codec::parse_object(&output.stdout).unwrap()
    }
    fn author(&mut self, name: &str, value: Value) -> Value {
        self.calls += 1;
        let mut child = self
            .command()
            .args(["author", name, "--input", "-"])
            .stdin(std::process::Stdio::piped())
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::piped())
            .spawn()
            .unwrap();
        child
            .stdin
            .take()
            .unwrap()
            .write_all(&codec::stable_bytes(&value).unwrap())
            .unwrap();
        let output = child.wait_with_output().unwrap();
        assert!(
            output.status.success(),
            "{name}: {}",
            String::from_utf8_lossy(&output.stderr)
        );
        codec::parse_object(&output.stdout).unwrap()
    }
}
fn fixture(node: &str) -> &'static str {
    match node {
        "step_00" => include_str!("fixtures/narrative/step_00.md"),
        "step_02" => include_str!("fixtures/narrative/step_02.md"),
        "step_06" => include_str!("fixtures/narrative/step_06.md"),
        "step_07_part1" | "step_07_part2" | "step_07_part3" | "step_07_part4" => {
            include_str!("fixtures/narrative/step_07_part1.md")
        }
        "step_07_full" | "step_08_apply" => {
            include_str!("fixtures/narrative/story_source.md")
        }
        "step_08_reverse" => include_str!("fixtures/narrative/step_08_reverse.md"),
        "step_08_verify" => include_str!("fixtures/narrative/step_08_verify.md"),
        _ => include_str!("fixtures/narrative/generic.md"),
    }
}
#[test]
fn public_native_text_to_exact_reviewed_package_without_interpreters() {
    let root = root();
    let geultu = std::env::var_os("STORY_TEST_GEULTU")
        .map(PathBuf::from)
        .unwrap_or_else(|| root.join("geultu/target/release/geultu"));
    let nvc = std::env::var_os("STORY_TEST_NVC")
        .map(PathBuf::from)
        .unwrap_or_else(|| root.join("NarrativeCourier/target/release/narrative-courier-cli"));
    if !geultu.is_file() || !nvc.is_file() {
        assert!(
            std::env::var_os("STORY_REQUIRE_OWNER_TESTS").is_none(),
            "native owner binaries required"
        );
        eprintln!("NOT_RUN: native owner binaries missing");
        return;
    }
    let temp = tempfile::tempdir().unwrap();
    let temp_root = temp.path().canonicalize().unwrap();
    let source = temp_root.join("source.md");
    std::fs::write(&source, STORY).unwrap();
    let mut f = Flow {
        root,
        workspace: temp_root.join("story"),
        geultu,
        nvc,
        calls: 0,
    };
    assert_eq!(
        f.call(&[
            "create",
            "--project",
            "native-test",
            "--title",
            "통합 E2E 이야기",
            "--source",
            source.to_str().unwrap(),
            "--actor",
            "synthetic-human"
        ])["status"],
        "source_saved"
    );
    let source_before = std::fs::read(f.workspace.join("story-project.json")).unwrap();
    f.call(&[
        "create",
        "--project",
        "native-test",
        "--title",
        "통합 E2E 이야기",
        "--source",
        source.to_str().unwrap(),
        "--actor",
        "synthetic-human",
    ]);
    assert_eq!(
        source_before,
        std::fs::read(f.workspace.join("story-project.json")).unwrap()
    );
    for (i, kind) in ["story", "synopsis"].iter().enumerate() {
        let packet = f.call(&["next"]);
        assert_eq!(packet["status"], "agent_task");
        assert_eq!(packet["packet"]["task"]["target"], *kind);
        let id = format!("draft-{i}");
        let task = packet["task_id"].as_str().unwrap().to_owned();
        f.author("authoring.submit",json!({"command_id":format!("submit-{i}"),"candidate_id":id,"task_id":task,"draft":{"blocks":[{"id":"main","heading":"통합 E2E 이야기","text":format!("## 시놉시스\n\n{STORY}")}],"change_summary":"synthetic preserved text","assumptions":[],"open_questions":[]}}));
        let candidate = f.author("authoring.candidate", json!({"id":id}));
        f.author("authoring.review",json!({"command_id":format!("review-{i}"),"candidate_id":id,"expected_candidate_hash":candidate["candidate_hash"],"accept":true,"actor":"synthetic-human","note":"test fixture only"}));
    }
    let export = f.author("authoring.export", json!({"kind":"synopsis"}));
    let handed=f.author("authoring.handoff",json!({"actor":"synthetic-human","language":"ko","expected_revision":export["revision"],"expected_markdown_sha256":export["markdown_sha256"]}));
    let handoff_root = handed["result"]["handoff_root"]
        .as_str()
        .or_else(|| handed["handoff_root"].as_str())
        .expect("handoff root")
        .to_owned();
    f.call(&[
        "import-handoff",
        "--handoff-root",
        &handoff_root,
        "--duration",
        "100",
    ]);
    let project = f.workspace.join("narrative/native-test");
    let mut tasks = 0;
    let mut gates = 0;
    let mut completed = false;
    for _ in 0..100 {
        let packet = f.call(&["scene", "agent-task"]);
        match packet["status"].as_str().unwrap() {
            "agent_task" => {
                let node = packet["node"].as_str().unwrap();
                let content = fixture(node).replace("# 통합 E2E 이야기 |", "## 통합 E2E 이야기 |");
                let path = format!("input/native-fixtures/{tasks}.md");
                let absolute = project.join(&path);
                std::fs::create_dir_all(absolute.parent().unwrap()).unwrap();
                std::fs::write(absolute, &content).unwrap();
                if tasks == 0 {
                    // Swap the project leaf after the adapter has selected its
                    // FD but before the actual STS executable dispatches.
                    use std::os::unix::fs::PermissionsExt;
                    let moved = temp_root.join("selected-nvc-project");
                    let wrapper = temp_root.join("swap-nvc-project");
                    let native = f.nvc.clone();
                    std::fs::write(
                        &wrapper,
                        format!(
                            "#!/bin/sh\nif [ \"$1\" = submit ]; then\n/bin/mv '{}' '{}'\n/bin/mkdir '{}'\nfi\nexec '{}' \"$@\"\n",
                            project.display(),
                            moved.display(),
                            project.display(),
                            native.display()
                        ),
                    )
                    .unwrap();
                    std::fs::set_permissions(&wrapper, std::fs::Permissions::from_mode(0o700))
                        .unwrap();
                    f.nvc = wrapper;
                    let output = f
                        .command()
                        .args([
                            "scene",
                            "submit",
                            &path,
                            "--task-token",
                            packet["task_token"].as_str().unwrap(),
                        ])
                        .output()
                        .unwrap();
                    f.calls += 1;
                    f.nvc = native;
                    assert!(!output.status.success());
                    assert!(String::from_utf8_lossy(&output.stderr).contains("outcome_unknown"));
                    assert!(std::fs::read_dir(&project).unwrap().next().is_none());
                    assert_eq!(
                        std::fs::read_to_string(moved.join(packet["output"].as_str().unwrap()))
                            .unwrap(),
                        content
                    );
                    std::fs::remove_dir(&project).unwrap();
                    std::fs::rename(moved, &project).unwrap();
                } else {
                    f.call(&[
                        "scene",
                        "submit",
                        &path,
                        "--task-token",
                        packet["task_token"].as_str().unwrap(),
                    ]);
                }
                tasks += 1;
            }
            "awaiting_human" => {
                f.call(&[
                    "scene",
                    "decide",
                    "approve",
                    "--gate-token",
                    packet["gate_token"].as_str().unwrap(),
                ]);
                gates += 1;
            }
            "completed" => {
                completed = true;
                break;
            }
            v => panic!("unexpected {v}: {packet}"),
        }
    }
    assert!(completed);
    assert!(tasks > 10 && gates > 0);
    let context = f.call(&[
        "storyboard",
        "prepare",
        "--task-id",
        "full",
        "--brief",
        "preserve released scene occurrences",
        "--output",
        "storyboards/tasks/full.json",
    ]);
    let mut scenes = vec![];
    for (i, scene) in context["scenes"].as_array().unwrap().iter().enumerate() {
        let sid = scene["scene_id"].as_str().unwrap();
        let refs: Vec<_> = context["semanticOccurrences"]
            .as_array()
            .unwrap()
            .iter()
            .filter(|e| e["scene_id"] == sid)
            .map(|e| json!({"semanticId":e["semantic_id"],"entryHash":e["entry_hash"]}))
            .collect();
        scenes.push(json!({"sceneId":sid,"intent":"원 씬의 사건을 보존한다","panels":[{"panelId":format!("panel-{i}"),"composition":"기록과 인물을 함께 담는다","staging":"원 씬 행동 순서대로 진행한다","sourceRefs":refs,"soundNotes":"원 대사와 소리를 보존한다","transition":"원 씬 순서대로 연결한다"}]}));
    }
    let draft = json!({"scenes":scenes,"changeSummary":"전체 씬의 의미를 보존한 합성 콘티","assumptions":[],"openQuestions":[]});
    std::fs::create_dir_all(project.join("storyboards/drafts")).unwrap();
    std::fs::write(
        project.join("storyboards/drafts/full.json"),
        codec::stable_bytes(&draft).unwrap(),
    )
    .unwrap();
    let board = f.call(&[
        "storyboard",
        "submit",
        "--task",
        "storyboards/tasks/full.json",
        "--draft",
        "storyboards/drafts/full.json",
        "--output",
        "storyboards/boards/full.json",
    ]);
    let hash = board["contentHash"].as_str().unwrap().to_owned();
    f.call(&[
        "storyboard",
        "review",
        "--board",
        "storyboards/boards/full.json",
        "--expected-hash",
        &hash,
        "--reviewer",
        "synthetic-human",
        "--decision",
        "approved",
        "--output",
        "storyboards/reviews/full.json",
    ]);
    assert_eq!(f.call(&["status"])["completion"], "complete");
    let output = temp_root.join("edition");
    let packaged = f.call(&[
        "package",
        "--board",
        "storyboards/boards/full.json",
        "--review",
        "storyboards/reviews/full.json",
        "--output",
        output.to_str().unwrap(),
    ]);
    assert_eq!(packaged["status"], "complete");
    let manifest =
        codec::parse_object(&std::fs::read(output.join("story-package.json")).unwrap()).unwrap();
    let files = manifest["files"].as_object().unwrap();
    assert_eq!(files.len(), 9);
    for (name, entry) in files {
        assert_eq!(
            entry["sha256"],
            hash::sha256_bytes(&std::fs::read(output.join(name)).unwrap())
        );
    }
    assert!(std::fs::read_to_string(output.join("screenplay.md"))
        .unwrap()
        .contains("9시 40분"));
    // A Python-v1 manifest is immutable transport data. Native export must
    // reuse its exact persisted profile without rewriting or conflicting.
    let writing = f.author("authoring.bundle", json!({}));
    let mut documents = serde_json::Map::new();
    for kind in ["source", "story", "synopsis"] {
        let doc = &writing["documents"][kind];
        documents.insert(kind.into(),json!({"path":format!("{kind}.md"),"sha256":doc["markdown_sha256"],"revision":doc["revision"]}));
    }
    let legacy = json!({"schemaVersion":"story.text-export/1","projectId":"native-test","writingRevision":writing["project_revision"],"status":"writing_documents_exported","documents":documents,"nextAction":"finish STS and use storyboard export for scenes and the reviewed text board"});
    let legacy_bytes = codec::stable_bytes(&legacy).unwrap();
    let legacy_root = temp_root.join("legacy-export");
    std::fs::create_dir(&legacy_root).unwrap();
    std::fs::write(legacy_root.join("writing-manifest.json"), &legacy_bytes).unwrap();
    f.call(&["export", "--output", legacy_root.to_str().unwrap()]);
    assert_eq!(
        std::fs::read(legacy_root.join("writing-manifest.json")).unwrap(),
        legacy_bytes
    );
    // Same file path with different bytes cannot commit a partial edition.
    let partial = temp_root.join("partial");
    std::fs::create_dir(&partial).unwrap();
    std::fs::write(partial.join("source.md"), b"different").unwrap();
    let refused = f
        .command()
        .args([
            "package",
            "--board",
            "storyboards/boards/full.json",
            "--review",
            "storyboards/reviews/full.json",
            "--output",
            partial.to_str().unwrap(),
        ])
        .output()
        .unwrap();
    assert_eq!(refused.status.code(), Some(2));
    assert!(!partial.join("story-package.json").exists());
    assert_eq!(
        std::fs::read(partial.join("source.md")).unwrap(),
        b"different"
    );
    // A descriptor identity mismatch is rejected before a real valid mutation.
    let before = f.author("authoring.status", json!({}));
    let mut child = f
        .command()
        .args(["author", "authoring.prepare", "--input", "-"])
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .unwrap();
    child.stdin.take().unwrap().write_all(&codec::stable_bytes(&json!({"expected_project_id":"different","command_id":"must-not-commit","task_id":"must-not-be-created","target":"synopsis","instruction":"synthetic mutation","expected_project_revision":before["project_revision"]})).unwrap()).unwrap();
    let refused = child.wait_with_output().unwrap();
    assert_eq!(refused.status.code(), Some(2));
    assert!(String::from_utf8_lossy(&refused.stderr).contains("not_applied"));
    assert_eq!(f.author("authoring.status", json!({})), before);
    if std::env::var("STORY_REQUIRE_MEDIA_TESTS").as_deref() == Ok("1") {
        animatic::verify(
            &f.root,
            &f.workspace,
            &temp_root,
            &storyctl_binary(),
            &f.geultu,
            &f.nvc,
        )
        .unwrap();
    } else {
        eprintln!("NOT_RUN: Story media vertical; set STORY_REQUIRE_MEDIA_TESTS=1 with explicit local media tools");
    }
    eprintln!("PASS: {} native CLI calls; {tasks} tasks; {gates} simulated gates; exact nine-file package; no interpreter PATH",f.calls);
}

#[test]
fn writer_attempt_recovery_and_concurrent_reservation_do_not_repeat_provider() {
    let root = root();
    let geultu = root.join("geultu/target/release/geultu");
    let nvc = root.join("NarrativeCourier/target/release/narrative-courier-cli");
    if !geultu.is_file() {
        assert!(
            std::env::var_os("STORY_REQUIRE_OWNER_TESTS").is_none(),
            "geultu binary required"
        );
        return;
    }
    let temp = tempfile::tempdir().unwrap();
    let path = temp.path().canonicalize().unwrap();
    let source = path.join("source.md");
    std::fs::write(&source, STORY).unwrap();
    let mut f = Flow {
        root,
        workspace: path.join("writer-story"),
        geultu,
        nvc,
        calls: 0,
    };
    f.call(&[
        "create",
        "--project",
        "writer-test",
        "--title",
        "Writer test",
        "--source",
        source.to_str().unwrap(),
        "--actor",
        "synthetic-human",
    ]);
    let log = path.join("calls.txt");
    let script = path.join("writer.sh");
    let draft = json!({"blocks":[{"id":"main","heading":"story","text":STORY}],"change_summary":"test candidate","assumptions":[],"open_questions":[]});
    let receipt = json!({"schemaVersion":"story.text-writer-result/1","attemptId":"one","status":"succeeded","content":serde_json::to_string(&draft).unwrap(),"error":null});
    let quoted = serde_json::to_string(&receipt)
        .unwrap()
        .replace('\'', "'\\''");
    std::fs::write(&script,format!("#!/bin/sh\n/bin/cat >/dev/null\nprintf 'called\\n' >> '{}'\n/bin/sleep 0.2\nprintf '%s' '{}'\n",log.display(),quoted)).unwrap();
    let args = [
        "run",
        "--attempt",
        "one",
        "--writer-program",
        "/bin/sh",
        "--writer-arg",
        script.to_str().unwrap(),
    ];
    let first = f
        .command()
        .args(args)
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .unwrap();
    let second = f.command().args(args).output().unwrap();
    let first = first.wait_with_output().unwrap();
    let mut statuses = vec![];
    for output in [first, second] {
        let value = codec::parse_object(&output.stdout)
            .unwrap_or_else(|_| panic!("{}", String::from_utf8_lossy(&output.stderr)));
        statuses.push(value["status"].as_str().unwrap().to_owned());
    }
    assert!(statuses.contains(&"candidate_stored".into()));
    assert!(statuses
        .iter()
        .all(|v| ["candidate_stored", "outcome_unknown"].contains(&v.as_str())));
    assert_eq!(std::fs::read_to_string(&log).unwrap(), "called\n");
    let persisted = f.call(&args);
    assert_eq!(persisted["status"], "candidate_stored");
    assert_eq!(persisted["providerStatus"], "succeeded");
    assert_eq!(
        f.call(&["recover", "--attempt", "one"])["status"],
        "candidate_stored"
    );
    // Test-owned deletion models a lost submission acknowledgment. The saved
    // request/response and owner command id admit replay without generation.
    std::fs::remove_file(f.workspace.join("attempts/one/result.json")).unwrap();
    assert_eq!(
        f.call(&["recover", "--attempt", "one"])["status"],
        "candidate_stored"
    );
    assert_eq!(std::fs::read_to_string(log).unwrap(), "called\n");
    // The provider's held directory selects the original workspace even after
    // its former pathname is replaced. This runs real Geultu through storyctl.
    use std::os::fd::AsRawFd;
    use std::os::unix::process::CommandExt;
    let anchor = std::fs::File::open(&f.workspace).unwrap();
    let fd = anchor.as_raw_fd();
    let expected = f.author("authoring.status", json!({}));
    let moved = path.join("moved-workspace");
    std::fs::rename(&f.workspace, &moved).unwrap();
    std::fs::create_dir(&f.workspace).unwrap();
    std::fs::write(
        f.workspace.join("story-project.json"),
        b"{\"schemaVersion\":\"story.text-project/1\",\"projectId\":\"replacement\"}",
    )
    .unwrap();
    let mut command = Command::new(storyctl_binary());
    command
        .args([
            "--story-root",
            f.root.to_str().unwrap(),
            "text",
            "--workspace",
            &format!("/dev/fd/{fd}"),
            "--geultu-bin",
            f.geultu.to_str().unwrap(),
            "author",
            "authoring.status",
            "--input",
            "-",
        ])
        .env("PATH", "/story-test-no-interpreters")
        .env("STORY_SEMI_WORKSPACE_FD", fd.to_string())
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped());
    unsafe {
        command.pre_exec(move || {
            let flags = libc::fcntl(fd, libc::F_GETFD);
            if flags < 0 || libc::fcntl(fd, libc::F_SETFD, flags & !libc::FD_CLOEXEC) < 0 {
                return Err(std::io::Error::last_os_error());
            }
            Ok(())
        });
    }
    let mut child = command.spawn().unwrap();
    child.stdin.take().unwrap().write_all(b"{}").unwrap();
    let output = child.wait_with_output().unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(codec::parse_object(&output.stdout).unwrap(), expected);
}

fn root_binding_flow(base: &Path, name: &str, source_text: &str) -> Option<Flow> {
    let root = root();
    let geultu = std::env::var_os("STORY_TEST_GEULTU")
        .map(PathBuf::from)
        .unwrap_or_else(|| root.join("geultu/target/release/geultu"));
    if !geultu.is_file() {
        assert!(
            std::env::var_os("STORY_REQUIRE_OWNER_TESTS").is_none(),
            "native Geultu binary required"
        );
        eprintln!("NOT_RUN: native Geultu binary missing");
        return None;
    }
    let source = base.join(format!("{name}.md"));
    std::fs::write(&source, source_text).unwrap();
    let mut flow = Flow {
        nvc: root.join("NarrativeCourier/target/release/narrative-courier-cli"),
        root,
        workspace: base.join(name),
        geultu,
        calls: 0,
    };
    flow.call(&[
        "create",
        "--project",
        "binding-probe",
        "--title",
        "binding fixture",
        "--source",
        source.to_str().unwrap(),
        "--actor",
        "synthetic-human",
    ]);
    assert_eq!(flow.call(&["next"])["status"], "agent_task");
    Some(flow)
}
fn waiting_writer(base: &Path) -> (PathBuf, Vec<u8>) {
    use std::os::unix::fs::PermissionsExt;
    let draft = json!({"blocks":[{"id":"main","heading":"story","text":"draft from selected source A"}],"change_summary":"fixture only","assumptions":[],"open_questions":[]});
    let response=serde_json::to_vec(&json!({"schemaVersion":"story.text-writer-result/1","attemptId":"bound","status":"succeeded","content":serde_json::to_string(&draft).unwrap(),"error":null})).unwrap();
    let writer = base.join("binding-writer");
    std::fs::write(&writer,format!("#!/bin/sh\n/bin/cat > '{}'\nprintf 'called\\n' >> '{}'\n: > '{}'\nwhile [ ! -e '{}' ]; do /bin/sleep 0.01; done\nprintf '%s' '{}'\n",base.join("captured-request.json").display(),base.join("provider-calls").display(),base.join("provider-ready").display(),base.join("release-provider").display(),String::from_utf8_lossy(&response))).unwrap();
    std::fs::set_permissions(&writer, std::fs::Permissions::from_mode(0o700)).unwrap();
    (writer, response)
}
fn wait_for_provider(base: &Path) {
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(10);
    while !base.join("provider-ready").exists() && std::time::Instant::now() < deadline {
        std::thread::sleep(std::time::Duration::from_millis(10));
    }
    assert!(
        base.join("provider-ready").exists(),
        "writer did not dispatch"
    );
}

#[test]
fn ordinary_cli_root_replacement_cannot_submit_into_a_same_id_foreign_workspace() {
    let temp = tempfile::tempdir().unwrap();
    let base = temp.path().canonicalize().unwrap();
    let Some(flow) = root_binding_flow(&base, "selected", "selected source A") else {
        return;
    };
    let Some(mut replacement) = root_binding_flow(&base, "replacement", "replacement source B")
    else {
        return;
    };
    let (writer, response) = waiting_writer(&base);
    let running = flow
        .command()
        .args([
            "run",
            "--attempt",
            "bound",
            "--writer-program",
            writer.to_str().unwrap(),
        ])
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .unwrap();
    wait_for_provider(&base);
    let moved = base.join("selected-moved");
    std::fs::rename(&flow.workspace, &moved).unwrap();
    std::fs::rename(&replacement.workspace, &flow.workspace).unwrap();
    std::fs::write(base.join("release-provider"), b"release").unwrap();
    let output = running.wait_with_output().unwrap();
    assert_eq!(output.status.code(), Some(3));
    assert_eq!(
        codec::parse_object(&output.stdout).unwrap()["status"],
        "outcome_unknown"
    );
    assert!(!String::from_utf8_lossy(&output.stdout).contains("/dev/fd/"));
    assert_eq!(
        std::fs::read(moved.join("attempts/bound/response.json")).unwrap(),
        response
    );
    assert!(!flow.workspace.join("attempts").exists());
    replacement.workspace = flow.workspace;
    let state = replacement.author("authoring.status", json!({}));
    assert!(state["pending_candidates"].as_array().unwrap().is_empty());
    assert_eq!(
        replacement.author("authoring.document", json!({"kind":"source"}))["document"]["blocks"][0]
            ["text"],
        "replacement source B"
    );
    assert_eq!(
        std::fs::read_to_string(base.join("provider-calls")).unwrap(),
        "called\n"
    );
}

#[test]
fn owner_writing_root_replacement_cannot_change_the_current_candidate_authority() {
    let temp = tempfile::tempdir().unwrap();
    let base = temp.path().canonicalize().unwrap();
    let Some(mut flow) = root_binding_flow(&base, "selected", "selected source A") else {
        return;
    };
    let Some(replacement) = root_binding_flow(&base, "replacement", "replacement source B") else {
        return;
    };
    let (writer, response) = waiting_writer(&base);
    let running = flow
        .command()
        .args([
            "run",
            "--attempt",
            "bound",
            "--writer-program",
            writer.to_str().unwrap(),
        ])
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .unwrap();
    wait_for_provider(&base);
    std::fs::rename(flow.workspace.join("writing"), base.join("writing-moved")).unwrap();
    std::fs::rename(
        replacement.workspace.join("writing"),
        flow.workspace.join("writing"),
    )
    .unwrap();
    std::fs::write(base.join("release-provider"), b"release").unwrap();
    let output = running.wait_with_output().unwrap();
    assert_eq!(
        codec::parse_object(&output.stdout).unwrap()["status"],
        "outcome_unknown"
    );
    assert!(
        flow.author("authoring.status", json!({}))["pending_candidates"]
            .as_array()
            .unwrap()
            .is_empty()
    );
    let recovered = flow
        .command()
        .args(["recover", "--attempt", "bound"])
        .output()
        .unwrap();
    assert!(String::from_utf8_lossy(&recovered.stderr).contains("outcome_unknown"));
    assert!(String::from_utf8_lossy(&recovered.stderr).contains("immutable"));
    assert!(
        flow.author("authoring.status", json!({}))["pending_candidates"]
            .as_array()
            .unwrap()
            .is_empty()
    );
    assert_eq!(
        std::fs::read(flow.workspace.join("attempts/bound/response.json")).unwrap(),
        response
    );
    assert_eq!(
        std::fs::read_to_string(base.join("provider-calls")).unwrap(),
        "called\n"
    );
}

#[test]
fn a_bound_root_attempt_parent_or_leaf_swap_cannot_dispatch_the_same_attempt_twice() {
    use std::os::fd::AsRawFd;
    use std::os::unix::process::CommandExt;
    for parent_swap in [true, false] {
        let temp = tempfile::tempdir().unwrap();
        let base = temp.path().canonicalize().unwrap();
        let Some(flow) = root_binding_flow(&base, "selected", "selected source A") else {
            return;
        };
        let (writer, response) = waiting_writer(&base);
        let anchor = std::fs::File::open(&flow.workspace).unwrap();
        let fd = anchor.as_raw_fd();
        let command = || {
            let mut c = Command::new(storyctl_binary());
            c.args([
                "--story-root",
                flow.root.to_str().unwrap(),
                "text",
                "--workspace",
                &format!("/dev/fd/{fd}"),
                "--geultu-bin",
                flow.geultu.to_str().unwrap(),
                "run",
                "--attempt",
                "bound",
                "--writer-program",
                writer.to_str().unwrap(),
            ])
            .env("STORY_SEMI_WORKSPACE_FD", fd.to_string())
            .env("PATH", "/story-test-no-interpreters");
            unsafe {
                c.pre_exec(move || {
                    let flags = libc::fcntl(fd, libc::F_GETFD);
                    if flags < 0 || libc::fcntl(fd, libc::F_SETFD, flags & !libc::FD_CLOEXEC) < 0 {
                        return Err(std::io::Error::last_os_error());
                    }
                    Ok(())
                });
            }
            c
        };
        let running = command()
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::piped())
            .spawn()
            .unwrap();
        wait_for_provider(&base);
        let old = if parent_swap {
            flow.workspace.join("attempts")
        } else {
            flow.workspace.join("attempts/bound")
        };
        let moved = flow.workspace.join(if parent_swap {
            "attempts-moved"
        } else {
            "attempt-moved"
        });
        std::fs::rename(&old, &moved).unwrap();
        if parent_swap {
            std::fs::create_dir(&old).unwrap();
        }
        let second = command().output().unwrap();
        assert_eq!(
            codec::parse_object(&second.stdout).unwrap()["status"],
            "outcome_unknown"
        );
        assert_eq!(
            std::fs::read_to_string(base.join("provider-calls")).unwrap(),
            "called\n"
        );
        std::fs::write(base.join("release-provider"), b"release").unwrap();
        let first = running.wait_with_output().unwrap();
        assert_eq!(
            codec::parse_object(&first.stdout).unwrap()["status"],
            "outcome_unknown"
        );
        let old_attempt = if parent_swap {
            moved.join("bound")
        } else {
            moved
        };
        assert_eq!(
            std::fs::read(old_attempt.join("response.json")).unwrap(),
            response
        );
        assert!(!flow.workspace.join("attempts/bound").exists());
        assert!(flow.workspace.join(".story-attempt-bound.claim").is_file());
    }
}

#[test]
fn project_descriptor_drift_cannot_route_a_saved_writer_draft_into_another_project() {
    let temp = tempfile::tempdir().unwrap();
    let base = temp.path().canonicalize().unwrap();
    let Some(mut flow) = root_binding_flow(&base, "selected", "selected source A") else {
        return;
    };
    let packet = flow.call(&["next"]);
    let owner = |operation: &str, input: Value| {
        let mut child = Command::new(&flow.geultu)
            .args([
                "--workspace",
                flow.workspace.join("writing").to_str().unwrap(),
                "call",
                operation,
                "-",
            ])
            .stdin(std::process::Stdio::piped())
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::piped())
            .spawn()
            .unwrap();
        child
            .stdin
            .take()
            .unwrap()
            .write_all(&codec::stable_bytes(&input).unwrap())
            .unwrap();
        let out = child.wait_with_output().unwrap();
        assert!(
            out.status.success(),
            "{}",
            String::from_utf8_lossy(&out.stderr)
        );
        codec::parse_object(&out.stdout).unwrap()["result"].clone()
    };
    owner(
        "project.create",
        json!({"project_id":"binding-other","title":"B","command_id":"create-b"}),
    );
    owner(
        "authoring.start",
        json!({"project_id":"binding-other","command_id":"start-b","source":"replacement source B","actor":"synthetic-human"}),
    );
    let status = owner("authoring.status", json!({"project_id":"binding-other"}));
    owner(
        "authoring.prepare",
        json!({"project_id":"binding-other","command_id":"prepare-b","task_id":packet["task_id"],"target":"story","instruction":packet["packet"]["task"]["instruction"],"expected_project_revision":status["project_revision"]}),
    );
    let (writer, response) = waiting_writer(&base);
    let running = flow
        .command()
        .args([
            "run",
            "--attempt",
            "bound",
            "--writer-program",
            writer.to_str().unwrap(),
        ])
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .unwrap();
    wait_for_provider(&base);
    let descriptor = flow.workspace.join("story-project.json");
    let original = std::fs::read(&descriptor).unwrap();
    let mut changed = codec::parse_object(&original).unwrap();
    changed["projectId"] = json!("binding-other");
    std::fs::write(&descriptor, codec::stable_bytes(&changed).unwrap()).unwrap();
    std::fs::write(base.join("release-provider"), b"release").unwrap();
    let output = running.wait_with_output().unwrap();
    assert_eq!(output.status.code(), Some(3));
    assert_eq!(
        codec::parse_object(&output.stdout).unwrap()["status"],
        "source_changed"
    );
    assert!(
        owner("authoring.status", json!({"project_id":"binding-other"}))["pending_candidates"]
            .as_array()
            .unwrap()
            .is_empty()
    );
    assert_eq!(
        std::fs::read(flow.workspace.join("attempts/bound/response.json")).unwrap(),
        response
    );
    let request = codec::parse_object(
        &std::fs::read(flow.workspace.join("attempts/bound/request.json")).unwrap(),
    )
    .unwrap();
    assert_eq!(request["task"]["storyProjectId"], "binding-probe");
    let refused = flow
        .command()
        .args(["recover", "--attempt", "bound"])
        .output()
        .unwrap();
    assert!(String::from_utf8_lossy(&refused.stderr).contains("source_changed"));
    std::fs::write(descriptor, original).unwrap();
    assert_eq!(
        flow.call(&["recover", "--attempt", "bound"])["status"],
        "candidate_stored"
    );
    assert_eq!(
        std::fs::read_to_string(base.join("provider-calls")).unwrap(),
        "called\n"
    );
}
