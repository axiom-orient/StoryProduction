#[path = "commands/delivery.rs"]
mod delivery_commands;
mod delivery_preview;
#[path = "commands/sts.rs"]
mod sts_commands;
mod worker;

use sts_workflow::Engine;
use tauri::Manager;

#[derive(Clone)]
pub(crate) struct AppState {
    engine: Engine,
}

fn ui_error<E: std::fmt::Display>(error: E) -> String {
    error.to_string()
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .setup(|app| {
            let resource_root = app.path().resource_dir()?;
            let data_root = app.path().app_data_dir()?;
            let engine = Engine::with_roots(resource_root, data_root.join("projects"))?;
            engine
                .load_manifest()
                .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e.to_string()))?;
            std::fs::create_dir_all(&data_root)?;
            app.manage(AppState { engine });
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            sts_commands::pick_reference_files,
            sts_commands::pick_synopsis_file,
            delivery_commands::pick_delivery_asset,
            sts_commands::create_project,
            sts_commands::list_projects,
            sts_commands::get_project,
            sts_commands::runtime_status,
            sts_commands::run_next_agent,
            sts_commands::cancel_agent_run,
            sts_commands::list_artifacts,
            sts_commands::read_artifact,
            sts_commands::decide_review,
            sts_commands::resume_project,
            sts_commands::export_docx,
            delivery_commands::delivery_context,
            delivery_commands::run_delivery,
            delivery_commands::cancel_delivery,
            delivery_commands::review_delivery,
            delivery_commands::list_deliveries,
            delivery_commands::read_delivery_preview
        ])
        .run(tauri::generate_context!())
        .expect("failed to run NarrativeCourier");
}
