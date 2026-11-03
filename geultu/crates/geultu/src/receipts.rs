//! Receipt and human-decision issuance (H3): adapters from engine outputs
//! to canonical contracts, with decisions bound to exact artifact bytes.

mod decision;
mod subject;
mod timestamp;
pub(crate) mod validation;

pub use decision::{bind_decision, human_decision};
pub use subject::candidate_subject;
pub use validation::{map_findings, outcome, proof_receipt, review_receipt};

/// Validator identity stamped onto review receipts.
pub const REVIEW_VALIDATOR_ID: &str = "geultu-review";
/// Validator identity stamped onto proof receipts.
pub const PROOF_VALIDATOR_ID: &str = "geultu-proof";

/// Engine version reported as the receipt validator version.
#[must_use]
fn validator_version() -> String {
    env!("CARGO_PKG_VERSION").into()
}
