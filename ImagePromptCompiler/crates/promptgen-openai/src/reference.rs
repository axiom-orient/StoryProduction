use std::fs::{self, File, OpenOptions};
use std::io::{self, Read};
use std::path::{Path, PathBuf};

#[cfg(unix)]
use std::os::unix::fs::MetadataExt;

use super::OpenAiExecutionError;
use super::files::{lexical_absolute, write_private_file};
use super::support::io_error;
use promptgen_core::image_artifact::sha256::digest_hex;

pub(super) const MAX_REFERENCE_BYTES: u64 = 50 * 1024 * 1024;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum ImageMime {
    Png,
    Jpeg,
    Webp,
}

impl ImageMime {
    pub(super) const fn as_str(self) -> &'static str {
        match self {
            Self::Png => "image/png",
            Self::Jpeg => "image/jpeg",
            Self::Webp => "image/webp",
        }
    }

    const fn extension(self) -> &'static str {
        match self {
            Self::Png => "png",
            Self::Jpeg => "jpg",
            Self::Webp => "webp",
        }
    }
}

#[derive(Debug)]
pub(super) struct CopiedReference {
    pub(super) path: PathBuf,
    pub(super) mime: ImageMime,
    pub(super) sha256: String,
    pub(super) bytes: u64,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) struct ReferenceFileIdentity {
    regular: bool,
    length: u64,
    #[cfg(unix)]
    device: u64,
    #[cfg(unix)]
    inode: u64,
}

pub(super) fn reference_file_identity(metadata: &fs::Metadata) -> ReferenceFileIdentity {
    ReferenceFileIdentity {
        regular: metadata.is_file(),
        length: metadata.len(),
        #[cfg(unix)]
        device: metadata.dev(),
        #[cfg(unix)]
        inode: metadata.ino(),
    }
}

fn reference_changed_error() -> OpenAiExecutionError {
    OpenAiExecutionError::new(
        "OPENAI_REFERENCE_CHANGED",
        "reference image path or opened-file identity changed during the copy",
    )
}

pub(super) fn ensure_reference_identity(
    source: &Path,
    input: &File,
    initial: ReferenceFileIdentity,
    bytes_read: usize,
) -> Result<(), OpenAiExecutionError> {
    let opened_metadata = input.metadata().map_err(|_| reference_changed_error())?;
    let opened = reference_file_identity(&opened_metadata);
    let final_metadata = fs::symlink_metadata(source).map_err(|_| reference_changed_error())?;
    if final_metadata.file_type().is_symlink() {
        return Err(reference_changed_error());
    }
    let final_path = reference_file_identity(&final_metadata);
    if !initial.regular
        || !opened.regular
        || !final_path.regular
        || initial != opened
        || initial != final_path
        || opened.length != bytes_read as u64
    {
        return Err(reference_changed_error());
    }
    Ok(())
}

#[cfg(unix)]
pub(super) fn open_reference(path: &Path) -> io::Result<File> {
    use std::os::unix::fs::OpenOptionsExt;

    OpenOptions::new()
        .read(true)
        .custom_flags(libc::O_NOFOLLOW | libc::O_NONBLOCK)
        .open(path)
}

#[cfg(not(unix))]
pub(super) fn open_reference(path: &Path) -> io::Result<File> {
    File::open(path)
}

#[derive(Debug)]
struct AliasPath {
    resolved: PathBuf,
    identity: Option<AliasFileIdentity>,
}

#[cfg(unix)]
type AliasFileIdentity = (u64, u64);

#[cfg(not(unix))]
type AliasFileIdentity = PathBuf;

/// Resolve OpenAI edit input/output aliases, including symlinks and existing-file identity.
pub fn openai_paths_alias(left: &Path, right: &Path) -> io::Result<bool> {
    let left_absolute = lexical_absolute(left)?;
    let right_absolute = lexical_absolute(right)?;
    if left_absolute == right_absolute {
        return Ok(true);
    }
    let left = resolve_alias_path(left_absolute)?;
    let right = resolve_alias_path(right_absolute)?;
    Ok(left.resolved == right.resolved
        || matches!((left.identity, right.identity), (Some(left), Some(right)) if left == right))
}

