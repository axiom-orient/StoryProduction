//! Backend descriptor shape and producer-owned fingerprint grammar.

use std::collections::BTreeSet;

use narrative_courier_contracts::OutputFormat;
use narrative_courier_runtime::token::hash_jcs;
use narrative_courier_runtime::{Error, Result};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
#[serde(rename_all = "UPPERCASE")]
pub enum BackendOperation {
    Render,
    Materialize,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct BackendLimits {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub width_multiple: Option<u32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub height_multiple: Option<u32>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct BackendDescriptor {
    pub backend_id: String,
    pub version: String,
    pub fingerprint: String,
    pub operations: Vec<BackendOperation>,
    pub formats: Vec<OutputFormat>,
    pub limits: BackendLimits,
}

pub(super) fn fingerprint(descriptor: &BackendDescriptor) -> Result<String> {
    #[derive(Serialize)]
    struct Payload<'a> {
        backend_id: &'a str,
        version: &'a str,
        operations: &'a [BackendOperation],
        formats: &'a [OutputFormat],
        limits: &'a BackendLimits,
    }
    hash_jcs(&Payload {
        backend_id: &descriptor.backend_id,
        version: &descriptor.version,
        operations: &descriptor.operations,
        formats: &descriptor.formats,
        limits: &descriptor.limits,
    })
}

pub(super) fn validate(descriptor: &BackendDescriptor) -> Result<()> {
    if descriptor.backend_id.trim().is_empty()
        || descriptor.version.trim().is_empty()
        || descriptor.operations.is_empty()
        || descriptor.formats.is_empty()
        || descriptor.limits.width_multiple == Some(0)
        || descriptor.limits.height_multiple == Some(0)
    {
        return Err(Error::Contract(
            "INVALID_BACKEND: empty identity/capability or zero dimension multiple".into(),
        ));
    }
    let operation_count = descriptor.operations.iter().collect::<BTreeSet<_>>().len();
    let format_count = descriptor.formats.iter().collect::<BTreeSet<_>>().len();
    if operation_count != descriptor.operations.len()
        || format_count != descriptor.formats.len()
        || fingerprint(descriptor)? != descriptor.fingerprint
    {
        return Err(Error::Contract(
            "INVALID_BACKEND: duplicate capability or fingerprint mismatch".into(),
        ));
    }
    Ok(())
}
