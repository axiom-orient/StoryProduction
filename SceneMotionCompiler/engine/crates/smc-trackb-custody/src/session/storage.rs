use std::fs::File;
use std::io;
use std::path::PathBuf;

use super::{ArtifactDirectories, Session};
use crate::error::{CustodyError, Result};
use crate::filesystem::admission::{
    validate_safe_relative_path, ArtifactDirectory, SCENES_DIRECTORY, TRACKB_DIRECTORY,
};
use crate::filesystem::descriptors::{
    directory_identity, ensure_directory_at, fstatat_no_follow, open_directory_at,
    open_existing_directory_at, stat_identity, stat_is_directory,
};
use crate::lock_release::LockHandle;

impl Session {
    pub(super) fn require_lock(&self) -> Result<&LockHandle> {
        self.lock.as_ref().ok_or_else(|| {
            CustodyError::input("a narrative lock must be acquired before storage operations")
        })
    }

    pub(super) fn open_narrative_directory(
        &self,
        narrative_id: &str,
    ) -> Result<(PathBuf, File, ArtifactDirectories)> {
        let project = self
            .project_fd
            .as_ref()
            .ok_or_else(|| CustodyError::input("project root is not initialized"))?
            .try_clone()
            .map_err(|error| {
                CustodyError::io("project root descriptor cannot be cloned", &error)
            })?;
        let scenes = ensure_directory_at(&project, SCENES_DIRECTORY)
            .map_err(|error| CustodyError::io("scenes directory cannot be opened", &error))?;
        let trackb = ensure_directory_at(&scenes, TRACKB_DIRECTORY)
            .map_err(|error| CustodyError::io("trackb directory cannot be opened", &error))?;
        let narrative = ensure_directory_at(&trackb, narrative_id)
            .map_err(|error| CustodyError::io("narrative directory cannot be opened", &error))?;
        let artifact_directories = ArtifactDirectories {
            authority: ensure_directory_at(&narrative, ArtifactDirectory::Authority.as_segment())
                .map_err(|error| {
                CustodyError::io("authority directory cannot be opened", &error)
            })?,
            candidates: ensure_directory_at(&narrative, ArtifactDirectory::Candidates.as_segment())
                .map_err(|error| {
                    CustodyError::io("candidates directory cannot be opened", &error)
                })?,
            delivery_sets: ensure_directory_at(
                &narrative,
                ArtifactDirectory::DeliverySets.as_segment(),
            )
            .map_err(|error| {
                CustodyError::io("delivery-sets directory cannot be opened", &error)
            })?,
            approvals: ensure_directory_at(&narrative, ArtifactDirectory::Approvals.as_segment())
                .map_err(|error| {
                CustodyError::io("approvals directory cannot be opened", &error)
            })?,
        };
        let path = self
            .config
            .project_root
            .join(SCENES_DIRECTORY)
            .join(TRACKB_DIRECTORY)
            .join(narrative_id);
        Ok((path, narrative, artifact_directories))
    }

    pub(super) fn open_narrative_directory_read_only(
        &self,
        narrative_id: &str,
    ) -> Result<Option<(PathBuf, File)>> {
        let project = self
            .project_fd
            .as_ref()
            .ok_or_else(|| CustodyError::input("project root is not initialized"))?
            .try_clone()
            .map_err(|error| {
                CustodyError::io("project root descriptor cannot be cloned", &error)
            })?;
        let Some(scenes) = open_existing_directory_at(
            &project,
            SCENES_DIRECTORY,
            "scenes directory cannot be opened safely",
        )?
        else {
            return Ok(None);
        };
        let Some(trackb) = open_existing_directory_at(
            &scenes,
            TRACKB_DIRECTORY,
            "trackb directory cannot be opened safely",
        )?
        else {
            return Ok(None);
        };
        let Some(narrative) = open_existing_directory_at(
            &trackb,
            narrative_id,
            "narrative directory cannot be opened safely",
        )?
        else {
            return Ok(None);
        };
        let path = self
            .config
            .project_root
            .join(SCENES_DIRECTORY)
            .join(TRACKB_DIRECTORY)
            .join(narrative_id);
        Ok(Some((path, narrative)))
    }

