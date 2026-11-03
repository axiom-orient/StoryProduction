use promptgen_core::image::ImagePromptRequest;
use promptgen_core::json::JsonValue;

use super::{
    CodexExecutionError, FidelityCheck, SubjectCountCheck, TextFidelityCheck, VisibleInstance,
};

mod instruction;
mod panels;
mod placement;
mod response;
mod schema;
mod text;

pub(super) use self::instruction::{build_count_instruction, build_validation_instruction};
pub(super) use self::panels::expected_instances;
pub(super) use self::placement::placement_failure_details;
use self::placement::{counts_are_well_formed, subject_instances_match_request};
use self::response::{
    invalid, required_bool, required_string, string_array, subject_counts, text_checks,
};
pub(super) use self::schema::{count_schema, schema};
use self::text::{text_failure_details, text_matches_request};

pub(super) const PLACEMENT_MEASUREMENT_TOLERANCE_PERCENT: u8 = 2;

pub(super) fn decode_counts(
    value: JsonValue,
    request: &ImagePromptRequest,
) -> Result<Vec<SubjectCountCheck>, CodexExecutionError> {
    let object = value
        .as_object()
        .ok_or_else(|| invalid("count result is not an object"))?;
    let counts = subject_counts(object)?;
    if !counts_are_well_formed(&counts, request) {
        return Err(invalid(
            "count result does not enumerate every typed subject exactly once",
        ));
    }
    Ok(counts)
}

pub(super) fn subject_instances_pass(
    checks: &[SubjectCountCheck],
    request: &ImagePromptRequest,
) -> bool {
    subject_instances_match_request(checks, request)
}

pub(super) fn decode_check(
    value: JsonValue,
    request: &ImagePromptRequest,
    candidate_sha256: String,
) -> Result<FidelityCheck, CodexExecutionError> {
    let object = value
        .as_object()
        .ok_or_else(|| invalid("result is not an object"))?;
    let reported_pass = required_bool(object, "pass")?;
    let mut summary = required_string(object, "summary")?.to_owned();
    let mut repair_instruction = required_string(object, "repair_instruction")?.to_owned();
    let mut violations = string_array(object, "violations")?;
    let subject_counts = subject_counts(object)?;
    let text_checks = text_checks(object)?;

    let subject_contract_pass = subject_instances_match_request(&subject_counts, request);
    let text_contract_pass = text_matches_request(&text_checks, request);
    if !text_contract_pass {
        let (violation, repair) = text_failure_details(&text_checks, request);
        if !violations.iter().any(|value| value == &violation) {
            violations.push(violation.clone());
        }
        if repair_instruction.trim().is_empty() {
            repair_instruction = repair;
        }
        if reported_pass {
            summary = violation;
        }
    }
    let pass = reported_pass
        && violations.is_empty()
        && repair_instruction.is_empty()
        && subject_contract_pass
        && text_contract_pass;

    Ok(FidelityCheck {
        candidate_sha256,
        pass,
        summary,
        violations,
        repair_instruction,
        subject_counts,
        text_checks,
    })
}

fn subject_counts_json(checks: &[SubjectCountCheck]) -> JsonValue {
    JsonValue::array(checks.iter().map(|check| {
        JsonValue::object([
            ("evidence", JsonValue::from(check.evidence.clone())),
            ("expected", JsonValue::from(u64::from(check.expected))),
            ("id", JsonValue::from(check.id.clone())),
            (
                "instances",
                JsonValue::array(check.instances.iter().map(|instance| {
                    JsonValue::object([
                        ("evidence", JsonValue::from(instance.evidence.clone())),
                        ("x_percent", JsonValue::from(u64::from(instance.x_percent))),
                        ("y_percent", JsonValue::from(u64::from(instance.y_percent))),
                    ])
                })),
            ),
            ("observed", JsonValue::from(u64::from(check.observed))),
        ])
    }))
}

#[cfg(test)]
mod tests {
    use promptgen_core::image::ImagePromptRequest;
    use promptgen_core::json::parse;

    use super::*;

    fn request() -> ImagePromptRequest {
        ImagePromptRequest::from_json(
            parse(include_str!(concat!(
                env!("CARGO_MANIFEST_DIR"),
                "/../../examples/image-photo-lut.json"
            )))
            .unwrap(),
        )
        .unwrap()
    }

