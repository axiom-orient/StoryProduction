#[path = "../../../test_support/sts_workflow_fixture.rs"]
mod sts_workflow_fixture;

use narrative_courier_contracts::{
    Coverage, DeliveryIntent, DeliveryRequest, OutputFormat, OutputRequest,
};
use narrative_courier_mcp::StsMcp;
use narrative_courier_runtime::token::hash_jcs;
use rmcp::{
    model::{CallToolRequestParams, JsonObject},
    service::ServiceExt,
};
use serde_json::json;
use std::fs;
use std::path::{Path, PathBuf};
use sts_workflow::{Engine, NewProject, NextPacket};
use tempfile::TempDir;

fn copy_tree(src: &Path, dst: &Path) {
    fs::create_dir_all(dst).unwrap();
    for entry in fs::read_dir(src).unwrap() {
        let entry = entry.unwrap();
        let target = dst.join(entry.file_name());
        if entry.file_type().unwrap().is_dir() {
            copy_tree(&entry.path(), &target);
        } else {
            fs::copy(entry.path(), target).unwrap();
        }
    }
}

fn fixture_root(tmp: &Path) -> Engine {
    let source = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");
    copy_tree(&source.join("skills"), &tmp.join("skills"));
    Engine::new(tmp).expect("fixture roots resolve")
}

fn drive_to_completed(engine: &Engine, name: &str) {
    for _ in 0..200 {
        match engine.next(name).unwrap() {
            NextPacket::AgentTask {
                node, task_token, ..
            } => {
                engine
                    .submit(name, sts_workflow_fixture::artifact_for(&node), &task_token)
                    .unwrap();
            }
            NextPacket::AwaitingHuman { gate_token, .. } => {
                engine
                    .decide(name, "approve", None, None, &gate_token)
                    .unwrap();
            }
            NextPacket::Paused => panic!("project paused unexpectedly"),
            NextPacket::Completed { .. } => return,
        }
    }
    panic!("workflow did not complete within step budget");
}

fn completed_project() -> (TempDir, Engine, String) {
    let tmp = TempDir::new().unwrap();
    let root = tmp.path().to_path_buf();
    let engine = fixture_root(&root);
    let name = "demo".to_string();
    engine
        .create_project(&NewProject {
            project_name: name.clone(),
            title: "테스트".into(),
            synopsis: sts_workflow_fixture::SYNOPSIS.into(),
            source_format: "original".into(),
            target_duration: 100,
            genre: vec!["드라마".into()],
            tone: "절제".into(),
            theme_direction: "오해와 책임".into(),
            references: vec![],
        })
        .unwrap();
    drive_to_completed(&engine, &name);
    (tmp, engine, name)
}

