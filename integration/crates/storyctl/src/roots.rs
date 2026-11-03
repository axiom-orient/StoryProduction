//! Distinct Story resource and capability workspace roots.
//!
//! Resource roots are selected and checked once at the CLI boundary. Capability
//! files live in a separate user-selected namespace when running an installed
//! bundle; source-tree callers retain the historical single-root layout.

use crate::{
    effects::files,
    error::{Error, Result},
    runtime::{self, RootMode},
};
use std::path::{Path, PathBuf};

#[derive(Clone, Debug)]
pub(crate) struct StoryResourceRoot {
    path: PathBuf,
    mode: RootMode,
}

#[derive(Clone, Debug)]
pub(crate) struct CapabilityRoot(PathBuf);

impl StoryResourceRoot {
    pub(crate) fn select(path: &Path) -> Result<Self> {
        let path = path.canonicalize()?;
        if !path.is_dir() {
            return Err(Error::not_started("selected Story root is not a directory"));
        }
        let mode = runtime::check_root(&path)?;
        Ok(Self { path, mode })
    }

    pub(crate) fn path(&self) -> &Path {
        &self.path
    }

    pub(crate) fn mode(&self) -> RootMode {
        self.mode
    }

    pub(crate) fn capability_root(&self, selected: Option<&Path>) -> Result<CapabilityRoot> {
        let path = match selected {
            Some(path) => {
                let path = path.canonicalize()?;
                if !path.is_dir() {
                    return Err(Error::not_applied(
                        "--capability-root must name an existing directory",
                    ));
                }
                path
            }
            None if self.mode == RootMode::Source => self.path.clone(),
            None => {
                return Err(Error::not_applied(
                    "installed Story runtime requires an external --capability-root directory",
                ))
            }
        };

        if self.mode == RootMode::Installed
            && (path.starts_with(&self.path) || self.path.starts_with(&path))
        {
            return Err(Error::not_applied(
                "--capability-root must be separate from the installed Story runtime",
            ));
        }
        Ok(CapabilityRoot(path))
    }

    pub(crate) fn workspace_path(&self, selected: &Path) -> Result<PathBuf> {
        self.writable_path(selected)
    }

    pub(crate) fn writable_path(&self, selected: &Path) -> Result<PathBuf> {
        if self.mode == RootMode::Source {
            return Ok(selected.to_path_buf());
        }
        let path = files::non_symlink_path(selected)?;
        if path.starts_with(&self.path) {
            return Err(Error::not_applied(
                "write path cannot be inside the installed Story runtime",
            ));
        }
        Ok(path)
    }
}

impl CapabilityRoot {
    pub(crate) fn path(&self) -> &Path {
        &self.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::{fs, time::SystemTime};

    fn temporary(name: &str) -> PathBuf {
        let unique = SystemTime::now()
            .duration_since(SystemTime::UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        std::env::temp_dir().join(format!("storyctl-roots-{name}-{unique}"))
    }

    #[test]
    fn installed_capability_root_must_be_existing_and_disjoint() {
        let runtime = temporary("runtime");
        let data = temporary("data");
        fs::create_dir_all(&runtime).unwrap();
        fs::create_dir_all(&data).unwrap();
        fs::create_dir_all(runtime.join("nested")).unwrap();
        let selected = StoryResourceRoot {
            path: runtime.canonicalize().unwrap(),
            mode: RootMode::Installed,
        };
        let root = selected.capability_root(Some(&data)).unwrap();
        assert_eq!(root.path(), data.canonicalize().unwrap());
        assert!(selected
            .capability_root(Some(&runtime.join("nested")))
            .is_err());
        assert!(selected
            .capability_root(Some(runtime.parent().unwrap()))
            .is_err());
        assert!(selected.capability_root(None).is_err());
        assert!(selected
            .writable_path(&selected.path.join("generated.json"))
            .is_err());
        assert!(selected
            .writable_path(&data.canonicalize().unwrap().join("generated.json"))
            .is_ok());
        fs::remove_dir_all(runtime).unwrap();
        fs::remove_dir_all(data).unwrap();
    }

    #[test]
    fn source_capability_root_defaults_to_resources_and_installed_workspace_is_guarded() {
        let runtime = temporary("source");
        fs::create_dir_all(&runtime).unwrap();
        let source = StoryResourceRoot {
            path: runtime.canonicalize().unwrap(),
            mode: RootMode::Source,
        };
        assert_eq!(source.capability_root(None).unwrap().path(), source.path());
        assert_eq!(
            source
                .writable_path(&runtime.join("source-output.json"))
                .unwrap(),
            runtime.join("source-output.json")
        );
        assert!(source
            .workspace_path(&source.path().join("writer-data"))
            .is_ok());

        let installed = StoryResourceRoot {
            path: source.path().to_path_buf(),
            mode: RootMode::Installed,
        };
        assert!(installed
            .workspace_path(&source.path().join("writer-data"))
            .is_err());
        fs::remove_dir_all(runtime).unwrap();
    }
}
