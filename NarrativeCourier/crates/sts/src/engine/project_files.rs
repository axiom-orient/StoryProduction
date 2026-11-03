//! STS project names, reference filenames, and supported input type classification.

use narrative_courier_runtime::store::timestamp_compact;
use narrative_courier_runtime::{Error, Result};
use std::fs::{self, File};
use std::io::Read;
use std::path::Path;

pub(super) fn generated_project_name(title: &str) -> String {
    let mut base = String::new();
    let mut dash = false;
    for c in title.chars() {
        if c.is_ascii_alphanumeric() {
            base.push(c.to_ascii_lowercase());
            dash = false;
        } else if !base.is_empty() && !dash {
            base.push('-');
            dash = true;
        }
        if base.len() >= 28 {
            break;
        }
    }
    while base.ends_with('-') {
        base.pop();
    }
    if base.is_empty() {
        base.push_str("project");
    }
    format!("{}-{}", base, timestamp_compact())
}

pub(super) fn sanitize_filename(name: &str) -> Result<String> {
    let path = Path::new(name);
    if path.is_absolute()
        || name.contains(':')
        || path.components().count() != 1
        || matches!(
            path.components().next(),
            Some(std::path::Component::ParentDir | std::path::Component::RootDir)
        )
    {
        return Err(Error::Contract(format!("unsafe filename: {name}")));
    }
    let clean: String = name
        .chars()
        .map(|c| {
            if c.is_control() || matches!(c, '/' | '\\' | ':') {
                '_'
            } else {
                c
            }
        })
        .collect();
    if clean.trim().is_empty() || clean == "." || clean == ".." {
        return Err(Error::Contract("invalid filename".into()));
    }
    Ok(clean)
}

pub(super) fn validate_reference_type(name: &str) -> Result<()> {
    let ext = Path::new(name)
        .extension()
        .and_then(|v| v.to_str())
        .map(|v| v.to_ascii_lowercase())
        .ok_or_else(|| Error::Contract(format!("reference file extension is required: {name}")))?;
    if !matches!(
        ext.as_str(),
        "pdf" | "docx" | "txt" | "md" | "json" | "png" | "jpg" | "jpeg" | "webp" | "gif"
    ) {
        return Err(Error::Contract(format!(
            "unsupported reference type: {name}"
        )));
    }
    Ok(())
}

pub(super) fn is_text_file(path: &Path) -> bool {
    matches!(
        path.extension()
            .and_then(|v| v.to_str())
            .map(|v| v.to_ascii_lowercase())
            .as_deref(),
        Some("md" | "txt" | "yaml" | "yml" | "json" | "toml" | "csv")
    )
}

pub(super) fn is_extractable_document(path: &Path) -> bool {
    matches!(
        path.extension()
            .and_then(|v| v.to_str())
            .map(|v| v.to_ascii_lowercase())
            .as_deref(),
        Some("pdf" | "docx")
    )
}

/// Read reference bytes under the existing 25 MiB admission limit, including growth during read.
pub(super) fn read_reference_bytes(path: &Path) -> Result<Vec<u8>> {
    const MAX_BYTES: u64 = 25 * 1024 * 1024;
    if !fs::metadata(path)?.is_file() {
        return Err(Error::Contract(format!(
            "reference is not a regular file: {}",
            path.display()
        )));
    }
    let file = File::open(path)?;
    let metadata = file.metadata()?;
    if !metadata.is_file() || metadata.len() > MAX_BYTES {
        return Err(Error::Contract(format!(
            "reference is not regular or exceeds 25 MiB: {}",
            path.display()
        )));
    }
    let mut bytes = Vec::new();
    file.take(MAX_BYTES + 1).read_to_end(&mut bytes)?;
    if bytes.len() as u64 > MAX_BYTES {
        return Err(Error::Contract(format!(
            "reference exceeds 25 MiB: {}",
            path.display()
        )));
    }
    Ok(bytes)
}
