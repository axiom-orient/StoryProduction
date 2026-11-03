//! Delivery UI commands delegate owned effects through one blocking-worker boundary.

use super::{
    delivery_preview::{self, DeliveryPreview},
    ui_error,
    worker::blocking,
    AppState,
};
use narrative_courier_contracts::{DeliveryRequest, DeliverySet};
use narrative_courier_delivery::{
    DeliveryContext, DeliveryEngine, DeliveryRecord, ImportedDeliveryAsset,
};
use tauri::{AppHandle, State};
use tauri_plugin_dialog::DialogExt;

#[tauri::command]
pub(super) async fn pick_delivery_asset(
    app: AppHandle,
    state: State<'_, AppState>,
    name: String,
) -> Result<Option<ImportedDeliveryAsset>, String> {
    let engine = state.engine.clone();
    blocking("pick delivery asset", move || {
        let Some(file) = app
            .dialog()
            .file()
            .add_filter("Visual reference", &["png", "jpg", "jpeg", "webp"])
            .blocking_pick_file()
        else {
            return Ok(None);
        };
        let path = file.into_path().map_err(ui_error)?;
        let package = engine.narrative_package(&name).map_err(ui_error)?;
        let project = engine.project_dir(&name).map_err(ui_error)?;
        DeliveryEngine::import_asset(&project, &package, &path)
            .map(Some)
            .map_err(ui_error)
    })
    .await
}
#[tauri::command]
pub(super) async fn delivery_context(
    state: State<'_, AppState>,
    name: String,
) -> Result<DeliveryContext, String> {
    let engine = state.engine.clone();
    blocking("delivery context", move || {
        let package = engine.narrative_package(&name).map_err(ui_error)?;
        DeliveryEngine::context(&engine.project_dir(&name).map_err(ui_error)?, &package)
            .map_err(ui_error)
    })
    .await
}
#[tauri::command]
pub(super) async fn run_delivery(
    state: State<'_, AppState>,
    name: String,
    request: DeliveryRequest,
) -> Result<DeliverySet, String> {
    let engine = state.engine.clone();
    blocking("run delivery", move || {
        let package = engine.narrative_package(&name).map_err(ui_error)?;
        DeliveryEngine::run(
            &engine.project_dir(&name).map_err(ui_error)?,
            &package,
            request,
        )
        .map_err(ui_error)
    })
    .await
}
#[tauri::command]
pub(super) async fn cancel_delivery(
    state: State<'_, AppState>,
    name: String,
) -> Result<(), String> {
    let engine = state.engine.clone();
    blocking("cancel delivery", move || {
        DeliveryEngine::cancel(&engine.project_dir(&name).map_err(ui_error)?).map_err(ui_error)
    })
    .await
}
#[tauri::command]
pub(super) async fn review_delivery(
    state: State<'_, AppState>,
    name: String,
    request_id: String,
    output_id: String,
    approve: bool,
    note: Option<String>,
) -> Result<DeliverySet, String> {
    let engine = state.engine.clone();
    blocking("review delivery", move || {
        let package = engine.narrative_package(&name).map_err(ui_error)?;
        DeliveryEngine::review(
            &engine.project_dir(&name).map_err(ui_error)?,
            &package,
            &request_id,
            &output_id,
            approve,
            note.as_deref(),
        )
        .map_err(ui_error)
    })
    .await
}
#[tauri::command]
pub(super) async fn list_deliveries(
    state: State<'_, AppState>,
    name: String,
) -> Result<Vec<DeliveryRecord>, String> {
    let engine = state.engine.clone();
    blocking("list deliveries", move || {
        let package = engine.narrative_package(&name).map_err(ui_error)?;
        DeliveryEngine::list(&engine.project_dir(&name).map_err(ui_error)?, &package)
            .map_err(ui_error)
    })
    .await
}
#[tauri::command]
pub(super) async fn read_delivery_preview(
    state: State<'_, AppState>,
    name: String,
    request_id: String,
    output_id: String,
) -> Result<DeliveryPreview, String> {
    let engine = state.engine.clone();
    blocking("read delivery preview", move || {
        let package = engine.narrative_package(&name).map_err(ui_error)?;
        let (artifact, bytes) = DeliveryEngine::read_artifact_bytes(
            &engine.project_dir(&name).map_err(ui_error)?,
            &package,
            &request_id,
            &output_id,
            64 * 1024 * 1024,
        )
        .map_err(ui_error)?;
        delivery_preview::from_verified_artifact(&artifact, bytes)
    })
    .await
}
