//! WritingAdapter registration, selection, and deterministic prompt planning.

use serde_json::Value;

use super::super::{GenerationInput, GenerationPlan, WritingAdapter, MAX_GENERATION_PROMPT_BYTES};
use super::GenerationRegistry;
use crate::error::{Error, Result};
use std::sync::Arc;

impl GenerationRegistry {
    pub fn add_writing_adapter(&mut self, adapter: impl WritingAdapter + 'static) {
        self.writing_adapters.push(Arc::new(adapter));
    }

    pub fn writing_types(&self) -> Vec<Value> {
        let mut types = self
            .writing_adapters
            .iter()
            .map(|adapter| Value::String(adapter.writing_type().as_str().to_string()))
            .collect::<Vec<_>>();
        types.sort_by(|left, right| left.as_str().cmp(&right.as_str()));
        types.dedup();
        types
    }

    pub fn prepare(&self, input: GenerationInput) -> Result<GenerationPlan> {
        if input.request.brief.trim().is_empty() {
            return Err(Error::InvalidInput(
                "generation brief must not be empty".into(),
            ));
        }
        if input.request.brief.len() > crate::model::MAX_TEXT_BYTES {
            return Err(Error::InvalidInput(
                "generation brief exceeds text limit".into(),
            ));
        }
        if !input.request.parameters.is_object() {
            return Err(Error::InvalidInput(
                "generation parameters must be a JSON object".into(),
            ));
        }
        if let Some(expected) = input.request.expected_content_revision {
            if expected != input.context.source_content_revision {
                return Err(Error::Conflict(format!(
                    "generation source content revision mismatch: expected {expected}, actual {}",
                    input.context.source_content_revision
                )));
            }
        }
        let mut matches = self
            .writing_adapters
            .iter()
            .filter(|adapter| adapter.writing_type() == &input.request.writing_type);
        let adapter = matches.next().ok_or_else(|| {
            Error::AdapterUnavailable(format!(
                "no writing adapter is registered for type '{}'",
                input.request.writing_type.as_str()
            ))
        })?;
        if matches.next().is_some() {
            return Err(Error::InvalidInput(format!(
                "multiple writing adapters are registered for type '{}'",
                input.request.writing_type.as_str()
            )));
        }
        let plan = adapter.prepare(&input)?;
        if input.request.project_id != input.context.project_id
            || plan.project_id != input.context.project_id
            || plan.writing_type != input.request.writing_type
            || plan.adapter_id != adapter.id()
            || plan.source_content_revision != input.context.source_content_revision
            || input
                .request
                .stage
                .as_ref()
                .is_some_and(|stage| stage != &plan.stage)
        {
            return Err(Error::InvalidInput(
                "writing adapter returned a plan for a different input identity, stage, or revision".into(),
            ));
        }
        if plan.prompt.trim().is_empty() {
            return Err(Error::InvalidInput(
                "writing adapter returned an empty prompt".into(),
            ));
        }
        if plan.prompt.len() > MAX_GENERATION_PROMPT_BYTES {
            return Err(Error::InvalidInput(
                "generation prompt exceeds text limit".into(),
            ));
        }
        Ok(plan)
    }
}
