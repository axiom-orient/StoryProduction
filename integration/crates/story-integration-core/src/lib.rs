//! Pure Story execution invariants. No producer domain, I/O, JSON or clock.
#![forbid(unsafe_code)]
mod capability;
mod identity;
mod outcome;
pub use capability::{CapabilityState, JobStatus};
pub use identity::{AttemptId, ProjectId, Sha256};
pub use outcome::Outcome;
