//! Craft-pack integrity and refinement proposal routes.

use super::super::requests::refinement::{RefinementDiagnose, RefinementPropose};
use super::super::{command_id, parse, Application};
use super::unknown_tool;
use crate::error::{Error, Result};
use serde_json::{json, Value};

pub(in crate::application) fn call(
    app: &Application,
    name: &str,
    arguments: Value,
) -> Result<Value> {
    match name {
        "craftpack.status" => {
            let paths = app.require_craftpack_paths()?;
            let pack = crate::craftpack::load_craft_pack(&paths.craft_pack_dir)?;
            Ok(pack.status())
        }
        "refinement.diagnose" => {
            let req: RefinementDiagnose = parse(arguments)?;
            let pack = app.require_refinement_pack()?;
            if !pack.catalog().policy.allows("diagnose") {
                return Err(Error::InvalidInput(
                    "refinement catalog policy does not grant diagnose".into(),
                ));
            }
            let project = app.workspace.load_project(&req.project_id)?;
            let findings = match &req.scene_id {
                Some(scene_id) => {
                    let scene = project.scenes.get(scene_id).ok_or_else(|| {
                        Error::NotFound(format!("scene '{scene_id}' was not found"))
                    })?;
                    crate::refinement::diagnose_text(scene_id, &scene.text)
                }
                None => {
                    let mut scenes: Vec<_> = project.scenes.values().collect();
                    scenes.sort_by(|left, right| {
                        (left.order, &left.id).cmp(&(right.order, &right.id))
                    });
                    scenes
                        .iter()
                        .flat_map(|scene| crate::refinement::diagnose_text(&scene.id, &scene.text))
                        .collect()
                }
            };
            let finding_count = findings.len();
            Ok(json!({
                "project_id": req.project_id,
                "source_content_revision": project.content_revision,
                "catalog_id": pack.catalog().catalog_id,
                "policy": pack.catalog().policy,
                "locales": pack.catalog().locale_archives.keys().collect::<Vec<_>>(),
                "findings": findings,
                "finding_count": finding_count,
            }))
        }
        "refinement.propose_patch" => {
            let req: RefinementPropose = parse(arguments)?;
            let command_id = command_id(req.command_id);
            let pack = app.require_refinement_pack()?;
            let project = app.workspace.load_project(&req.project_id)?;
            let outcome = crate::refinement::propose_refinement_patch(
                &app.workspace,
                &pack,
                &project,
                &crate::refinement::PatchProposalInput {
                    scene_id: req.scene_id,
                    candidate_id: req.candidate_id,
                    changeset_id: req.changeset_id,
                    command_id,
                },
            )?;
            Ok(serde_json::to_value(outcome)?)
        }
        _ => Err(unknown_tool(name)),
    }
}
