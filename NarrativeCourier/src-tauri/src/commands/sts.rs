use narrative_courier_runtime::document::extract_document_text;
use serde::Serialize;
use sts_workflow::{
    ArtifactListItem, ArtifactView, DecisionResult, NewProject, ProjectListItem, ProjectView,
    RunResult, RuntimeStatus,
};
use tauri::{AppHandle, Manager, State};
use tauri_plugin_dialog::DialogExt;

use super::{ui_error, worker::blocking, AppState};

#[derive(Clone, Serialize)]
pub(super) struct PickedFile {
    source_path: String,
    name: String,
    size: u64,
}

#[derive(Clone, Serialize)]
pub(super) struct ImportedSynopsis {
    name: String,
    content: String,
}

fn selected_file(path: &std::path::Path, label: &str) -> Result<PickedFile, String> {
    let metadata = std::fs::symlink_metadata(path).map_err(ui_error)?;
    if metadata.file_type().is_symlink() || !metadata.is_file() {
        return Err(format!("{label} must be a regular non-symlink file"));
    }
    if metadata.len() > 25 * 1024 * 1024 {
        return Err(format!("{label} exceeds 25 MiB"));
    }
    let source_path = path
        .to_str()
        .ok_or_else(|| format!("{label} path cannot be represented by the UTF-8 request contract"))?
        .to_owned();
    let name = path
        .file_name()
        .and_then(|value| value.to_str())
        .ok_or_else(|| format!("{label} has no UTF-8 filename"))?
        .to_owned();
    Ok(PickedFile {
        source_path,
        name,
        size: metadata.len(),
    })
}

#[tauri::command]
pub(super) async fn pick_reference_files(
    app: AppHandle,
) -> Result<Option<Vec<PickedFile>>, String> {
    blocking("pick references", move || {
        let Some(files) = app
            .dialog()
            .file()
            .add_filter(
                "NarrativeCourier references",
                &[
                    "pdf", "docx", "txt", "md", "png", "jpg", "jpeg", "webp", "gif",
                ],
            )
            .blocking_pick_files()
        else {
            return Ok(None);
        };
        let mut picked = Vec::new();
        for file in files {
            picked.push(selected_file(
                &file.into_path().map_err(ui_error)?,
                "reference",
            )?);
        }
        Ok(Some(picked))
    })
    .await
}

#[tauri::command]
pub(super) async fn pick_synopsis_file(app: AppHandle) -> Result<Option<ImportedSynopsis>, String> {
    blocking("pick synopsis", move || {
        let Some(file) = app
            .dialog()
            .file()
            .add_filter("Synopsis", &["md", "txt", "pdf", "docx"])
            .blocking_pick_file()
        else {
            return Ok(None);
        };
        let path = file.into_path().map_err(ui_error)?;
        let picked = selected_file(&path, "synopsis")?;
        let content = extract_document_text(&path).map_err(ui_error)?;
        Ok(Some(ImportedSynopsis {
            name: picked.name,
            content,
        }))
    })
    .await
}

#[tauri::command]
pub(super) async fn create_project(
    state: State<'_, AppState>,
    request: NewProject,
) -> Result<ProjectView, String> {
    let engine = state.engine.clone();
    blocking("create project", move || {
        engine.create_project(&request).map_err(ui_error)
    })
    .await
}
#[tauri::command]
pub(super) async fn list_projects(
    state: State<'_, AppState>,
) -> Result<Vec<ProjectListItem>, String> {
    let engine = state.engine.clone();
    blocking("list projects", move || {
        engine.list_projects().map_err(ui_error)
    })
    .await
}
#[tauri::command]
pub(super) async fn get_project(
    state: State<'_, AppState>,
    name: String,
) -> Result<ProjectView, String> {
    let engine = state.engine.clone();
    blocking("get project", move || {
        engine.project_view(&name).map_err(ui_error)
    })
    .await
}
#[tauri::command]
pub(super) async fn runtime_status(state: State<'_, AppState>) -> Result<RuntimeStatus, String> {
    let engine = state.engine.clone();
    blocking("runtime status", move || Ok(engine.codex_runtime_status())).await
}
#[tauri::command]
pub(super) async fn run_next_agent(
    state: State<'_, AppState>,
    name: String,
) -> Result<RunResult, String> {
    let engine = state.engine.clone();
    blocking("run agent", move || {
        engine.run_one_codex(&name).map_err(ui_error)
    })
    .await
}
#[tauri::command]
pub(super) async fn cancel_agent_run(
    state: State<'_, AppState>,
    name: String,
) -> Result<(), String> {
    let engine = state.engine.clone();
    blocking("cancel agent", move || {
        engine.cancel_codex(&name).map_err(ui_error)
    })
    .await
}
#[tauri::command]
pub(super) async fn list_artifacts(
    state: State<'_, AppState>,
    name: String,
) -> Result<Vec<ArtifactListItem>, String> {
    let engine = state.engine.clone();
    blocking("list artifacts", move || {
        engine.list_artifacts(&name).map_err(ui_error)
    })
    .await
}
#[tauri::command]
pub(super) async fn read_artifact(
    state: State<'_, AppState>,
    name: String,
    node: String,
) -> Result<ArtifactView, String> {
    let engine = state.engine.clone();
    blocking("read artifact", move || {
        engine.read_artifact(&name, &node).map_err(ui_error)
    })
    .await
}
#[tauri::command]
pub(super) async fn decide_review(
    state: State<'_, AppState>,
    name: String,
    decision: String,
    step: Option<u8>,
    feedback: Option<String>,
    gate_token: String,
) -> Result<DecisionResult, String> {
    let engine = state.engine.clone();
    blocking("decide review", move || {
        engine
            .decide(&name, &decision, step, feedback.as_deref(), &gate_token)
            .map_err(ui_error)
    })
    .await
}
#[tauri::command]
pub(super) async fn resume_project(
    state: State<'_, AppState>,
    name: String,
) -> Result<ProjectView, String> {
    let engine = state.engine.clone();
    blocking("resume project", move || {
        engine.resume(&name).map_err(ui_error)?;
        engine.project_view(&name).map_err(ui_error)
    })
    .await
}
#[tauri::command]
pub(super) async fn export_docx(
    app: AppHandle,
    state: State<'_, AppState>,
    name: String,
) -> Result<String, String> {
    let engine = state.engine.clone();
    blocking("export DOCX", move || {
        let export_root = app.path().download_dir().map_err(ui_error)?;
        let project = engine.project_view(&name).map_err(ui_error)?;
        let file = format!(
            "{}-{}-screenplay.docx",
            safe_export_name(&project.title),
            name
        );
        let target = export_root.join(file);
        let encoded = target
            .to_str()
            .ok_or("export path cannot be represented by the UTF-8 UI contract")?
            .to_owned();
        engine.export_docx(&name, &target).map_err(ui_error)?;
        Ok(encoded)
    })
    .await
}

fn safe_export_name(value: &str) -> String {
    let out: String = value
        .chars()
        .map(|c| {
            if c.is_alphanumeric() || c == '-' || c == '_' {
                c
            } else {
                '-'
            }
        })
        .collect();
    let out = out.trim_matches('-');
    if out.is_empty() {
        "screenplay".into()
    } else {
        out.into()
    }
}
