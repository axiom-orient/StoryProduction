//! Screenplay-format adapters: deterministic Fountain/FDX normalization and export.

mod export;
mod import;
mod types;

pub use export::{to_fdx, to_fountain};
pub use import::{detect_format, normalize_screenplay};
pub use types::{NormalizedScreenplay, ScreenplaySummary, ScriptFormat};
