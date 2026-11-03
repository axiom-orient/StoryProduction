//! Delivery run/cancel lifecycle and canonical publish ownership.

mod lease;

use super::{
    materialize_assets, project_owned_path, read_json, verify_delivery_artifacts,
    verify_released_package, write_json, DeliveryEngine,
};
use crate::backend::{describe_backends, route_output};
use crate::planner::plan::build_plan;
use crate::planner::plan::request_policy::validate_supported_request;
use crate::planner::preflight::semantic_preflight;
use crate::render::render_output;
use narrative_courier_contracts::{
    validate_delivery_request, validate_delivery_set, DeliveryRequest, DeliverySet,
    NarrativePackage,
};
use narrative_courier_runtime::paths::validate_path_segment;
use narrative_courier_runtime::store::{lock_project, sha256_bytes, sha256_file};
use narrative_courier_runtime::token::hash_jcs;
use narrative_courier_runtime::{Error, Result};
use std::collections::BTreeMap;
use std::fs;
use std::path::Path;

impl DeliveryEngine {
    pub fn run(
        project: &Path,
        package: &NarrativePackage,
        request: DeliveryRequest,
    ) -> Result<DeliverySet> {
        validate_delivery_request(&request)?;
        validate_supported_request(&request)?;
        validate_path_segment(&request.request_id, "request_id")?;
        for output in &request.outputs {
            validate_path_segment(&output.output_id, "output_id")?;
        }
        project_owned_path(project, ".delivery-active")?;
        let _lock = lock_project(project)?;
        let package_sha = verify_released_package(project, package)?;
        if request.narrative_package_sha256 != package_sha {
            return Err(Error::Contract(
                "HASH_MISMATCH: request does not target current NarrativePackage".into(),
            ));
        }

        let request_sha = hash_jcs(&request)?;
        let run_root_relative = format!("visual/requests/{}", request.request_id);
        let request_path = format!("{run_root_relative}/delivery-request.json");
        let plan_path = format!("{run_root_relative}/director-plan.json");
        let asset_index_path = format!("{run_root_relative}/asset-index.json");
        let set_path = format!("{run_root_relative}/delivery-set.json");
        let prior_request_path = project_owned_path(project, &request_path)?;
        match fs::symlink_metadata(&prior_request_path) {
            Ok(metadata) if !metadata.is_file() || metadata.file_type().is_symlink() => {
                return Err(Error::Contract(
                    "delivery request marker is not a regular file".into(),
                ))
            }
            Ok(_) => {}
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
            Err(error) => return Err(error.into()),
        }
        let prior_set_path = project_owned_path(project, &set_path)?;
        if prior_set_path.try_exists()? && !prior_request_path.try_exists()? {
            return Err(Error::Contract(
                "delivery set exists without its request; explicit recovery is required".into(),
            ));
        }
        if prior_request_path.try_exists()? {
            let prior_request: DeliveryRequest = read_json(&prior_request_path)?;
            if hash_jcs(&prior_request)? != request_sha {
                return Err(Error::Contract(
                    "INVALID_INVARIANT: request_id cannot identify different request content"
                        .into(),
                ));
            }
            if prior_set_path.try_exists()? {
                return Self::load(project, package, &request.request_id);
            }
        }

        let descriptors = describe_backends()?;
        let mut routed = BTreeMap::new();
        for output in &request.outputs {
            routed.insert(
                output.output_id.clone(),
                route_output(output, &descriptors)?,
            );
        }

        let screenplay_path =
            project_owned_path(project, &package.canonical_narrative.artifact.path)?;
        let screenplay = fs::read_to_string(&screenplay_path)?;
        let plan = build_plan(package, &request, &screenplay)?;
        let _semantic_evidence = semantic_preflight(&plan, &request, &screenplay)?;
        let plan_sha = hash_jcs(&plan)?;
        if plan.delivery_request_sha256 != request_sha
            || plan.narrative_package_sha256 != package_sha
        {
            return Err(Error::Contract(
                "INVALID_INVARIANT: plan identity hash chain mismatch".into(),
            ));
        }

        let (control, lease) = lease::begin(project)?;
        let run_root = project_owned_path(project, &run_root_relative)?;
        fs::create_dir_all(&run_root)?;
        write_json(&project_owned_path(project, &request_path)?, &request)?;
        write_json(&project_owned_path(project, &plan_path)?, &plan)?;

        let asset_index = materialize_assets(project, &plan, &plan_sha)?;
        let asset_index_sha = hash_jcs(&asset_index)?;
        write_json(
            &project_owned_path(project, &asset_index_path)?,
            &asset_index,
        )?;

        let mut receipts = Vec::new();
        for output in &request.outputs {
            let backend = routed.get(&output.output_id).ok_or_else(|| {
                Error::Contract(format!(
                    "delivery backend route missing: {}",
                    output.output_id
                ))
            })?;
            receipts.push(render_output(
                project,
                &plan,
                &asset_index,
                output,
                backend,
                &control,
            )?);
        }

        // Reject stale/tampered NarrativePackage or authority immediately before canonical result publish.
        if verify_released_package(project, package)? != package_sha {
            return Err(Error::Contract(
                "STALE_RESULT: STS release changed during delivery run".into(),
            ));
        }

        let status = DeliverySet::aggregate_status(&receipts)?;
        let set_seed = sha256_bytes(
            format!("{package_sha}\n{request_sha}\n{plan_sha}\n{asset_index_sha}").as_bytes(),
        );
        let set = DeliverySet {
            schema_version: 1,
            set_id: format!("SET-{set_seed}"),
            narrative_package_sha256: package_sha,
            delivery_request_sha256: request_sha,
            director_plan_sha256: plan_sha,
            asset_index_sha256: asset_index_sha,
            status,
            deliveries: receipts,
        };
        validate_delivery_set(&set, &request)?;
        for asset in &asset_index.assets {
            let path = project_owned_path(project, &asset.path)?;
            if fs::metadata(&path)?.len() != asset.size_bytes || sha256_file(&path)? != asset.sha256
            {
                return Err(Error::Contract(format!(
                    "ASSET_HASH_MISMATCH: {} changed before delivery commit",
                    asset.asset_id
                )));
            }
        }
        verify_delivery_artifacts(project, &set)?;
        lease::claim_publish_ownership(project, &control)?;
        write_json(&project_owned_path(project, &set_path)?, &set)?;
        let committed = Self::load(project, package, &request.request_id);
        let cleanup = lease.finish();
        match (committed, cleanup) {
            (Ok(committed), Ok(())) if committed == set => Ok(committed),
            (Ok(_), Ok(())) => Err(Error::Contract(
                "delivery set changed during final commit".into(),
            )),
            (Err(error), Ok(())) => Err(error),
            (Ok(_), Err(cleanup)) => Err(Error::Contract(format!(
                "delivery set was written but lease cleanup failed: {cleanup}"
            ))),
            (Err(error), Err(cleanup)) => Err(Error::Contract(format!(
                "{error}; delivery lease cleanup also failed: {cleanup}"
            ))),
        }
    }
}
