//! Runtime-owned Unix process-group lifecycle, stdin and private capture entrypoints.

#[cfg(not(unix))]
use crate::{Error, Result};
#[cfg(not(unix))]
use std::{
    path::Path,
    process::{Command, ExitStatus, Output},
    time::Duration,
};

#[cfg(unix)]
mod capture;
#[cfg(unix)]
mod lifecycle;
#[cfg(unix)]
pub use capture::run_piped_command_capture;
#[cfg(unix)]
pub use lifecycle::run_piped_command;

#[cfg(not(unix))]
pub fn run_piped_command(
    command: &mut Command,
    stdin_bytes: &[u8],
    timeout: Duration,
    cancel_path: &Path,
    run_id: &str,
) -> Result<ExitStatus> {
    let _ = (command, stdin_bytes, timeout, cancel_path, run_id);
    Err(Error::Executor("process-tree lifecycle is not implemented on this platform; Agent execution requires Unix semantics".into()))
}
#[cfg(not(unix))]
pub fn run_piped_command_capture(
    command: &mut Command,
    stdin_bytes: &[u8],
    timeout: Duration,
    cancel_path: &Path,
    run_id: &str,
) -> Result<Output> {
    let _ = (command, stdin_bytes, timeout, cancel_path, run_id);
    Err(Error::Executor("process-tree lifecycle is not implemented on this platform; capture requires Unix semantics".into()))
}

#[cfg(all(test, unix))]
mod lifecycle_regression {
    use super::lifecycle::cancellation_requested;
    use super::run_piped_command;
    use crate::Error;
    use std::{
        process::Command,
        time::{Duration, Instant},
    };

    #[test]
    fn missing_cancel_marker_means_no_request() {
        let path = std::env::temp_dir().join(format!("nvc-cancel-missing-{}", std::process::id()));
        let _ = std::fs::remove_file(&path);
        assert!(!cancellation_requested(&path, "run-1").unwrap());
    }

    #[test]
    fn non_regular_cancel_marker_is_rejected() {
        let path =
            std::env::temp_dir().join(format!("nvc-cancel-directory-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&path);
        std::fs::create_dir(&path).unwrap();
        let result = cancellation_requested(&path, "run-1");
        std::fs::remove_dir_all(&path).unwrap();
        assert!(matches!(result, Err(Error::Contract(_))));
    }

    #[test]
    fn exited_leader_cannot_leave_stdin_delivery_unbounded() {
        let mut command = Command::new("sh");
        command.args(["-c", "(trap '' TERM; exec sleep 30) <&0 & exit 0"]);
        let started = Instant::now();
        let result = run_piped_command(
            &mut command,
            &vec![b'x'; 8 * 1024 * 1024],
            Duration::from_millis(200),
            &std::env::temp_dir().join(format!("nvc-no-cancel-{}", std::process::id())),
            "fixture",
        );
        assert!(result.is_err());
        assert!(started.elapsed() < Duration::from_secs(5));
    }
}
