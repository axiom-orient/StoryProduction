use std::path::{Path, PathBuf};
use std::{env, io};

use super::error::CliError;

pub(super) fn resolve_codex_home(explicit: Option<PathBuf>) -> Result<PathBuf, CliError> {
    if let Some(path) = explicit {
        return Ok(path);
    }
    if let Some(path) = env::var_os("CODEX_HOME") {
        return Ok(PathBuf::from(path));
    }
    let home = env::var_os("HOME")
        .or_else(|| env::var_os("USERPROFILE"))
        .ok_or_else(|| CliError::usage("cannot resolve CODEX_HOME; pass --codex-home"))?;
    Ok(PathBuf::from(home).join(".codex"))
}

pub(super) fn same_lexical_path(left: &Path, right: &Path) -> Result<bool, CliError> {
    let left_absolute = absolute_lexical(left)
        .map_err(|error| CliError::io("resolve lexical output path", left, error))?;
    let right_absolute = absolute_lexical(right)
        .map_err(|error| CliError::io("resolve lexical output path", right, error))?;
    Ok(left_absolute == right_absolute)
}

fn absolute_lexical(path: &Path) -> io::Result<PathBuf> {
    let mut output = if path.is_absolute() {
        PathBuf::new()
    } else {
        env::current_dir()?
    };
    for component in path.components() {
        match component {
            std::path::Component::Prefix(prefix) => output.push(prefix.as_os_str()),
            std::path::Component::RootDir => {
                output.push(Path::new(std::path::MAIN_SEPARATOR_STR));
            }
            std::path::Component::CurDir => {}
            std::path::Component::ParentDir => {
                output.pop();
            }
            std::path::Component::Normal(value) => output.push(value),
        }
    }
    Ok(output)
}
