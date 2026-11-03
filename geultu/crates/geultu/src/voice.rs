//! Deterministic, request-local 글투 measurement.
//!
//! Sample prose is accepted by this module only for the duration of a request.
//! The returned profile contains bounded measurements, counts, an identity
//! digest, and fixed Korean guidance; it never contains sample excerpts.

mod compare;
mod guide;
mod measurements;
mod profile;

pub use compare::compare_to_profile;
pub use guide::build_guide;
pub use profile::{build_profile, validate_profile};

pub const MAX_VOICE_SAMPLES: usize = 32;
pub const MAX_VOICE_SAMPLE_BYTES: usize = 2 * 1024 * 1024;
pub const MAX_VOICE_CORPUS_BYTES: usize = 20 * 1024 * 1024;
pub const VOICE_ANALYZER_VERSION: &str = "geultu-voice-stdlib-v1";

#[derive(Debug, Clone)]
pub struct VoiceSample {
    pub id: String,
    pub text: String,
}
