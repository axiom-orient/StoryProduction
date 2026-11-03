use story_integration_core::Outcome;
pub type Result<T> = std::result::Result<T, Error>;
#[derive(Debug)]
pub struct Error {
    pub outcome: Outcome,
    pub message: String,
    pub stdout: Vec<u8>,
    pub stderr: Vec<u8>,
    pub spawned: bool,
}
impl Error {
    fn new(outcome: Outcome, message: impl Into<String>) -> Self {
        Self {
            outcome,
            message: message.into(),
            stdout: Vec::new(),
            stderr: Vec::new(),
            spawned: false,
        }
    }
    pub fn not_applied(message: impl Into<String>) -> Self {
        Self::new(Outcome::NotApplied, message)
    }
    pub fn not_started(message: impl Into<String>) -> Self {
        Self::new(Outcome::NotStarted, message)
    }
    pub fn unknown(message: impl Into<String>) -> Self {
        Self {
            spawned: true,
            ..Self::new(Outcome::Unknown, message)
        }
    }
    pub fn with_stdout(mut self, value: Vec<u8>) -> Self {
        self.stdout = value;
        self
    }
    pub fn with_stderr(mut self, value: Vec<u8>) -> Self {
        self.stderr = value;
        self
    }
}
impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.message)
    }
}
impl std::error::Error for Error {}
impl From<std::io::Error> for Error {
    fn from(value: std::io::Error) -> Self {
        Self::not_applied(value.to_string())
    }
}
impl From<serde_json::Error> for Error {
    fn from(value: serde_json::Error) -> Self {
        Self::not_applied(value.to_string())
    }
}
