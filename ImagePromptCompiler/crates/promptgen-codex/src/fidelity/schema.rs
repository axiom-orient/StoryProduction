use promptgen_core::json::JsonValue;

pub(crate) fn schema() -> JsonValue {
    let string = || JsonValue::object([("type", JsonValue::from("string"))]);
    let visible_instance = JsonValue::object([
        ("additionalProperties", JsonValue::from(false)),
        (
            "properties",
            JsonValue::object([
                ("evidence", string()),
                ("x_percent", percent_schema()),
                ("y_percent", percent_schema()),
            ]),
        ),
        (
            "required",
            JsonValue::strings(&["x_percent", "y_percent", "evidence"]),
        ),
        ("type", JsonValue::from("object")),
    ]);
    let subject_count = JsonValue::object([
        ("additionalProperties", JsonValue::from(false)),
        (
            "properties",
            JsonValue::object([
                ("evidence", string()),
                ("expected", integer_schema()),
                ("id", string()),
                (
                    "instances",
                    JsonValue::object([
                        ("items", visible_instance),
                        ("type", JsonValue::from("array")),
                    ]),
                ),
                ("observed", integer_schema()),
            ]),
        ),
        (
            "required",
            JsonValue::strings(&["id", "expected", "observed", "evidence", "instances"]),
        ),
        ("type", JsonValue::from("object")),
    ]);
    let text_check = JsonValue::object([
        ("additionalProperties", JsonValue::from(false)),
        (
            "properties",
            JsonValue::object([
                ("evidence", string()),
                (
                    "exact",
                    JsonValue::object([("type", JsonValue::from("boolean"))]),
                ),
                ("id", string()),
                ("observed_text", string()),
            ]),
        ),
        (
            "required",
            JsonValue::strings(&["id", "exact", "observed_text", "evidence"]),
        ),
        ("type", JsonValue::from("object")),
    ]);
    JsonValue::object([
        ("additionalProperties", JsonValue::from(false)),
        (
            "properties",
            JsonValue::object([
                (
                    "pass",
                    JsonValue::object([("type", JsonValue::from("boolean"))]),
                ),
                ("repair_instruction", string()),
                (
                    "subject_counts",
                    JsonValue::object([
                        ("items", subject_count),
                        ("type", JsonValue::from("array")),
                    ]),
                ),
                ("summary", string()),
                (
                    "text_checks",
                    JsonValue::object([("items", text_check), ("type", JsonValue::from("array"))]),
                ),
                (
                    "violations",
                    JsonValue::object([("items", string()), ("type", JsonValue::from("array"))]),
                ),
            ]),
        ),
        (
            "required",
            JsonValue::strings(&[
                "pass",
                "summary",
                "subject_counts",
                "text_checks",
                "violations",
                "repair_instruction",
            ]),
        ),
        ("type", JsonValue::from("object")),
    ])
}

pub(crate) fn count_schema() -> JsonValue {
    let string = || JsonValue::object([("type", JsonValue::from("string"))]);
    let instance = JsonValue::object([
        ("additionalProperties", JsonValue::from(false)),
        (
            "properties",
            JsonValue::object([
                ("evidence", string()),
                ("x_percent", percent_schema()),
                ("y_percent", percent_schema()),
            ]),
        ),
        (
            "required",
            JsonValue::strings(&["x_percent", "y_percent", "evidence"]),
        ),
        ("type", JsonValue::from("object")),
    ]);
    let count = JsonValue::object([
        ("additionalProperties", JsonValue::from(false)),
        (
            "properties",
            JsonValue::object([
                ("evidence", string()),
                ("expected", integer_schema()),
                ("id", string()),
                (
                    "instances",
                    JsonValue::object([("items", instance), ("type", JsonValue::from("array"))]),
                ),
                ("observed", integer_schema()),
            ]),
        ),
        (
            "required",
            JsonValue::strings(&["id", "expected", "observed", "evidence", "instances"]),
        ),
        ("type", JsonValue::from("object")),
    ]);
    JsonValue::object([
        ("additionalProperties", JsonValue::from(false)),
        (
            "properties",
            JsonValue::object([(
                "subject_counts",
                JsonValue::object([("items", count), ("type", JsonValue::from("array"))]),
            )]),
        ),
        ("required", JsonValue::strings(&["subject_counts"])),
        ("type", JsonValue::from("object")),
    ])
}

fn integer_schema() -> JsonValue {
    JsonValue::object([
        ("minimum", JsonValue::from(0_u64)),
        ("type", JsonValue::from("integer")),
    ])
}

fn percent_schema() -> JsonValue {
    JsonValue::object([
        ("maximum", JsonValue::from(100_u64)),
        ("minimum", JsonValue::from(0_u64)),
        ("type", JsonValue::from("integer")),
    ])
}
