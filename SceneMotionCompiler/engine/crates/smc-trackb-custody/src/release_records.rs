//! Parsing and identity binding for immutable pointer and DeliverySet records.

use serde::Deserialize;
use serde_json::{Map, Value};

use crate::error::{CustodyError, Result};
use crate::filesystem::admission::{
    is_valid_opaque_id, validate_narrative_id, validate_safe_relative_path, ArtifactDirectory,
    SCENES_DIRECTORY, TRACKB_DIRECTORY,
};
use crate::protocol::{canonical_json, is_sha256_hash, sha256_prefixed};
use crate::record_fields::has_exact_value_keys;

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct DeliverySetArtifacts {
    projection_receipt_hash: String,
    scene_graph_hash: String,
    asset_handoff_hash: String,
    asset_handoff_scene_graph_hash: String,
    html_hash: String,
    provenance_hash: String,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(super) struct DeliverySetRecord {
    schema_version: u64,
    delivery_set_id: String,
    narrative_id: String,
    candidate_hash: String,
    authority_hash: String,
    meaning_qualification_hash: String,
    base_delivery_set_hash: String,
    approval_hash: String,
    artifacts: DeliverySetArtifacts,
    evidence_cell_hashes: Vec<String>,
    revision: u64,
    state: String,
    delivery_set_hash: String,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(super) struct PointerRecord {
    schema_version: u64,
    pointer_key: String,
    narrative_id: String,
    revision: u64,
    state: String,
    delivery_set_hash: String,
    candidate_hash: String,
    approval_hash: String,
    meaning_qualification_hash: String,
    pub(super) base_delivery_set_hash: String,
    delivery_set_path: String,
    pointer_hash: String,
}

impl PointerRecord {
    pub(super) fn delivery_set_hash(&self) -> &str {
        &self.delivery_set_hash
    }
}

pub(super) fn parse_pointer_record(bytes: &[u8], narrative_id: &str) -> Result<PointerRecord> {
    let value: Value = serde_json::from_slice(bytes)
        .map_err(|error| CustodyError::storage(format!("pointer JSON is malformed: {error}")))?;
    let object = value
        .as_object()
        .ok_or_else(|| CustodyError::storage("delivery pointer must be a JSON object"))?;
    require_exact_value_keys(
        object,
        &[
            "schemaVersion",
            "pointerKey",
            "narrativeId",
            "revision",
            "state",
            "deliverySetHash",
            "candidateHash",
            "approvalHash",
            "meaningQualificationHash",
            "baseDeliverySetHash",
            "deliverySetPath",
            "pointerHash",
        ],
        "delivery pointer",
    )?;
    let pointer: PointerRecord = serde_json::from_value(value.clone()).map_err(|error| {
        CustodyError::storage(format!("delivery pointer fields are invalid: {error}"))
    })?;
    if pointer.schema_version != 2
        || pointer.narrative_id != narrative_id
        || pointer.revision == 0
        || pointer.revision > i32::MAX as u64
        || pointer.state != "PROMOTED"
        || !is_valid_opaque_id(&pointer.pointer_key)
        || !is_sha256_hash(&pointer.delivery_set_hash)
        || !is_sha256_hash(&pointer.candidate_hash)
        || !is_sha256_hash(&pointer.approval_hash)
        || !is_sha256_hash(&pointer.meaning_qualification_hash)
        || !is_base_delivery_set_hash(&pointer.base_delivery_set_hash)
    {
        return Err(CustodyError::storage(
            "delivery pointer schema or primitive value is invalid",
        ));
    }
    validate_narrative_id(&pointer.narrative_id)?;
    validate_safe_relative_path(&pointer.delivery_set_path)?;
    let mut hash_object = object.clone();
    hash_object.remove("pointerHash");
    if pointer.pointer_hash
        != sha256_prefixed(canonical_json(&Value::Object(hash_object)).as_bytes())
        || !is_sha256_hash(&pointer.pointer_hash)
    {
        return Err(CustodyError::storage(
            "delivery pointer pointerHash does not match canonical content",
        ));
    }
    // The contract stores a project-relative path.  The helper derives the
    // narrative-relative path only after checking this exact binding.
    let _ = pointer_delivery_set_relative(&pointer, narrative_id)?;
    Ok(pointer)
}

pub(super) fn parse_delivery_set_record(
    bytes: &[u8],
    narrative_id: &str,
) -> Result<DeliverySetRecord> {
    let value: Value = serde_json::from_slice(bytes).map_err(|error| {
        CustodyError::storage(format!("delivery set JSON is malformed: {error}"))
    })?;
    let object = value
        .as_object()
        .ok_or_else(|| CustodyError::storage("delivery set must be a JSON object"))?;
    require_exact_value_keys(
        object,
        &[
            "schemaVersion",
            "deliverySetId",
            "narrativeId",
            "candidateHash",
            "authorityHash",
            "meaningQualificationHash",
            "baseDeliverySetHash",
            "approvalHash",
            "artifacts",
            "evidenceCellHashes",
            "revision",
            "state",
            "deliverySetHash",
        ],
        "delivery set",
    )?;
    let delivery_set: DeliverySetRecord =
        serde_json::from_value(value.clone()).map_err(|error| {
            CustodyError::storage(format!("delivery set fields are invalid: {error}"))
        })?;
    if delivery_set.schema_version != 2
        || delivery_set.narrative_id != narrative_id
        || !is_valid_opaque_id(&delivery_set.delivery_set_id)
        || !is_sha256_hash(&delivery_set.candidate_hash)
        || !is_sha256_hash(&delivery_set.authority_hash)
        || !is_sha256_hash(&delivery_set.meaning_qualification_hash)
        || !is_base_delivery_set_hash(&delivery_set.base_delivery_set_hash)
        || !is_sha256_hash(&delivery_set.approval_hash)
        || delivery_set.revision == 0
        || delivery_set.revision > i32::MAX as u64
        || delivery_set.state != "PROMOTED"
        || delivery_set.evidence_cell_hashes.is_empty()
        || delivery_set
            .evidence_cell_hashes
            .iter()
            .any(|hash| !is_sha256_hash(hash))
        || !delivery_set_artifacts_valid(&delivery_set.artifacts)
    {
        return Err(CustodyError::storage(
            "delivery set schema or primitive value is invalid",
        ));
    }
    let mut hash_object = object.clone();
    hash_object.remove("deliverySetHash");
    if delivery_set.delivery_set_hash
        != sha256_prefixed(canonical_json(&Value::Object(hash_object)).as_bytes())
        || !is_sha256_hash(&delivery_set.delivery_set_hash)
    {
        return Err(CustodyError::storage(
            "delivery set deliverySetHash does not match canonical content",
        ));
    }
    Ok(delivery_set)
}

pub(super) fn validate_pointer_delivery_set(
    pointer: &PointerRecord,
    delivery_set: &DeliverySetRecord,
) -> Result<()> {
    for (label, left, right) in [
        (
            "narrativeId",
            &pointer.narrative_id,
            &delivery_set.narrative_id,
        ),
        (
            "revision",
            &pointer.revision.to_string(),
            &delivery_set.revision.to_string(),
        ),
        ("state", &pointer.state, &delivery_set.state),
        (
            "deliverySetHash",
            &pointer.delivery_set_hash,
            &delivery_set.delivery_set_hash,
        ),
        (
            "candidateHash",
            &pointer.candidate_hash,
            &delivery_set.candidate_hash,
        ),
        (
            "meaningQualificationHash",
            &pointer.meaning_qualification_hash,
            &delivery_set.meaning_qualification_hash,
        ),
        (
            "approvalHash",
            &pointer.approval_hash,
            &delivery_set.approval_hash,
        ),
        (
            "baseDeliverySetHash",
            &pointer.base_delivery_set_hash,
            &delivery_set.base_delivery_set_hash,
        ),
    ] {
        if left != right {
            return Err(CustodyError::storage(format!(
                "pointer and delivery set {label} values do not match",
            )));
        }
    }
    Ok(())
}

fn delivery_set_artifacts_valid(artifacts: &DeliverySetArtifacts) -> bool {
    [
        &artifacts.projection_receipt_hash,
        &artifacts.scene_graph_hash,
        &artifacts.asset_handoff_hash,
        &artifacts.asset_handoff_scene_graph_hash,
        &artifacts.html_hash,
        &artifacts.provenance_hash,
    ]
    .into_iter()
    .all(|hash| is_sha256_hash(hash))
}

pub(super) fn pointer_delivery_set_relative(
    pointer: &PointerRecord,
    narrative_id: &str,
) -> Result<String> {
    let hash_segment = pointer
        .delivery_set_hash
        .strip_prefix("sha256:")
        .ok_or_else(|| CustodyError::storage("pointer deliverySetHash is invalid"))?;
    let expected_project_path = expected_delivery_set_project_path(pointer, narrative_id)?;
    if pointer.delivery_set_path != expected_project_path {
        return Err(CustodyError::storage(
            "pointer deliverySetPath is not bound to narrative and deliverySetHash",
        ));
    }
    Ok(format!(
        "{}/{hash_segment}/trackb-delivery-set-v2.json",
        ArtifactDirectory::DeliverySets.as_segment()
    ))
}

pub(super) fn expected_delivery_set_project_path(
    pointer: &PointerRecord,
    narrative_id: &str,
) -> Result<String> {
    let hash_segment = pointer
        .delivery_set_hash
        .strip_prefix("sha256:")
        .ok_or_else(|| CustodyError::storage("pointer deliverySetHash is invalid"))?;
    Ok(format!(
        "{}/{}/{narrative_id}/{}/{hash_segment}/trackb-delivery-set-v2.json",
        SCENES_DIRECTORY,
        TRACKB_DIRECTORY,
        ArtifactDirectory::DeliverySets.as_segment(),
    ))
}

fn require_exact_value_keys(
    object: &Map<String, Value>,
    expected: &[&str],
    label: &str,
) -> Result<()> {
    if !has_exact_value_keys(object, expected) {
        return Err(CustodyError::storage(format!(
            "{label} fields must be the exact v1 set",
        )));
    }
    Ok(())
}

fn is_base_delivery_set_hash(value: &str) -> bool {
    value == "NONE" || is_sha256_hash(value)
}

pub(super) fn validate_base_delivery_set_hash(value: &str) -> Result<()> {
    if is_base_delivery_set_hash(value) {
        Ok(())
    } else {
        Err(CustodyError::input(
            "expectedBaseDeliverySetHash must be NONE or sha256:<64 lowercase hex>",
        ))
    }
}
