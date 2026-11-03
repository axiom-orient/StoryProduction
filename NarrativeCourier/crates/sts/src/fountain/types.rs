//! Format-independent screenplay normalization result and summary DTOs.

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct ScreenplaySummary {
    pub scene_count: usize,
    pub interior_count: usize,
    pub exterior_count: usize,
    pub cast: Vec<String>,
    pub skipped_lines: usize,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NormalizedScreenplay {
    pub text: String,
    pub summary: ScreenplaySummary,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ScriptFormat {
    Fountain,
    Fdx,
}
