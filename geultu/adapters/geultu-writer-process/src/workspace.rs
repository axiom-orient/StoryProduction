use geultu::{Error, Result};
use std::fs::{self, File, OpenOptions};
use std::io::{Read, Seek, SeekFrom};
use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};

const TEMP_DIRECTORY_ATTEMPTS: usize = 64;
static NEXT_TEMP_ID: AtomicU64 = AtomicU64::new(1);

pub(super) struct ProcessFiles(PathBuf);
impl ProcessFiles {
    pub(super) fn create() -> Result<Self> {
        for _ in 0..TEMP_DIRECTORY_ATTEMPTS {
            let path = std::env::temp_dir().join(format!(
                "geultu-writer-{}-{}",
                std::process::id(),
                NEXT_TEMP_ID.fetch_add(1, Ordering::Relaxed)
            ));
            let mut builder = fs::DirBuilder::new();
            #[cfg(unix)]
            {
                use std::os::unix::fs::DirBuilderExt;
                builder.mode(0o700);
            }
            match builder.create(&path) {
                Ok(()) => return Ok(Self(path)),
                Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => continue,
                Err(error) => return Err(error.into()),
            }
        }
        Err(Error::AdapterUnavailable(
            "cannot allocate a private writer directory".into(),
        ))
    }

    pub(super) fn file(&self, name: &str) -> Result<File> {
        Ok(OpenOptions::new()
            .read(true)
            .write(true)
            .create_new(true)
            .open(self.0.join(name))?)
    }
}
impl Drop for ProcessFiles {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

pub(super) fn check_size(file: &File, limit: usize, label: &str) -> Result<()> {
    if file.metadata()?.len() > limit as u64 {
        return Err(Error::AdapterUnavailable(format!(
            "writer {label} exceeded its byte limit"
        )));
    }
    Ok(())
}

pub(super) fn read_bounded(file: &mut File, limit: usize, label: &str) -> Result<Vec<u8>> {
    check_size(file, limit, label)?;
    file.seek(SeekFrom::Start(0))?;
    let mut bytes = Vec::new();
    file.take(limit as u64 + 1).read_to_end(&mut bytes)?;
    if bytes.len() > limit {
        return Err(Error::AdapterUnavailable(format!(
            "writer {label} exceeded its byte limit"
        )));
    }
    Ok(bytes)
}
