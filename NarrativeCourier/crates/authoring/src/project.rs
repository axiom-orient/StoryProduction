//! Held project directories, bounded capture, immutable publication, and current release proof.
//! Direction writes share STS's workflow lock and never update STS state or its release marker.

use crate::codec::{json, Value};
use crate::{codec, require, Error, Result, MAX_JSON_BYTES};
use narrative_courier_contracts::{
    validate_narrative_package, validate_narrative_scene_map, validate_nvc_release_bundle,
    NarrativePackage, NarrativeSceneMap, NvcReleaseBundle,
};
use std::fs::File;
use std::io::{Read, Write};
use std::path::Path;

pub(crate) struct Release {
    pub package: Value,
    pub scene_map: Value,
    pub binding: Value,
    pub state: Value,
}

#[cfg(all(test, unix))]
mod tests {
    use super::*;

    #[test]
    fn opened_project_stays_on_original_inode_after_root_path_replacement() {
        let temporary = tempfile::TempDir::new().unwrap();
        let selected = temporary.path().join("selected");
        let moved = temporary.path().join("moved");
        std::fs::create_dir(&selected).unwrap();
        std::fs::write(selected.join("source.txt"), "original").unwrap();
        let project = Project::open(&selected).unwrap();
        std::fs::rename(&selected, &moved).unwrap();
        std::fs::create_dir(&selected).unwrap();
        std::fs::write(selected.join("source.txt"), "replacement").unwrap();
        assert_eq!(project.read("source.txt", None).unwrap(), b"original");
        project
            .publish("storyboards/captured.txt", b"captured")
            .unwrap();
        assert_eq!(
            std::fs::read(moved.join("storyboards/captured.txt")).unwrap(),
            b"captured"
        );
        assert!(!selected.join("storyboards/captured.txt").exists());
    }
}

pub(crate) struct Project {
    directory: File,
}
impl Project {
    pub fn open(path: &Path) -> Result<Self> {
        #[cfg(unix)]
        {
            use std::os::unix::ffi::OsStrExt;
            let path = std::ffi::CString::new(path.as_os_str().as_bytes())
                .map_err(|_| Error::refusal("PROJECT_ROOT_REQUIRED"))?;
            let directory = unix::open(
                libc::AT_FDCWD,
                &path,
                libc::O_RDONLY | libc::O_DIRECTORY | libc::O_NOFOLLOW,
                0,
            )?;
            Ok(Self { directory })
        }
        #[cfg(not(unix))]
        {
            let _ = path;
            Err(Error::refusal(
                "TEXT_STORYBOARD_REQUIRES_POSIX_WORKFLOW_LOCK",
            ))
        }
    }
    pub fn lock(&self) -> Result<File> {
        #[cfg(unix)]
        {
            use fs2::FileExt;
            use std::os::fd::AsRawFd;
            let name = unix::name(".workflow.lock")?;
            let file = unix::open(
                self.directory.as_raw_fd(),
                &name,
                libc::O_RDWR | libc::O_CREAT | libc::O_NOFOLLOW | libc::O_NONBLOCK,
                0o600,
            )?;
            require(
                file.metadata()?.is_file(),
                "WORKFLOW_LOCK_MUST_BE_REGULAR_FILE",
            )?;
            file.try_lock_exclusive().map_err(|error| {
                if error.kind() == std::io::ErrorKind::WouldBlock {
                    Error::refusal(
                        "PROJECT_BUSY: retry this same operation after the current STS write",
                    )
                } else {
                    error.into()
                }
            })?;
            Ok(file)
        }
        #[cfg(not(unix))]
        {
            Err(Error::refusal(
                "TEXT_STORYBOARD_REQUIRES_POSIX_WORKFLOW_LOCK",
            ))
        }
    }
    pub fn read(&self, relative: &str, expected: Option<&str>) -> Result<Vec<u8>> {
        let data = self.read_bounded(relative, MAX_JSON_BYTES)?;
        if let Some(expected) = expected {
            require(
                codec::raw_hash(&data)
                    == format!(
                        "sha256:{}",
                        expected.strip_prefix("sha256:").unwrap_or(expected)
                    ),
                format!("SOURCE_CHANGED:{relative}"),
            )?;
        }
        Ok(data)
    }
    fn read_bounded(&self, relative: &str, maximum: usize) -> Result<Vec<u8>> {
        #[cfg(unix)]
        {
            use std::os::fd::AsRawFd;
            let (parent, leaf) = unix::parent(&self.directory, relative, false)?;
            unix::read_regular(parent.as_raw_fd(), &leaf, maximum, relative)
        }
        #[cfg(not(unix))]
        {
            let _ = (relative, maximum);
            Err(Error::refusal("POSIX_REQUIRED"))
        }
    }
    pub fn artifact(&self, relative: &str, expected: Option<&str>) -> Result<Value> {
        codec::decode(&self.read(relative, expected)?)
    }
    pub fn exists(&self, relative: &str) -> Result<bool> {
        #[cfg(unix)]
        {
            use std::os::fd::AsRawFd;
            let (parent, leaf) = match unix::parent(&self.directory, relative, false) {
                Ok(value) => value,
                Err(error) if error.message.starts_with("NOT_FOUND:") => return Ok(false),
                Err(error) => return Err(error),
            };
            match unix::metadata(parent.as_raw_fd(), &leaf) {
                Ok(metadata) => {
                    require(
                        unix::is_regular(&metadata),
                        format!("REGULAR_FILE_REQUIRED:{relative}"),
                    )?;
                    Ok(true)
                }
                Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(false),
                Err(error) => Err(error.into()),
            }
        }
        #[cfg(not(unix))]
        {
            let _ = relative;
            Err(Error::refusal("POSIX_REQUIRED"))
        }
    }
    pub fn publish(&self, relative: &str, data: &[u8]) -> Result<()> {
        require(data.len() <= MAX_JSON_BYTES, "AUTHORING_ARTIFACT_TOO_LARGE")?;
        #[cfg(unix)]
        {
            unix::publish(&self.directory, relative, data)
        }
        #[cfg(not(unix))]
        {
            let _ = (relative, data);
            Err(Error::refusal("POSIX_REQUIRED"))
        }
    }

