use std::collections::BTreeMap;
use std::sync::OnceLock;

use crate::json::{
    DecodeError, JsonValue, into_array, into_object, parse, reject_unknown, take_required,
    take_string,
};

use super::validate::validate_catalog;
use super::{
    CATALOG_JSON, CATALOG_SCHEMA_VERSION, Catalog, CatalogEntry, CatalogError, CatalogFamily,
    CatalogMetadata, MethodSource,
};

#[derive(Debug)]
pub(super) struct CatalogStore {
    pub(super) catalog: Catalog,
    pub(super) entry_index: BTreeMap<String, usize>,
}

static CATALOG_STORE: OnceLock<Result<CatalogStore, CatalogError>> = OnceLock::new();

pub(super) fn catalog_store() -> Result<&'static CatalogStore, CatalogError> {
    match CATALOG_STORE.get_or_init(parse_catalog_store) {
        Ok(store) => Ok(store),
        Err(error) => Err(error.clone()),
    }
}

pub fn catalog() -> Result<&'static Catalog, CatalogError> {
    Ok(&catalog_store()?.catalog)
}

pub fn find_entry(id: &str) -> Result<Option<&'static CatalogEntry>, CatalogError> {
    let normalized = id.trim().to_ascii_uppercase();
    let store = catalog_store()?;
    let Some(position) = store.entry_index.get(&normalized).copied() else {
        return Ok(None);
    };
    Ok(store.catalog.entries.get(position))
}

pub fn catalog_json(family: Option<CatalogFamily>) -> Result<JsonValue, CatalogError> {
    let catalog = catalog()?;
    let entries = catalog
        .entries
        .iter()
        .filter(|entry| family.is_none_or(|family| entry.family == family))
        .map(CatalogEntry::to_json);
    Ok(JsonValue::object([
        ("entries", JsonValue::array(entries)),
        ("metadata", catalog.metadata.to_json()),
        ("navigation", catalog.navigation.clone()),
    ]))
}

fn parse_catalog_store() -> Result<CatalogStore, CatalogError> {
    let root = parse(CATALOG_JSON).map_err(|error| CatalogError(error.to_string()))?;
    let mut fields = into_object(root, "$catalog").map_err(decode_error)?;
    let schema_version =
        match take_required(&mut fields, "schema_version", "$catalog").map_err(decode_error)? {
            JsonValue::Number(value) => value.parse::<u64>().map_err(|_| {
                CatalogError("$catalog.schema_version must be an integer".to_owned())
            })?,
            _ => {
                return Err(CatalogError(
                    "$catalog.schema_version must be a number".to_owned(),
                ));
            }
        };
    if schema_version != CATALOG_SCHEMA_VERSION {
        return Err(CatalogError(format!(
            "$catalog.schema_version must be {CATALOG_SCHEMA_VERSION}, found {schema_version}"
        )));
    }
    let mut source = into_object(
        take_required(&mut fields, "source", "$catalog").map_err(decode_error)?,
        "$catalog.source",
    )
    .map_err(decode_error)?;
    let source_repository =
        take_string(&mut source, "repository", "$catalog.source").map_err(decode_error)?;
    let source_commit =
        take_string(&mut source, "archive_commit", "$catalog.source").map_err(decode_error)?;
    let source_license =
        take_string(&mut source, "license", "$catalog.source").map_err(decode_error)?;
    let adaptation =
        take_string(&mut source, "adaptation", "$catalog.source").map_err(decode_error)?;
    reject_unknown(source, "$catalog.source").map_err(decode_error)?;

    let family_counts_value =
        take_required(&mut fields, "family_counts", "$catalog").map_err(decode_error)?;
    let family_count_fields =
        into_object(family_counts_value, "$catalog.family_counts").map_err(decode_error)?;
    let mut family_counts = BTreeMap::new();
    for (key, value) in family_count_fields {
        let count = match value {
            JsonValue::Number(value) => value
                .parse::<u64>()
                .map_err(|_| CatalogError(format!("family count {key:?} must be an integer")))?,
            _ => {
                return Err(CatalogError(format!(
                    "family count {key:?} must be numeric"
                )));
            }
        };
        family_counts.insert(key, count);
    }

    let method_source_values = into_array(
        take_required(&mut fields, "method_sources", "$catalog").map_err(decode_error)?,
        "$catalog.method_sources",
    )
    .map_err(decode_error)?;
    let method_sources = method_source_values
        .into_iter()
        .enumerate()
        .map(|(index, value)| {
            MethodSource::from_json(value, &format!("$catalog.method_sources[{index}]"))
                .map_err(decode_error)
        })
        .collect::<Result<Vec<_>, _>>()?;

    let navigation = into_object(
        take_required(&mut fields, "navigation", "$catalog").map_err(decode_error)?,
        "$catalog.navigation",
    )
    .map(JsonValue::Object)
    .map_err(decode_error)?;

    let entries_value = take_required(&mut fields, "entries", "$catalog").map_err(decode_error)?;
    let values = into_array(entries_value, "$catalog.entries").map_err(decode_error)?;
    let entries = values
        .into_iter()
        .enumerate()
        .map(|(index, value)| {
            CatalogEntry::from_json(value, &format!("$catalog.entries[{index}]"))
                .map_err(decode_error)
        })
        .collect::<Result<Vec<_>, _>>()?;
    reject_unknown(fields, "$catalog").map_err(decode_error)?;

    validate_catalog(&entries, &family_counts)?;
    let mut entry_index = BTreeMap::new();
    for (position, entry) in entries.iter().enumerate() {
        let normalized = entry.id.to_ascii_uppercase();
        if entry_index.insert(normalized.clone(), position).is_some() {
            return Err(CatalogError(format!(
                "duplicate normalized catalog id {normalized}"
            )));
        }
    }
    Ok(CatalogStore {
        catalog: Catalog {
            metadata: CatalogMetadata {
                schema_version,
                source_repository,
                source_commit,
                source_license,
                adaptation,
                family_counts,
                method_sources,
            },
            navigation,
            entries,
        },
        entry_index,
    })
}

fn decode_error(error: DecodeError) -> CatalogError {
    CatalogError(error.to_string())
}
