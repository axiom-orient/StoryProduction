//! Public domain-neutral digest, durable publication, and lock entrypoints.

mod digest;
mod locks;
mod publication;

pub use digest::{sha256_bytes, sha256_file};
pub use locks::{agent_run_is_active, lock_project, try_lock_agent_run, ProjectLock};
pub use publication::{atomic_publish_file, atomic_write, atomic_write_text};

pub fn timestamp_compact() -> String {
    chrono::Local::now().format("%Y%m%d%H%M%S%3f").to_string()
}

#[cfg(unix)]
fn same_file(left: &std::fs::Metadata, right: &std::fs::Metadata) -> bool {
    use std::os::unix::fs::MetadataExt;
    left.dev() == right.dev() && left.ino() == right.ino()
}
