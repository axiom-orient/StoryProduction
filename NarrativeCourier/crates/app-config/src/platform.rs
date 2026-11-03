//! Platform application-data selection; only absent values permit fallback.

use super::ConfigError;
use std::env;
use std::path::PathBuf;

const PRODUCT_IDENTIFIER: &str = "app.narrative-courier.desktop";

fn platform_path(setting: &'static str) -> Result<Option<PathBuf>, ConfigError> {
    let Some(value) = env::var_os(setting) else {
        return Ok(None);
    };
    let path = PathBuf::from(value);
    if !path.is_absolute() {
        return Err(ConfigError::InvalidPath {
            setting,
            reason: "platform data path must be nonempty and absolute",
        });
    }
    Ok(Some(path))
}

pub(super) fn default_product_data_dir() -> Result<Option<PathBuf>, ConfigError> {
    #[cfg(target_os = "macos")]
    let data = platform_path("HOME")?.map(|home| home.join("Library/Application Support"));
    #[cfg(target_os = "windows")]
    let data = match platform_path("APPDATA")? {
        Some(path) => Some(path),
        None => platform_path("USERPROFILE")?.map(|home| home.join("AppData/Roaming")),
    };
    #[cfg(not(any(target_os = "macos", target_os = "windows")))]
    let data = match platform_path("XDG_DATA_HOME")? {
        Some(path) => Some(path),
        None => platform_path("HOME")?.map(|home| home.join(".local/share")),
    };
    Ok(data.map(|path| path.join(PRODUCT_IDENTIFIER)))
}
