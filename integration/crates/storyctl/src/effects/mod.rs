//! Story transport effects. Owner receipts and domain decisions stay in adapters.
#[allow(unsafe_code)]
pub mod files;
pub mod hash;
pub mod json;
#[allow(unsafe_code)]
pub mod process;
#[allow(unsafe_code)]
pub mod workspace;

pub const MAX_BYTES: usize = 16 * 1024 * 1024;
