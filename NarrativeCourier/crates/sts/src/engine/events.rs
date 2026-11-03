//! STS timestamps and durable event append representation.

use crate::model::workflow::{Event, State};
use chrono::Local;
use serde_json::Value;
use std::collections::BTreeMap;

pub(super) fn now_iso() -> String {
    Local::now().to_rfc3339_opts(chrono::SecondsFormat::Secs, false)
}

pub(super) fn push_event<const N: usize>(
    state: &mut State,
    event_type: &str,
    pairs: [(&str, Value); N],
) {
    let mut details = BTreeMap::new();
    for (k, v) in pairs {
        details.insert(k.to_string(), v);
    }
    state.events.push(Event {
        at: now_iso(),
        event_type: event_type.into(),
        details,
    });
}