    pub fn source(&self) -> Result<Release> {
        let bundle_bytes = self.read("release-bundle.json", None)?;
        let bundle_value = codec::decode(&bundle_bytes)?;
        let bundle: NvcReleaseBundle = serde_json::from_value(bundle_value.to_semantic())?;
        validate_nvc_release_bundle(&bundle)?;
        let state = self.artifact(
            &bundle.completed_state.path,
            Some(&bundle.completed_state.sha256),
        )?;
        require(
            state["status"] == "completed"
                && state["nodes"]["final_release"]["status"] == "completed",
            "NARRATIVE_NOT_COMPLETED",
        )?;
        let snapshot = &state["source_snapshot"];
        require(
            codec::raw_hash(&codec::canonical_bytes(snapshot)?)
                == format!("sha256:{}", bundle.source_snapshot_sha256),
            "SOURCE_SNAPSHOT_BINDING",
        )?;
        let package_bytes = self.read(
            &bundle.narrative_package.path,
            Some(&bundle.narrative_package.sha256),
        )?;
        let package_value = codec::decode(&package_bytes)?;
        let package: NarrativePackage = serde_json::from_value(package_value.to_semantic())?;
        validate_narrative_package(&package)?;
        let scene_bytes = self.read(&bundle.scene_map.path, Some(&bundle.scene_map.sha256))?;
        let scene_value = codec::decode(&scene_bytes)?;
        let scene_map: NarrativeSceneMap = serde_json::from_value(scene_value.to_semantic())?;
        validate_narrative_scene_map(&scene_map)?;
        require(
            codec::raw_hash(&package_bytes) == format!("sha256:{}", bundle.package_sha256)
                && package.package_id == bundle.package_id,
            "PACKAGE_BINDING",
        )?;
        require(
            package.source_snapshot_sha256 == bundle.source_snapshot_sha256,
            "PACKAGE_SNAPSHOT_BINDING",
        )?;
        require(
            package.narrative_id == bundle.project_name
                && package.narrative_id == scene_map.narrative_id,
            "NARRATIVE_ID_BINDING",
        )?;
        require(
            package.scene_map.path == bundle.scene_map.path
                && codec::raw_hash(&scene_bytes) == format!("sha256:{}", package.scene_map.sha256),
            "SCENE_MAP_BINDING",
        )?;
        let references = snapshot["references"]
            .as_array()
            .ok_or_else(|| Error::refusal("SOURCE_SNAPSHOT_REFERENCES_REQUIRED"))?;
        for reference in std::iter::once(&snapshot["config"])
            .chain(std::iter::once(&snapshot["synopsis"]))
            .chain(references)
        {
            self.read(
                reference["path"]
                    .as_str()
                    .ok_or_else(|| Error::refusal("SOURCE_PATH_REQUIRED"))?,
                Some(
                    reference["sha256"]
                        .as_str()
                        .ok_or_else(|| Error::refusal("SOURCE_HASH_REQUIRED"))?,
                ),
            )?;
        }
        for (_, reference) in package.artifacts() {
            self.read(&reference.path, Some(&reference.sha256))?;
        }
        Ok(Release {
            package: package_value,
            scene_map: scene_value,
            binding: json!({"narrativePackageSha256": codec::raw_hash(&package_bytes), "sceneMapSha256": codec::raw_hash(&scene_bytes), "releaseBundleSha256": codec::raw_hash(&bundle_bytes)}),
            state,
        })
    }
}

