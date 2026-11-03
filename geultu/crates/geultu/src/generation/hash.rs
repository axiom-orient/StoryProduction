//! Stable content identities for generation inputs and plans.

use super::GenerationInput;
use crate::contracts::digest::serde_json_sha256;

pub fn context_digest(input: &GenerationInput) -> String {
    serde_json_sha256(input).expect("generation input is serializable")
}