fn args(pairs: Vec<(&str, serde_json::Value)>) -> JsonObject {
    let mut map = JsonObject::new();
    for (k, v) in pairs {
        map.insert(k.to_string(), v);
    }
    map
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn mcp_surface_exposes_read_and_mechanical_tools_only() {
    let (_tmp, _engine, _name) = completed_project();
    let server = StsMcp::new(
        Engine::with_roots(
            PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../.."),
            _tmp.path().to_path_buf(),
        )
        .expect("fixture roots resolve"),
    );
    let (client_transport, server_transport) = tokio::io::duplex(256 * 1024);
    let server_task = tokio::spawn(async move {
        server
            .serve(server_transport)
            .await
            .unwrap()
            .waiting()
            .await
            .unwrap();
    });
    let client = ().serve(client_transport).await.unwrap();

    let tools = client.list_tools(None).await.unwrap();
    let names: Vec<&str> = tools.tools.iter().map(|t| t.name.as_ref()).collect();
    let expected = [
        "list_projects",
        "project_status",
        "doctor",
        "delivery_context",
        "list_deliveries",
        "run_delivery",
        "cancel_delivery",
        "import_screenplay",
        "import_verified_synopsis",
        "export_screenplay",
    ];
    for name in expected {
        assert!(names.contains(&name), "missing tool {name}: {names:?}");
    }
    assert!(
        !names
            .iter()
            .any(|n| n.contains("decide") || n.contains("review") || n.contains("approve")),
        "human-only operations must not be exposed as tools: {names:?}"
    );

    let annotations_of = |name: &str| {
        tools
            .tools
            .iter()
            .find(|t| t.name == name)
            .and_then(|t| t.annotations.clone())
            .unwrap_or_else(|| panic!("tool {name} lacks annotations"))
    };
    let ro = annotations_of("list_projects");
    assert_eq!(ro.read_only_hint, Some(true));
    assert_eq!(ro.destructive_hint, Some(false));
    assert_eq!(ro.open_world_hint, Some(false));
    let run = annotations_of("run_delivery");
    assert_eq!(run.read_only_hint, Some(false));
    assert_eq!(run.destructive_hint, Some(false));
    assert_eq!(run.idempotent_hint, Some(true));
    let cancel = annotations_of("cancel_delivery");
    assert_eq!(cancel.read_only_hint, Some(false));
    assert_eq!(cancel.destructive_hint, Some(true));
    let import = annotations_of("import_screenplay");
    assert_eq!(import.read_only_hint, Some(false));
    assert_eq!(import.destructive_hint, Some(false));
    assert_eq!(import.idempotent_hint, Some(false));
    let export = annotations_of("export_screenplay");
    assert_eq!(export.read_only_hint, Some(false));
    assert_eq!(export.destructive_hint, Some(false));
    assert_eq!(export.idempotent_hint, Some(true));
    assert!(
        export.title.as_deref().unwrap_or("").contains("Export"),
        "tool title must be human-readable"
    );

    client.cancel().await.unwrap();
    server_task.await.unwrap();
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn mcp_delivers_html_for_released_project_end_to_end() {
    let (_tmp, engine, name) = completed_project();
    let projects_root = engine
        .project_dir(&name)
        .unwrap()
        .parent()
        .unwrap()
        .to_path_buf();
    let assets_root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");

    let server =
        StsMcp::new(Engine::with_roots(assets_root, projects_root).expect("fixture roots resolve"));
    let (client_transport, server_transport) = tokio::io::duplex(512 * 1024);
    let server_task = tokio::spawn(async move {
        server
            .serve(server_transport)
            .await
            .unwrap()
            .waiting()
            .await
            .unwrap();
    });
    let client = ().serve(client_transport).await.unwrap();

    let listed = client
        .call_tool(CallToolRequestParams::new("list_projects"))
        .await
        .unwrap();
    let listed_text = match listed.content.first().cloned().unwrap() {
        rmcp::model::ContentBlock::Text(text) => text.text,
        other => panic!("unexpected content: {other:?}"),
    };
    assert!(listed_text.contains("\"demo\""), "{listed_text}");

    let status = client
        .call_tool(
            CallToolRequestParams::new("doctor")
                .with_arguments(args(vec![("project", json!("demo"))])),
        )
        .await
        .unwrap();
    assert!(!status.is_error.unwrap_or(false));

    let context = client
        .call_tool(
            CallToolRequestParams::new("delivery_context")
                .with_arguments(args(vec![("project", json!("demo"))])),
        )
        .await
        .unwrap();
    assert!(!context.is_error.unwrap_or(false));

    let package = engine.narrative_package(&name).unwrap();
    let request = DeliveryRequest {
        schema_version: 1,
        request_id: "REQ-MCP-1".into(),
        narrative_package_sha256: hash_jcs(&package).unwrap(),
        intent: DeliveryIntent::Presentation,
        coverage: Coverage::Full,
        audience: None,
        locale: None,
        must_include_refs: vec![],
        constraints: vec![],
        asset_inputs: vec![],
        outputs: vec![OutputRequest {
            output_id: "html".into(),
            format: OutputFormat::Html,
            backend_id: Some("internal-html".into()),
            width: None,
            height: None,
            aspect_ratio: None,
            fps: None,
            duration_frames: None,
            constraints: vec![],
        }],
    };
    let run = client
        .call_tool(
            CallToolRequestParams::new("run_delivery").with_arguments(args(vec![
                ("project", json!("demo")),
                ("request", serde_json::to_value(&request).unwrap()),
            ])),
        )
        .await
        .unwrap();
    assert!(
        !run.is_error.unwrap_or(false),
        "run_delivery failed: {:?}",
        run.content
    );
    let run_text = match run.content.first().cloned().unwrap() {
        rmcp::model::ContentBlock::Text(text) => text.text,
        other => panic!("unexpected content: {other:?}"),
    };
    assert!(run_text.contains("\"NEEDS_REVIEW\""), "{run_text}");
    assert!(
        run_text.contains(&hash_jcs(&package).unwrap()),
        "set must bind the released package hash: {run_text}"
    );

    let deliveries = client
        .call_tool(
            CallToolRequestParams::new("list_deliveries")
                .with_arguments(args(vec![("project", json!("demo"))])),
        )
        .await
        .unwrap();
    let deliveries_text = match deliveries.content.first().cloned().unwrap() {
        rmcp::model::ContentBlock::Text(text) => text.text,
        other => panic!("unexpected content: {other:?}"),
    };
    assert!(deliveries_text.contains("REQ-MCP-1"), "{deliveries_text}");

    let project_dir = engine.project_dir(&name).unwrap();
    let html_files: Vec<_> = walk_files(&project_dir.join("visual/deliveries"))
        .into_iter()
        .filter(|p| p.extension().map(|e| e == "html").unwrap_or(false))
        .collect();
    assert!(!html_files.is_empty(), "no html artifact published");
    let body = fs::read_to_string(&html_files[0]).unwrap();
    assert!(body.contains("SCN-001"), "{body}");

    client.cancel().await.unwrap();
    server_task.await.unwrap();
}

fn walk_files(root: &Path) -> Vec<PathBuf> {
    let mut out = vec![];
    let mut stack = vec![root.to_path_buf()];
    while let Some(dir) = stack.pop() {
        if let Ok(entries) = fs::read_dir(&dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.is_dir() {
                    stack.push(path);
                } else {
                    out.push(path);
                }
            }
        }
    }
    out
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn mcp_rejects_invalid_delivery_request_without_side_effects() {
    let (_tmp, engine, name) = completed_project();
    let projects_root = engine
        .project_dir(&name)
        .unwrap()
        .parent()
        .unwrap()
        .to_path_buf();
    let assets_root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");

    let server = StsMcp::new(
        Engine::with_roots(assets_root, projects_root.clone()).expect("fixture roots resolve"),
    );
    let (client_transport, server_transport) = tokio::io::duplex(256 * 1024);
    let server_task = tokio::spawn(async move {
        server
            .serve(server_transport)
            .await
            .unwrap()
            .waiting()
            .await
            .unwrap();
    });
    let client = ().serve(client_transport).await.unwrap();

    let bad = client
        .call_tool(
            CallToolRequestParams::new("run_delivery").with_arguments(args(vec![
                ("project", json!("demo")),
                ("request", json!({"schema_version": 1})),
            ])),
        )
        .await;
    assert!(bad.is_err(), "invalid DeliveryRequest must fail the call");

    let requests_dir = project_requests_root(&projects_root.join("demo"));
    assert!(
        !requests_dir.exists(),
        "failed request must not create contract chain"
    );

    client.cancel().await.unwrap();
    server_task.await.unwrap();
}

fn project_requests_root(project: &Path) -> PathBuf {
    project.join("visual/requests")
}
