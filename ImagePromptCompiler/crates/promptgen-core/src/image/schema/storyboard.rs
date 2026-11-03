use super::contracts::{enum_schema, nullable, object_schema, string_schema};
use crate::json::JsonValue;

pub(super) fn consistency_schema() -> JsonValue {
    object_schema(
        &["dimensions", "shared_anchors"],
        [
            (
                "dimensions",
                JsonValue::object([
                    ("type", JsonValue::from("array")),
                    ("minItems", JsonValue::from(1_u64)),
                    ("maxItems", JsonValue::from(4_u64)),
                    (
                        "items",
                        enum_schema(&["multi_view", "character", "temporal", "semantic"]),
                    ),
                ]),
            ),
            (
                "shared_anchors",
                JsonValue::object([
                    ("type", JsonValue::from("array")),
                    ("minItems", JsonValue::from(1_u64)),
                    ("maxItems", JsonValue::from(8_u64)),
                    ("items", string_schema()),
                ]),
            ),
            ("sequence", nullable(consistency_sequence_schema())),
        ],
    )
}

pub(super) fn consistency_sequence_schema() -> JsonValue {
    object_schema(
        &["steps", "condition_on_previous"],
        [
            (
                "steps",
                JsonValue::object([
                    ("type", JsonValue::from("array")),
                    ("minItems", JsonValue::from(2_u64)),
                    ("maxItems", JsonValue::from(24_u64)),
                    ("items", string_schema()),
                ]),
            ),
            (
                "condition_on_previous",
                JsonValue::object([("type", JsonValue::from("boolean"))]),
            ),
        ],
    )
}

pub(super) fn storyboard_schema() -> JsonValue {
    object_schema(
        &["layout", "screen_direction", "identity_anchors", "panels"],
        [
            (
                "layout",
                enum_schema(&["three_panel_strip", "two_by_two", "three_by_two"]),
            ),
            (
                "screen_direction",
                enum_schema(&["left_to_right", "right_to_left"]),
            ),
            (
                "identity_anchors",
                JsonValue::object([
                    ("type", JsonValue::from("array")),
                    ("minItems", JsonValue::from(3_u64)),
                    ("maxItems", JsonValue::from(6_u64)),
                    ("items", string_schema()),
                ]),
            ),
            (
                "panels",
                JsonValue::object([
                    ("type", JsonValue::from("array")),
                    ("minItems", JsonValue::from(3_u64)),
                    ("maxItems", JsonValue::from(6_u64)),
                    ("items", storyboard_panel_schema()),
                ]),
            ),
        ],
    )
}

pub(super) fn storyboard_panel_schema() -> JsonValue {
    object_schema(
        &[
            "id",
            "shot_size",
            "camera_angle",
            "camera_move",
            "action",
            "body_facing",
            "gaze_target",
            "emotional_beat",
        ],
        [
            ("id", string_schema()),
            (
                "shot_size",
                enum_schema(&[
                    "establishing",
                    "wide",
                    "medium",
                    "medium_close_up",
                    "close_up",
                    "extreme_close_up",
                    "over_the_shoulder",
                    "insert",
                ]),
            ),
            ("camera_angle", string_schema()),
            ("camera_move", string_schema()),
            ("action", string_schema()),
            (
                "body_facing",
                enum_schema(&[
                    "screen_left",
                    "screen_right",
                    "toward_camera",
                    "away_from_camera",
                    "three_quarter_left",
                    "three_quarter_right",
                ]),
            ),
            (
                "gaze_target",
                enum_schema(&[
                    "screen_left",
                    "screen_right",
                    "toward_camera",
                    "up",
                    "down",
                    "offscreen_left",
                    "offscreen_right",
                    "object_in_frame",
                ]),
            ),
            ("emotional_beat", string_schema()),
        ],
    )
}
