//! Blocking adapter effects run off the IPC task; join failure preserves unknown outcome.

pub(crate) async fn blocking<T, F>(operation: &'static str, work: F) -> Result<T, String>
where
    T: Send + 'static,
    F: FnOnce() -> Result<T, String> + Send + 'static,
{
    tauri::async_runtime::spawn_blocking(work)
        .await
        .map_err(|error| {
            format!("{operation} worker failed; effects may already have been applied: {error}")
        })?
}
