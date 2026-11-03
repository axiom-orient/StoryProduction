//! GenerationProvider registration, selection, and untrusted draft admission.

use super::super::{GeneratedDraft, GenerationPlan, GenerationProvider};
use super::GenerationRegistry;
use crate::error::{Error, Result};
use std::sync::Arc;

impl GenerationRegistry {
    pub fn add_provider(&mut self, provider: impl GenerationProvider + 'static) {
        self.providers.push(Arc::new(provider));
    }

    pub fn provider_ids(&self) -> Vec<String> {
        let mut ids = self
            .providers
            .iter()
            .map(|provider| provider.id().to_string())
            .collect::<Vec<_>>();
        ids.sort();
        ids.dedup();
        ids
    }

    pub fn generate(&self, plan: &GenerationPlan, provider_id: &str) -> Result<GeneratedDraft> {
        let mut matches = self
            .providers
            .iter()
            .filter(|provider| provider.id() == provider_id);
        let provider = matches.next().ok_or_else(|| {
            Error::AdapterUnavailable(format!(
                "generation provider '{}' is not registered",
                provider_id
            ))
        })?;
        if matches.next().is_some() {
            return Err(Error::InvalidInput(format!(
                "multiple generation providers are registered for '{provider_id}'"
            )));
        }
        let draft = provider.generate(plan)?;
        if draft.content.trim().is_empty() {
            return Err(Error::InvalidInput(
                "generation provider returned empty content".into(),
            ));
        }
        if draft.content.len() > crate::model::MAX_TEXT_BYTES {
            return Err(Error::InvalidInput(
                "generated content exceeds text limit".into(),
            ));
        }
        if draft.model.trim().is_empty() {
            return Err(Error::InvalidInput(
                "generation provider returned an empty model".into(),
            ));
        }
        Ok(draft)
    }
}
