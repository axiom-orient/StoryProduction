//! Screenplay interchange through the STS-owned Fountain/FDX format adapter.

use super::super::{
    ensure_atomic_file_target, generated_project_name, read_reference_bytes, Engine,
};
use crate::model::project::{ImportReport, ImportScreenplayRequest, NewProject, ReferenceInput};
use narrative_courier_runtime::store::{atomic_write_text, timestamp_compact};
use narrative_courier_runtime::{Error, Result};
use std::fs;
use std::path::Path;

impl Engine {
    pub fn export_docx(&self, name: &str, output: &Path) -> Result<()> {
        let markdown = self.completed_screenplay(name)?;
        crate::docx::export_screenplay_docx(&markdown, output)
    }

    pub fn export_screenplay_fountain(&self, name: &str, output: &Path) -> Result<()> {
        let markdown = self.completed_screenplay(name)?;
        let title = self.project_view(name)?.title;
        let text = crate::fountain::to_fountain(&markdown, Some(&title))?;
        Self::write_export(output, &text)
    }

    pub fn export_screenplay_fdx(&self, name: &str, output: &Path) -> Result<()> {
        let markdown = self.completed_screenplay(name)?;
        let title = self.project_view(name)?.title;
        let text = crate::fountain::to_fdx(&markdown, Some(&title))?;
        Self::write_export(output, &text)
    }

    fn completed_screenplay(&self, name: &str) -> Result<String> {
        match self.next(name)? {
            crate::NextPacket::Completed { .. } => {}
            _ => return Err(Error::Contract("workflow is not complete".into())),
        }
        let manifest = self.load_manifest()?;
        let project = self.project_dir(name)?;
        let state = self.load_state(&project, &manifest)?;
        self.verify_release_bundle_at(&project, &state, &manifest)?;
        let source =
            self.verify_durable_output(&project, &state, &manifest, "output/final_screenplay.md")?;
        let bytes = fs::read(source)?;
        let package = self.verify_narrative_package_at(&project, &state, &manifest)?;
        if narrative_courier_runtime::store::sha256_bytes(&bytes)
            != package.canonical_narrative.artifact.sha256
        {
            return Err(Error::Contract(
                "screenplay export source changed after release".into(),
            ));
        }
        String::from_utf8(bytes)
            .map_err(|error| Error::Contract(format!("screenplay export is not UTF-8: {error}")))
    }

    fn write_export(output: &Path, text: &str) -> Result<()> {
        if let Some(parent) = output.parent() {
            fs::create_dir_all(parent)?;
        }
        ensure_atomic_file_target(output, "screenplay export")?;
        atomic_write_text(output, text)
    }

    pub fn import_screenplay(&self, request: &ImportScreenplayRequest) -> Result<ImportReport> {
        if request.title.trim().is_empty() || request.synopsis.trim().is_empty() {
            return Err(Error::Contract("title and synopsis are required".into()));
        }
        let body = String::from_utf8(read_reference_bytes(Path::new(&request.script_path))?)
            .map_err(|error| Error::Contract(format!("screenplay input is not UTF-8: {error}")))?;
        let format = crate::fountain::detect_format(&request.script_path, &body)?;
        let normalized = crate::fountain::normalize_screenplay(format, &body)?;
        if normalized.summary.scene_count == 0 {
            return Err(Error::Contract(
                "no scene headings detected in screenplay".into(),
            ));
        }

        let project_name = if request.project_name.trim().is_empty() {
            generated_project_name(&request.title)
        } else {
            request.project_name.clone()
        };
        if self.project_dir(&project_name)?.exists() {
            return Err(Error::Contract(format!(
                "project already exists: {project_name}"
            )));
        }

        let staging = self
            .runtime
            .projects_root()
            .join(format!(".import-{project_name}-{}", timestamp_compact()));
        fs::create_dir(&staging)?;
        let normalized_path = staging.join(format!("{project_name}.screenplay.txt"));
        let source_copy_path = staging.join(format!("{project_name}.source.txt"));
        let built = (|| -> Result<crate::model::views::ProjectView> {
            atomic_write_text(&normalized_path, &normalized.text)?;
            atomic_write_text(&source_copy_path, &body)?;

            let new_project = NewProject {
                project_name: project_name.clone(),
                title: request.title.trim().to_string(),
                synopsis: request.synopsis.clone(),
                source_format: request.source_format.clone(),
                target_duration: request.target_duration,
                genre: request.genre.clone(),
                tone: request.tone.clone(),
                theme_direction: request.theme_direction.clone(),
                references: vec![
                    ReferenceInput {
                        source_path: normalized_path.display().to_string(),
                    },
                    ReferenceInput {
                        source_path: source_copy_path.display().to_string(),
                    },
                ],
            };
            self.create_project(&new_project)
        })();
        let cleanup = fs::remove_dir_all(&staging);
        let built = match (built, cleanup) {
            (result, Ok(())) => result,
            (Ok(_), Err(error)) => Err(Error::Contract(format!(
                "screenplay project created but staging cleanup failed: {error}"
            ))),
            (Err(error), Err(cleanup)) => Err(Error::Contract(format!(
                "{error}; screenplay staging cleanup also failed: {cleanup}"
            ))),
        };
        let view = built?;

        Ok(ImportReport {
            project: view,
            summary: normalized.summary,
            reference_files: vec![
                format!("{project_name}.screenplay.txt"),
                format!("{project_name}.source.txt"),
            ],
        })
    }
}
