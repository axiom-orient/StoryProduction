//! HumanDecision v1 schema contract and timestamp/identity validation.

use super::validate::{check_length, valid_prefixed_sha256};
use crate::contracts::ArtifactRef;
use crate::error::{Error, Result};
use serde::{Deserialize, Serialize};

/// `decision` enum of `human-decision.schema.json`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "UPPERCASE")]
pub enum DecisionKind {
    Approve,
    Reject,
}

/// HumanDecision v1 (`human-decision.schema.json`).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct HumanDecision {
    pub decision_id: String,
    pub actor_id: String,
    pub subject: ArtifactRef,
    pub scope: String,
    pub decision: DecisionKind,
    pub occurred_at: String,
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        deserialize_with = "super::validate::deserialize_present"
    )]
    pub reason: Option<String>,
}

impl HumanDecision {
    /// Validates every schema constraint: `decision_id`
    /// (`^human_[0-9a-f]{64}$`), `actor_id` (1..=256 characters),
    /// subject ref, `scope` (1..=512 characters), RFC 3339 `occurred_at`,
    /// and optional `reason` (at most 4000 characters).
    ///
    /// # Errors
    /// Returns [`Error::InvalidInput`] naming the first violated constraint.
    pub fn validate(&self) -> Result<()> {
        if !valid_decision_id(self.decision_id.as_str()) {
            return Err(Error::InvalidInput(format!(
                "decision_id '{}' must match ^human_[0-9a-f]{{64}}$",
                self.decision_id
            )));
        }
        check_length("actor_id", &self.actor_id, 1, 256)?;
        self.subject.validate()?;
        check_length("scope", &self.scope, 1, 512)?;
        if !valid_rfc3339_datetime(self.occurred_at.as_str()) {
            return Err(Error::InvalidInput(format!(
                "occurred_at '{}' must be an RFC 3339 date-time",
                self.occurred_at
            )));
        }
        if let Some(reason) = &self.reason {
            check_length("reason", reason, 0, 4000)?;
        }
        Ok(())
    }
}

/// `^human_[0-9a-f]{64}$`
fn valid_decision_id(value: &str) -> bool {
    valid_prefixed_sha256(value, "human_")
}

/// Shape-and-range RFC 3339 check:
/// `YYYY-MM-DDTHH:MM:SS(.fraction)?(Z|±HH:MM)` with range bounds on month,
/// day, hour, minute, and second, including Gregorian month/leap-year bounds.
/// Leap-second value 60 is retained; no external leap-second schedule is consulted.
fn valid_rfc3339_datetime(value: &str) -> bool {
    let Some((date, rest)) = value.split_once(['T', 't']) else {
        return false;
    };
    let clock = if let Some(stripped) = rest.strip_suffix('Z').or_else(|| rest.strip_suffix('z')) {
        stripped
    } else {
        let Some((separator_index, _)) = rest
            .char_indices()
            .rev()
            .find(|(_, character)| matches!(character, '+' | '-'))
        else {
            return false;
        };
        let (candidate_clock, offset) = (&rest[..separator_index], &rest[separator_index + 1..]);
        if !valid_offset(offset) {
            return false;
        }
        candidate_clock
    };
    let mut date_parts = date.split('-');
    let (Some(year), Some(month), Some(day)) =
        (date_parts.next(), date_parts.next(), date_parts.next())
    else {
        return false;
    };
    let digits = |part: &str| !part.is_empty() && part.bytes().all(|byte| byte.is_ascii_digit());
    if date_parts.next().is_some() || year.len() != 4 || !digits(year) {
        return false;
    }
    if !two_digit_in_range(month, 1, 12) || !two_digit_in_range(day, 1, 31) {
        return false;
    }
    let Ok(year) = year.parse::<u32>() else {
        return false;
    };
    let max_day = match month {
        "02" if year % 4 == 0 && (year % 100 != 0 || year % 400 == 0) => 29,
        "02" => 28,
        "04" | "06" | "09" | "11" => 30,
        _ => 31,
    };
    if !two_digit_in_range(day, 1, max_day) {
        return false;
    }
    match clock.split_once('.') {
        Some((time, fraction)) => digits(fraction) && valid_clock(time),
        None => valid_clock(clock),
    }
}

fn valid_offset(offset: &str) -> bool {
    let Some((hour, minute)) = offset.split_once(':') else {
        return false;
    };
    two_digit_in_range(hour, 0, 23) && two_digit_in_range(minute, 0, 59)
}

fn two_digit_in_range(value: &str, low: u32, high: u32) -> bool {
    value.len() == 2
        && value.bytes().all(|byte| byte.is_ascii_digit())
        && value
            .parse::<u32>()
            .is_ok_and(|parsed| (low..=high).contains(&parsed))
}

fn valid_clock(clock: &str) -> bool {
    let mut parts = clock.split(':');
    let (Some(hour), Some(minute), Some(second)) = (parts.next(), parts.next(), parts.next())
    else {
        return false;
    };
    parts.next().is_none()
        && two_digit_in_range(hour, 0, 23)
        && two_digit_in_range(minute, 0, 59)
        && two_digit_in_range(second, 0, 60)
}
