//! Deterministic UTC timestamp rendering for durable receipt records.

/// Renders unix milliseconds as an RFC 3339 UTC timestamp
/// (`YYYY-MM-DDTHH:MM:SS.mmmZ`) using the civil-from-days algorithm.
#[must_use]
pub(super) fn rfc3339_utc(unix_ms: u64) -> String {
    let seconds = unix_ms / 1000;
    let millis = unix_ms % 1000;
    let rem = seconds % 86_400;
    let (year, month, day) = civil_from_days(seconds / 86_400);
    format!(
        "{year:04}-{month:02}-{day:02}T{:02}:{:02}:{:02}.{millis:03}Z",
        rem / 3_600,
        rem % 3_600 / 60,
        rem % 60
    )
}

/// Howard Hinnant's `civil_from_days`: days since 1970-01-01 to (y, m, d).
fn civil_from_days(days: u64) -> (i64, u32, u32) {
    let z = days.min(i64::MAX as u64 - 719_468) as i64 + 719_468;
    let era = if z >= 0 { z } else { z - 146_096 } / 146_097;
    let doe = (z - era * 146_097) as u64;
    let yoe = (doe - doe / 1_460 + doe / 36_524 - doe / 146_096) / 365;
    let year = yoe as i64 + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let day = (doy - (153 * mp + 2) / 5 + 1) as u32;
    let month = if mp < 10 { mp + 3 } else { mp - 9 } as u32;
    (if month <= 2 { year + 1 } else { year }, month, day)
}
