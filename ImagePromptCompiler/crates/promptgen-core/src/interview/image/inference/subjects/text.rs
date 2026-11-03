//! Shared ASCII-case-insensitive phrase replacement for subject normalization.

pub(super) fn replace_ascii_case_insensitive(
    value: &str,
    needle: &str,
    replacement: &str,
) -> String {
    if needle.is_empty() {
        return value.to_owned();
    }
    let mut output = value.to_owned();
    loop {
        let lower = output.to_lowercase();
        let Some(index) = lower.find(&needle.to_lowercase()) else {
            break;
        };
        output.replace_range(index..index + needle.len(), replacement);
    }
    output
}
