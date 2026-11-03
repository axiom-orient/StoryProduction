#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Outcome {
    NotStarted,
    NotApplied,
    Failed,
    Cancelled,
    Unknown,
    SourceChanged,
    SourceStale,
}
impl Outcome {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::NotStarted => "not_started",
            Self::NotApplied => "not_applied",
            Self::Failed => "failed",
            Self::Cancelled => "cancelled",
            Self::Unknown => "outcome_unknown",
            Self::SourceChanged => "source_changed",
            Self::SourceStale => "source_stale",
        }
    }
}
