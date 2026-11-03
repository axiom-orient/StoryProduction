use super::super::common::{
    PromptLanguage, take_language, take_object_array, take_optional_object_array,
};
use super::continuity::ImageConsistencySpec;
use super::request_parts::{
    ImageChangeContract, ImageConstraints, ImageGenerationParameters, ImageProfile, ImageReference,
    ImageRenderProfile, ImageTaskMode, TextElement,
};
use super::scene::{
    CinematicStoryboardSpec, CompositionSpec, SceneSpec, SubjectSpec, TaxonomySelection,
    VisualMedium,
};
use super::visual::{CameraSpec, ColorSpec, LightingSpec, SurfaceSpec};
use crate::json::{
    DecodeError, JsonValue, expect_string, into_object, reject_unknown, take_optional,
    take_required, take_string,
};

#[derive(Clone, Debug, PartialEq)]
pub struct ImagePromptRequest {
    pub language: PromptLanguage,
    pub profile: ImageProfile,
    pub render_profile: ImageRenderProfile,
    pub task_mode: ImageTaskMode,
    pub references: Vec<ImageReference>,
    pub change_contract: ImageChangeContract,
    pub use_case: String,
    pub medium: VisualMedium,
    pub taxonomy: TaxonomySelection,
    pub scene: SceneSpec,
    pub subjects: Vec<SubjectSpec>,
    pub composition: CompositionSpec,
    pub storyboard: Option<CinematicStoryboardSpec>,
    pub consistency: Option<ImageConsistencySpec>,
    pub camera: Option<CameraSpec>,
    pub lighting: LightingSpec,
    pub color: ColorSpec,
    pub surfaces: Vec<SurfaceSpec>,
    pub text_elements: Vec<TextElement>,
    pub constraints: ImageConstraints,
    pub output: ImageGenerationParameters,
}

impl ImagePromptRequest {
    pub fn from_json(value: JsonValue) -> Result<Self, DecodeError> {
        let path = "$";
        let mut fields = into_object(value, path)?;
        let medium_value = take_string(&mut fields, "medium", path)?;
        let render_profile = match take_optional(&mut fields, "render_profile") {
            None | Some(JsonValue::Null) => ImageRenderProfile::Structured,
            Some(value) => {
                let value = expect_string(value, "$.render_profile")?;
                ImageRenderProfile::parse(&value, "$.render_profile")?
            }
        };
        let profile = match take_optional(&mut fields, "profile") {
            None | Some(JsonValue::Null) => ImageProfile::Standard,
            Some(value) => {
                let value = expect_string(value, "$.profile")?;
                ImageProfile::parse(&value, "$.profile")?
            }
        };
        let task_mode = match take_optional(&mut fields, "task_mode") {
            None | Some(JsonValue::Null) => ImageTaskMode::Generate,
            Some(value) => {
                let value = expect_string(value, "$.task_mode")?;
                ImageTaskMode::parse(&value, "$.task_mode")?
            }
        };
        let references =
            take_optional_object_array(&mut fields, "references", path, ImageReference::from_json)?;
        let change_contract = match take_optional(&mut fields, "change_contract") {
            None | Some(JsonValue::Null) => ImageChangeContract::default(),
            Some(value) => ImageChangeContract::from_json(value, "$.change_contract")?,
        };
        let result = Self {
            language: take_language(&mut fields, path)?,
            profile,
            render_profile,
            task_mode,
            references,
            change_contract,
            use_case: take_string(&mut fields, "use_case", path)?,
            medium: VisualMedium::parse(&medium_value, "$.medium")?,
            taxonomy: TaxonomySelection::from_json(
                take_required(&mut fields, "taxonomy", path)?,
                "$.taxonomy",
            )?,
            scene: SceneSpec::from_json(take_required(&mut fields, "scene", path)?, "$.scene")?,
            subjects: take_object_array(&mut fields, "subjects", path, SubjectSpec::from_json)?,
            composition: CompositionSpec::from_json(
                take_required(&mut fields, "composition", path)?,
                "$.composition",
            )?,
            storyboard: take_optional(&mut fields, "storyboard")
                .map(|value| CinematicStoryboardSpec::from_json(value, "$.storyboard"))
                .transpose()?,
            consistency: take_optional(&mut fields, "consistency")
                .map(|value| ImageConsistencySpec::from_json(value, "$.consistency"))
                .transpose()?,
            camera: take_optional(&mut fields, "camera")
                .map(|value| CameraSpec::from_json(value, "$.camera"))
                .transpose()?,
            lighting: LightingSpec::from_json(
                take_required(&mut fields, "lighting", path)?,
                "$.lighting",
            )?,
            color: ColorSpec::from_json(take_required(&mut fields, "color", path)?, "$.color")?,
            surfaces: take_optional_object_array(
                &mut fields,
                "surfaces",
                path,
                SurfaceSpec::from_json,
            )?,
            text_elements: take_optional_object_array(
                &mut fields,
                "text_elements",
                path,
                TextElement::from_json,
            )?,
            constraints: ImageConstraints::from_json(
                take_required(&mut fields, "constraints", path)?,
                "$.constraints",
            )?,
            output: ImageGenerationParameters::from_json(
                take_required(&mut fields, "output", path)?,
                "$.output",
            )?,
        };
        reject_unknown(fields, path)?;
        Ok(result)
    }

    pub fn to_json(&self) -> JsonValue {
        let mut fields = vec![
            ("color", self.color.to_json()),
            ("composition", self.composition.to_json()),
            ("constraints", self.constraints.to_json()),
            ("language", JsonValue::from(self.language.code())),
            ("profile", JsonValue::from(self.profile.as_str())),
            ("lighting", self.lighting.to_json()),
            ("medium", JsonValue::from(self.medium.as_str())),
            ("output", self.output.to_json()),
            (
                "render_profile",
                JsonValue::from(self.render_profile.as_str()),
            ),
            ("task_mode", JsonValue::from(self.task_mode.as_str())),
            (
                "references",
                JsonValue::array(self.references.iter().map(ImageReference::to_json)),
            ),
            ("change_contract", self.change_contract.to_json()),
            ("scene", self.scene.to_json()),
            (
                "storyboard",
                self.storyboard
                    .as_ref()
                    .map_or(JsonValue::Null, CinematicStoryboardSpec::to_json),
            ),
            (
                "consistency",
                self.consistency
                    .as_ref()
                    .map_or(JsonValue::Null, ImageConsistencySpec::to_json),
            ),
            (
                "subjects",
                JsonValue::array(self.subjects.iter().map(SubjectSpec::to_json)),
            ),
            (
                "surfaces",
                JsonValue::array(self.surfaces.iter().map(SurfaceSpec::to_json)),
            ),
            ("taxonomy", self.taxonomy.to_json()),
            (
                "text_elements",
                JsonValue::array(self.text_elements.iter().map(TextElement::to_json)),
            ),
            ("use_case", JsonValue::from(self.use_case.clone())),
        ];
        if let Some(camera) = &self.camera {
            fields.push(("camera", camera.to_json()));
        }
        JsonValue::object(fields)
    }
}
