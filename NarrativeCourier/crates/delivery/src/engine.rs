//! Delivery engine composition and stable public DTOs.

use crate::backend::BackendDescriptor;
use narrative_courier_contracts::{AssetSource, DeliverySet};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone)]
pub struct DeliveryEngine;

mod assets;
mod context;
mod release;
mod review;
mod run;
mod storage;
mod views;

use assets::materialize::materialize_assets;
pub(crate) use assets::read_materialized_image;
use release::{load_contract_set, verify_delivery_artifacts, verify_released_package};
pub(crate) use storage::project_owned_path;
use storage::{read_json, write_json};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeliveryContext {
    pub narrative_package_sha256: String,
    pub backends: Vec<BackendDescriptor>,
    pub scenes: Vec<crate::planner::scene::Scene>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImportedDeliveryAsset {
    pub asset_id: String,
    pub name: String,
    pub source: AssetSource,
    pub preview_data_url: String,
    pub width: u32,
    pub height: u32,
    pub size_bytes: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeliveryRecord {
    pub request_id: String,
    pub set: DeliverySet,
}
