use std::collections::BTreeMap;
use std::fmt;

use crate::json::{
    DecodeError, JsonValue, into_array, into_object, reject_unknown, take_bool,
    take_optional_string, take_optional_string_array, take_required, take_string,
    take_string_array, take_u64,
};

use super::PRIMARY_OUTPUT_TIER_1;

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum CatalogFamily {
    Category,
    Promo,
    Typography,
    Look,
    Movement,
    Response,
    TypeArt,
    Editorial,
}

impl CatalogFamily {
    /// Parses one of the canonical family identifiers used by catalog filters.
    pub fn parse(value: &str) -> Option<Self> {
        match value {
            "category" => Some(Self::Category),
            "promo" => Some(Self::Promo),
            "typography" => Some(Self::Typography),
            "look" => Some(Self::Look),
            "movement" => Some(Self::Movement),
            "response" => Some(Self::Response),
            "type_art" => Some(Self::TypeArt),
            "editorial" => Some(Self::Editorial),
            _ => None,
        }
    }

    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Category => "category",
            Self::Promo => "promo",
            Self::Typography => "typography",
            Self::Look => "look",
            Self::Movement => "movement",
            Self::Response => "response",
            Self::TypeArt => "type_art",
            Self::Editorial => "editorial",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum DirectiveSlot {
    Subject,
    Scene,
    Composition,
    Camera,
    Lighting,
    Color,
    Material,
    Typography,
    Motion,
    Constraint,
}

impl DirectiveSlot {
    fn parse(value: &str) -> Option<Self> {
        match value {
            "subject" => Some(Self::Subject),
            "scene" => Some(Self::Scene),
            "composition" => Some(Self::Composition),
            "camera" => Some(Self::Camera),
            "lighting" => Some(Self::Lighting),
            "color" => Some(Self::Color),
            "material" => Some(Self::Material),
            "typography" => Some(Self::Typography),
            "motion" => Some(Self::Motion),
            "constraint" => Some(Self::Constraint),
            _ => None,
        }
    }

    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Subject => "subject",
            Self::Scene => "scene",
            Self::Composition => "composition",
            Self::Camera => "camera",
            Self::Lighting => "lighting",
            Self::Color => "color",
            Self::Material => "material",
            Self::Typography => "typography",
            Self::Motion => "motion",
            Self::Constraint => "constraint",
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PromptDirective {
    pub slot: DirectiveSlot,
    pub text: String,
}

impl PromptDirective {
    fn from_json(value: JsonValue, path: &str) -> Result<Self, DecodeError> {
        let mut fields = into_object(value, path)?;
        let slot_value = take_string(&mut fields, "slot", path)?;
        let slot = DirectiveSlot::parse(&slot_value).ok_or_else(|| {
            DecodeError::new(
                format!("{path}.slot"),
                format!("unknown directive slot {slot_value:?}"),
            )
        })?;
        let result = Self {
            slot,
            text: take_string(&mut fields, "text", path)?,
        };
        reject_unknown(fields, path)?;
        Ok(result)
    }

