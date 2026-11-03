use serde::de::{Deserialize, Deserializer, MapAccess, SeqAccess, Visitor};
use serde_json::{Map, Number, Value};
use sha2::{Digest, Sha256};
use std::fmt;
use std::fs::{self, File, OpenOptions};
use std::io::{Read, Write};
use std::os::unix::fs::{MetadataExt, OpenOptionsExt};
use std::path::Path;
use std::sync::atomic::{AtomicBool, Ordering};

pub type Result<T> = std::result::Result<T, String>;
pub static INTERRUPTED: AtomicBool = AtomicBool::new(false);
pub fn check(ok: bool, code: &str) -> Result<()> {
    if ok {
        Ok(())
    } else {
        Err(code.into())
    }
}
pub fn io<T>(result: std::io::Result<T>) -> Result<T> {
    result.map_err(|error| error.to_string())
}
pub fn sha(bytes: &[u8]) -> String {
    format!("sha256:{:x}", Sha256::digest(bytes))
}

struct Strict(Value);
impl<'de> Deserialize<'de> for Strict {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> std::result::Result<Self, D::Error> {
        struct StrictVisitor;
        impl<'de> Visitor<'de> for StrictVisitor {
            type Value = Strict;
            fn expecting(&self, f: &mut fmt::Formatter) -> fmt::Result {
                f.write_str("strict JSON")
            }
            fn visit_bool<E: serde::de::Error>(self, v: bool) -> std::result::Result<Strict, E> {
                Ok(Strict(Value::Bool(v)))
            }
            fn visit_i64<E: serde::de::Error>(self, v: i64) -> std::result::Result<Strict, E> {
                Ok(Strict(v.into()))
            }
            fn visit_u64<E: serde::de::Error>(self, v: u64) -> std::result::Result<Strict, E> {
                Ok(Strict(v.into()))
            }
            fn visit_f64<E: serde::de::Error>(self, v: f64) -> std::result::Result<Strict, E> {
                Number::from_f64(v)
                    .map(|n| Strict(Value::Number(n)))
                    .ok_or_else(|| E::custom("non-finite JSON"))
            }
            fn visit_str<E: serde::de::Error>(self, v: &str) -> std::result::Result<Strict, E> {
                Ok(Strict(v.into()))
            }
            fn visit_string<E: serde::de::Error>(
                self,
                v: String,
            ) -> std::result::Result<Strict, E> {
                Ok(Strict(v.into()))
            }
            fn visit_unit<E: serde::de::Error>(self) -> std::result::Result<Strict, E> {
                Ok(Strict(Value::Null))
            }
            fn visit_seq<A: SeqAccess<'de>>(
                self,
                mut seq: A,
            ) -> std::result::Result<Strict, A::Error> {
                let mut values = Vec::new();
                while let Some(Strict(v)) = seq.next_element()? {
                    values.push(v);
                }
                Ok(Strict(values.into()))
            }
            fn visit_map<A: MapAccess<'de>>(
                self,
                mut map: A,
            ) -> std::result::Result<Strict, A::Error> {
                let mut values = Map::new();
                while let Some((key, Strict(v))) = map.next_entry::<String, Strict>()? {
                    if values.insert(key.clone(), v).is_some() {
                        return Err(serde::de::Error::custom(format!(
                            "duplicate JSON key: {key}"
                        )));
                    }
                }
                Ok(Strict(values.into()))
            }
        }
        deserializer.deserialize_any(StrictVisitor)
    }
}

pub fn parse(bytes: &[u8]) -> Result<Value> {
    let Strict(value) = serde_json::from_slice(bytes).map_err(|error| error.to_string())?;
    check(value.is_object(), "JSON_OBJECT")?;
    Ok(value)
}

