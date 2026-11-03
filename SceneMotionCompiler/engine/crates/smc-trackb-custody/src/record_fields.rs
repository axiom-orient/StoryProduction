//! Exact JSON-object field-set comparison shared by durable record owners.

use std::collections::BTreeSet;

use serde_json::{Map, Value};

pub(super) fn has_exact_value_keys(object: &Map<String, Value>, expected: &[&str]) -> bool {
    let expected: BTreeSet<&str> = expected.iter().copied().collect();
    let actual: BTreeSet<&str> = object.keys().map(String::as_str).collect();
    actual == expected
}
