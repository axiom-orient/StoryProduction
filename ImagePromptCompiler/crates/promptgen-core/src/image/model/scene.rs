use super::super::common::take_object_array;
use super::take_u8;
use crate::json::{
    DecodeError, JsonValue, expect_string, expect_string_array, into_object, reject_unknown,
    take_optional, take_optional_string, take_optional_string_array, take_required, take_string,
    take_string_array, take_u64,
};

fn optional_fields<const N: usize>(
    mut base: Vec<(&'static str, JsonValue)>,
    optional: [(&'static str, Option<&String>); N],
) -> JsonValue {
    for (key, value) in optional {
        if let Some(value) = value {
            base.push((key, JsonValue::from(value.clone())));
        }
    }
    JsonValue::object(base)
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum VisualMedium {
    Photo,
    Illustration,
    ThreeD,
    GraphicDesign,
    Mixed,
}

impl VisualMedium {
    pub(super) fn parse(value: &str, path: &str) -> Result<Self, DecodeError> {
        match value {
            "photo" => Ok(Self::Photo),
            "illustration" => Ok(Self::Illustration),
            "3d" => Ok(Self::ThreeD),
            "graphic_design" | "graphic-design" => Ok(Self::GraphicDesign),
            "mixed" => Ok(Self::Mixed),
            _ => Err(DecodeError::new(
                path,
                "expected one of: photo, illustration, 3d, graphic_design, mixed",
            )),
        }
    }

    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Photo => "photo",
            Self::Illustration => "illustration",
            Self::ThreeD => "3d",
            Self::GraphicDesign => "graphic_design",
            Self::Mixed => "mixed",
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TaxonomySelection {
    pub category: Option<String>,
    pub patterns: Vec<String>,
}

impl TaxonomySelection {
    pub(super) fn from_json(value: JsonValue, path: &str) -> Result<Self, DecodeError> {
        let mut fields = into_object(value, path)?;
        let category = match take_optional(&mut fields, "category") {
            None | Some(JsonValue::Null) => None,
            Some(value) => Some(expect_string(value, &format!("{path}.category"))?),
        };
        let patterns = match take_optional(&mut fields, "patterns") {
            None => Vec::new(),
            Some(value) => expect_string_array(value, &format!("{path}.patterns"))?,
        };
        reject_unknown(fields, path)?;
        Ok(Self { category, patterns })
    }

    pub fn all_ids(&self) -> Vec<String> {
        self.category
            .iter()
            .cloned()
            .chain(self.patterns.iter().cloned())
            .collect()
    }

    pub(super) fn to_json(&self) -> JsonValue {
        JsonValue::object([
            (
                "category",
                self.category
                    .as_ref()
                    .map_or(JsonValue::Null, |value| JsonValue::from(value.clone())),
            ),
            ("patterns", JsonValue::strings(&self.patterns)),
        ])
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CanvasZone {
    TopLeft,
    TopCenter,
    TopRight,
    MiddleLeft,
    Center,
    MiddleRight,
    BottomLeft,
    BottomCenter,
    BottomRight,
}

impl CanvasZone {
    pub(super) fn parse(value: &str, path: &str) -> Result<Self, DecodeError> {
        match value {
            "top_left" => Ok(Self::TopLeft),
            "top_center" => Ok(Self::TopCenter),
            "top_right" => Ok(Self::TopRight),
            "middle_left" => Ok(Self::MiddleLeft),
            "center" | "dead_center" => Ok(Self::Center),
            "middle_right" => Ok(Self::MiddleRight),
            "bottom_left" => Ok(Self::BottomLeft),
            "bottom_center" => Ok(Self::BottomCenter),
            "bottom_right" => Ok(Self::BottomRight),
            _ => Err(DecodeError::new(
                path,
                "unknown canvas zone; use top_left/top_center/top_right/middle_left/center/middle_right/bottom_left/bottom_center/bottom_right/custom",
            )),
        }
    }

    pub const fn as_str(self) -> &'static str {
        match self {
            Self::TopLeft => "top_left",
            Self::TopCenter => "top_center",
            Self::TopRight => "top_right",
            Self::MiddleLeft => "middle_left",
            Self::Center => "center",
            Self::MiddleRight => "middle_right",
            Self::BottomLeft => "bottom_left",
            Self::BottomCenter => "bottom_center",
            Self::BottomRight => "bottom_right",
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum CanvasPlacement {
    Zone(CanvasZone),
    Custom {
        x_percent: u8,
        y_percent: u8,
        width_percent: u8,
        height_percent: u8,
    },
}

impl CanvasPlacement {
    pub(super) fn from_json(value: JsonValue, path: &str) -> Result<Self, DecodeError> {
        let mut fields = into_object(value, path)?;
        let zone = take_string(&mut fields, "zone", path)?;
        if zone == "custom" {
            let x = take_u8(&mut fields, "x_percent", path)?;
            let y = take_u8(&mut fields, "y_percent", path)?;
            let width = take_u8(&mut fields, "width_percent", path)?;
            let height = take_u8(&mut fields, "height_percent", path)?;
            reject_unknown(fields, path)?;
            Ok(Self::Custom {
                x_percent: x,
                y_percent: y,
                width_percent: width,
                height_percent: height,
            })
        } else {
            let zone = CanvasZone::parse(&zone, &format!("{path}.zone"))?;
            reject_unknown(fields, path)?;
            Ok(Self::Zone(zone))
        }
    }

    pub fn to_json(&self) -> JsonValue {
        match self {
            Self::Zone(zone) => JsonValue::object([("zone", JsonValue::from(zone.as_str()))]),
            Self::Custom {
                x_percent,
                y_percent,
                width_percent,
                height_percent,
            } => JsonValue::object([
                (
                    "height_percent",
                    JsonValue::from(u64::from(*height_percent)),
                ),
                ("width_percent", JsonValue::from(u64::from(*width_percent))),
                ("x_percent", JsonValue::from(u64::from(*x_percent))),
                ("y_percent", JsonValue::from(u64::from(*y_percent))),
                ("zone", JsonValue::from("custom")),
            ]),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SceneSpec {
    pub environment: String,
    pub background: String,
    pub atmosphere: String,
    pub time_of_day: Option<String>,
    pub weather: Option<String>,
}

impl SceneSpec {
    pub(super) fn from_json(value: JsonValue, path: &str) -> Result<Self, DecodeError> {
        let mut fields = into_object(value, path)?;
        let result = Self {
            environment: take_string(&mut fields, "environment", path)?,
            background: take_string(&mut fields, "background", path)?,
            atmosphere: take_string(&mut fields, "atmosphere", path)?,
            time_of_day: take_optional_string(&mut fields, "time_of_day", path)?,
            weather: take_optional_string(&mut fields, "weather", path)?,
        };
        reject_unknown(fields, path)?;
        Ok(result)
    }

    pub(super) fn to_json(&self) -> JsonValue {
        optional_fields(
            vec![
                ("atmosphere", JsonValue::from(self.atmosphere.clone())),
                ("background", JsonValue::from(self.background.clone())),
                ("environment", JsonValue::from(self.environment.clone())),
            ],
            [
                ("time_of_day", self.time_of_day.as_ref()),
                ("weather", self.weather.as_ref()),
            ],
        )
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SubjectSpec {
    pub id: String,
    pub count: u16,
    pub description: String,
    pub placement: CanvasPlacement,
    pub scale: String,
    pub pose: String,
    pub gaze: String,
    pub action: String,
    pub face: Option<String>,
    pub hair: Option<String>,
    pub appearance: Option<String>,
    pub distinguishing_features: Vec<String>,
}

impl SubjectSpec {
    pub(super) fn from_json(value: JsonValue, path: &str) -> Result<Self, DecodeError> {
        let mut fields = into_object(value, path)?;
        let count = take_u64(&mut fields, "count", path)?;
        let count = u16::try_from(count)
            .map_err(|_| DecodeError::new(format!("{path}.count"), "value exceeds u16"))?;
        let result = Self {
            id: take_string(&mut fields, "id", path)?,
            count,
            description: take_string(&mut fields, "description", path)?,
            placement: CanvasPlacement::from_json(
                take_required(&mut fields, "placement", path)?,
                &format!("{path}.placement"),
            )?,
            scale: take_string(&mut fields, "scale", path)?,
            pose: take_string(&mut fields, "pose", path)?,
            gaze: take_string(&mut fields, "gaze", path)?,
            action: take_string(&mut fields, "action", path)?,
            face: take_optional_string(&mut fields, "face", path)?,
            hair: take_optional_string(&mut fields, "hair", path)?,
            appearance: take_optional_string(&mut fields, "appearance", path)?,
            distinguishing_features: take_optional_string_array(
                &mut fields,
                "distinguishing_features",
                path,
            )?
            .unwrap_or_default(),
        };
        reject_unknown(fields, path)?;
        Ok(result)
    }

    pub(super) fn to_json(&self) -> JsonValue {
        let mut values = vec![
            ("action", JsonValue::from(self.action.clone())),
            ("count", JsonValue::from(u64::from(self.count))),
            ("description", JsonValue::from(self.description.clone())),
            (
                "distinguishing_features",
                JsonValue::strings(&self.distinguishing_features),
            ),
            ("gaze", JsonValue::from(self.gaze.clone())),
            ("id", JsonValue::from(self.id.clone())),
            ("placement", self.placement.to_json()),
            ("pose", JsonValue::from(self.pose.clone())),
            ("scale", JsonValue::from(self.scale.clone())),
        ];
        if let Some(value) = &self.face {
            values.push(("face", JsonValue::from(value.clone())));
        }
        if let Some(value) = &self.hair {
            values.push(("hair", JsonValue::from(value.clone())));
        }
        if let Some(value) = &self.appearance {
            values.push(("appearance", JsonValue::from(value.clone())));
        }
        JsonValue::object(values)
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CompositionSpec {
    pub framing: String,
    pub viewpoint: String,
    pub camera_angle: String,
    pub balance: String,
    pub visual_hierarchy: Vec<String>,
    pub depth_layers: Vec<String>,
    pub negative_space: Vec<CanvasZone>,
}

impl CompositionSpec {
    pub(super) fn from_json(value: JsonValue, path: &str) -> Result<Self, DecodeError> {
        let mut fields = into_object(value, path)?;
        let negative_space = take_optional(&mut fields, "negative_space")
            .map(|value| {
                let values = match value {
                    JsonValue::Array(values) => Ok(values),
                    _ => Err(DecodeError::new(
                        format!("{path}.negative_space"),
                        "expected array",
                    )),
                }?;
                values
                    .into_iter()
                    .enumerate()
                    .map(|(index, value)| {
                        let path = format!("{path}.negative_space[{index}]");
                        let value = expect_string(value, &path)?;
                        CanvasZone::parse(&value, &path)
                    })
                    .collect()
            })
            .transpose()?
            .unwrap_or_default();
        let result = Self {
            framing: take_string(&mut fields, "framing", path)?,
            viewpoint: take_string(&mut fields, "viewpoint", path)?,
            camera_angle: take_string(&mut fields, "camera_angle", path)?,
            balance: take_string(&mut fields, "balance", path)?,
            visual_hierarchy: take_string_array(&mut fields, "visual_hierarchy", path)?,
            depth_layers: take_string_array(&mut fields, "depth_layers", path)?,
            negative_space,
        };
        reject_unknown(fields, path)?;
        Ok(result)
    }

    pub(super) fn to_json(&self) -> JsonValue {
        JsonValue::object([
            ("balance", JsonValue::from(self.balance.clone())),
            ("camera_angle", JsonValue::from(self.camera_angle.clone())),
            ("depth_layers", JsonValue::strings(&self.depth_layers)),
            ("framing", JsonValue::from(self.framing.clone())),
            (
                "negative_space",
                JsonValue::array(
                    self.negative_space
                        .iter()
                        .map(|zone| JsonValue::from(zone.as_str())),
                ),
            ),
            ("viewpoint", JsonValue::from(self.viewpoint.clone())),
            (
                "visual_hierarchy",
                JsonValue::strings(&self.visual_hierarchy),
            ),
        ])
    }
}

/// A textless, designer-facing visual sequence. This is intentionally distinct
/// from a generic multi-panel illustration: every panel has an explicit camera
/// and eyeline responsibility, while identity and screen direction are shared.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CinematicStoryboardPanel {
    pub id: String,
    pub shot_size: String,
    pub camera_angle: String,
    pub camera_move: String,
    pub action: String,
    pub body_facing: String,
    pub gaze_target: String,
    pub emotional_beat: String,
}

impl CinematicStoryboardPanel {
    pub(super) fn from_json(value: JsonValue, path: &str) -> Result<Self, DecodeError> {
        let mut fields = into_object(value, path)?;
        let result = Self {
            id: take_string(&mut fields, "id", path)?,
            shot_size: take_string(&mut fields, "shot_size", path)?,
            camera_angle: take_string(&mut fields, "camera_angle", path)?,
            camera_move: take_string(&mut fields, "camera_move", path)?,
            action: take_string(&mut fields, "action", path)?,
            body_facing: take_string(&mut fields, "body_facing", path)?,
            gaze_target: take_string(&mut fields, "gaze_target", path)?,
            emotional_beat: take_string(&mut fields, "emotional_beat", path)?,
        };
        reject_unknown(fields, path)?;
        Ok(result)
    }

    pub(super) fn to_json(&self) -> JsonValue {
        JsonValue::object([
            ("action", JsonValue::from(self.action.clone())),
            ("body_facing", JsonValue::from(self.body_facing.clone())),
            ("camera_angle", JsonValue::from(self.camera_angle.clone())),
            ("camera_move", JsonValue::from(self.camera_move.clone())),
            (
                "emotional_beat",
                JsonValue::from(self.emotional_beat.clone()),
            ),
            ("gaze_target", JsonValue::from(self.gaze_target.clone())),
            ("id", JsonValue::from(self.id.clone())),
            ("shot_size", JsonValue::from(self.shot_size.clone())),
        ])
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CinematicStoryboardSpec {
    pub layout: String,
    pub screen_direction: String,
    pub identity_anchors: Vec<String>,
    pub panels: Vec<CinematicStoryboardPanel>,
}

impl CinematicStoryboardSpec {
    pub(super) fn from_json(value: JsonValue, path: &str) -> Result<Self, DecodeError> {
        let mut fields = into_object(value, path)?;
        let panels = take_object_array(
            &mut fields,
            "panels",
            path,
            CinematicStoryboardPanel::from_json,
        )?;
        let result = Self {
            layout: take_string(&mut fields, "layout", path)?,
            screen_direction: take_string(&mut fields, "screen_direction", path)?,
            identity_anchors: take_string_array(&mut fields, "identity_anchors", path)?,
            panels,
        };
        reject_unknown(fields, path)?;
        Ok(result)
    }

    pub(super) fn to_json(&self) -> JsonValue {
        JsonValue::object([
            (
                "identity_anchors",
                JsonValue::strings(&self.identity_anchors),
            ),
            ("layout", JsonValue::from(self.layout.clone())),
            (
                "panels",
                JsonValue::array(self.panels.iter().map(CinematicStoryboardPanel::to_json)),
            ),
            (
                "screen_direction",
                JsonValue::from(self.screen_direction.clone()),
            ),
        ])
    }
}