fn canonical(value: &Value, output: &mut String) -> Result<()> {
    match value {
        Value::Object(map) => {
            output.push('{');
            let mut entries: Vec<_> = map.iter().collect();
            entries.sort_by(|a, b| a.0.encode_utf16().cmp(b.0.encode_utf16()));
            for (index, (key, value)) in entries.into_iter().enumerate() {
                if index > 0 {
                    output.push(',');
                }
                output.push_str(&serde_json::to_string(key).map_err(|e| e.to_string())?);
                output.push(':');
                canonical(value, output)?;
            }
            output.push('}');
        }
        Value::Array(values) => {
            output.push('[');
            for (index, value) in values.iter().enumerate() {
                if index > 0 {
                    output.push(',');
                }
                canonical(value, output)?;
            }
            output.push(']');
        }
        Value::Number(number) if number.is_f64() => {
            let value = number.as_f64().ok_or("NUMBER_REQUIRED")?;
            check(value.is_finite(), "NON_FINITE_NUMBER")?;
            if value == 0.0 {
                output.push('0');
            } else if value.fract() == 0.0 && value.abs() < 1e21 {
                output.push_str(&format!("{value:.0}"));
            } else {
                let raw = serde_json::to_string(&value).map_err(|e| e.to_string())?;
                output.push_str(&raw);
            }
        }
        _ => output.push_str(&serde_json::to_string(value).map_err(|e| e.to_string())?),
    }
    Ok(())
}
pub fn canonical_hash(value: &Value) -> Result<String> {
    let mut bytes = String::new();
    canonical(value, &mut bytes)?;
    Ok(sha(bytes.as_bytes()))
}
pub fn seal(mut value: Value) -> Result<Value> {
    let object = value.as_object_mut().ok_or("OBJECT_REQUIRED")?;
    object.remove("contentHash");
    let hash = canonical_hash(&value)?;
    value["contentHash"] = hash.into();
    Ok(value)
}
pub fn stored_bytes(value: &Value) -> Result<Vec<u8>> {
    let mut bytes = serde_json::to_vec_pretty(value).map_err(|e| e.to_string())?;
    bytes.push(b'\n');
    Ok(bytes)
}