fn resolve_alias_path(absolute: PathBuf) -> io::Result<AliasPath> {
    let mut existing = absolute.clone();
    let mut missing_components = Vec::new();
    loop {
        match fs::symlink_metadata(&existing) {
            Ok(_) => break,
            Err(error) if error.kind() == io::ErrorKind::NotFound => {
                let component = existing.file_name().ok_or_else(|| {
                    io::Error::new(
                        io::ErrorKind::NotFound,
                        format!("path has no existing prefix: {}", absolute.display()),
                    )
                })?;
                missing_components.push(PathBuf::from(component));
                if !existing.pop() {
                    return Err(error);
                }
            }
            Err(error) => return Err(error),
        }
    }
    let mut resolved = fs::canonicalize(&existing)?;
    if !missing_components.is_empty() && !fs::metadata(&resolved)?.is_dir() {
        return Err(io::Error::new(
            io::ErrorKind::NotADirectory,
            format!("path prefix is not a directory: {}", existing.display()),
        ));
    }
    for component in missing_components.iter().rev() {
        resolved.push(component);
    }
    let identity = match fs::symlink_metadata(&absolute) {
        Ok(_) => {
            resolved = fs::canonicalize(&absolute)?;
            alias_file_identity(&absolute)?
        }
        Err(error) if error.kind() == io::ErrorKind::NotFound => None,
        Err(error) => return Err(error),
    };
    Ok(AliasPath { resolved, identity })
}

fn alias_file_identity(path: &Path) -> io::Result<Option<AliasFileIdentity>> {
    let metadata = fs::metadata(path)?;
    if !metadata.is_file() {
        return Ok(None);
    }
    #[cfg(unix)]
    {
        Ok(Some((metadata.dev(), metadata.ino())))
    }
    #[cfg(not(unix))]
    {
        Ok(Some(fs::canonicalize(path)?))
    }
}

pub(super) fn reject_reference_output_alias(
    reference_path: &Path,
    output_path: &Path,
) -> Result<(), OpenAiExecutionError> {
    if openai_paths_alias(reference_path, output_path)
        .map_err(|error| io_error("resolve reference/output alias", reference_path, error))?
    {
        return Err(OpenAiExecutionError::new(
            "OPENAI_REFERENCE_OUTPUT_ALIAS",
            "reference image and image output must resolve to different files",
        ));
    }
    Ok(())
}

pub(super) fn copy_reference(
    source: &Path,
    workspace: &Path,
) -> Result<CopiedReference, OpenAiExecutionError> {
    let metadata = fs::symlink_metadata(source).map_err(|_| {
        OpenAiExecutionError::new(
            "OPENAI_REFERENCE_INPUT",
            "reference image must be an accessible regular file",
        )
    })?;
    if metadata.file_type().is_symlink() || !metadata.is_file() {
        return Err(OpenAiExecutionError::new(
            "OPENAI_REFERENCE_INPUT",
            "reference image must be a regular non-symlink file",
        ));
    }
    if metadata.len() >= MAX_REFERENCE_BYTES {
        return Err(OpenAiExecutionError::new(
            "OPENAI_REFERENCE_SIZE",
            "reference image must be smaller than 50 MiB",
        ));
    }
    let initial = reference_file_identity(&metadata);
    let input = open_reference(source).map_err(|_| {
        OpenAiExecutionError::new(
            "OPENAI_REFERENCE_INPUT",
            "reference image could not be opened",
        )
    })?;
    let opened_metadata = input.metadata().map_err(|_| reference_changed_error())?;
    if initial != reference_file_identity(&opened_metadata) {
        return Err(reference_changed_error());
    }
    let mut bytes = Vec::with_capacity(metadata.len() as usize);
    (&input)
        .take(MAX_REFERENCE_BYTES)
        .read_to_end(&mut bytes)
        .map_err(|_| {
            OpenAiExecutionError::new(
                "OPENAI_REFERENCE_INPUT",
                "reference image could not be read",
            )
        })?;
    if bytes.len() as u64 >= MAX_REFERENCE_BYTES {
        return Err(OpenAiExecutionError::new(
            "OPENAI_REFERENCE_SIZE",
            "reference image must be smaller than 50 MiB",
        ));
    }
    ensure_reference_identity(source, &input, initial, bytes.len())?;
    let mime = detect_mime(&bytes).ok_or_else(|| {
        OpenAiExecutionError::new(
            "OPENAI_REFERENCE_SIGNATURE",
            "reference image signature must be PNG, JPEG, or WebP",
        )
    })?;
    let destination = workspace.join(format!("reference-1.{}", mime.extension()));
    write_private_file(&destination, &bytes)?;
    Ok(CopiedReference {
        path: destination,
        mime,
        sha256: digest_hex(&bytes),
        bytes: bytes.len() as u64,
    })
}

fn detect_mime(bytes: &[u8]) -> Option<ImageMime> {
    if bytes.starts_with(b"\x89PNG\r\n\x1a\n") {
        return Some(ImageMime::Png);
    }
    if bytes.starts_with(b"\xff\xd8\xff") {
        return Some(ImageMime::Jpeg);
    }
    if bytes.len() >= 12 && &bytes[..4] == b"RIFF" && &bytes[8..12] == b"WEBP" {
        return Some(ImageMime::Webp);
    }
    None
}
