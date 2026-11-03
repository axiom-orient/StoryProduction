use super::scene::CanvasPlacement;
use super::{push_optional_string, take_u8};
use crate::json::{
    DecodeError, JsonValue, into_object, reject_unknown, take_optional_bool, take_optional_string,
    take_required, take_string, take_string_array, take_u64,
};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ImageTaskMode {
    Generate,
    Edit,
    Composite,
}

impl ImageTaskMode {
    pub(super) fn parse(value: &str, path: &str) -> Result<Self, DecodeError> {
        match value {
            "generate" => Ok(Self::Generate),
            "edit" => Ok(Self::Edit),
            "composite" => Ok(Self::Composite),
            _ => Err(DecodeError::new(path, "expected generate/edit/composite")),
        }
    }

    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Generate => "generate",
            Self::Edit => "edit",
            Self::Composite => "composite",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ImageReferenceRole {
    Base,
    Pose,
    Subject,
    Style,
    Product,
    Layout,
    Palette,
    Mask,
    Other,
}

impl ImageReferenceRole {
    pub(super) fn parse(value: &str, path: &str) -> Result<Self, DecodeError> {
        match value {
            "base" => Ok(Self::Base),
            "pose" => Ok(Self::Pose),
            "subject" => Ok(Self::Subject),
            "style" => Ok(Self::Style),
            "product" => Ok(Self::Product),
            "layout" => Ok(Self::Layout),
            "palette" => Ok(Self::Palette),
            "mask" => Ok(Self::Mask),
            "other" => Ok(Self::Other),
            _ => Err(DecodeError::new(
                path,
                "expected base/pose/subject/style/product/layout/palette/mask/other",
            )),
        }
    }

    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Base => "base",
            Self::Pose => "pose",
            Self::Subject => "subject",
            Self::Style => "style",
            Self::Product => "product",
            Self::Layout => "layout",
            Self::Palette => "palette",
            Self::Mask => "mask",
            Self::Other => "other",
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ImageReference {
    pub index: u8,
    pub role: ImageReferenceRole,
    pub description: String,
    pub use_for: Vec<String>,
}

impl ImageReference {
    pub(super) fn from_json(value: JsonValue, path: &str) -> Result<Self, DecodeError> {
        let mut fields = into_object(value, path)?;
        let role = take_string(&mut fields, "role", path)?;
        let result = Self {
            index: take_u8(&mut fields, "index", path)?,
            role: ImageReferenceRole::parse(&role, &format!("{path}.role"))?,
            description: take_string(&mut fields, "description", path)?,
            use_for: take_string_array(&mut fields, "use_for", path)?,
        };
        reject_unknown(fields, path)?;
        Ok(result)
    }

    pub(super) fn to_json(&self) -> JsonValue {
        JsonValue::object([
            ("description", JsonValue::from(self.description.clone())),
            ("index", JsonValue::from(u64::from(self.index))),
            ("role", JsonValue::from(self.role.as_str())),
            ("use_for", JsonValue::strings(&self.use_for)),
        ])
    }
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct ImageChangeContract {
    pub change_only: Vec<String>,
    pub preserve: Vec<String>,
}

impl ImageChangeContract {
    pub(super) fn from_json(value: JsonValue, path: &str) -> Result<Self, DecodeError> {
        let mut fields = into_object(value, path)?;
        let result = Self {
            change_only: take_string_array(&mut fields, "change_only", path)?,
            preserve: take_string_array(&mut fields, "preserve", path)?,
        };
        reject_unknown(fields, path)?;
        Ok(result)
    }

