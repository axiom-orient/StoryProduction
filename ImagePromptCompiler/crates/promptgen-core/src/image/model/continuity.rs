use crate::json::{
    DecodeError, JsonValue, into_object, reject_unknown, take_bool, take_optional,
    take_string_array,
};

/// Cross-image invariants distilled from the multi-image generation research
/// taxonomy. The enum is deliberately provider-neutral: it describes what
/// must remain stable, not which model or paper is used to achieve it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ImageConsistencyDimension {
    MultiView,
    Character,
    Temporal,
    Semantic,
}

impl ImageConsistencyDimension {
    fn parse(value: &str, path: &str) -> Result<Self, DecodeError> {
        match value {
            "multi_view" => Ok(Self::MultiView),
            "character" => Ok(Self::Character),
            "temporal" => Ok(Self::Temporal),
            "semantic" => Ok(Self::Semantic),
            _ => Err(DecodeError::new(
                path,
                "expected multi_view, character, temporal, or semantic",
            )),
        }
    }

    pub const fn as_str(self) -> &'static str {
        match self {
            Self::MultiView => "multi_view",
            Self::Character => "character",
            Self::Temporal => "temporal",
            Self::Semantic => "semantic",
        }
    }
}

/// An ordered handoff between images or storyboard panels. The
/// condition_on_previous flag is an explicit execution requirement; adapters
/// that cannot bind the prior output must fail closed instead of silently
/// treating a temporal request as independent generations.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ImageConsistencySequence {
    pub steps: Vec<String>,
    pub condition_on_previous: bool,
}

impl ImageConsistencySequence {
    pub(super) fn from_json(value: JsonValue, path: &str) -> Result<Self, DecodeError> {
        let mut fields = into_object(value, path)?;
        let result = Self {
            steps: take_string_array(&mut fields, "steps", path)?,
            condition_on_previous: take_bool(&mut fields, "condition_on_previous", path)?,
        };
        reject_unknown(fields, path)?;
        Ok(result)
    }

    pub(super) fn to_json(&self) -> JsonValue {
        JsonValue::object([
            (
                "condition_on_previous",
                JsonValue::from(self.condition_on_previous),
            ),
            ("steps", JsonValue::strings(&self.steps)),
        ])
    }
}

/// Optional cross-image consistency contract shared by structured image
/// requests and the cinematic storyboard profile.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ImageConsistencySpec {
    pub dimensions: Vec<ImageConsistencyDimension>,
    pub shared_anchors: Vec<String>,
    pub sequence: Option<ImageConsistencySequence>,
}

impl ImageConsistencySpec {
    pub(super) fn from_json(value: JsonValue, path: &str) -> Result<Self, DecodeError> {
        let mut fields = into_object(value, path)?;
        let dimensions = take_string_array(&mut fields, "dimensions", path)?
            .into_iter()
            .enumerate()
            .map(|(index, value)| {
                ImageConsistencyDimension::parse(&value, &format!("{path}.dimensions[{index}]"))
            })
            .collect::<Result<Vec<_>, _>>()?;
        let result = Self {
            dimensions,
            shared_anchors: take_string_array(&mut fields, "shared_anchors", path)?,
            sequence: take_optional(&mut fields, "sequence")
                .map(|value| {
                    ImageConsistencySequence::from_json(value, &format!("{path}.sequence"))
                })
                .transpose()?,
        };
        reject_unknown(fields, path)?;
        Ok(result)
    }

    pub(super) fn to_json(&self) -> JsonValue {
        JsonValue::object([
            (
                "dimensions",
                JsonValue::array(
                    self.dimensions
                        .iter()
                        .map(|dimension| JsonValue::from(dimension.as_str())),
                ),
            ),
            ("shared_anchors", JsonValue::strings(&self.shared_anchors)),
            (
                "sequence",
                self.sequence
                    .as_ref()
                    .map_or(JsonValue::Null, ImageConsistencySequence::to_json),
            ),
        ])
    }
}