/// A selected directory is an owned descriptor, never a pathname that can be rebound.
pub struct Directory {
    file: File,
}
fn name(value: &str) -> Result<std::ffi::CString> {
    std::ffi::CString::new(value).map_err(|_| "UNSAFE_PATH".into())
}
fn parts(relative: &str) -> Result<Vec<&str>> {
    check(
        !relative.is_empty()
            && !relative.contains(['\\', '\0'])
            && !Path::new(relative).is_absolute()
            && relative
                .split('/')
                .all(|part| !part.is_empty() && part != "." && part != ".."),
        "UNSAFE_PATH",
    )?;
    Ok(relative.split('/').collect())
}
fn open_at(
    parent: std::os::fd::RawFd,
    name: &std::ffi::CStr,
    flags: i32,
    mode: u32,
) -> Result<File> {
    use std::os::fd::FromRawFd;
    let fd = unsafe {
        libc::openat(
            parent,
            name.as_ptr(),
            flags | libc::O_CLOEXEC,
            mode as libc::c_uint,
        )
    };
    if fd < 0 {
        let error = std::io::Error::last_os_error();
        return Err(if error.kind() == std::io::ErrorKind::NotFound {
            format!("NOT_FOUND:{error}")
        } else {
            error.to_string()
        });
    }
    Ok(unsafe { File::from_raw_fd(fd) })
}
fn metadata_at(parent: std::os::fd::RawFd, name: &std::ffi::CStr) -> Result<libc::stat> {
    let mut metadata: libc::stat = unsafe { std::mem::zeroed() };
    if unsafe {
        libc::fstatat(
            parent,
            name.as_ptr(),
            &mut metadata,
            libc::AT_SYMLINK_NOFOLLOW,
        )
    } < 0
    {
        let error = std::io::Error::last_os_error();
        return Err(if error.kind() == std::io::ErrorKind::NotFound {
            format!("NOT_FOUND:{error}")
        } else {
            error.to_string()
        });
    }
    Ok(metadata)
}
pub fn root(path: &Path) -> Result<Directory> {
    use std::os::fd::AsRawFd;
    let before = io(fs::symlink_metadata(path))?;
    check(
        before.is_dir() && !before.file_type().is_symlink(),
        "ROOT_NOT_DIRECTORY",
    )?;
    let file = io(OpenOptions::new()
        .read(true)
        .custom_flags(libc::O_DIRECTORY | libc::O_NOFOLLOW | libc::O_CLOEXEC)
        .open(path))?;
    let opened = io(file.metadata())?;
    check(
        before.dev() == opened.dev() && before.ino() == opened.ino(),
        "ROOT_CHANGED_DURING_OPEN",
    )?;
    check(file.as_raw_fd() >= 0, "ROOT_DESCRIPTOR")?;
    Ok(Directory { file })
}
impl Directory {
    pub fn descriptor(&self) -> std::os::fd::RawFd {
        use std::os::fd::AsRawFd;
        self.file.as_raw_fd()
    }
    pub fn directory(&self, relative: &str, create: bool) -> Result<Self> {
        let mut current = Self {
            file: io(self.file.try_clone())?,
        };
        for part in parts(relative)? {
            let name = name(part)?;
            if create {
                let status = unsafe { libc::mkdirat(current.descriptor(), name.as_ptr(), 0o700) };
                if status < 0
                    && std::io::Error::last_os_error().kind() != std::io::ErrorKind::AlreadyExists
                {
                    return Err(std::io::Error::last_os_error().to_string());
                }
                if status == 0 {
                    io(current.file.sync_all())?;
                }
            }
            current = Self {
                file: open_at(
                    current.descriptor(),
                    &name,
                    libc::O_RDONLY | libc::O_DIRECTORY | libc::O_NOFOLLOW,
                    0,
                )?,
            };
        }
        Ok(current)
    }
    fn parent(&self, relative: &str, create: bool) -> Result<(Self, std::ffi::CString)> {
        let mut parts = parts(relative)?;
        let leaf = name(parts.pop().unwrap())?;
        let parent = if parts.is_empty() {
            Self {
                file: io(self.file.try_clone())?,
            }
        } else {
            self.directory(&parts.join("/"), create)?
        };
        Ok((parent, leaf))
    }
    pub fn create_exclusive_directory(&self, relative: &str) -> Result<Option<Self>> {
        let (parent, name) = self.parent(relative, true)?;
        if unsafe { libc::mkdirat(parent.descriptor(), name.as_ptr(), 0o700) } < 0 {
            let error = std::io::Error::last_os_error();
            return if error.kind() == std::io::ErrorKind::AlreadyExists {
                Ok(None)
            } else {
                Err(error.to_string())
            };
        }
        let file = open_at(
            parent.descriptor(),
            &name,
            libc::O_RDONLY | libc::O_DIRECTORY | libc::O_NOFOLLOW,
            0,
        )
        .map_err(|e| format!("PUBLICATION_NOT_CONFIRMED:{e}"))?;
        io(parent.file.sync_all()).map_err(|e| format!("PUBLICATION_NOT_CONFIRMED:{e}"))?;
        Ok(Some(Self { file }))
    }
    pub fn same_directory(&self, other: &Self) -> Result<bool> {
        let a = io(self.file.metadata())?;
        let b = io(other.file.metadata())?;
        Ok(a.dev() == b.dev() && a.ino() == b.ino())
    }
    pub fn partial_media(&self, leaf: &str) -> Result<Media> {
        check(parts(leaf)?.len() == 1, "LEAF_REQUIRED")?;
        let name = name(leaf)?;
        let file = open_at(
            self.descriptor(),
            &name,
            libc::O_RDWR | libc::O_CREAT | libc::O_EXCL | libc::O_NOFOLLOW,
            0o600,
        )?;
        Ok(Media {
            file,
            parent: Self {
                file: io(self.file.try_clone())?,
            },
            name,
        })
    }
}
fn capture(mut file: File, limit: usize) -> Result<Vec<u8>> {
    let before = io(file.metadata())?;
    check(
        before.is_file() && before.len() <= limit as u64,
        "INPUT_TOO_LARGE_OR_NOT_REGULAR",
    )?;
    let mut bytes = Vec::new();
    io(Read::by_ref(&mut file)
        .take(limit as u64 + 1)
        .read_to_end(&mut bytes))?;
    let after = io(file.metadata())?;
    check(
        bytes.len() as u64 == before.len() && same_file_version(&before, &after),
        "INPUT_CHANGED_DURING_READ",
    )?;
    Ok(bytes)
}
fn same_file_version(a: &fs::Metadata, b: &fs::Metadata) -> bool {
    a.dev() == b.dev()
        && a.ino() == b.ino()
        && a.len() == b.len()
        && a.mtime() == b.mtime()
        && a.mtime_nsec() == b.mtime_nsec()
        && a.ctime() == b.ctime()
        && a.ctime_nsec() == b.ctime_nsec()
}
fn regular_at(parent: &Directory, name: &std::ffi::CStr) -> Result<File> {
    let before = metadata_at(parent.descriptor(), name)?;
    check(
        before.st_mode & libc::S_IFMT == libc::S_IFREG,
        "INPUT_NOT_REGULAR",
    )?;
    let file = open_at(
        parent.descriptor(),
        name,
        libc::O_RDONLY | libc::O_NOFOLLOW | libc::O_NONBLOCK,
        0,
    )?;
    let opened = io(file.metadata())?;
    check(
        opened.is_file() && opened.dev() == before.st_dev as u64 && opened.ino() == before.st_ino,
        "INPUT_CHANGED_DURING_OPEN",
    )?;
    Ok(file)
}
/// Absolute executable/font reads do not participate in the project directory namespace.
pub fn read(path: &Path, limit: usize) -> Result<Vec<u8>> {
    let before = io(fs::symlink_metadata(path))?;
    check(
        before.is_file() && !before.file_type().is_symlink(),
        "INPUT_NOT_REGULAR",
    )?;
    let file = io(OpenOptions::new()
        .read(true)
        .custom_flags(libc::O_NOFOLLOW | libc::O_NONBLOCK | libc::O_CLOEXEC)
        .open(path))?;
    check(
        same_file_version(&before, &io(file.metadata())?),
        "INPUT_CHANGED_DURING_OPEN",
    )?;
    capture(file, limit)
}
pub fn read_at(root: &Directory, relative: &str, limit: usize) -> Result<Vec<u8>> {
    let (parent, name) = root.parent(relative, false)?;
    capture(regular_at(&parent, &name)?, limit)
}
pub fn read_json(root: &Directory, relative: &str) -> Result<Value> {
    parse(&read_at(root, relative, 8 * 1024 * 1024)?)
}
pub fn optional(root: &Directory, relative: &str) -> Result<Option<Value>> {
    match read_json(root, relative) {
        Ok(value) => Ok(Some(value)),
        Err(error) if error.starts_with("NOT_FOUND:") => Ok(None),
        Err(error) => Err(error),
    }
}
static TEMP_SEQUENCE: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
struct Stage {
    parent: Directory,
    name: std::ffi::CString,
    file: File,
}
impl Stage {
    fn new(parent: Directory) -> Result<Self> {
        use std::os::fd::FromRawFd;
        for _ in 0..100 {
            let seq = TEMP_SEQUENCE.fetch_add(1, Ordering::Relaxed);
            let name = name(&format!(".smc-animatic-{}-{seq}.tmp", std::process::id()))?;
            let fd = unsafe {
                libc::openat(
                    parent.descriptor(),
                    name.as_ptr(),
                    libc::O_RDWR
                        | libc::O_CREAT
                        | libc::O_EXCL
                        | libc::O_NOFOLLOW
                        | libc::O_CLOEXEC,
                    0o600,
                )
            };
            if fd >= 0 {
                return Ok(Self {
                    parent,
                    name,
                    file: unsafe { File::from_raw_fd(fd) },
                });
            }
            let error = std::io::Error::last_os_error();
            if error.kind() != std::io::ErrorKind::AlreadyExists {
                return Err(error.to_string());
            }
        }
        Err("PUBLICATION_TEMP_RESERVATION_FAILED".into())
    }
    fn publish(self, leaf: &std::ffi::CStr) -> Result<()> {
        io(self.file.sync_all())?;
        self.ensure_owned()?;
        if unsafe {
            libc::linkat(
                self.parent.descriptor(),
                self.name.as_ptr(),
                self.parent.descriptor(),
                leaf.as_ptr(),
                0,
            )
        } < 0
        {
            let error = std::io::Error::last_os_error();
            if error.kind() != std::io::ErrorKind::AlreadyExists {
                return Err(format!("PUBLICATION_NOT_CONFIRMED:{error}"));
            }
            let existing = regular_at(&self.parent, leaf)?;
            check(
                equal_open_files(existing, io(self.file.try_clone())?)?,
                "IMMUTABLE_OUTPUT_CONFLICT",
            )?;
        } else {
            let published = metadata_at(self.parent.descriptor(), leaf)
                .map_err(|e| format!("PUBLICATION_NOT_CONFIRMED:{e}"))?;
            let owned =
                io(self.file.metadata()).map_err(|e| format!("PUBLICATION_NOT_CONFIRMED:{e}"))?;
            check(
                published.st_mode & libc::S_IFMT == libc::S_IFREG
                    && published.st_dev as u64 == owned.dev()
                    && published.st_ino == owned.ino(),
                "PUBLICATION_NOT_CONFIRMED:TEMPORARY_REPLACED",
            )?;
        }
        self.ensure_owned()?;
        if unsafe { libc::unlinkat(self.parent.descriptor(), self.name.as_ptr(), 0) } < 0 {
            return Err(format!(
                "PUBLICATION_NOT_CONFIRMED:{}",
                std::io::Error::last_os_error()
            ));
        }
        io(self.parent.file.sync_all()).map_err(|e| format!("PUBLICATION_NOT_CONFIRMED:{e}"))
    }
    fn ensure_owned(&self) -> Result<()> {
        let path = metadata_at(self.parent.descriptor(), &self.name)
            .map_err(|e| format!("PUBLICATION_NOT_CONFIRMED:{e}"))?;
        let owned =
            io(self.file.metadata()).map_err(|e| format!("PUBLICATION_NOT_CONFIRMED:{e}"))?;
        check(
            path.st_mode & libc::S_IFMT == libc::S_IFREG
                && path.st_dev as u64 == owned.dev()
                && path.st_ino == owned.ino(),
            "PUBLICATION_NOT_CONFIRMED:TEMPORARY_REPLACED",
        )
    }
}
impl Drop for Stage {
    fn drop(&mut self) {
        if self.ensure_owned().is_ok() {
            unsafe {
                libc::unlinkat(self.parent.descriptor(), self.name.as_ptr(), 0);
            }
        }
    }
}
pub fn publish(root: &Directory, relative: &str, bytes: &[u8]) -> Result<()> {
    let (parent, leaf) = root.parent(relative, true)?;
    let mut stage = Stage::new(parent)?;
    io(stage.file.write_all(bytes))?;
    stage.publish(&leaf)
}
pub fn publish_json(root: &Directory, relative: &str, value: &Value) -> Result<()> {
    publish(root, relative, &stored_bytes(value)?)
}
fn equal_open_files(mut a: File, mut b: File) -> Result<bool> {
    use std::io::{Seek, SeekFrom};
    let before_a = io(a.metadata())?;
    let before_b = io(b.metadata())?;
    if before_a.len() != before_b.len() {
        return Ok(false);
    }
    check(before_a.len() <= 2 * 1024 * 1024 * 1024, "MEDIA_SIZE")?;
    io(a.seek(SeekFrom::Start(0)))?;
    io(b.seek(SeekFrom::Start(0)))?;
    let mut remaining = before_a.len();
    let mut left = [0u8; 65536];
    let mut right = [0u8; 65536];
    while remaining > 0 {
        let length = remaining.min(left.len() as u64) as usize;
        io(a.read_exact(&mut left[..length]))?;
        io(b.read_exact(&mut right[..length]))?;
        if left[..length] != right[..length] {
            return Ok(false);
        }
        remaining -= length as u64;
    }
    check(
        same_file_version(&before_a, &io(a.metadata())?)
            && same_file_version(&before_b, &io(b.metadata())?),
        "INPUT_CHANGED_DURING_REPLAY",
    )?;
    Ok(true)
}
fn hash_open_file(mut file: File) -> Result<String> {
    use std::io::{Seek, SeekFrom};
    io(file.seek(SeekFrom::Start(0)))?;
    let before = io(file.metadata())?;
    check(
        before.is_file() && before.len() > 0 && before.len() <= 2 * 1024 * 1024 * 1024,
        "MEDIA_SIZE",
    )?;
    let mut hasher = Sha256::new();
    let mut bytes = [0u8; 65536];
    let mut count = 0;
    loop {
        let length = io(file.read(&mut bytes))?;
        if length == 0 {
            break;
        }
        count += length as u64;
        check(count <= before.len(), "MEDIA_CHANGED_DURING_READ")?;
        hasher.update(&bytes[..length]);
    }
    check(
        count == before.len() && same_file_version(&before, &io(file.metadata())?),
        "MEDIA_CHANGED_DURING_READ",
    )?;
    Ok(format!("sha256:{:x}", hasher.finalize()))
}
pub fn hash_file(root: &Directory, relative: &str) -> Result<String> {
    let (parent, name) = root.parent(relative, false)?;
    hash_open_file(regular_at(&parent, &name)?)
}
pub struct Media {
    file: File,
    parent: Directory,
    name: std::ffi::CString,
}
impl Media {
    pub fn file(&self) -> &File {
        &self.file
    }
    pub fn reader(&mut self) -> Result<File> {
        use std::io::{Seek, SeekFrom};
        io(self.file.seek(SeekFrom::Start(0)))?;
        io(self.file.try_clone())
    }
    pub fn publish(&mut self, root: &Directory, relative: &str) -> Result<()> {
        use std::io::{Seek, SeekFrom};
        io(self.file.sync_all())?;
        let before = io(self.file.metadata())?;
        check(
            before.is_file() && before.len() > 0 && before.len() <= 2 * 1024 * 1024 * 1024,
            "MEDIA_SIZE",
        )?;
        let (parent, leaf) = root.parent(relative, true)?;
        let mut stage = Stage::new(parent)?;
        io(self.file.seek(SeekFrom::Start(0)))?;
        let count = io(std::io::copy(
            &mut Read::by_ref(&mut self.file).take(before.len() + 1),
            &mut stage.file,
        ))?;
        check(
            count == before.len() && same_file_version(&before, &io(self.file.metadata())?),
            "MEDIA_CHANGED_DURING_PUBLICATION",
        )?;
        stage.publish(&leaf)?;
        let path = metadata_at(self.parent.descriptor(), &self.name)?;
        let owned = io(self.file.metadata())?;
        check(
            path.st_dev as u64 == owned.dev() && path.st_ino == owned.ino(),
            "PUBLICATION_NOT_CONFIRMED:PARTIAL_MEDIA_REPLACED",
        )?;
        if unsafe { libc::unlinkat(self.parent.descriptor(), self.name.as_ptr(), 0) } < 0 {
            return Err(format!(
                "PUBLICATION_NOT_CONFIRMED:{}",
                std::io::Error::last_os_error()
            ));
        }
        io(self.parent.file.sync_all()).map_err(|e| format!("PUBLICATION_NOT_CONFIRMED:{e}"))
    }
}
pub fn interrupted() -> bool {
    INTERRUPTED.load(Ordering::SeqCst)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn root_and_opened_parent_never_follow_path_replacements() {
        let temp = tempfile::tempdir().unwrap();
        let selected = temp.path().join("selected");
        let moved = temp.path().join("moved");
        fs::create_dir(&selected).unwrap();
        let root = root(&selected).unwrap();
        let parent = root.directory("animatic", true).unwrap();
        fs::rename(&selected, &moved).unwrap();
        fs::create_dir(&selected).unwrap();
        publish(&root, "animatic/root.json", b"original root").unwrap();
        assert!(!selected.join("animatic").exists());
        fs::rename(moved.join("animatic"), moved.join("original-animatic")).unwrap();
        std::os::unix::fs::symlink(&selected, moved.join("animatic")).unwrap();
        publish(&parent, "held.json", b"original parent").unwrap();
        assert_eq!(
            fs::read(moved.join("original-animatic/held.json")).unwrap(),
            b"original parent"
        );
        assert!(publish(&root, "animatic/outside.json", b"must not escape").is_err());
        assert!(!selected.join("outside.json").exists());
    }
    #[test]
    fn exclusive_media_refuses_existing_leaf_symlinks() {
        let temp = tempfile::tempdir().unwrap();
        let root = root(temp.path()).unwrap();
        let outside = temp.path().join("outside");
        fs::write(&outside, b"preserve").unwrap();
        std::os::unix::fs::symlink(&outside, temp.path().join("partial.webm")).unwrap();
        assert!(root.partial_media("partial.webm").is_err());
        assert_eq!(fs::read(outside).unwrap(), b"preserve");
    }
}
