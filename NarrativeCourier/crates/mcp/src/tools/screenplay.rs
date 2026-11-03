use crate::params::{ExportScreenplayParams, ImportScreenplayParams, ScreenplayFormat};
use rmcp::{model::CallToolResult, ErrorData as McpError};
use sts_workflow::Engine;

use super::support::blocking_json;

pub(super) async fn import(
    engine: Engine,
    params: ImportScreenplayParams,
) -> Result<CallToolResult, McpError> {
    let title = params.title.unwrap_or_else(|| {
        std::path::Path::new(&params.script_path)
            .file_stem()
            .and_then(|value| value.to_str())
            .unwrap_or("imported screenplay")
            .to_owned()
    });
    blocking_json(move || {
        engine
            .import_screenplay(&sts_workflow::ImportScreenplayRequest {
                project_name: params.project.unwrap_or_default(),
                title,
                synopsis: params.synopsis,
                source_format: "original".into(),
                target_duration: params.target_duration,
                genre: vec![],
                tone: String::new(),
                theme_direction: String::new(),
                script_path: params.script_path,
            })
            .map_err(|error| error.to_string())
    })
    .await
}

pub(super) async fn export(
    engine: Engine,
    params: ExportScreenplayParams,
) -> Result<CallToolResult, McpError> {
    let format = params.format;
    let extension = format.extension();
    let file_name = match params.output_path {
        None => format!("final_screenplay.{extension}"),
        Some(value) => {
            let candidate = std::path::Path::new(&value);
            if candidate.is_absolute()
                || candidate.components().count() != 1
                || candidate.file_name().and_then(|part| part.to_str()) != Some(value.as_str())
                || candidate.extension().and_then(|part| part.to_str()) != Some(extension)
            {
                return Err(McpError::invalid_params("output_path must be a bare filename with the selected format extension inside the project output directory", None));
            }
            value
        }
    };
    blocking_json(move || {
        let out = engine
            .project_dir(&params.project)
            .map_err(|error| error.to_string())?
            .join("output")
            .join(&file_name);
        match format {
            ScreenplayFormat::Fountain => engine.export_screenplay_fountain(&params.project, &out),
            ScreenplayFormat::Fdx => engine.export_screenplay_fdx(&params.project, &out),
            ScreenplayFormat::Docx => engine.export_docx(&params.project, &out),
        }
        .map_err(|error| error.to_string())?;
        Ok(serde_json::json!({
            "format": extension,
            "path": out.display().to_string(),
            "file_name": file_name,
        }))
    })
    .await
}
