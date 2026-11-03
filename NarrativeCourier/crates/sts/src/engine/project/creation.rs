//! Generic project initialization; reviewed-synopsis admission lives in a sibling owner.

mod reviewed_synopsis;

use super::super::{
    generated_project_name, read_reference_bytes, resolve_regular_file_within_root,
    sanitize_filename, validate_reference_type, Engine, SOURCE_FORMATS,
};
use crate::model::project::{NewProject, ProjectConfig, ReviewedSynopsisBinding};
use crate::model::views::ProjectView;
use narrative_courier_runtime::store::{
    atomic_write, atomic_write_text, sha256_bytes, timestamp_compact,
};
use narrative_courier_runtime::{Error, Result};
use std::collections::BTreeSet;
use std::fs;
use std::path::PathBuf;

impl Engine {
    pub fn create_project(&self, request: &NewProject) -> Result<ProjectView> {
        if request.title.trim().is_empty() || request.synopsis.trim().is_empty() {
            return Err(Error::Contract("title and synopsis are required".into()));
        }
        self.create_project_with_synopsis_bytes(request, request.synopsis.as_bytes(), None)
    }

    /// Create a project while preserving the exact bytes of an already-verified
    /// synopsis artifact. The reviewed-synopsis ingress uses this path so its
    /// raw hash remains meaningful after persistence. Direct authoring also
    /// preserves its supplied UTF-8 bytes without newline normalization.
    fn create_project_with_synopsis_bytes(
        &self,
        request: &NewProject,
        synopsis_bytes: &[u8],
        reviewed: Option<ReviewedSynopsisInput>,
    ) -> Result<ProjectView> {
        if !SOURCE_FORMATS.contains(&request.source_format.as_str()) {
            return Err(Error::Contract(format!(
                "unsupported source_format: {}",
                request.source_format
            )));
        }
        if request.target_duration == 0 {
            return Err(Error::Contract("target_duration must be > 0".into()));
        }
        let manifest = self.load_manifest()?;
        let project_name = if request.project_name.trim().is_empty() {
            generated_project_name(&request.title)
        } else {
            request.project_name.clone()
        };
        let project = self.project_dir(&project_name)?;
        if entry_exists(&project)? {
            return Err(Error::Contract(format!(
                "project already exists: {project_name}"
            )));
        }

        let temp = self
            .runtime
            .projects_root()
            .join(format!(".{project_name}.{}.tmp", timestamp_compact()));
        // Exclusive ownership: never delete or reuse an existing staging directory.
        fs::create_dir(&temp)?;
        let build = (|| -> Result<()> {
            fs::create_dir_all(temp.join("input/references"))?;
            fs::create_dir_all(temp.join("input/human_feedback"))?;
            fs::create_dir_all(temp.join("output"))?;
            let synopsis_name = "synopsis.md".to_string();
            let staged_synopsis = temp.join("input/synopsis.md");
            atomic_write(&staged_synopsis, synopsis_bytes)?;
            if let Some(reviewed) = &reviewed {
                if sha256_bytes(synopsis_bytes) != reviewed.binding.synopsis_sha256 {
                    return Err(Error::Contract(
                        "reviewed synopsis hash changed before persistence".into(),
                    ));
                }
            }

            let mut reference_files = Vec::new();
            let mut reference_names = BTreeSet::new();
            if let Some(reviewed) = &reviewed {
                let name = sanitize_filename(&reviewed.reference_name)?;
                validate_reference_type(&name)?;
                if sha256_bytes(&reviewed.handoff_bytes) != reviewed.binding.handoff_sha256 {
                    return Err(Error::Contract(
                        "reviewed handoff bytes changed before staging".into(),
                    ));
                }
                atomic_write(
                    &temp.join("input/references").join(&name),
                    &reviewed.handoff_bytes,
                )?;
                reference_names.insert(name.clone());
                reference_files.push(name);
            }
            for reference in &request.references {
                let source = PathBuf::from(&reference.source_path);
                let source = source.canonicalize()?;
                let name =
                    sanitize_filename(source.file_name().and_then(|v| v.to_str()).unwrap_or(""))?;
                validate_reference_type(&name)?;
                if !reference_names.insert(name.clone()) {
                    return Err(Error::Contract(format!(
                        "duplicate reference filename: {name}"
                    )));
                }
                let bytes = read_reference_bytes(&source)?;
                atomic_write(&temp.join("input/references").join(&name), &bytes)?;
                reference_files.push(name);
            }
            let config = ProjectConfig {
                title: request.title.trim().to_string(),
                source_format: request.source_format.clone(),
                target_duration: request.target_duration,
                genre: request.genre.clone(),
                tone: request.tone.clone(),
                theme_direction: request.theme_direction.clone(),
                synopsis_file: synopsis_name,
                reference_files,
                reviewed_synopsis: reviewed.as_ref().map(|input| input.binding.clone()),
            };
            atomic_write_text(&temp.join("config.yaml"), &serde_yaml::to_string(&config)?)?;
            let state = self.initial_state(&project_name, &manifest)?;
            atomic_write_text(
                &temp.join("state.json"),
                &(serde_json::to_string_pretty(&state)? + "\n"),
            )?;
            let persisted = fs::read(resolve_regular_file_within_root(
                &temp,
                "input/synopsis.md",
            )?)?;
            if persisted != synopsis_bytes {
                return Err(Error::Contract(
                    "staged synopsis differs from the input bytes".into(),
                ));
            }
            Ok(())
        })();

        if let Err(e) = build {
            let _ = fs::remove_dir_all(&temp);
            return Err(e);
        }
        if let Err(e) = fs::rename(&temp, &project) {
            let _ = fs::remove_dir_all(&temp);
            return Err(Error::Io(e));
        }
        self.project_view(&project_name)
    }
}

/// Only the verified ingress constructs this binding and its captured reference bytes.
struct ReviewedSynopsisInput {
    binding: ReviewedSynopsisBinding,
    handoff_bytes: Vec<u8>,
    reference_name: String,
}

fn entry_exists(path: &std::path::Path) -> Result<bool> {
    match fs::symlink_metadata(path) {
        Ok(_) => Ok(true),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(false),
        Err(error) => Err(error.into()),
    }
}
