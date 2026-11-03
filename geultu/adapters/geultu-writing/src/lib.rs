//! Deterministic built-in planning; core owns state and providers own external effects.

mod adapter;
mod builtins;
mod prompt;

pub use adapter::PromptAdapter;
pub use builtins::builtin_adapters;

#[cfg(test)]
mod tests {
    use super::*;
    use geultu::generation::{
        GenerationContext, GenerationInput, GenerationRequest, OutputFormat, WritingAdapter,
        WritingType, FICTION, NOVEL, STORYBOARD, SYNOPSIS,
    };
    use geultu::model::Project;
    use serde_json::json;

    #[test]
    fn builtins_cover_requested_types() {
        let types: Vec<_> = builtin_adapters()
            .into_iter()
            .map(|adapter| adapter.writing_type().as_str().to_string())
            .collect();
        assert_eq!(types, vec![NOVEL, FICTION, SYNOPSIS, STORYBOARD]);
    }

    #[test]
    fn novel_plan_contains_voice_and_canonical_context() {
        let adapter = builtin_adapters().remove(0);
        let project = Project::empty("book".into(), "Book".into());
        let input = GenerationInput {
            request: GenerationRequest {
                project_id: "book".into(),
                writing_type: WritingType::new(NOVEL).unwrap(),
                stage: None,
                brief: "Write the opening chapter".into(),
                parameters: json!({"target_words": 1200}),
                expected_content_revision: Some(0),
            },
            context: GenerationContext::from_project(&project),
        };
        let plan = adapter.prepare(&input).unwrap();
        assert!(plan.prompt.contains("Write the opening chapter"));
        assert_eq!(plan.stage.as_str(), "draft");
        assert_eq!(plan.output_format, OutputFormat::Markdown);
    }
}
