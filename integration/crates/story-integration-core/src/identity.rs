#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ProjectId(String);
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AttemptId(String);
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Sha256(String);

fn identifier(value: &str, maximum: usize) -> bool {
    !value.is_empty()
        && value.len() <= maximum
        && value
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b == b'_' || b == b'-')
}
impl ProjectId {
    pub fn new(value: &str) -> Result<Self, String> {
        identifier(value, 64)
            .then(|| Self(value.into()))
            .ok_or_else(|| "project id must contain 1..64 ASCII letters, digits, _ or -".into())
    }
    pub fn as_str(&self) -> &str {
        &self.0
    }
}
impl AttemptId {
    pub fn new(value: &str) -> Result<Self, String> {
        identifier(value, 40)
            .then(|| Self(value.into()))
            .ok_or_else(|| "attempt id must contain 1..40 ASCII letters, digits, _ or -".into())
    }
    pub fn as_str(&self) -> &str {
        &self.0
    }
}
impl Sha256 {
    pub fn new(value: &str) -> Result<Self, String> {
        let valid = value.strip_prefix("sha256:").is_some_and(|v| {
            v.len() == 64
                && v.bytes()
                    .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
        });
        valid
            .then(|| Self(value.into()))
            .ok_or_else(|| "hash must be sha256:<64 lowercase hex>".into())
    }
    pub fn as_str(&self) -> &str {
        &self.0
    }
}
