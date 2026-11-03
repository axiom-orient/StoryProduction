use std::sync::atomic::{AtomicU64, Ordering};

static NEXT_TEMP_PATH_ID: AtomicU64 = AtomicU64::new(1);

pub fn unique_temp_id() -> u64 {
    NEXT_TEMP_PATH_ID.fetch_add(1, Ordering::Relaxed)
}
