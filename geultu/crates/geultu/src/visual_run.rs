//! H7 Geultu-owned visual-run boundary for a T33 GeultuStoryPackage submitted
//! with an NVC route claim.
//!
//! This module validates a current-source package and records only a local
//! [`VisualRunState`]. The caller-supplied NVC route is shape-checked; this
//! module neither attests to NVC execution nor asserts an NVC Completed
//! release. Ordered preflight checks fail closed, and only a full pass journals
//! `Event::VisualRunAdmitted` plus its structural validation receipt.
//!
//! Identity rules delegate to the producer-owned `GeultuStoryPackage`
//! contract:
//! - the claimed `package_id` must equal `"np_" + jcs_sha256(package minus
//!   package_id)` — a tampered id is rejected before any journal write
//! - the run's `narrative_package` artifact ref is a GeultuStoryPackage ref
//!   and digests exactly those bytes,
//!   so `digest == "sha256:" + package_id[3..]`
//!
//! Double-admission policy (explicit conflict, documented): re-admitting the
//! same package through the same command is handled by the journal's command
//! idempotency; a second admission call with identical run state returns the
//! recorded state with zero new events (`idempotent: true`), while any other
//! collision — same run_id with different content, or the same package under
//! a different run_id — is an [`Error::Conflict`].

use crate::contracts::{
    ArtifactRef, GeultuStoryPackage, RouteAdmission, RouteId, ValidationReceipt, VisualRunState,
    VisualRunStatus, VISUAL_RUN_STATE_SCHEMA_ID,
};
use crate::error::{Error, Result};
use crate::model::Project;
use serde::{Deserialize, Serialize};

/// Validator identity stamped onto admission receipts.
pub const ADMISSION_VALIDATOR_ID: &str = "geultu-nvc-admission";

/// Caller input for source-bound local visual admission. The complete inline body is required.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AdmitPackageInput {
    pub project_id: String,
    pub run_id: String,
    pub stage: String,
    pub package: GeultuStoryPackage,
    pub route_admission: RouteAdmission,
}

/// One explicit preflight checklist result; a checklist is only ever
/// returned complete (all items passed), because any failure aborts
/// admission fail-closed before a journal write.
#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct PreflightCheck {
    pub check: &'static str,
    pub detail: String,
}

/// The admission outcome: the recorded run state, its PASS receipt, and the
/// preflight checklist that admitted it.
#[derive(Debug, Clone, Serialize)]
pub struct AdmissionOutcome {
    pub run: VisualRunState,
    pub receipt: ValidationReceipt,
    pub preflight: Vec<PreflightCheck>,
}

/// Runs the full preflight checklist for admitting `input` into project
/// state and, when every item passes, returns the run state to journal, its
/// receipt, and the checklist.
///
/// Checklist order (each failure aborts with a named error, journal
/// untouched):
/// 1. `input_shape` — required inline body and matching project identity
/// 2. `t33_contract` — the package satisfies every t33 constraint
/// 3. `package_id_recomputation` — claimed id equals the recomputed identity
/// 4. `current_source_binding` — body matches the current Geultu export projection
/// 5. `route_admission_shape` — route contract valid, `route_id == NVC`, and
///    its `command_id` names this admission command
/// 6. `run_state_shape` — assembled initial READY state validates
/// 7. `not_already_admitted` — no conflicting prior admission
///
/// # Errors
/// Returns [`Error::InvalidInput`] for shape/contract violations and
/// [`Error::Conflict`] for stale source or double-admission collisions.
pub fn admit_package(project: &Project, input: &AdmitPackageInput) -> Result<AdmissionOutcome> {
    if input.project_id != project.id {
        return Err(Error::InvalidInput(
            "preflight input_shape failed: project identity mismatch".into(),
        ));
    }
    let package = &input.package;
    let mut preflight = vec![PreflightCheck {
        check: "input_shape",
        detail: format!(
            "required inline package '{}' supplied for the loaded project",
            package.package_id
        ),
    }];

    package
        .validate()
        .map_err(|error| Error::InvalidInput(format!("preflight t33_contract failed: {error}")))?;
    preflight.push(PreflightCheck {
        check: "t33_contract",
        detail: format!(
            "package '{}' satisfies every geultu-story-package constraint",
            package.package_id
        ),
    });

    verify_package_identity(package)?;
    preflight.push(PreflightCheck {
        check: "package_id_recomputation",
        detail: "claimed package_id equals np_ + jcs_sha256(package minus package_id)".into(),
    });

    validate_current_source(project, package)?;
    preflight.push(PreflightCheck {
        check: "current_source_binding",
        detail: "package matches the current Geultu export projection and source references".into(),
    });

    validate_route_admission(&input.route_admission)?;
    preflight.push(PreflightCheck {
        check: "route_admission_shape",
        detail: format!(
            "route '{}:{}' satisfies route-admission constraints with budget {}/{}ms",
            input.route_admission.route_id.as_str(),
            input.route_admission.command_id,
            input.route_admission.budget.output_bytes,
            input.route_admission.budget.time_ms,
        ),
    });

    let narrative_package = package_artifact_ref(project, package);
    let run = VisualRunState {
        schema_id: VISUAL_RUN_STATE_SCHEMA_ID.into(),
        project_id: input.project_id.clone(),
        run_id: input.run_id.clone(),
        revision: 0,
        stage: input.stage.clone(),
        status: VisualRunStatus::Ready,
        narrative_package,
        accepted_outputs: std::collections::BTreeMap::new(),
        pending_effect: None,
    };
    run.validate().map_err(|error| {
        Error::InvalidInput(format!("preflight run_state_shape failed: {error}"))
    })?;
    preflight.push(PreflightCheck {
        check: "run_state_shape",
        detail: format!(
            "assembled visual-run-state '{}' at stage '{}' validates",
            run.run_id, run.stage
        ),
    });

    ensure_not_already_admitted(project, &run)?;
    preflight.push(PreflightCheck {
        check: "not_already_admitted",
        detail: format!(
            "no prior admission collides with run '{}' or package '{}'",
            run.run_id, package.package_id
        ),
    });

    let receipt = admission_receipt(&run)?;
    Ok(AdmissionOutcome {
        run,
        receipt,
        preflight,
    })
}

