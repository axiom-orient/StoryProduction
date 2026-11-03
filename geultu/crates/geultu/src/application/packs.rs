//! Application-local craft/refinement pack location and loading policy.

use super::Application;
use crate::craftpack::{CraftPack, CraftpackPaths};
use crate::error::{Error, Result};

impl Application {
    fn resolve_craftpack_paths(&self) -> Option<CraftpackPaths> {
        self.craftpack_paths
            .clone()
            .or_else(CraftpackPaths::discover)
    }

    /// Mandatory pack operations fail when neither explicit nor discovered paths exist.
    pub(in crate::application) fn require_craftpack_paths(&self) -> Result<CraftpackPaths> {
        self.resolve_craftpack_paths().ok_or_else(|| {
            Error::NotFound(format!(
                "no craft pack could be located; set {} or {} or pass explicit \
                 craftpack paths to the application",
                crate::craftpack::ASSETS_ROOT_ENV,
                crate::craftpack::CRAFT_PACK_ROOT_ENV
            ))
        })
    }

    /// Absence is optional for generation; any located pack must load successfully.
    pub(in crate::application) fn try_craft_pack(&self) -> Result<Option<CraftPack>> {
        let Some(paths) = self.resolve_craftpack_paths() else {
            return Ok(None);
        };
        crate::craftpack::load_craft_pack(&paths.craft_pack_dir).map(Some)
    }

    pub(in crate::application) fn require_refinement_pack(
        &self,
    ) -> Result<crate::refinement::RefinementPack> {
        let paths = self.require_craftpack_paths()?;
        crate::refinement::load_refinement_pack(&paths.refinement_pack_dir)
    }
}
