//! H4 craft-pack loader adapter: fail-closed consumption of
//! `assets/craft-packs-scriptforge-v4` for generation planning.
//!
//! Loading order is verify-then-parse: [`load_craft_pack`] first verifies
//! every entry of the pack's own `SHA256SUMS` ([`crate::integrity`]) and
//! only then parses `craft-pack.json` and the workflow manifest. Any digest
//! mismatch, missing file, or unsafe sums entry aborts the load with an
//! error naming the offending file — a corrupt pack is never partially
//! consumed.
//!
//! Location resolution ([`CraftpackPaths::discover`]) tries, in order: an
//! explicit configuration on [`crate::Application`], the `GEULTU_ASSETS_ROOT`
//! assets directory plus optional direct `GEULTU_CRAFT_PACK_ROOT`, then a
//! repository assets directory found from the current directory or compile-time
//! crate layout. When no pack can be located, `generation.prepare` produces
//! plans without a craft-pack reference (the engine remains usable outside a
//! repository checkout). Explicit Application/environment roots are never
//! replaced by discovered defaults: missing or corrupt configured packs are
//! hard errors. `craftpack.status` is the explicit authority for asserting
//! pack identity and integrity.

mod loader;
mod nodes;
mod paths;
mod reference;
mod status;

pub use loader::load_craft_pack;
pub use nodes::{
    AgentNode, CraftPackManifest, WorkflowAgentNode, WorkflowHumanNode, WorkflowManifest,
    WorkflowNode,
};

use std::collections::BTreeMap;
use std::path::PathBuf;

/// Directory name of the scriptforge craft pack below the assets root.
pub const CRAFT_PACK_DIR_NAME: &str = "craft-packs-scriptforge-v4";
/// Directory name of the language refinement pack below the assets root.
pub const REFINEMENT_PACK_DIR_NAME: &str = "refinement-packs-language-v1";
/// `api_version` this loader understands; other majors fail closed.
pub const CRAFT_PACK_API_VERSION: &str = "geultu.dev/craft-pack/v1";
/// Environment variable overriding the assets root directory.
pub const ASSETS_ROOT_ENV: &str = "GEULTU_ASSETS_ROOT";
/// Environment variable selecting one direct craft-pack directory.
pub const CRAFT_PACK_ROOT_ENV: &str = "GEULTU_CRAFT_PACK_ROOT";

/// Explicit asset locations handed to the application; construction call
/// sites that do not configure it fall back to [`CraftpackPaths::discover`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CraftpackPaths {
    pub craft_pack_dir: PathBuf,
    pub refinement_pack_dir: PathBuf,
}

/// A fully verified craft pack: parsed manifests plus the verified
/// path→digest map of every SHA256SUMS entry.
#[derive(Debug, Clone)]
pub struct CraftPack {
    pub(super) directory: PathBuf,
    pub(super) manifest: CraftPackManifest,
    pub(super) workflow: WorkflowManifest,
    pub(super) files: BTreeMap<String, String>,
}

impl CraftPack {
    pub fn manifest(&self) -> &CraftPackManifest {
        &self.manifest
    }
    pub fn workflow(&self) -> &WorkflowManifest {
        &self.workflow
    }
    pub fn files(&self) -> &BTreeMap<String, String> {
        &self.files
    }
}
