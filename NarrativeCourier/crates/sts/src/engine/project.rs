//! STS project path entrypoint; Runtime owns name validation and children own creation/interchange.

mod creation;
mod screenplay;

use super::Engine;
use narrative_courier_runtime::Result;
use std::fs;
use std::path::PathBuf;

impl Engine {
    pub fn project_dir(&self, name: &str) -> Result<PathBuf> {
        if let Some(selected) = &self.selected_project {
            narrative_courier_runtime::paths::validate_project_name(name)?;
            if selected != name {
                return Err(narrative_courier_runtime::Error::Contract(
                    "selected project logical identity differs".into(),
                ));
            }
            return Ok(PathBuf::from("."));
        }
        let project = self.runtime.project_dir(name)?;
        fs::create_dir_all(self.runtime.projects_root())?;
        Ok(project)
    }
}
