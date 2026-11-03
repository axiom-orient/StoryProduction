//! Bounded text/block admission shared by authoring mutations and persisted graph checks.

use super::Block;
use crate::domain::validation::validate_id;
use crate::model::MAX_TEXT_BYTES;
use crate::{Error, Result};
use std::collections::BTreeSet;

pub(super) fn text(value: &str, label: &str) -> Result<()> {
    if value.trim().is_empty() || value.len() > MAX_TEXT_BYTES {
        return Err(Error::InvalidInput(format!(
            "{label} must be nonempty and at most {MAX_TEXT_BYTES} UTF-8 bytes"
        )));
    }
    Ok(())
}

pub(super) fn blocks_valid(blocks: &[Block]) -> Result<()> {
    if blocks.is_empty() || blocks.len() > 512 {
        return Err(Error::InvalidInput("provide 1..=512 blocks".into()));
    }
    if serde_json::to_vec(blocks)?.len() > MAX_TEXT_BYTES {
        return Err(Error::InvalidInput("document exceeds text limit".into()));
    }
    let mut ids = BTreeSet::new();
    for block in blocks {
        validate_id("block id", &block.id)?;
        text(&block.heading, "block heading")?;
        text(&block.text, "block text")?;
        if !ids.insert(&block.id) {
            return Err(Error::InvalidInput(format!("duplicate block {}", block.id)));
        }
    }
    Ok(())
}
