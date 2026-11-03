//! Pure validation-receipt recording decision.

use super::super::Event;
use crate::contracts::ValidationReceipt;
use crate::error::{Error, Result};
use crate::model::Project;

pub(super) fn record(project: &Project, receipt: ValidationReceipt) -> Result<Vec<Event>> {
    receipt.validate()?;
    match project.validation_receipts.get(&receipt.receipt_id) {
        Some(existing) if existing == &receipt => Ok(Vec::new()),
        Some(_) => Err(Error::Corrupt(format!(
            "validation receipt '{}' already recorded with different content",
            receipt.receipt_id
        ))),
        None => Ok(vec![Event::ValidationReceiptRecorded { receipt }]),
    }
}
