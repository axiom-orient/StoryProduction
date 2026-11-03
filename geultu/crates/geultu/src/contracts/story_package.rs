//! Producer-owned flat Geultu story package wire contract and identity.

use super::validate::{
    check_items, check_length, check_unique, valid_language_tag, valid_package_id, valid_scene_id,
};
use super::ArtifactRef;
use crate::error::{Error, Result};
use serde::{Deserialize, Serialize};

/// `schema_id` const of `geultu-story-package.schema.json`.
pub const GEULTU_STORY_PACKAGE_SCHEMA_ID: &str = "geultu.dev/geultu-story-package/v1";

/// `source` object of a GeultuStoryPackage (two required artifact refs).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct GeultuStoryPackageSource {
    pub text_master: ArtifactRef,
    pub export_spec: ArtifactRef,
}

impl GeultuStoryPackageSource {
    /// Validates both artifact refs.
    ///
    /// # Errors
    /// Returns [`Error::InvalidInput`] from the first invalid ref.
    pub fn validate(&self) -> Result<()> {
        self.text_master.validate()?;
        self.export_spec.validate()
    }
}

/// One scene of a GeultuStoryPackage (`geultu-story-package.schema.json`).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct GeultuStoryPackageScene {
    pub scene_id: String,
    pub order: u64,
    pub summary: String,
    pub narrative: String,
    pub characters: Vec<String>,
    pub locations: Vec<String>,
    pub continuity: Vec<String>,
    pub visual_priority: f64,
}

impl GeultuStoryPackageScene {
    /// Validates every scene-level constraint of the t33 schema: id pattern,
    /// order minimum, string length bounds, array bounds and uniqueness, and
    /// the closed `[0, 1]` range of `visual_priority`.
    ///
    /// # Errors
    /// Returns [`Error::InvalidInput`] naming the first violated field.
    pub fn validate(&self) -> Result<()> {
        if !valid_scene_id(self.scene_id.as_str()) {
            return Err(Error::InvalidInput(format!(
                "scene_id '{}' must match ^scene_[A-Za-z0-9_.-]{{1,120}}$",
                self.scene_id
            )));
        }
        if self.order < 1 {
            return Err(Error::InvalidInput(
                "scene order must be an integer of at least 1".into(),
            ));
        }
        check_length("scene summary", &self.summary, 1, 4000)?;
        check_length("scene narrative", &self.narrative, 1, 200_000)?;
        check_items("scene characters", &self.characters, 256, 300)?;
        check_unique("scene characters", &self.characters)?;
        check_items("scene locations", &self.locations, 64, 500)?;
        check_unique("scene locations", &self.locations)?;
        check_items("scene continuity", &self.continuity, 256, 2000)?;
        if !(0.0..=1.0).contains(&self.visual_priority) || !self.visual_priority.is_finite() {
            return Err(Error::InvalidInput(
                "scene visual_priority must be a number between 0 and 1".into(),
            ));
        }
        Ok(())
    }
}

/// Flat GeultuStoryPackage (`geultu-story-package.schema.json`, v1).
///
/// The flat schema rejects wrapped artifact documents by denying unknown
/// top-level fields at deserialization.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct GeultuStoryPackage {
    pub schema_id: String,
    pub package_id: String,
    pub source: GeultuStoryPackageSource,
    pub title: String,
    pub language: String,
    pub logline: String,
    pub scenes: Vec<GeultuStoryPackageScene>,
    #[serde(default)]
    pub global_constraints: Vec<String>,
}

impl GeultuStoryPackage {
    /// Recomputes the producer-owned `np_` identity from the package body,
    /// excluding only its claimed `package_id` field.
    ///
    /// Exporters use this to issue the identity; independent admission
    /// boundaries call it again before accepting an untrusted package.
    ///
    /// # Errors
    /// Returns [`Error::Json`] when the package cannot be serialized and
    /// [`Error::Corrupt`] if its serialized contract is not an object.
    pub(crate) fn recomputed_package_id(&self) -> Result<String> {
        let mut body = serde_json::to_value(self)?;
        let Some(object) = body.as_object_mut() else {
            return Err(Error::Corrupt(
                "serialized narrative package is not a JSON object".into(),
            ));
        };
        object.remove("package_id");
        Ok(format!(
            "np_{}",
            crate::contracts::digest::jcs_sha256(&body)?
        ))
    }

    /// Validates every package-level constraint of the t33 schema, including
    /// the `schema_id` const, the `np_` identity pattern, string bounds, the
    /// scene count bound, and per-scene validation.
    ///
    /// # Errors
    /// Returns [`Error::InvalidInput`] naming the first violated constraint.
    pub fn validate(&self) -> Result<()> {
        if self.schema_id != GEULTU_STORY_PACKAGE_SCHEMA_ID {
            return Err(Error::InvalidInput(format!(
                "schema_id '{}' must be '{GEULTU_STORY_PACKAGE_SCHEMA_ID}'",
                self.schema_id
            )));
        }
        if !valid_package_id(self.package_id.as_str()) {
            return Err(Error::InvalidInput(format!(
                "package_id '{}' must match ^np_[0-9a-f]{{64}}$",
                self.package_id
            )));
        }
        self.source.validate()?;
        check_length("title", &self.title, 1, 300)?;
        if !valid_language_tag(self.language.as_str(), 2) {
            return Err(Error::InvalidInput(format!(
                "language '{}' must match ^[A-Za-z]{{2,3}}(?:-[A-Za-z0-9]{{2,8}})*$",
                self.language
            )));
        }
        check_length("logline", &self.logline, 1, 2000)?;
        if self.scenes.is_empty() || self.scenes.len() > 10_000 {
            return Err(Error::InvalidInput(
                "scenes must contain between 1 and 10000 items".into(),
            ));
        }
        for scene in &self.scenes {
            scene.validate()?;
        }
        if self.global_constraints.len() > 256 {
            return Err(Error::InvalidInput(
                "global_constraints must contain at most 256 items".into(),
            ));
        }
        for constraint in &self.global_constraints {
            check_length("global_constraints entry", constraint, 1, 1000)?;
        }
        check_unique("global_constraints", &self.global_constraints)?;
        Ok(())
    }
}