#[cfg(unix)]
mod unix {
    use super::*;
    use std::ffi::CString;
    use std::os::fd::{AsRawFd, FromRawFd, RawFd};
    use std::sync::atomic::{AtomicU64, Ordering};
    static SEQUENCE: AtomicU64 = AtomicU64::new(0);

    pub fn name(value: &str) -> Result<CString> {
        CString::new(value).map_err(|_| Error::refusal("NUL_PATH"))
    }
    pub fn open(parent: RawFd, leaf: &CString, flags: i32, mode: u32) -> Result<File> {
        // Only effects use raw descriptors; every returned File owns exactly one descriptor.
        let fd = unsafe {
            libc::openat(
                parent,
                leaf.as_ptr(),
                flags | libc::O_CLOEXEC,
                mode as libc::c_uint,
            )
        };
        if fd < 0 {
            let error = std::io::Error::last_os_error();
            if error.kind() == std::io::ErrorKind::NotFound {
                return Err(Error::refusal(format!("NOT_FOUND:{error}")));
            }
            return Err(error.into());
        }
        Ok(unsafe { File::from_raw_fd(fd) })
    }
    pub fn metadata(parent: RawFd, leaf: &CString) -> std::io::Result<libc::stat> {
        let mut value = std::mem::MaybeUninit::<libc::stat>::uninit();
        let result = unsafe {
            libc::fstatat(
                parent,
                leaf.as_ptr(),
                value.as_mut_ptr(),
                libc::AT_SYMLINK_NOFOLLOW,
            )
        };
        if result != 0 {
            Err(std::io::Error::last_os_error())
        } else {
            Ok(unsafe { value.assume_init() })
        }
    }
    pub fn is_regular(value: &libc::stat) -> bool {
        value.st_mode & libc::S_IFMT == libc::S_IFREG
    }
    pub fn parent(root: &File, relative: &str, create: bool) -> Result<(File, CString)> {
        require(
            !relative.is_empty()
                && !relative.starts_with('/')
                && !relative.contains(['\\', '\0'])
                && !relative.split('/').any(|part| part == ".."),
            format!("UNSAFE_PATH:{relative}"),
        )?;
        let parts: Vec<_> = relative
            .split('/')
            .filter(|part| !part.is_empty() && *part != ".")
            .collect();
        let (leaf, parents) = parts
            .split_last()
            .ok_or_else(|| Error::refusal("FILE_PATH_REQUIRED"))?;
        let mut directory = root.try_clone()?;
        for part in parents {
            let part = name(part)?;
            if create {
                let result = unsafe { libc::mkdirat(directory.as_raw_fd(), part.as_ptr(), 0o700) };
                if result != 0
                    && std::io::Error::last_os_error().kind() != std::io::ErrorKind::AlreadyExists
                {
                    return Err(std::io::Error::last_os_error().into());
                }
            }
            directory = open(
                directory.as_raw_fd(),
                &part,
                libc::O_RDONLY | libc::O_DIRECTORY | libc::O_NOFOLLOW,
                0,
            )?;
        }
        Ok((directory, name(leaf)?))
    }
    pub fn read_regular(
        parent: RawFd,
        leaf: &CString,
        maximum: usize,
        relative: &str,
    ) -> Result<Vec<u8>> {
        use std::os::unix::fs::MetadataExt;
        let before = metadata(parent, leaf)?;
        require(
            is_regular(&before),
            format!("REGULAR_FILE_REQUIRED:{relative}"),
        )?;
        let mut file = open(
            parent,
            leaf,
            libc::O_RDONLY | libc::O_NOFOLLOW | libc::O_NONBLOCK,
            0,
        )?;
        let opened = file.metadata()?;
        require(
            opened.is_file()
                && opened.dev() == before.st_dev as u64
                && opened.ino() == before.st_ino
                && opened.len() == before.st_size as u64,
            format!("FILE_CHANGED_DURING_OPEN:{relative}"),
        )?;
        let mut bytes = Vec::new();
        (&mut file)
            .take(maximum as u64 + 1)
            .read_to_end(&mut bytes)?;
        require(bytes.len() <= maximum, format!("TEXT_TOO_LARGE:{relative}"))?;
        require(
            bytes.len() as u64 == opened.len() && file.metadata()?.len() == opened.len(),
            format!("FILE_SIZE_CHANGED:{relative}"),
        )?;
        Ok(bytes)
    }
    pub fn publish(root: &File, relative: &str, bytes: &[u8]) -> Result<()> {
        let (parent, leaf) = parent(root, relative, true)?;
        let fd = parent.as_raw_fd();
        match metadata(fd, &leaf) {
            Ok(_) => {
                require(
                    read_regular(fd, &leaf, bytes.len(), relative)? == bytes,
                    format!("IMMUTABLE_OUTPUT_EXISTS:{relative}"),
                )?;
                return Ok(());
            }
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
            Err(error) => return Err(error.into()),
        }
        let sequence = SEQUENCE.fetch_add(1, Ordering::Relaxed);
        let stage = name(&format!(".cinematic-{}-{sequence}", std::process::id()))?;
        let mut file = open(
            fd,
            &stage,
            libc::O_WRONLY | libc::O_CREAT | libc::O_EXCL | libc::O_NOFOLLOW,
            0o600,
        )?;
        let mut published = false;
        let result = (|| -> Result<()> {
            file.write_all(bytes)?;
            file.sync_all()?;
            let result = unsafe { libc::linkat(fd, stage.as_ptr(), fd, leaf.as_ptr(), 0) };
            if result != 0 {
                let error = std::io::Error::last_os_error();
                if error.kind() != std::io::ErrorKind::AlreadyExists {
                    return Err(error.into());
                }
                require(
                    read_regular(fd, &leaf, bytes.len(), relative)? == bytes,
                    format!("IMMUTABLE_OUTPUT_RACE:{relative}"),
                )?;
            }
            published = true;
            parent.sync_all()?;
            Ok(())
        })();
        let cleanup = unsafe { libc::unlinkat(fd, stage.as_ptr(), 0) };
        if cleanup != 0 {
            return Err(Error::unknown(format!(
                "PUBLICATION_CLEANUP_FAILED:{relative}:{}",
                std::io::Error::last_os_error()
            )));
        }
        if published {
            result.map_err(|error| {
                Error::unknown(format!("PUBLICATION_NOT_CONFIRMED:{relative}:{error}"))
            })
        } else {
            result
        }
    }
}
