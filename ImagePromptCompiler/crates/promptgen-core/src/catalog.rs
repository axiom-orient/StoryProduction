#[cfg(test)]
use std::collections::BTreeSet;

mod model;
pub(crate) mod selection;
mod store;
mod validate;

pub use self::model::{
    Catalog, CatalogEntry, CatalogError, CatalogFamily, CatalogMetadata, DirectiveSlot,
    MethodSource, PromptDirective, ReferenceTextMode, TextRequirement,
};
pub use self::store::{catalog, catalog_json, find_entry};
const CATALOG_JSON: &str = include_str!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../../catalog/image_catalog.json"
));
const CATALOG_SCHEMA_VERSION: u64 = 4;
const EXPECTED_ENTRY_COUNT: usize = 6;
pub const PRIMARY_OUTPUT_TIER_1: &str = "결과물";
pub const MAX_SUPPORTING_DIRECTIONS: usize = 0;

#[cfg(test)]
#[path = "catalog/tests.rs"]
mod tests;
