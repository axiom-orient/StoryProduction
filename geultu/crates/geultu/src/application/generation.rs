//! Generation planning and optional craft-pack enrichment policy.

use super::Application;
use crate::error::Result;
use crate::generation::{GenerationContext, GenerationInput, GenerationRequest};

impl Application {
    pub(in crate::application) fn prepare_generation(
        &self,
        request: GenerationRequest,
    ) -> Result<crate::generation::GenerationPlan> {
        let project = self.workspace.load_project(&request.project_id)?;
        let mut plan = self.generation.prepare(GenerationInput {
            context: GenerationContext::from_project(&project),
            request,
        })?;
        if let Some(pack) = self.try_craft_pack()? {
            plan.craftpack = Some(pack.plan_reference());
        }
        Ok(plan)
    }
}