fn validate_current_source(project: &Project, package: &GeultuStoryPackage) -> Result<()> {
    let current = crate::story_package::export_geultu_story_package(
        project,
        &crate::story_package::GeultuStoryExportInput {
            language: package.language.clone(),
            logline: package.logline.clone(),
            global_constraints: package.global_constraints.clone(),
        },
    )?;
    if current.package_id != package.package_id {
        return Err(Error::Conflict(
            "preflight current_source_binding failed: package differs from the current Geultu projection".into(),
        ));
    }
    Ok(())
}

/// Verifies the claimed identity against the recomputed one; a mismatch is
/// a tampering signal and aborts before any journal write.
///
/// # Errors
/// Returns [`Error::InvalidInput`] naming both ids when they differ, or
/// [`Error::Json`] when serialization fails.
fn verify_package_identity(package: &GeultuStoryPackage) -> Result<()> {
    let recomputed = package.recomputed_package_id()?;
    if recomputed != package.package_id {
        return Err(Error::InvalidInput(format!(
            "preflight package_id_recomputation failed: claimed '{}' does not match recomputed '{}'",
            package.package_id, recomputed
        )));
    }
    Ok(())
}

fn validate_route_admission(route: &RouteAdmission) -> Result<()> {
    route.validate().map_err(|error| {
        Error::InvalidInput(format!("preflight route_admission_shape failed: {error}"))
    })?;
    if route.route_id != RouteId::Nvc {
        return Err(Error::InvalidInput(format!(
            "preflight route_admission_shape failed: route_id '{}' must be 'NVC' for visual.admit_package",
            route.route_id.as_str()
        )));
    }
    if route.command_id != "visual.admit_package" {
        return Err(Error::InvalidInput(format!(
            "preflight route_admission_shape failed: command_id '{}' must be 'visual.admit_package'",
            route.command_id
        )));
    }
    Ok(())
}

/// Canonical artifact reference of the admitted package, following the
/// `story_package.rs` "+1 past authored revision" convention: `id` is
/// `story:<package_id>`, `kind` is `geultu_story_package`, `revision` is
/// `content_revision + 1`, and the digest commits to exactly the bytes the
/// `np_` identity commits to.
fn package_artifact_ref(project: &Project, package: &GeultuStoryPackage) -> ArtifactRef {
    ArtifactRef {
        id: format!("story:{}", package.package_id),
        kind: "geultu_story_package".into(),
        revision: project.content_revision + 1,
        digest: format!("sha256:{}", &package.package_id[3..]),
    }
}

/// Explicit conflict policy: identical replay of an already-recorded run is
/// reported by the caller as idempotent; every other collision fails closed.
///
/// # Errors
/// Returns [`Error::Conflict`] when the run_id exists with different content
/// or when the same package is already admitted under another run_id.
fn ensure_not_already_admitted(project: &Project, run: &VisualRunState) -> Result<()> {
    if let Some(existing) = project.visual_runs.get(&run.run_id) {
        if existing == run {
            return Ok(());
        }
        return Err(Error::Conflict(format!(
            "visual run '{}' is already admitted with different content",
            run.run_id
        )));
    }
    if let Some(existing) = project
        .visual_runs
        .values()
        .find(|recorded| recorded.narrative_package == run.narrative_package)
    {
        return Err(Error::Conflict(format!(
            "package '{}' is already admitted under visual run '{}'",
            run.narrative_package.id, existing.run_id
        )));
    }
    Ok(())
}

/// Builds the PASS receipt binding the admission to the exact package
/// artifact; `receipt_id` follows the M2 derivation
/// (`vr_` + jcs_sha256 minus the id).
///
/// # Errors
/// Returns [`Error::Json`] when serialization fails or the assembled
/// receipt violates the validation-receipt contract.
fn admission_receipt(run: &VisualRunState) -> Result<ValidationReceipt> {
    crate::receipts::validation::finish_receipt(
        ADMISSION_VALIDATOR_ID,
        run.narrative_package.clone(),
        crate::contracts::ReceiptOutcome::Pass,
        Vec::new(),
    )
}
