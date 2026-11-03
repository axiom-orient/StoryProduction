//! Delivery-owned project storage paths and canonical JSON I/O.

use narrative_courier_runtime::paths::validate_relative_path;
use narrative_courier_runtime::store::atomic_write;
use narrative_courier_runtime::{Error, Result};
use serde::{de::DeserializeOwned, Serialize};
use std::fs;
use std::path::{Component, Path, PathBuf};

pub(super) fn read_json_bytes<T: DeserializeOwned>(path: &Path) -> Result<(T, Vec<u8>)> {
    let metadata = fs::symlink_metadata(path)?;
    if metadata.file_type().is_symlink() || !metadata.is_file() {
        return Err(Error::Contract(format!(
            "INVALID_PATH: JSON input is not a regular file: {}",
            path.display()
        )));
    }
    let bytes = fs::read(path)?;
    Ok((serde_json::from_slice::<T>(&bytes)?, bytes))
}

pub(super) fn read_json<T: DeserializeOwned>(path: &Path) -> Result<T> {
    Ok(read_json_bytes(path)?.0)
}
pub(super) fn write_json<T: Serialize>(path: &Path, value: &T) -> Result<()> {
    let mut bytes = serde_json::to_vec_pretty(value)?;
    bytes.push(b'\n');
    atomic_write(path, &bytes)
}
pub(crate) fn project_owned_path(project: &Path, relative: &str) -> Result<PathBuf> {
    validate_relative_path(relative)?;
    let root_metadata = fs::symlink_metadata(project)?;
    if root_metadata.file_type().is_symlink() || !root_metadata.is_dir() {
        return Err(Error::Contract(
            "INVALID_PROJECT: root must be a regular directory".into(),
        ));
    }
    let canonical_project = project.canonicalize().map_err(|e| {
        Error::Contract(format!(
            "INVALID_PROJECT: cannot canonicalize project root: {e}"
        ))
    })?;
    let target = project.join(relative);
    let mut current = project.to_path_buf();
    let components: Vec<_> = Path::new(relative).components().collect();
    for (index, component) in components.iter().enumerate() {
        let Component::Normal(component) = component else {
            return Err(Error::Contract(
                "INVALID_PATH: project-relative path contains non-normal component".into(),
            ));
        };
        current.push(component);
        match fs::symlink_metadata(&current) {
            Ok(metadata) => {
                if metadata.file_type().is_symlink() {
                    return Err(Error::Contract(format!(
                        "INVALID_PATH: project-owned path contains symlink: {relative}"
                    )));
                }
                if index + 1 < components.len() && !metadata.is_dir() {
                    return Err(Error::Contract(format!(
                        "INVALID_PATH: parent is not a directory: {relative}"
                    )));
                }
                let canonical = current.canonicalize()?;
                if !canonical.starts_with(&canonical_project) {
                    return Err(Error::Contract(format!(
                        "INVALID_PATH: project-owned path escapes project: {relative}"
                    )));
                }
            }
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
            Err(error) => return Err(error.into()),
        }
    }
    Ok(target)
}

#[cfg(test)]
mod tests {
    use super::project_owned_path;
    use std::fs;
    use tempfile::TempDir;

    #[cfg(unix)]
    #[test]
    fn project_owned_paths_reject_symlink_components_and_targets() {
        use std::os::unix::fs::symlink;

        let tmp = TempDir::new().expect("temp project");
        let project = tmp.path().join("project");
        fs::create_dir_all(project.join("visual/assets")).expect("project dirs");
        let outside = tmp.path().join("outside");
        fs::create_dir_all(&outside).expect("outside dir");
        symlink(&outside, project.join("visual/assets/imports")).expect("symlink component");
        let err = project_owned_path(&project, "visual/assets/imports/image.png")
            .expect_err("symlink component must fail");
        assert!(err.to_string().contains("symlink"));

        fs::remove_file(project.join("visual/assets/imports")).expect("remove component symlink");
        fs::create_dir_all(project.join("visual/assets/imports")).expect("import dir");
        let outside_file = outside.join("image.png");
        fs::write(&outside_file, b"outside").expect("outside file");
        symlink(
            &outside_file,
            project.join("visual/assets/imports/image.png"),
        )
        .expect("symlink target");
        let err = project_owned_path(&project, "visual/assets/imports/image.png")
            .expect_err("symlink target must fail");
        assert!(err.to_string().contains("symlink"));
    }
}
