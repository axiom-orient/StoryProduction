//! Validation report and proof receipt routes.

use super::super::super::{command_id, Application};
use crate::application::requests::common::ProjectRef;
use crate::application::requests::review::{ProofVerify, ReviewRunResponse};
use crate::domain::Command;
use crate::error::Result;
use crate::review::{review_project, verify_proof};
use serde_json::{json, Value};

pub(super) fn call(app: &Application, name: &str, arguments: Value) -> Result<Value> {
    match name {
        "review.run" => {
            let req: ProjectRef = super::super::super::parse(arguments)?;
            let project = app.workspace.load_project(&req.project_id)?;
            let report = review_project(&project);
            let receipt = crate::receipts::review_receipt(&project, &report)?;
            let result = app.workspace.transact(
                &req.project_id,
                &command_id(None),
                Command::RecordValidationReceipt {
                    receipt: receipt.clone(),
                },
            )?;
            Ok(serde_json::to_value(ReviewRunResponse {
                project_id: report.project_id,
                source_revision: report.source_revision,
                metrics: report.metrics,
                findings: report.findings,
                receipt,
                idempotent: result.idempotent,
            })?)
        }
        "proof.verify" => {
            let req: ProofVerify = super::super::super::parse(arguments)?;
            let report = verify_proof(
                &req.source,
                &req.candidate,
                &req.evidence,
                &req.protected_terms,
            );
            let anchor = match &req.project_id {
                Some(project_id) => Some(app.workspace.load_project(project_id)?),
                None => None,
            };
            let receipt = crate::receipts::proof_receipt(&req.candidate, anchor.as_ref(), &report)?;
            let mut recorded = false;
            if let Some(project_id) = &req.project_id {
                app.workspace.transact(
                    project_id,
                    &command_id(req.command_id),
                    Command::RecordValidationReceipt {
                        receipt: receipt.clone(),
                    },
                )?;
                recorded = true;
            }
            Ok(json!({
                "fidelity": report.fidelity,
                "grounding": report.grounding,
                "naturalness": report.naturalness,
                "protected_terms_preserved": report.protected_terms_preserved,
                "findings": report.findings,
                "receipt": receipt,
                "recorded": recorded,
            }))
        }
        _ => Err(super::super::unknown_tool(name)),
    }
}
