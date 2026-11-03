//! Startup-only CLI argument parsing and purpose selection.

use std::env;
use std::path::PathBuf;

use crate::error::{CustodyError, Result};
use crate::protocol::PROTOCOL_VERSION;

#[derive(Debug, Clone)]
pub(super) struct Config {
    pub(super) project_root: PathBuf,
    pub(super) purpose: SessionPurpose,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum SessionPurpose {
    ProjectIngress,
    Measurement,
    Narrative,
}

impl SessionPurpose {
    fn parse(value: &str) -> Result<Self> {
        match value {
            "project-ingress" => Ok(Self::ProjectIngress),
            "measurement" => Ok(Self::Measurement),
            "narrative" => Ok(Self::Narrative),
            _ => Err(CustodyError::input(
                "purpose must be exactly project-ingress, measurement, or narrative",
            )),
        }
    }
}

pub(super) fn parse_config() -> Result<Config> {
    let args: Vec<String> = env::args().skip(1).collect();
    let mut project_root = None;
    let mut purpose = None;
    let mut index = 0;
    while index < args.len() {
        match args[index].as_str() {
            "--project-root" => {
                index += 1;
                let value = args.get(index).ok_or_else(|| {
                    CustodyError::input("--project-root requires an absolute path")
                })?;
                if project_root.is_some() {
                    return Err(CustodyError::input("--project-root may be provided once"));
                }
                project_root = Some(PathBuf::from(value));
            }
            "--protocol-version" => {
                index += 1;
                let value = args
                    .get(index)
                    .ok_or_else(|| CustodyError::input("--protocol-version requires a value"))?;
                if value.parse::<u64>().ok() != Some(PROTOCOL_VERSION) {
                    return Err(CustodyError::input(format!(
                        "unsupported protocol version: {value}"
                    )));
                }
            }
            "--purpose" => {
                index += 1;
                let value = args
                    .get(index)
                    .ok_or_else(|| CustodyError::input("--purpose requires a value"))?;
                if purpose.is_some() {
                    return Err(CustodyError::input("--purpose may be provided once"));
                }
                purpose = Some(SessionPurpose::parse(value)?);
            }
            "--help" => {
                println!("smc-trackb-custody --project-root ABSOLUTE_PATH --purpose project-ingress|measurement|narrative [--protocol-version 1]");
                std::process::exit(0);
            }
            unknown => {
                return Err(CustodyError::input(format!(
                    "unknown startup option: {unknown}"
                )));
            }
        }
        index += 1;
    }
    let project_root = project_root
        .ok_or_else(|| CustodyError::input("--project-root is required and must be absolute"))?;
    if !project_root.is_absolute() {
        return Err(CustodyError::input("--project-root must be absolute"));
    }
    let purpose = purpose.ok_or_else(|| {
        CustodyError::input(
            "--purpose is required and must be project-ingress, measurement, or narrative",
        )
    })?;
    Ok(Config {
        project_root,
        purpose,
    })
}