    fn text_request() -> ImagePromptRequest {
        ImagePromptRequest::from_json(
            parse(include_str!(concat!(
                env!("CARGO_MANIFEST_DIR"),
                "/../../examples/image-typography-poster.json"
            )))
            .unwrap(),
        )
        .unwrap()
    }

    fn valid_subject_checks(request: &ImagePromptRequest) -> Vec<SubjectCountCheck> {
        request
            .subjects
            .iter()
            .map(|subject| {
                let instances = if subject.id == "coffee_cup" {
                    vec![VisibleInstance {
                        x_percent: 50,
                        y_percent: 60,
                        evidence: "cup".to_owned(),
                    }]
                } else {
                    (0..subject.count)
                        .map(|index| VisibleInstance {
                            x_percent: 30 + u8::try_from(index).unwrap() * 4,
                            y_percent: 80,
                            evidence: format!("instance {}", index + 1),
                        })
                        .collect()
                };
                SubjectCountCheck {
                    id: subject.id.clone(),
                    expected: subject.count,
                    observed: subject.count,
                    evidence: format!("{} visible instance(s)", subject.count),
                    instances,
                }
            })
            .collect()
    }

    #[test]
    fn cinematic_counts_and_placement_are_checked_in_each_panel() {
        let mut request = ImagePromptRequest::from_json(
            parse(include_str!(concat!(
                env!("CARGO_MANIFEST_DIR"),
                "/../../examples/image-cinematic-storyboard.json"
            )))
            .unwrap(),
        )
        .unwrap();
        for (layout, centers) in [
            ("three_panel_strip", vec![(16, 50), (50, 50), (83, 50)]),
            ("two_by_two", vec![(25, 25), (75, 25), (25, 75), (75, 75)]),
            (
                "three_by_two",
                vec![(16, 25), (50, 25), (83, 25), (16, 75), (50, 75), (83, 75)],
            ),
        ] {
            request.storyboard.as_mut().unwrap().layout = layout.to_owned();
            let total = centers.len() as u16;
            let check = SubjectCountCheck {
                id: request.subjects[0].id.clone(),
                expected: total,
                observed: total,
                evidence: "one appearance per panel".to_owned(),
                instances: centers
                    .iter()
                    .map(|(x, y)| VisibleInstance {
                        x_percent: *x,
                        y_percent: *y,
                        evidence: "visible in panel".to_owned(),
                    })
                    .collect(),
            };
            assert!(
                subject_instances_pass(std::slice::from_ref(&check), &request),
                "{layout}"
            );
            let mut wrong = check.clone();
            wrong.expected = 1;
            assert!(!subject_instances_pass(&[wrong], &request));
            let mut wrong = check.clone();
            wrong.instances[0].x_percent = 1;
            assert!(!subject_instances_pass(&[wrong], &request));
            let mut wrong = check.clone();
            wrong.instances[0].x_percent = wrong.instances[1].x_percent - 1;
            assert!(
                !subject_instances_pass(&[wrong], &request),
                "same total, missing panel"
            );
            let mut stacked = check.clone();
            stacked
                .instances
                .iter_mut()
                .for_each(|instance| instance.x_percent = 50);
            if layout == "three_panel_strip" {
                let (_, _, correction) = placement_failure_details(&[stacked], &request).unwrap();
                assert!(correction.contains("3 columns by 1 rows"));
                assert!(correction.contains("Do not retain a transposed or stacked layout"));
            }
            let mut wrong = check;
            wrong.observed -= 1;
            wrong.instances.pop();
            assert!(!subject_instances_pass(&[wrong], &request));
        }
    }

    #[test]
    fn count_mismatch_cannot_be_overridden_by_reported_pass() {
        let request = request();
        let subject = &request.subjects[0];
        let value = parse(&format!(
            r#"{{"pass":true,"summary":"looks fine","subject_counts":[{{"id":"{}","expected":{},"observed":{},"evidence":"one extra","instances":[]}}],"text_checks":[],"violations":[],"repair_instruction":""}}"#,
            subject.id,
            subject.count,
            subject.count + 1
        ))
        .unwrap();
        let check = decode_check(value, &request, "sha".to_owned()).unwrap();
        assert!(!check.pass);
    }

