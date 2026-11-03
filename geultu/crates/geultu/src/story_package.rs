//! T33 GeultuStoryPackage export: identity projection from canonical project
//! state to a flat package with a deterministic `np_` identity.
//!
//! Projection rules (identity where a counterpart exists):
//! - `title` ← project title; scenes sorted by `(order, id)` like `scene.list`
//! - per scene: `scene_id` ← scene id, `order` ← scene order, `summary` ←
//!   scene title, `narrative` ← scene text
//! - `characters`, `locations`, `continuity` are empty arrays: the engine does
//!   not track these registries yet, and the schema allows empty arrays
//! - `visual_priority` is [`DEFAULT_VISUAL_PRIORITY`], the neutral value of
//!   the export-spec `visual_policy.default_priority` concept until a stored
//!   spec flow exists (plan H7)
//! - `source.text_master` references the canonical markdown text
//!   (`project.export_markdown` content) at `content_revision + 1`
//! - `source.export_spec` references the effective fixed spec applied by this
//!   exporter, digested with JCS
//!
//! `package_id` = `"np_" + jcs_sha256(serialized package minus package_id)`,
//! matching the fixture derivation in
//! `docs/canonical/fixtures/t33/valid-minimal.json`.

use crate::contracts::digest::{jcs_sha256, sha256_hex};
use crate::contracts::{
    ArtifactRef, GeultuStoryPackage, GeultuStoryPackageScene, GeultuStoryPackageSource,
    GEULTU_STORY_PACKAGE_SCHEMA_ID,
};
use crate::error::{Error, Result};
use crate::markdown::export_markdown;
use crate::model::Project;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

/// Neutral scene priority applied while no stored export-spec flow exists.
pub const DEFAULT_VISUAL_PRIORITY: f64 = 0.0;

/// Caller-supplied package metadata that has no counterpart in project state.
///
/// `language` and `logline` are required by the t33 schema (`minLength: 1`)
/// and are never invented here: callers must state them explicitly.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GeultuStoryExportInput {
    pub language: String,
    pub logline: String,
    #[serde(default)]
    pub global_constraints: Vec<String>,
}

/// Projects canonical project state into a flat GeultuStoryPackage.
///
/// The projection is a pure read: it never writes the event journal. The
/// returned package passes [`GeultuStoryPackage::validate`] before being
/// returned, so every schema constraint is enforced fail-closed.
///
/// # Errors
/// Returns [`Error::InvalidInput`] when the assembled package violates any
/// t33 constraint (empty scenes, scene ids outside the `scene_` pattern,
/// empty narrative text, invalid language tag, out-of-range lengths, duplicate
/// constraints). Returns [`Error::Json`] when serialization fails.
pub fn export_geultu_story_package(
    project: &Project,
    input: &GeultuStoryExportInput,
) -> Result<GeultuStoryPackage> {
    let mut ordered_scenes: Vec<_> = project.scenes.values().collect();
    ordered_scenes.sort_by(|left, right| (left.order, &left.id).cmp(&(right.order, &right.id)));
    let scenes = ordered_scenes
        .iter()
        .map(|scene| {
            Ok(GeultuStoryPackageScene {
                scene_id: scene.id.clone(),
                order: u64::try_from(scene.order).map_err(|_| {
                    Error::InvalidInput(format!(
                        "scene '{}' order {} is below the geultu-story-package minimum of 1",
                        scene.id, scene.order
                    ))
                })?,
                summary: scene.title.clone(),
                narrative: scene.text.clone(),
                characters: Vec::new(),
                locations: Vec::new(),
                continuity: Vec::new(),
                visual_priority: DEFAULT_VISUAL_PRIORITY,
            })
        })
        .collect::<Result<Vec<_>>>()?;

    let mut package = GeultuStoryPackage {
        schema_id: GEULTU_STORY_PACKAGE_SCHEMA_ID.into(),
        package_id: String::new(),
        source: GeultuStoryPackageSource {
            text_master: text_master_ref(project),
            export_spec: export_spec_ref(project, input)?,
        },
        title: project.title.clone(),
        language: input.language.clone(),
        logline: input.logline.clone(),
        scenes,
        global_constraints: input.global_constraints.clone(),
    };

    // The producer contract owns this identity rule; independent consumers
    // use the same definition to recompute and verify the claimed ID.
    package.package_id = package.recomputed_package_id()?;
    package.validate()?;
    Ok(package)
}

/// Internal canonical artifact reference of the project's markdown text master:
/// `id` is `text:<project_id>`, `kind` is `text_master`, `revision` is
/// `content_revision + 1`, and the digest is the raw SHA-256 over the
/// exported markdown bytes. Shared by narrative export and review receipts
/// so identical state yields identical artifact identities.
#[must_use]
pub(crate) fn text_master_ref(project: &Project) -> ArtifactRef {
    ArtifactRef {
        id: format!("text:{}", project.id),
        kind: "text_master".into(),
        revision: project.content_revision + 1,
        digest: format!("sha256:{}", sha256_hex(export_markdown(project).as_bytes())),
    }
}

fn export_spec_value(input: &GeultuStoryExportInput) -> Value {
    json!({
        "schema_id": "geultu.dev/geultu-story-export-spec/v1",
        "target_schema_id": GEULTU_STORY_PACKAGE_SCHEMA_ID,
        "language": input.language,
        "scene_selection": {"mode": "ALL"},
        "continuity_policy": "REPORT_ONLY",
        "visual_policy": {
            "default_priority": DEFAULT_VISUAL_PRIORITY,
            "include_global_constraints": !input.global_constraints.is_empty()
        },
    })
}

fn export_spec_ref(project: &Project, input: &GeultuStoryExportInput) -> Result<ArtifactRef> {
    let spec = export_spec_value(input);
    Ok(ArtifactRef {
        id: format!("text:{}-geultu-story-export-spec", project.id),
        kind: "geultu_story_export_spec".into(),
        revision: project.content_revision + 1,
        digest: format!("sha256:{}", jcs_sha256(&spec)?),
    })
}
