//! Public STS Engine facade and shared module wiring.

mod events;
mod manifest;
mod paths;
mod project;
mod project_files;
mod release;
mod state;
mod views;
mod workflow;

use events::{now_iso, push_event};
pub(crate) use paths::ensure_atomic_file_target;
use paths::{path_parts, resolve_directory_within_root, resolve_regular_file_within_root};
use project_files::{
    generated_project_name, is_extractable_document, is_text_file, read_reference_bytes,
    sanitize_filename, validate_reference_type,
};

use narrative_courier_runtime::paths::RuntimePaths;
use std::path::PathBuf;

const SKILL_DIR: &str = "skills/sts";
const SOURCE_FORMATS: &[&str] = &[
    "shortform",
    "webdrama",
    "novel",
    "webtoon",
    "real_story",
    "original",
];

#[derive(Debug, Clone)]
pub struct Engine {
    runtime: RuntimePaths,
    selected_project: Option<String>,
}

impl Engine {
    pub fn new(repository_root: impl Into<PathBuf>) -> narrative_courier_runtime::Result<Self> {
        Ok(Self {
            runtime: RuntimePaths::new(repository_root, SKILL_DIR)?,
            selected_project: None,
        })
    }

    pub fn with_roots(
        assets_root: impl Into<PathBuf>,
        projects_root: impl Into<PathBuf>,
    ) -> narrative_courier_runtime::Result<Self> {
        Ok(Self {
            runtime: RuntimePaths::with_roots(assets_root, projects_root, SKILL_DIR)?,
            selected_project: None,
        })
    }

    /// The executable transport owns a selected process directory. The logical
    /// project identity remains unchanged in state, source snapshots and tokens.
    pub fn with_selected_project(
        assets_root: impl Into<PathBuf>,
        project: &str,
    ) -> narrative_courier_runtime::Result<Self> {
        narrative_courier_runtime::paths::validate_project_name(project)?;
        let engine = Self {
            runtime: RuntimePaths::with_working_projects_root(assets_root, SKILL_DIR)?,
            selected_project: Some(project.to_owned()),
        };
        let manifest = engine.load_manifest()?;
        engine.load_state(std::path::Path::new("."), &manifest)?;
        Ok(engine)
    }

    /// Namespace transport mode: relative projects remain on the selected cwd.
    pub fn with_working_projects_root(
        assets_root: impl Into<PathBuf>,
    ) -> narrative_courier_runtime::Result<Self> {
        Ok(Self {
            runtime: RuntimePaths::with_working_projects_root(assets_root, SKILL_DIR)?,
            selected_project: None,
        })
    }

    fn skill_root(&self) -> PathBuf {
        self.runtime.domain_root()
    }

    fn asset_root(&self, relative: &str) -> PathBuf {
        if relative.starts_with("prompts/")
            || relative.starts_with("schemas/")
            || relative.starts_with("principles/")
        {
            self.skill_root()
        } else {
            self.runtime.assets_root().to_path_buf()
        }
    }

    fn resolve_asset_file(&self, relative: &str) -> narrative_courier_runtime::Result<PathBuf> {
        resolve_regular_file_within_root(&self.asset_root(relative), relative)
    }
}
