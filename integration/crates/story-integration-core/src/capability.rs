#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum JobStatus {
    Queued,
    Running,
    Succeeded,
    Failed,
    Cancelled,
    Unknown,
}
impl JobStatus {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Queued => "queued",
            Self::Running => "running",
            Self::Succeeded => "succeeded",
            Self::Failed => "failed",
            Self::Cancelled => "cancelled",
            Self::Unknown => "outcome_unknown",
        }
    }
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CapabilityState {
    status: JobStatus,
    attempt: u32,
    cancel_requested: bool,
}
impl CapabilityState {
    pub fn queued() -> Self {
        Self {
            status: JobStatus::Queued,
            attempt: 0,
            cancel_requested: false,
        }
    }

    pub fn from_active_snapshot(
        status: JobStatus,
        attempt: u32,
        cancel_requested: bool,
    ) -> Result<Self, String> {
        match status {
            JobStatus::Queued if attempt == 0 => Ok(Self {
                status,
                attempt,
                cancel_requested,
            }),
            JobStatus::Queued => Err("queued capability must have attempt 0".into()),
            JobStatus::Running if attempt > 0 => Ok(Self {
                status,
                attempt,
                cancel_requested,
            }),
            JobStatus::Running => Err("running capability must have at least one attempt".into()),
            _ => Err("only active capability states can be imported".into()),
        }
    }

    pub fn status(&self) -> JobStatus {
        self.status
    }

    pub fn attempt(&self) -> u32 {
        self.attempt
    }

    pub fn cancel_requested(&self) -> bool {
        self.cancel_requested
    }

    pub fn request_cancellation(&self) -> Result<Self, String> {
        if !matches!(self.status, JobStatus::Queued | JobStatus::Running) {
            return Err("cannot cancel a terminal job".into());
        }
        Ok(Self {
            cancel_requested: true,
            ..self.clone()
        })
    }
    pub fn transition(
        &self,
        status: JobStatus,
        has_result: bool,
        has_failure: bool,
    ) -> Result<Self, String> {
        let allowed = match self.status {
            JobStatus::Queued => matches!(
                status,
                JobStatus::Running | JobStatus::Failed | JobStatus::Cancelled | JobStatus::Unknown
            ),
            JobStatus::Running => matches!(
                status,
                JobStatus::Succeeded
                    | JobStatus::Failed
                    | JobStatus::Cancelled
                    | JobStatus::Unknown
            ),
            _ => false,
        };
        if !allowed {
            return Err("invalid capability transition".into());
        }
        if status == JobStatus::Running && self.cancel_requested {
            return Err("cannot start a job after cancellation request".into());
        }
        if has_result != (status == JobStatus::Succeeded) {
            return Err("only a succeeded job requires a result".into());
        }
        if has_failure
            && matches!(
                status,
                JobStatus::Queued | JobStatus::Running | JobStatus::Succeeded
            )
        {
            return Err("active/succeeded job cannot carry failure".into());
        }
        if status == JobStatus::Failed && !has_failure {
            return Err("failed job requires failure".into());
        }
        if status == JobStatus::Succeeded && self.cancel_requested {
            return Err("cannot accept success after cancellation request".into());
        }
        Ok(Self {
            status,
            attempt: if status == JobStatus::Running {
                // The only valid transition into Running is from a queued
                // state, whose constructor guarantees attempt zero.
                1
            } else {
                self.attempt
            },
            cancel_requested: self.cancel_requested || status == JobStatus::Cancelled,
        })
    }
}
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn active_snapshots_enforce_status_attempt_invariants() {
        assert!(CapabilityState::from_active_snapshot(JobStatus::Queued, 1, false).is_err());
        assert!(CapabilityState::from_active_snapshot(JobStatus::Running, 0, false).is_err());
        for status in [
            JobStatus::Succeeded,
            JobStatus::Failed,
            JobStatus::Cancelled,
            JobStatus::Unknown,
        ] {
            assert!(CapabilityState::from_active_snapshot(status, 0, false).is_err());
        }

        let queued = CapabilityState::from_active_snapshot(JobStatus::Queued, 0, true).unwrap();
        assert_eq!(queued.status(), JobStatus::Queued);
        assert_eq!(queued.attempt(), 0);
        assert!(queued.cancel_requested());

        let running = CapabilityState::from_active_snapshot(JobStatus::Running, 1, false).unwrap();
        assert_eq!(running.status(), JobStatus::Running);
        assert_eq!(running.attempt(), 1);
        assert!(!running.cancel_requested());
    }

    #[test]
    fn cancel_and_terminal_are_not_success() {
        let running = CapabilityState::queued()
            .transition(JobStatus::Running, false, false)
            .unwrap();
        assert_eq!(running.attempt(), 1);
        let cancelled = running.request_cancellation().unwrap();
        assert!(cancelled.cancel_requested());
        assert!(cancelled
            .transition(JobStatus::Succeeded, true, false)
            .is_err());
        let terminal = cancelled
            .transition(JobStatus::Unknown, false, true)
            .unwrap();
        assert_eq!(terminal.status(), JobStatus::Unknown);
        assert_eq!(terminal.attempt(), 1);
        assert!(terminal.cancel_requested());
        assert!(terminal
            .transition(JobStatus::Succeeded, true, false)
            .is_err());
    }

    #[test]
    fn pre_dispatch_terminal_states_keep_attempt_zero() {
        let queued = CapabilityState::queued();
        let failed = queued.transition(JobStatus::Failed, false, true).unwrap();
        assert_eq!(failed.status(), JobStatus::Failed);
        assert_eq!(failed.attempt(), 0);
        assert!(!failed.cancel_requested());

        let cancellation_requested = queued.request_cancellation().unwrap();
        assert!(cancellation_requested
            .transition(JobStatus::Running, false, false)
            .is_err());
        let cancelled = cancellation_requested
            .transition(JobStatus::Cancelled, false, false)
            .unwrap();
        assert_eq!(cancelled.status(), JobStatus::Cancelled);
        assert_eq!(cancelled.attempt(), 0);
        assert!(cancelled.cancel_requested());

        let failed_after_cancel = cancellation_requested
            .transition(JobStatus::Failed, false, true)
            .unwrap();
        assert_eq!(failed_after_cancel.status(), JobStatus::Failed);
        assert_eq!(failed_after_cancel.attempt(), 0);
        assert!(failed_after_cancel.cancel_requested());
    }

    #[test]
    fn success_requires_a_running_attempt_and_a_result() {
        let running = CapabilityState::queued()
            .transition(JobStatus::Running, false, false)
            .unwrap();
        let succeeded = running
            .transition(JobStatus::Succeeded, true, false)
            .unwrap();
        assert_eq!(succeeded.status(), JobStatus::Succeeded);
        assert_eq!(succeeded.attempt(), 1);
        assert!(!succeeded.cancel_requested());
    }
}