    pub(super) fn storage_parent(
        &self,
        relative: &str,
        create_parent: bool,
    ) -> Result<(File, String)> {
        validate_safe_relative_path(relative)?;
        let narrative = self
            .narrative_fd
            .as_ref()
            .ok_or_else(|| CustodyError::input("narrative directory is not initialized"))?;
        let components: Vec<&str> = relative.split('/').collect();
        let name = components
            .last()
            .copied()
            .ok_or_else(|| CustodyError::storage("storage path has no file name"))?;
        let mut directory = if components.len() == 1 {
            narrative.try_clone().map_err(|error| {
                CustodyError::io("narrative descriptor cannot be cloned", &error)
            })?
        } else {
            let first = components[0];
            let artifact_kind = ArtifactDirectory::from_segment(first).ok_or_else(|| {
                CustodyError::storage("storage path is outside the fixed artifact namespace")
            })?;
            let artifact = self.artifact_directory(artifact_kind)?;
            artifact
                .try_clone()
                .map_err(|error| CustodyError::io("artifact descriptor cannot be cloned", &error))?
        };
        let start = if components.len() == 1 { 0 } else { 1 };
        for component in &components[start..components.len() - 1] {
            directory = if create_parent {
                ensure_directory_at(&directory, component).map_err(|error| {
                    CustodyError::io("storage directory cannot be opened", &error)
                })?
            } else {
                open_directory_at(&directory, component).map_err(|error| {
                    CustodyError::io("storage directory cannot be opened safely", &error)
                })?
            };
        }
        Ok((directory, name.to_owned()))
    }

    pub(super) fn storage_parent_for_read(&self, relative: &str) -> Result<Option<(File, String)>> {
        validate_safe_relative_path(relative)?;
        let Some(narrative) = self.narrative_fd.as_ref() else {
            if self.read_only {
                return Ok(None);
            }
            return Err(CustodyError::input(
                "narrative directory is not initialized",
            ));
        };
        let components: Vec<&str> = relative.split('/').collect();
        let name = components
            .last()
            .copied()
            .ok_or_else(|| CustodyError::storage("storage path has no file name"))?;
        let mut directory = if components.len() == 1 {
            narrative.try_clone().map_err(|error| {
                CustodyError::io("narrative descriptor cannot be cloned", &error)
            })?
        } else {
            let first = components[0];
            let artifact_kind = ArtifactDirectory::from_segment(first).ok_or_else(|| {
                CustodyError::storage("storage path is outside the fixed artifact namespace")
            })?;
            match open_directory_at(narrative, artifact_kind.as_segment()) {
                Ok(directory) => directory,
                Err(error) if error.kind() == io::ErrorKind::NotFound => return Ok(None),
                Err(error) => {
                    return Err(CustodyError::io(
                        "artifact directory cannot be opened safely",
                        &error,
                    ))
                }
            }
        };
        let start = if components.len() == 1 { 0 } else { 1 };
        for component in &components[start..components.len() - 1] {
            match open_directory_at(&directory, component) {
                Ok(next) => directory = next,
                Err(error) if error.kind() == io::ErrorKind::NotFound => return Ok(None),
                Err(error) => {
                    return Err(CustodyError::io(
                        "storage directory cannot be opened safely",
                        &error,
                    ))
                }
            }
        }
        Ok(Some((directory, name.to_owned())))
    }

    fn artifact_directory(&self, kind: ArtifactDirectory) -> Result<&File> {
        let directories = self
            .artifact_directories
            .as_ref()
            .ok_or_else(|| CustodyError::input("artifact directories are not initialized"))?;
        let directory = match kind {
            ArtifactDirectory::Authority => &directories.authority,
            ArtifactDirectory::Candidates => &directories.candidates,
            ArtifactDirectory::DeliverySets => &directories.delivery_sets,
            ArtifactDirectory::Approvals => &directories.approvals,
        };
        let narrative = self
            .narrative_fd
            .as_ref()
            .ok_or_else(|| CustodyError::input("narrative directory is not initialized"))?;
        let current = fstatat_no_follow(narrative, kind.as_segment()).map_err(|error| {
            CustodyError::storage(format!("artifact directory cannot be revalidated: {error}"))
        })?;
        if !stat_is_directory(&current)
            || stat_identity(&current)
                != directory_identity(directory).map_err(|error| {
                    CustodyError::storage(format!("artifact directory identity failed: {error}"))
                })?
        {
            return Err(CustodyError::storage(
                "artifact directory was replaced after initialization",
            ));
        }
        Ok(directory)
    }
}
