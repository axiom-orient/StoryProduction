//! Closed built-in writing types and their ordered craft instructions.

use crate::PromptAdapter;
use geultu::generation::{OutputFormat, FICTION, NOVEL, STORYBOARD, SYNOPSIS};

pub fn builtin_adapters() -> Vec<PromptAdapter> {
    vec![
        PromptAdapter::new(
            NOVEL,
            "draft",
            OutputFormat::Markdown,
            "Treat this as long-form book work. Keep chapter or section continuity, character arcs, world rules, and the requested target scope explicit.",
        ),
        PromptAdapter::new(
            FICTION,
            "draft",
            OutputFormat::Markdown,
            "Treat this as a complete short-fiction draft. Prioritize a clear premise, scene movement, point of view, conflict, and a deliberate ending.",
        ),
        PromptAdapter::new(
            SYNOPSIS,
            "outline",
            OutputFormat::Markdown,
            "Produce a concise synopsis that exposes premise, protagonist, central conflict, escalation, turning point, and ending without ornamental prose.",
        ),
        PromptAdapter::new(
            STORYBOARD,
            "draft",
            OutputFormat::Markdown,
            "Produce a shot-oriented storyboard. For each shot include index, framing, subject/action, camera or movement, dialogue/audio, and continuity notes.",
        ),
    ]
}