    fn to_json(&self) -> JsonValue {
        JsonValue::object([
            ("slot", JsonValue::from(self.slot.as_str())),
            ("text", JsonValue::from(self.text.clone())),
        ])
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum TextRequirement {
    Optional,
    Recommended,
    Required,
}

impl TextRequirement {
    fn parse(value: &str) -> Option<Self> {
        match value {
            "optional" => Some(Self::Optional),
            "recommended" => Some(Self::Recommended),
            "required" => Some(Self::Required),
            _ => None,
        }
    }

    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Optional => "optional",
            Self::Recommended => "recommended",
            Self::Required => "required",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum ReferenceTextMode {
    None,
    BlankZone,
    SampleCopy,
}

impl ReferenceTextMode {
    fn parse(value: &str) -> Option<Self> {
        match value {
            "none" => Some(Self::None),
            "blank_zone" => Some(Self::BlankZone),
            "sample_copy" => Some(Self::SampleCopy),
            _ => None,
        }
    }

    pub const fn as_str(self) -> &'static str {
        match self {
            Self::None => "none",
            Self::BlankZone => "blank_zone",
            Self::SampleCopy => "sample_copy",
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CatalogEntry {
    pub id: String,
    pub family: CatalogFamily,
    pub tier_1: String,
    pub tier_2: String,
    pub tier_3: Option<String>,
    pub asset_path: String,
    pub name_ko: String,
    pub name_en: String,
    pub intent: String,
    pub recipe: Vec<String>,
    pub prompt_directives: Vec<PromptDirective>,
    pub profiles: Vec<String>,
    pub text_requirement: TextRequirement,
    pub reference_text_mode: ReferenceTextMode,
    pub reference_exact_text: Vec<String>,
    pub repetition_text: bool,
    pub suggested_aspect_ratios: Vec<String>,
    pub suggested_palette: Vec<String>,
    pub default_safety_tier: u8,
    pub source_path: String,
    pub source_sha256: String,
}

impl CatalogEntry {
    /// A primary result is selected once at the first catalog level. `family`
    /// groups supporting directions for cardinality and profile policy only.
    pub fn is_primary_output(&self) -> bool {
        self.tier_1 == PRIMARY_OUTPUT_TIER_1
    }

    /// The English name as prose, or `None` when it carries no information.
    ///
    /// Catalog authoring may use an ID echo or a snake_case slug in `name_en`.
    /// User-facing and model-facing text must be prose, so ID echoes are omitted and
    /// underscores are converted to spaces.
    pub(crate) fn display_name_en(&self) -> Option<String> {
        if self.name_en.is_empty() || self.name_en == self.id {
            return None;
        }
        Some(self.name_en.replace('_', " "))
    }

    pub(super) fn from_json(value: JsonValue, path: &str) -> Result<Self, DecodeError> {
        let mut fields = into_object(value, path)?;
        let id = take_string(&mut fields, "id", path)?;
        let family_value = take_string(&mut fields, "family", path)?;
        let family = CatalogFamily::parse(&family_value).ok_or_else(|| {
            DecodeError::new(
                format!("{path}.family"),
                format!("unknown catalog family {family_value:?}"),
            )
        })?;
        let default_safety_tier = take_u64(&mut fields, "default_safety_tier", path)?;
        let default_safety_tier = u8::try_from(default_safety_tier).map_err(|_| {
            DecodeError::new(
                format!("{path}.default_safety_tier"),
                "value does not fit in u8",
            )
        })?;
        let prompt_directive_values = into_array(
            take_required(&mut fields, "prompt_directives", path)?,
            &format!("{path}.prompt_directives"),
        )?;
        let prompt_directives = prompt_directive_values
            .into_iter()
            .enumerate()
            .map(|(index, value)| {
                PromptDirective::from_json(value, &format!("{path}.prompt_directives[{index}]"))
            })
            .collect::<Result<Vec<_>, _>>()?;
        let text_requirement_value = take_string(&mut fields, "text_requirement", path)?;
        let text_requirement =
            TextRequirement::parse(&text_requirement_value).ok_or_else(|| {
                DecodeError::new(
                    format!("{path}.text_requirement"),
                    format!("unknown text requirement {text_requirement_value:?}"),
                )
            })?;
        let reference_text_mode_value = take_string(&mut fields, "reference_text_mode", path)?;
        let reference_text_mode =
            ReferenceTextMode::parse(&reference_text_mode_value).ok_or_else(|| {
                DecodeError::new(
                    format!("{path}.reference_text_mode"),
                    format!("unknown reference text mode {reference_text_mode_value:?}"),
                )
            })?;
        let result = Self {
            id,
            family,
            tier_1: take_string(&mut fields, "tier_1", path)?,
            tier_2: take_string(&mut fields, "tier_2", path)?,
            tier_3: take_optional_string(&mut fields, "tier_3", path)?,
            asset_path: take_string(&mut fields, "asset_path", path)?,
            name_ko: take_string(&mut fields, "name_ko", path)?,
            name_en: take_string(&mut fields, "name_en", path)?,
            intent: take_string(&mut fields, "intent", path)?,
            recipe: take_string_array(&mut fields, "recipe", path)?,
            prompt_directives,
            profiles: take_optional_string_array(&mut fields, "profiles", path)?
                .unwrap_or_default(),
            text_requirement,
            reference_text_mode,
            reference_exact_text: take_string_array(&mut fields, "reference_exact_text", path)?,
            repetition_text: take_bool(&mut fields, "repetition_text", path)?,
            suggested_aspect_ratios: take_string_array(
                &mut fields,
                "suggested_aspect_ratios",
                path,
            )?,
            suggested_palette: take_string_array(&mut fields, "suggested_palette", path)?,
            default_safety_tier,
            source_path: take_string(&mut fields, "source_path", path)?,
            source_sha256: take_string(&mut fields, "source_sha256", path)?,
        };
        reject_unknown(fields, path)?;
        Ok(result)
    }

    pub fn to_json(&self) -> JsonValue {
        JsonValue::object([
            ("asset_path", JsonValue::from(self.asset_path.clone())),
            (
                "default_safety_tier",
                JsonValue::from(u64::from(self.default_safety_tier)),
            ),
            ("family", JsonValue::from(self.family.as_str())),
            ("id", JsonValue::from(self.id.clone())),
            ("intent", JsonValue::from(self.intent.clone())),
            ("name_en", JsonValue::from(self.name_en.clone())),
            ("name_ko", JsonValue::from(self.name_ko.clone())),
            (
                "prompt_directives",
                JsonValue::array(self.prompt_directives.iter().map(PromptDirective::to_json)),
            ),
            ("profiles", JsonValue::strings(&self.profiles)),
            (
                "reference_exact_text",
                JsonValue::strings(&self.reference_exact_text),
            ),
            (
                "reference_text_mode",
                JsonValue::from(self.reference_text_mode.as_str()),
            ),
            ("source_recipe", JsonValue::strings(&self.recipe)),
            ("repetition_text", JsonValue::from(self.repetition_text)),
            (
                "suggested_aspect_ratios",
                JsonValue::strings(&self.suggested_aspect_ratios),
            ),
            (
                "suggested_palette",
                JsonValue::strings(&self.suggested_palette),
            ),
            ("source_path", JsonValue::from(self.source_path.clone())),
            ("source_sha256", JsonValue::from(self.source_sha256.clone())),
            ("tier_1", JsonValue::from(self.tier_1.clone())),
            ("tier_2", JsonValue::from(self.tier_2.clone())),
            (
                "tier_3",
                self.tier_3.clone().map_or(JsonValue::Null, JsonValue::from),
            ),
            (
                "text_requirement",
                JsonValue::from(self.text_requirement.as_str()),
            ),
        ])
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MethodSource {
    pub repository: String,
    pub artifact_sha256: String,
    pub license: String,
    pub usage: String,
}

impl MethodSource {
    pub(super) fn from_json(value: JsonValue, path: &str) -> Result<Self, DecodeError> {
        let mut fields = into_object(value, path)?;
        let result = Self {
            repository: take_string(&mut fields, "repository", path)?,
            artifact_sha256: take_string(&mut fields, "artifact_sha256", path)?,
            license: take_string(&mut fields, "license", path)?,
            usage: take_string(&mut fields, "usage", path)?,
        };
        reject_unknown(fields, path)?;
        Ok(result)
    }

    fn to_json(&self) -> JsonValue {
        JsonValue::object([
            (
                "artifact_sha256",
                JsonValue::from(self.artifact_sha256.clone()),
            ),
            ("license", JsonValue::from(self.license.clone())),
            ("repository", JsonValue::from(self.repository.clone())),
            ("usage", JsonValue::from(self.usage.clone())),
        ])
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CatalogMetadata {
    pub schema_version: u64,
    pub source_repository: String,
    pub source_commit: String,
    pub source_license: String,
    pub adaptation: String,
    pub family_counts: BTreeMap<String, u64>,
    pub method_sources: Vec<MethodSource>,
}

impl CatalogMetadata {
    pub fn to_json(&self) -> JsonValue {
        JsonValue::object([
            ("adaptation", JsonValue::from(self.adaptation.clone())),
            (
                "family_counts",
                JsonValue::Object(
                    self.family_counts
                        .iter()
                        .map(|(key, value)| (key.clone(), JsonValue::from(*value)))
                        .collect(),
                ),
            ),
            ("schema_version", JsonValue::from(self.schema_version)),
            ("source_commit", JsonValue::from(self.source_commit.clone())),
            (
                "source_license",
                JsonValue::from(self.source_license.clone()),
            ),
            (
                "source_repository",
                JsonValue::from(self.source_repository.clone()),
            ),
            (
                "method_sources",
                JsonValue::array(self.method_sources.iter().map(MethodSource::to_json)),
            ),
        ])
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct Catalog {
    pub metadata: CatalogMetadata,
    pub navigation: JsonValue,
    pub entries: Vec<CatalogEntry>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CatalogError(pub String);

impl fmt::Display for CatalogError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.0)
    }
}

impl std::error::Error for CatalogError {}
