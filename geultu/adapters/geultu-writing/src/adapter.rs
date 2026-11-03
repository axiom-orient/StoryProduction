//! One deterministic adapter's plan identity and requested/default stage selection.

use geultu::generation::{
    context_digest, GenerationInput, GenerationPlan, OutputFormat, WritingAdapter, WritingStage,
    WritingType,
};
use geultu::{Error, Result};

#[derive(Debug, Clone)]
pub struct PromptAdapter {
    writing_type: WritingType,
    default_stage: WritingStage,
    format: OutputFormat,
    instructions: &'static str,
}

impl PromptAdapter {
    pub(super) fn new(
        writing_type: &'static str,
        default_stage: &'static str,
        format: OutputFormat,
        instructions: &'static str,
    ) -> Self {
        Self {
            writing_type: WritingType::new(writing_type).expect("built-in writing type is valid"),
            default_stage: WritingStage::new(default_stage)
                .expect("built-in writing stage is valid"),
            format,
            instructions,
        }
    }
}

impl WritingAdapter for PromptAdapter {
    fn id(&self) -> &str {
        self.writing_type.as_str()
    }

    fn writing_type(&self) -> &WritingType {
        &self.writing_type
    }

    fn prepare(&self, input: &GenerationInput) -> Result<GenerationPlan> {
        if input.request.writing_type != self.writing_type {
            return Err(Error::AdapterUnavailable(format!(
                "writing adapter '{}' does not support type '{}'",
                self.writing_type.as_str(),
                input.request.writing_type.as_str()
            )));
        }
        let stage = input.request.stage.as_ref().unwrap_or(&self.default_stage);
        let prompt = crate::prompt::render(input, stage, self.instructions)?;
        let input_hash = context_digest(input);
        Ok(GenerationPlan {
            project_id: input.request.project_id.clone(),
            writing_type: self.writing_type.clone(),
            stage: stage.clone(),
            adapter_id: self.writing_type.as_str().to_owned(),
            source_content_revision: input.context.source_content_revision,
            prompt,
            output_format: self.format.clone(),
            input_hash,
            craftpack: None,
        })
    }
}
