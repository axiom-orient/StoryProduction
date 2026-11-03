//! NarrativeCourier application configuration shared by its transport adapters.

use std::env;
use std::path::PathBuf;

mod platform;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Roots {
    pub assets: PathBuf,
    pub projects: PathBuf,
}

#[derive(Debug)]
pub enum ConfigError {
    InvalidPath {
        setting: &'static str,
        reason: &'static str,
    },
    WorkingDirectory(std::io::Error),
}

impl std::fmt::Display for ConfigError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::InvalidPath { setting, reason } => write!(formatter, "{setting}: {reason}"),
            Self::WorkingDirectory(error) => write!(
                formatter,
                "cannot resolve relative application root: {error}"
            ),
        }
    }
}

impl std::error::Error for ConfigError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::WorkingDirectory(error) => Some(error),
            _ => None,
        }
    }
}

/// Resolve shared product roots. Explicit adapter values win over environment
/// variables, then the platform application-data directory and repository defaults.
pub fn resolve_roots(
    explicit_assets: Option<PathBuf>,
    explicit_projects: Option<PathBuf>,
) -> Result<Roots, ConfigError> {
    let environment_assets = env::var_os("STS_ASSETS_ROOT").map(PathBuf::from);
    let environment_projects = env::var_os("STS_PROJECTS_ROOT").map(PathBuf::from);
    // Unselected lower-priority platform values cannot invalidate an explicit project root.
    let product_data_dir = if explicit_projects.is_none() && environment_projects.is_none() {
        platform::default_product_data_dir()?
    } else {
        None
    };
    resolve_roots_from(
        repository_root(),
        environment_assets,
        environment_projects,
        product_data_dir,
        explicit_assets,
        explicit_projects,
    )
}

fn resolve_roots_from(
    repository: PathBuf,
    environment_assets: Option<PathBuf>,
    environment_projects: Option<PathBuf>,
    product_data_dir: Option<PathBuf>,
    explicit_assets: Option<PathBuf>,
    explicit_projects: Option<PathBuf>,
) -> Result<Roots, ConfigError> {
    let (assets, assets_setting) = explicit_assets
        .map(|path| (path, "explicit assets root"))
        .or_else(|| environment_assets.map(|path| (path, "STS_ASSETS_ROOT")))
        .unwrap_or_else(|| (repository.clone(), "repository assets root"));
    let (projects, projects_setting) = explicit_projects
        .map(|path| (path, "explicit projects root"))
        .or_else(|| environment_projects.map(|path| (path, "STS_PROJECTS_ROOT")))
        .or_else(|| product_data_dir.map(|path| (path.join("projects"), "platform projects root")))
        .unwrap_or_else(|| (repository.join("projects"), "repository projects root"));
    for (path, setting) in [(&assets, assets_setting), (&projects, projects_setting)] {
        if path.as_os_str().is_empty() {
            return Err(ConfigError::InvalidPath {
                setting,
                reason: "path must not be empty",
            });
        }
    }
    let working_directory = if !assets.is_absolute() || !projects.is_absolute() {
        Some(env::current_dir().map_err(ConfigError::WorkingDirectory)?)
    } else {
        None
    };
    let absolute = |path: PathBuf, setting| {
        let resolved = match &working_directory {
            Some(directory) if !path.is_absolute() => directory.join(path),
            _ => path,
        };
        if !resolved.is_absolute() {
            return Err(ConfigError::InvalidPath {
                setting,
                reason: "drive-relative root cannot be anchored to the working directory",
            });
        }
        Ok(resolved)
    };
    Ok(Roots {
        assets: absolute(assets, assets_setting)?,
        projects: absolute(projects, projects_setting)?,
    })
}

fn repository_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .ancestors()
        .nth(2)
        .expect("NarrativeCourier app-config crate lives inside the repository root")
        .to_path_buf()
}

#[cfg(test)]
mod tests {
    use super::resolve_roots_from;
    use std::path::PathBuf;

    fn fixture_path(path: &str) -> PathBuf {
        #[cfg(windows)]
        {
            PathBuf::from(format!("C:{path}"))
        }
        #[cfg(not(windows))]
        {
            PathBuf::from(path)
        }
    }

    #[test]
    fn explicit_adapter_roots_override_environment_and_platform_defaults() {
        let roots = resolve_roots_from(
            fixture_path("/repo"),
            Some(fixture_path("/environment/assets")),
            Some(fixture_path("/environment/projects")),
            Some(fixture_path("/platform/data")),
            Some(fixture_path("/explicit/assets")),
            Some(fixture_path("/explicit/projects")),
        )
        .expect("fixture roots resolve");

        assert_eq!(roots.assets, fixture_path("/explicit/assets"));
        assert_eq!(roots.projects, fixture_path("/explicit/projects"));
    }

    #[test]
    fn environment_roots_override_product_and_repository_defaults() {
        let roots = resolve_roots_from(
            fixture_path("/repo"),
            Some(fixture_path("/environment/assets")),
            Some(fixture_path("/environment/projects")),
            Some(fixture_path("/platform/data")),
            None,
            None,
        )
        .expect("fixture roots resolve");

        assert_eq!(roots.assets, fixture_path("/environment/assets"));
        assert_eq!(roots.projects, fixture_path("/environment/projects"));
    }

    #[test]
    fn platform_data_projects_precede_repository_fallback() {
        let roots = resolve_roots_from(
            fixture_path("/repo"),
            None,
            None,
            Some(fixture_path("/platform/data")),
            None,
            None,
        )
        .expect("fixture roots resolve");

        assert_eq!(roots.assets, fixture_path("/repo"));
        assert_eq!(roots.projects, fixture_path("/platform/data/projects"));
    }

    #[test]
    fn repository_projects_are_the_last_fallback() {
        let roots = resolve_roots_from(fixture_path("/repo"), None, None, None, None, None)
            .expect("fixture roots resolve");

        assert_eq!(roots.assets, fixture_path("/repo"));
        assert_eq!(roots.projects, fixture_path("/repo/projects"));
    }
}
