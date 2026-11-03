use crate::{Error, Result};
use std::path::{Path, PathBuf};

const MAX_PROJECT_NAME_LENGTH: usize = 64;
const MAX_PATH_SEGMENT_LENGTH: usize = 120;

fn has_portable_file_identity(value: &str) -> bool {
    if value.ends_with(['.', ' '])
        || value.contains(['<', '>', '"', '|', '?', '*'])
        || value.chars().any(|character| character.is_ascii_control())
    {
        return false;
    }
    let stem = value.split('.').next().unwrap_or("").to_ascii_uppercase();
    let device_number = stem
        .strip_prefix("COM")
        .or_else(|| stem.strip_prefix("LPT"))
        .is_some_and(|number| {
            matches!(
                number,
                "1" | "2" | "3" | "4" | "5" | "6" | "7" | "8" | "9" | "¹" | "²" | "³"
            )
        });
    !device_number
        && !matches!(
            stem.as_str(),
            "CON" | "PRN" | "AUX" | "NUL" | "CONIN$" | "CONOUT$"
        )
}

fn is_valid_ascii_path_segment(value: &str) -> bool {
    let mut bytes = value.bytes();
    let Some(first) = bytes.next() else {
        return false;
    };
    has_portable_file_identity(value)
        && first.is_ascii_alphanumeric()
        && bytes.all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_' | b'.'))
}

/// Admit one bounded ASCII path segment with no separator or traversal syntax.
pub fn validate_path_segment(value: &str, field: &str) -> Result<()> {
    if value.is_empty() || value.len() > MAX_PATH_SEGMENT_LENGTH {
        return Err(Error::Contract(format!(
            "{field} must be 1..{MAX_PATH_SEGMENT_LENGTH} characters"
        )));
    }
    if !is_valid_ascii_path_segment(value) {
        return Err(Error::Contract(format!(
            "{field} must start with ASCII alphanumeric and contain only ASCII alphanumeric/._- with portable filename identity"
        )));
    }
    Ok(())
}

/// Admit a non-absolute project-relative path without parent or drive-prefix escape.
pub fn validate_relative_path(value: &str) -> Result<()> {
    if value.is_empty() || value.starts_with('/') {
        return Err(Error::Contract(format!("relative path expected: {value}")));
    }
    if value.contains(['\\', '\0', ':'])
        || value.split('/').any(|part| {
            part.is_empty() || part == "." || part == ".." || !has_portable_file_identity(part)
        })
    {
        return Err(Error::Contract(format!(
            "path escapes boundary or has a non-canonical identity: {value}"
        )));
    }
    Ok(())
}

/// Domain-neutral filesystem boundary for workflow domains.
///
/// The runtime owns only roots and path safety. Domain semantics remain outside this crate.
#[derive(Debug, Clone)]
pub struct RuntimePaths {
    assets_root: PathBuf,
    projects_root: PathBuf,
    domain_dir: PathBuf,
}

impl RuntimePaths {
    pub fn new(
        repository_root: impl Into<PathBuf>,
        domain_dir: impl Into<PathBuf>,
    ) -> Result<Self> {
        let root = repository_root.into();
        Self::with_roots(root.clone(), root.join("projects"), domain_dir)
    }

    pub fn with_roots(
        assets_root: impl Into<PathBuf>,
        projects_root: impl Into<PathBuf>,
        domain_dir: impl Into<PathBuf>,
    ) -> Result<Self> {
        let domain_dir = domain_dir.into();
        let domain = domain_dir
            .to_str()
            .ok_or_else(|| Error::Contract("domain path must be UTF-8".into()))?;
        validate_relative_path(domain)?;
        let assets_root = assets_root.into();
        let projects_root = projects_root.into();
        for (name, path) in [
            ("assets_root", &assets_root),
            ("projects_root", &projects_root),
        ] {
            if path.as_os_str().is_empty() {
                return Err(Error::Contract(format!("{name} must not be empty")));
            }
        }
        let current = if !assets_root.is_absolute() || !projects_root.is_absolute() {
            Some(std::env::current_dir()?)
        } else {
            None
        };
        let anchor = |path: PathBuf| -> Result<PathBuf> {
            let resolved = match &current {
                Some(root) if !path.is_absolute() => root.join(path),
                _ => path,
            };
            if !resolved.is_absolute() {
                return Err(Error::Contract(
                    "drive-relative root cannot be anchored".into(),
                ));
            }
            Ok(resolved)
        };
        Ok(Self {
            assets_root: anchor(assets_root)?,
            projects_root: anchor(projects_root)?,
            domain_dir,
        })
    }

    /// The executable has selected its working directory by handle. Preserve
    /// relative projects paths instead of resolving the mutable directory name.
    pub fn with_working_projects_root(
        assets_root: impl Into<PathBuf>,
        domain_dir: impl Into<PathBuf>,
    ) -> Result<Self> {
        let mut paths = Self::with_roots(assets_root, ".", domain_dir)?;
        paths.projects_root = PathBuf::from(".");
        Ok(paths)
    }

    pub fn assets_root(&self) -> &Path {
        &self.assets_root
    }
    pub fn projects_root(&self) -> &Path {
        &self.projects_root
    }
    pub fn domain_root(&self) -> PathBuf {
        self.assets_root.join(&self.domain_dir)
    }

    pub fn domain_asset(&self, relative: &str) -> Result<PathBuf> {
        validate_relative_path(relative)?;
        Ok(self.domain_root().join(relative))
    }

    pub fn project_dir(&self, name: &str) -> Result<PathBuf> {
        validate_project_name(name)?;
        Ok(self.projects_root.join(name))
    }
}

pub fn validate_project_name(name: &str) -> Result<()> {
    if name.is_empty() || name.len() > MAX_PROJECT_NAME_LENGTH {
        return Err(Error::Contract(format!(
            "project_name must be 1..{MAX_PROJECT_NAME_LENGTH} characters"
        )));
    }
    if !is_valid_ascii_path_segment(name) {
        return Err(Error::Contract(
            "project_name: ASCII alnum first; then alnum/._- only with portable filename identity"
                .into(),
        ));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn project_name_rejects_escape() {
        for value in ["../x", "a/b", "a\\b", "a:b", ".."] {
            assert!(validate_project_name(value).is_err(), "{value}");
        }
    }

    #[test]
    fn domain_dir_is_relative() {
        assert!(RuntimePaths::new("/tmp/repo", "skills/example").is_ok());
        assert!(RuntimePaths::new("/tmp/repo", "../example").is_err());
    }
}