    #[test]
    fn custom_placement_rejects_an_outside_instance_center() {
        let request = request();
        let mut checks = valid_subject_checks(&request);
        assert!(subject_instances_pass(&checks, &request));
        checks[1].instances[0].y_percent = 65;
        assert!(!subject_instances_pass(&checks, &request));
        let (summary, violation, repair) =
            placement_failure_details(&checks, &request).expect("placement failure");
        assert_eq!(summary, violation);
        assert!(summary.contains("(30, 65)"));
        assert!(repair.contains("custom_box(x=24%, y=70%, width=52%, height=20%)"));
        assert!(repair.contains(
            "(37, 75), (50, 75), (63, 75), (37, 80), (50, 80), (63, 80), (37, 85), (50, 85), (63, 85)"
        ));
        assert!(repair.contains("strict center bounds x=24..76% and y=70..90%"));
        assert!(repair.contains("Do not add or remove any instance"));
    }

    #[test]
    fn reported_pass_cannot_override_typed_placement() {
        let request = request();
        let mut checks = valid_subject_checks(&request);
        checks[1].instances[0].y_percent = 65;
        let value = JsonValue::object([
            ("pass", JsonValue::from(true)),
            ("repair_instruction", JsonValue::from("")),
            ("subject_counts", subject_counts_json(&checks)),
            ("summary", JsonValue::from("looks fine")),
            ("text_checks", JsonValue::array(std::iter::empty())),
            ("violations", JsonValue::array(std::iter::empty())),
        ]);
        let check = decode_check(value, &request, "sha".to_owned()).unwrap();
        assert!(!check.pass);
    }

    #[test]
    fn model_exact_flag_cannot_override_wrong_observed_copy() {
        let request = text_request();
        let checks = vec![
            TextFidelityCheck {
                id: "headline".to_owned(),
                exact: true,
                observed_text: "URBAN\nS1GNAL".to_owned(),
                evidence: "headline pixels".to_owned(),
            },
            TextFidelityCheck {
                id: "date".to_owned(),
                exact: true,
                observed_text: "2026.09.18".to_owned(),
                evidence: "date pixels".to_owned(),
            },
        ];

        assert!(!text_matches_request(&checks, &request));
    }

    #[test]
    fn exact_copy_comparison_preserves_requested_line_breaks() {
        let request = text_request();
        let correct = vec![
            TextFidelityCheck {
                id: "headline".to_owned(),
                exact: true,
                observed_text: "URBAN\nSIGNAL".to_owned(),
                evidence: "headline pixels".to_owned(),
            },
            TextFidelityCheck {
                id: "date".to_owned(),
                exact: true,
                observed_text: "2026.09.18".to_owned(),
                evidence: "date pixels".to_owned(),
            },
        ];
        let mut wrong_break = correct.clone();
        wrong_break[0].observed_text = "URBAN SIGNAL".to_owned();

        assert!(text_matches_request(&correct, &request));
        assert!(!text_matches_request(&wrong_break, &request));
    }

    #[test]
    fn local_copy_mismatch_adds_a_repairable_failure() {
        let request = text_request();
        let counts = vec![SubjectCountCheck {
            id: "signal_disc".to_owned(),
            expected: 1,
            observed: 1,
            evidence: "one disc".to_owned(),
            instances: vec![VisibleInstance {
                x_percent: 70,
                y_percent: 25,
                evidence: "disc center".to_owned(),
            }],
        }];
        let value = JsonValue::object([
            ("pass", JsonValue::from(true)),
            ("repair_instruction", JsonValue::from("")),
            ("subject_counts", subject_counts_json(&counts)),
            ("summary", JsonValue::from("looks fine")),
            (
                "text_checks",
                JsonValue::array([
                    JsonValue::object([
                        ("evidence", JsonValue::from("headline pixels")),
                        ("exact", JsonValue::from(true)),
                        ("id", JsonValue::from("headline")),
                        ("observed_text", JsonValue::from("URBAN\nS1GNAL")),
                    ]),
                    JsonValue::object([
                        ("evidence", JsonValue::from("date pixels")),
                        ("exact", JsonValue::from(true)),
                        ("id", JsonValue::from("date")),
                        ("observed_text", JsonValue::from("2026.09.18")),
                    ]),
                ]),
            ),
            ("violations", JsonValue::array(std::iter::empty())),
        ]);

        let check = decode_check(value, &request, "sha".to_owned()).unwrap();
        assert!(!check.pass);
        assert!(
            check
                .violations
                .iter()
                .any(|value| value.contains("headline"))
        );
        assert!(check.repair_instruction.contains("URBAN\\nSIGNAL"));
    }
}
