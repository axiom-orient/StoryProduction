//! Direct-child dispatch, output/deadline observation, and RAII cleanup.

use super::workspace::check_size;
use super::ProcessPolicy;
use geultu::{Error, Result};
use std::fs::File;
use std::process::{Child, Command, ExitStatus};
use std::thread;
use std::time::{Duration, Instant};

const POLL_INTERVAL: Duration = Duration::from_millis(20);

pub(super) fn run_writer(
    command: &mut Command,
    stdout: &File,
    stderr: &File,
    policy: &ProcessPolicy,
    provider_id: &str,
) -> Result<ExitStatus> {
    let child = command.spawn().map_err(|error| {
        Error::AdapterUnavailable(format!(
            "could not start writer provider '{provider_id}': {error}"
        ))
    })?;
    let mut child = OwnedChild(child);
    let started = Instant::now();
    loop {
        check_output_bounds(stdout, stderr, policy)?;
        if let Some(status) = child.0.try_wait()? {
            // Exit may race the prior size observation. Both streams remain bounded.
            check_output_bounds(stdout, stderr, policy)?;
            return Ok(status);
        }
        if started.elapsed() >= policy.timeout {
            return Err(Error::AdapterUnavailable(format!(
                "writer provider '{provider_id}' exceeded process timeout"
            )));
        }
        thread::sleep(POLL_INTERVAL);
    }
}

fn check_output_bounds(stdout: &File, stderr: &File, policy: &ProcessPolicy) -> Result<()> {
    check_size(stdout, policy.max_stdout_bytes, "stdout")?;
    check_size(stderr, policy.max_stderr_bytes, "stderr")
}

struct OwnedChild(Child);
impl Drop for OwnedChild {
    fn drop(&mut self) {
        // Direct-child ownership does not revoke remote or descendant effects.
        if !matches!(self.0.try_wait(), Ok(Some(_))) {
            let _ = self.0.kill();
        }
        let _ = self.0.wait();
    }
}
