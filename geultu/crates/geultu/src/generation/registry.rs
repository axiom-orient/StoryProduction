//! Stable GenerationRegistry facade over writing-plan and external-provider owners.

#[cfg(test)]
use super::{GeneratedDraft, GenerationInput, GenerationPlan};
use super::{GenerationProvider, WritingAdapter};
#[cfg(test)]
use crate::error::Result;
use std::fmt;
use std::sync::Arc;

#[derive(Clone, Default)]
pub struct GenerationRegistry {
    /// Deterministic WritingType plan builders; no provider effect is owned here.
    writing_adapters: Vec<Arc<dyn WritingAdapter>>,
    /// External draft generators, selected only after a plan has been validated.
    providers: Vec<Arc<dyn GenerationProvider>>,
}

impl fmt::Debug for GenerationRegistry {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("GenerationRegistry")
            .field("writing_adapter_count", &self.writing_adapters.len())
            .field("provider_count", &self.providers.len())
            .finish()
    }
}
mod provider;
mod writing;

#[cfg(test)]
mod registry_boundary_tests {
    use super::*;
    use crate::generation::{
        context_digest, GenerationContext, GenerationRequest, OutputFormat, WritingStage,
        WritingType, NOVEL,
    };

    #[derive(Debug)]
    struct Adapter {
        writing_type: WritingType,
        wrong_project: bool,
    }
    impl WritingAdapter for Adapter {
        fn id(&self) -> &str {
            "fixture"
        }
        fn writing_type(&self) -> &WritingType {
            &self.writing_type
        }
        fn prepare(&self, input: &GenerationInput) -> Result<GenerationPlan> {
            Ok(GenerationPlan {
                project_id: if self.wrong_project {
                    "another".into()
                } else {
                    input.request.project_id.clone()
                },
                writing_type: self.writing_type.clone(),
                stage: WritingStage::new("draft")?,
                adapter_id: self.id().into(),
                source_content_revision: input.context.source_content_revision,
                prompt: "fixture prompt".into(),
                output_format: OutputFormat::Markdown,
                input_hash: context_digest(input),
                craftpack: None,
            })
        }
    }

    #[derive(Debug)]
    struct Provider;

    impl GenerationProvider for Provider {
        fn id(&self) -> &str {
            "fixture"
        }

        fn generate(&self, _: &GenerationPlan) -> Result<GeneratedDraft> {
            panic!("ambiguous provider must not be called")
        }
    }

    fn input() -> GenerationInput {
        GenerationInput {
            request: GenerationRequest {
                project_id: "book".into(),
                writing_type: WritingType::new(NOVEL).unwrap(),
                stage: None,
                brief: "write".into(),
                parameters: super::super::empty_parameters(),
                expected_content_revision: None,
            },
            context: GenerationContext {
                project_id: "book".into(),
                project_title: "Book".into(),
                source_content_revision: 1,
                scenes: vec![],
                facts: vec![],
                timeline: vec![],
                threads: vec![],
                voice_profile: None,
            },
        }
    }

    fn adapter(wrong_project: bool) -> Adapter {
        Adapter {
            writing_type: WritingType::new(NOVEL).unwrap(),
            wrong_project,
        }
    }

    #[test]
    fn rejects_ambiguous_writing_adapter_registration() {
        let mut registry = GenerationRegistry::default();
        registry.add_writing_adapter(adapter(false));
        registry.add_writing_adapter(adapter(false));
        assert!(registry
            .prepare(input())
            .unwrap_err()
            .to_string()
            .contains("multiple writing adapters"));
    }

    #[test]
    fn rejects_adapter_plan_for_different_project() {
        let mut registry = GenerationRegistry::default();
        registry.add_writing_adapter(adapter(true));
        assert!(registry
            .prepare(input())
            .unwrap_err()
            .to_string()
            .contains("different input"));
    }

    #[test]
    fn rejects_ambiguous_provider_before_effect() {
        let mut registry = GenerationRegistry::default();
        registry.add_writing_adapter(adapter(false));
        let plan = registry.prepare(input()).unwrap();
        registry.add_provider(Provider);
        registry.add_provider(Provider);
        assert!(registry
            .generate(&plan, "fixture")
            .unwrap_err()
            .to_string()
            .contains("multiple generation providers"));
    }
}
