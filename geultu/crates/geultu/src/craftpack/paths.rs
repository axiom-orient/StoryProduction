//! Asset-root configuration and default craft/refinement pack discovery.

use super::{
    CraftpackPaths, ASSETS_ROOT_ENV, CRAFT_PACK_DIR_NAME, CRAFT_PACK_ROOT_ENV,
    REFINEMENT_PACK_DIR_NAME,
};
use std::path::{Path, PathBuf};

impl CraftpackPaths {
    /// Builds both pack paths from a directory containing the `assets/`
    /// pack directories directly.
    #[must_use]
    pub fn from_assets_root(assets_root: impl AsRef<Path>) -> Self {
        let root = assets_root.as_ref();
        Self {
            craft_pack_dir: root.join(CRAFT_PACK_DIR_NAME),
            refinement_pack_dir: root.join(REFINEMENT_PACK_DIR_NAME),
        }
    }

    /// Resolves the default locations: an explicit assets root, then the
    /// working directory's ancestors, then the compile-time crate layout.
    /// A direct craft-pack override affects only `craft_pack_dir`; the assets
    /// root continues to select the optional refinement pack. Explicit
    /// environment configuration is returned as-is; loaders report
    /// missing/corrupt packs.
    #[must_use]
    pub fn discover() -> Option<Self> {
        let assets_root = std::env::var_os(ASSETS_ROOT_ENV)
            .map(PathBuf::from)
            .or_else(|| {
                discover_assets_root(
                    std::env::current_dir().ok(),
                    PathBuf::from(env!("CARGO_MANIFEST_DIR")),
                )
            });
        let craft_pack_root = std::env::var_os(CRAFT_PACK_ROOT_ENV).map(PathBuf::from);
        resolve_paths(assets_root, craft_pack_root)
    }
}

fn discover_assets_root(current_dir: Option<PathBuf>, crate_dir: PathBuf) -> Option<PathBuf> {
    let mut candidates = Vec::new();
    if let Some(cwd) = current_dir {
        candidates.push(cwd);
    }
    candidates.push(crate_dir);
    for start in candidates {
        for ancestor in start.ancestors() {
            let assets_root = ancestor.join("assets");
            if assets_root.join(CRAFT_PACK_DIR_NAME).is_dir() {
                return Some(assets_root);
            }
        }
    }
    None
}

fn resolve_paths(
    assets_root: Option<PathBuf>,
    craft_pack_root: Option<PathBuf>,
) -> Option<CraftpackPaths> {
    match (assets_root, craft_pack_root) {
        (Some(root), Some(craft_pack_dir)) => Some(CraftpackPaths {
            craft_pack_dir,
            refinement_pack_dir: root.join(REFINEMENT_PACK_DIR_NAME),
        }),
        (Some(root), None) => Some(CraftpackPaths::from_assets_root(root)),
        (None, Some(craft_pack_dir)) => {
            let refinement_pack_dir = craft_pack_dir
                .parent()
                .filter(|parent| !parent.as_os_str().is_empty())
                .unwrap_or_else(|| Path::new("."))
                .join(REFINEMENT_PACK_DIR_NAME);
            Some(CraftpackPaths {
                craft_pack_dir,
                refinement_pack_dir,
            })
        }
        (None, None) => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::time::{SystemTime, UNIX_EPOCH};

    fn temp_root() -> PathBuf {
        let nonce = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        std::env::temp_dir().join(format!("geultu-craftpack-paths-{nonce}"))
    }

    #[test]
    fn direct_craft_path_overrides_only_the_craft_pack() {
        let root = temp_root();
        let assets = root.join("selected-assets");
        let direct = root.join("story-baseline-v1");
        let paths = resolve_paths(Some(assets.clone()), Some(direct.clone())).unwrap();
        assert_eq!(paths.craft_pack_dir, direct);
        assert_eq!(
            paths.refinement_pack_dir,
            assets.join(REFINEMENT_PACK_DIR_NAME)
        );
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn direct_craft_path_alone_remains_an_explicit_configuration() {
        let root = temp_root();
        let direct = root.join("story-baseline-v1");
        let paths = resolve_paths(None, Some(direct.clone())).unwrap();
        assert_eq!(paths.craft_pack_dir, direct);
        assert_eq!(
            paths.refinement_pack_dir,
            root.join(REFINEMENT_PACK_DIR_NAME)
        );
        assert!(resolve_paths(None, None).is_none());
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn source_layout_discovery_still_prefers_the_current_directory() {
        let root = temp_root();
        let cwd = root.join("workspace");
        let crate_dir = root.join("crate");
        fs::create_dir_all(cwd.join("assets").join(CRAFT_PACK_DIR_NAME)).unwrap();
        fs::create_dir_all(crate_dir.join("assets").join(CRAFT_PACK_DIR_NAME)).unwrap();
        assert_eq!(
            discover_assets_root(Some(cwd.join("child")), crate_dir),
            Some(cwd.join("assets"))
        );
        let _ = fs::remove_dir_all(root);
    }
}