    pub(super) fn to_json(&self) -> JsonValue {
        JsonValue::object([
            ("change_only", JsonValue::strings(&self.change_only)),
            ("preserve", JsonValue::strings(&self.preserve)),
        ])
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TextRole {
    Headline,
    Subhead,
    Callout,
    Caption,
    Badge,
    Cta,
    Wordmark,
}

impl TextRole {
    pub(super) fn parse(value: &str, path: &str) -> Result<Self, DecodeError> {
        match value {
            "headline" => Ok(Self::Headline),
            "subhead" => Ok(Self::Subhead),
            "callout" => Ok(Self::Callout),
            "caption" => Ok(Self::Caption),
            "badge" => Ok(Self::Badge),
            "cta" => Ok(Self::Cta),
            "wordmark" => Ok(Self::Wordmark),
            _ => Err(DecodeError::new(
                path,
                "expected headline/subhead/callout/caption/badge/cta/wordmark",
            )),
        }
    }

    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Headline => "headline",
            Self::Subhead => "subhead",
            Self::Callout => "callout",
            Self::Caption => "caption",
            Self::Badge => "badge",
            Self::Cta => "cta",
            Self::Wordmark => "wordmark",
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TextElement {
    pub id: String,
    pub role: TextRole,
    pub lines: Vec<String>,
    pub language: String,
    pub placement: CanvasPlacement,
    pub font_family: String,
    pub weight: String,
    pub alignment: String,
    pub size_percent: u8,
    pub color_hex: String,
    pub letter_spacing: String,
    pub treatment: String,
    pub spelling_hint: Option<String>,
}

impl TextElement {
    pub(super) fn from_json(value: JsonValue, path: &str) -> Result<Self, DecodeError> {
        let mut fields = into_object(value, path)?;
        let role_value = take_string(&mut fields, "role", path)?;
        let result = Self {
            id: take_string(&mut fields, "id", path)?,
            role: TextRole::parse(&role_value, &format!("{path}.role"))?,
            lines: take_string_array(&mut fields, "lines", path)?,
            language: take_string(&mut fields, "language", path)?,
            placement: CanvasPlacement::from_json(
                take_required(&mut fields, "placement", path)?,
                &format!("{path}.placement"),
            )?,
            font_family: take_string(&mut fields, "font_family", path)?,
            weight: take_string(&mut fields, "weight", path)?,
            alignment: take_string(&mut fields, "alignment", path)?,
            size_percent: take_u8(&mut fields, "size_percent", path)?,
            color_hex: take_string(&mut fields, "color_hex", path)?,
            letter_spacing: take_string(&mut fields, "letter_spacing", path)?,
            treatment: take_string(&mut fields, "treatment", path)?,
            spelling_hint: take_optional_string(&mut fields, "spelling_hint", path)?,
        };
        reject_unknown(fields, path)?;
        Ok(result)
    }

    pub(super) fn to_json(&self) -> JsonValue {
        let mut fields = vec![
            ("alignment", JsonValue::from(self.alignment.clone())),
            ("color_hex", JsonValue::from(self.color_hex.clone())),
            ("font_family", JsonValue::from(self.font_family.clone())),
            ("id", JsonValue::from(self.id.clone())),
            ("language", JsonValue::from(self.language.clone())),
            (
                "letter_spacing",
                JsonValue::from(self.letter_spacing.clone()),
            ),
            ("lines", JsonValue::strings(&self.lines)),
            ("placement", self.placement.to_json()),
            ("role", JsonValue::from(self.role.as_str())),
            (
                "size_percent",
                JsonValue::from(u64::from(self.size_percent)),
            ),
            ("treatment", JsonValue::from(self.treatment.clone())),
            ("weight", JsonValue::from(self.weight.clone())),
        ];
        push_optional_string(&mut fields, "spelling_hint", self.spelling_hint.as_ref());
        JsonValue::object(fields)
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ImageConstraints {
    pub required_elements: Vec<String>,
    pub excluded_elements: Vec<String>,
    pub safety_tier: u8,
    pub adult_subjects_only: bool,
    pub original_characters_only: bool,
    pub clean_unbranded_finish: bool,
}

impl ImageConstraints {
    pub(super) fn from_json(value: JsonValue, path: &str) -> Result<Self, DecodeError> {
        let mut fields = into_object(value, path)?;
        let result = Self {
            required_elements: take_string_array(&mut fields, "required_elements", path)?,
            excluded_elements: take_string_array(&mut fields, "excluded_elements", path)?,
            safety_tier: take_u8(&mut fields, "safety_tier", path)?,
            adult_subjects_only: take_optional_bool(&mut fields, "adult_subjects_only", path)?
                .unwrap_or(false),
            original_characters_only: take_optional_bool(
                &mut fields,
                "original_characters_only",
                path,
            )?
            .unwrap_or(true),
            clean_unbranded_finish: take_optional_bool(
                &mut fields,
                "clean_unbranded_finish",
                path,
            )?
            .unwrap_or(true),
        };
        reject_unknown(fields, path)?;
        Ok(result)
    }

    pub(super) fn to_json(&self) -> JsonValue {
        JsonValue::object([
            (
                "adult_subjects_only",
                JsonValue::from(self.adult_subjects_only),
            ),
            (
                "clean_unbranded_finish",
                JsonValue::from(self.clean_unbranded_finish),
            ),
            (
                "excluded_elements",
                JsonValue::strings(&self.excluded_elements),
            ),
            (
                "original_characters_only",
                JsonValue::from(self.original_characters_only),
            ),
            (
                "required_elements",
                JsonValue::strings(&self.required_elements),
            ),
            ("safety_tier", JsonValue::from(u64::from(self.safety_tier))),
        ])
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ImageQuality {
    Auto,
    Low,
    Medium,
    High,
}

impl ImageQuality {
    pub(super) fn parse(value: &str, path: &str) -> Result<Self, DecodeError> {
        match value {
            "auto" => Ok(Self::Auto),
            "low" => Ok(Self::Low),
            "medium" => Ok(Self::Medium),
            "high" => Ok(Self::High),
            _ => Err(DecodeError::new(path, "expected auto/low/medium/high")),
        }
    }

    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Auto => "auto",
            Self::Low => "low",
            Self::Medium => "medium",
            Self::High => "high",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BackgroundMode {
    Auto,
    Opaque,
}

impl BackgroundMode {
    pub(super) fn parse(value: &str, path: &str) -> Result<Self, DecodeError> {
        match value {
            "auto" => Ok(Self::Auto),
            "opaque" => Ok(Self::Opaque),
            "transparent" => Err(DecodeError::new(
                path,
                "gpt-image-2 does not support transparent backgrounds",
            )),
            _ => Err(DecodeError::new(path, "expected auto or opaque")),
        }
    }

    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Auto => "auto",
            Self::Opaque => "opaque",
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ImageGenerationParameters {
    pub model: String,
    pub width: u32,
    pub height: u32,
    pub quality: ImageQuality,
    pub background: BackgroundMode,
    pub format: String,
}

impl ImageGenerationParameters {
    pub(super) fn from_json(value: JsonValue, path: &str) -> Result<Self, DecodeError> {
        let mut fields = into_object(value, path)?;
        let width = take_u64(&mut fields, "width", path)?;
        let height = take_u64(&mut fields, "height", path)?;
        let width = u32::try_from(width)
            .map_err(|_| DecodeError::new(format!("{path}.width"), "value exceeds u32"))?;
        let height = u32::try_from(height)
            .map_err(|_| DecodeError::new(format!("{path}.height"), "value exceeds u32"))?;
        let quality_value = take_string(&mut fields, "quality", path)?;
        let background_value = take_string(&mut fields, "background", path)?;
        let result = Self {
            model: take_string(&mut fields, "model", path)?,
            width,
            height,
            quality: ImageQuality::parse(&quality_value, &format!("{path}.quality"))?,
            background: BackgroundMode::parse(&background_value, &format!("{path}.background"))?,
            format: take_string(&mut fields, "format", path)?,
        };
        reject_unknown(fields, path)?;
        Ok(result)
    }

    pub fn to_json(&self) -> JsonValue {
        JsonValue::object([
            ("background", JsonValue::from(self.background.as_str())),
            ("format", JsonValue::from(self.format.clone())),
            ("height", JsonValue::from(u64::from(self.height))),
            ("model", JsonValue::from(self.model.clone())),
            ("quality", JsonValue::from(self.quality.as_str())),
            ("width", JsonValue::from(u64::from(self.width))),
        ])
    }

    pub fn aspect_ratio_label(&self) -> String {
        let (mut left, mut right) = (self.width, self.height);
        while right != 0 {
            let remainder = left % right;
            left = right;
            right = remainder;
        }
        let divisor = left;
        format!("{}:{}", self.width / divisor, self.height / divisor)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ImageRenderProfile {
    Structured,
    EditorialFlat,
    PoseTransfer,
    CinematicStoryboard,
}

/// Category-bound image intent profiles. These refine one of the six primary
/// catalog outcomes without creating a new catalog card or execution route.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ImageProfile {
    Standard,
    LogoIdentity,
    AppIcon,
    AppWebUi,
    InformationDesign,
    CharacterPose,
    PoomsaePose,
}

impl ImageProfile {
    pub(super) fn parse(value: &str, path: &str) -> Result<Self, DecodeError> {
        match value {
            "standard" => Ok(Self::Standard),
            "logo_identity" => Ok(Self::LogoIdentity),
            "app_icon" => Ok(Self::AppIcon),
            "app_web_ui" => Ok(Self::AppWebUi),
            "information_design" => Ok(Self::InformationDesign),
            "character_pose" => Ok(Self::CharacterPose),
            "poomsae_pose" => Ok(Self::PoomsaePose),
            _ => Err(DecodeError::new(
                path,
                "expected standard, logo_identity, app_icon, app_web_ui, information_design, character_pose, or poomsae_pose",
            )),
        }
    }

    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Standard => "standard",
            Self::LogoIdentity => "logo_identity",
            Self::AppIcon => "app_icon",
            Self::AppWebUi => "app_web_ui",
            Self::InformationDesign => "information_design",
            Self::CharacterPose => "character_pose",
            Self::PoomsaePose => "poomsae_pose",
        }
    }
}

impl ImageRenderProfile {
    pub(super) fn parse(value: &str, path: &str) -> Result<Self, DecodeError> {
        match value {
            "structured" => Ok(Self::Structured),
            "editorial_flat" => Ok(Self::EditorialFlat),
            "pose_transfer" => Ok(Self::PoseTransfer),
            "cinematic_storyboard" => Ok(Self::CinematicStoryboard),
            _ => Err(DecodeError::new(
                path,
                "expected structured, editorial_flat, pose_transfer, or cinematic_storyboard",
            )),
        }
    }

    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Structured => "structured",
            Self::EditorialFlat => "editorial_flat",
            Self::PoseTransfer => "pose_transfer",
            Self::CinematicStoryboard => "cinematic_storyboard",
        }
    }
}
