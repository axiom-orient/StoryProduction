use crate::image::lut::resolve_photo_lut;
use crate::image::model::ImagePromptRequest;

pub(super) fn collect_rendered_free_text(request: &ImagePromptRequest) -> Vec<(String, String)> {
    let mut fields = Vec::new();
    let mut push = |path: String, value: &str| fields.push((path, value.to_owned()));

    push("$.use_case".to_owned(), &request.use_case);
    push("$.scene.environment".to_owned(), &request.scene.environment);
    push("$.scene.background".to_owned(), &request.scene.background);
    push("$.scene.atmosphere".to_owned(), &request.scene.atmosphere);
    if let Some(value) = &request.scene.time_of_day {
        push("$.scene.time_of_day".to_owned(), value);
    }
    if let Some(value) = &request.scene.weather {
        push("$.scene.weather".to_owned(), value);
    }

    for (index, reference) in request.references.iter().enumerate() {
        let base = format!("$.references[{index}]");
        push(format!("{base}.description"), &reference.description);
        for (use_for_index, value) in reference.use_for.iter().enumerate() {
            push(format!("{base}.use_for[{use_for_index}]"), value);
        }
    }
    for (index, value) in request.change_contract.change_only.iter().enumerate() {
        push(format!("$.change_contract.change_only[{index}]"), value);
    }
    for (index, value) in request.change_contract.preserve.iter().enumerate() {
        push(format!("$.change_contract.preserve[{index}]"), value);
    }

    for (index, subject) in request.subjects.iter().enumerate() {
        let base = format!("$.subjects[{index}]");
        push(format!("{base}.id"), &subject.id);
        push(format!("{base}.description"), &subject.description);
        push(format!("{base}.scale"), &subject.scale);
        push(format!("{base}.pose"), &subject.pose);
        push(format!("{base}.gaze"), &subject.gaze);
        push(format!("{base}.action"), &subject.action);
        if let Some(value) = &subject.face {
            push(format!("{base}.face"), value);
        }
        if let Some(value) = &subject.hair {
            push(format!("{base}.hair"), value);
        }
        if let Some(value) = &subject.appearance {
            push(format!("{base}.appearance"), value);
        }
        for (feature_index, value) in subject.distinguishing_features.iter().enumerate() {
            push(
                format!("{base}.distinguishing_features[{feature_index}]"),
                value,
            );
        }
    }

    push(
        "$.composition.framing".to_owned(),
        &request.composition.framing,
    );
    push(
        "$.composition.viewpoint".to_owned(),
        &request.composition.viewpoint,
    );
    push(
        "$.composition.camera_angle".to_owned(),
        &request.composition.camera_angle,
    );
    push(
        "$.composition.balance".to_owned(),
        &request.composition.balance,
    );
    for (index, value) in request.composition.visual_hierarchy.iter().enumerate() {
        push(format!("$.composition.visual_hierarchy[{index}]"), value);
    }
    for (index, value) in request.composition.depth_layers.iter().enumerate() {
        push(format!("$.composition.depth_layers[{index}]"), value);
    }

    push(
        "$.lighting.key_direction".to_owned(),
        &request.lighting.key_direction,
    );
    push(
        "$.lighting.key_quality".to_owned(),
        &request.lighting.key_quality,
    );
    if let Some(value) = &request.lighting.key_color_hex {
        push("$.lighting.key_color_hex".to_owned(), value);
    }
    push(
        "$.lighting.fill_description".to_owned(),
        &request.lighting.fill_description,
    );
    if let Some(value) = &request.lighting.rim_description {
        push("$.lighting.rim_description".to_owned(), value);
    }
    push(
        "$.lighting.shadow_character".to_owned(),
        &request.lighting.shadow_character,
    );
    push("$.lighting.exposure".to_owned(), &request.lighting.exposure);

    for (index, entry) in request.color.palette.iter().enumerate() {
        push(format!("$.color.palette[{index}].hex"), &entry.hex);
        push(format!("$.color.palette[{index}].usage"), &entry.usage);
    }
    push("$.color.harmony".to_owned(), &request.color.harmony);
    push("$.color.contrast".to_owned(), &request.color.contrast);
    push("$.color.saturation".to_owned(), &request.color.saturation);
    if let Some(selection) = &request.color.photo_lut
        && let Some(lut) = resolve_photo_lut(selection)
    {
        push("$.color.photo_lut.preset".to_owned(), &lut.preset);
        push("$.color.photo_lut.tint".to_owned(), &lut.tint);
        push("$.color.photo_lut.tone_curve".to_owned(), &lut.tone_curve);
        push(
            "$.color.photo_lut.black_response".to_owned(),
            &lut.black_response,
        );
        push("$.color.photo_lut.contrast".to_owned(), &lut.contrast);
        push(
            "$.color.photo_lut.shadow_bias_hex".to_owned(),
            &lut.shadow_bias_hex,
        );
        push(
            "$.color.photo_lut.highlight_bias_hex".to_owned(),
            &lut.highlight_bias_hex,
        );
        push(
            "$.color.photo_lut.highlight_rolloff".to_owned(),
            &lut.highlight_rolloff,
        );
        push(
            "$.color.photo_lut.skin_tone_policy".to_owned(),
            &lut.skin_tone_policy,
        );
        push("$.color.photo_lut.grain_size".to_owned(), &lut.grain_size);
    }

    for (index, surface) in request.surfaces.iter().enumerate() {
        let base = format!("$.surfaces[{index}]");
        push(format!("{base}.id"), &surface.id);
        push(format!("{base}.material"), &surface.material);
        push(format!("{base}.finish"), &surface.finish);
        push(format!("{base}.micro_detail"), &surface.micro_detail);
        push(format!("{base}.light_response"), &surface.light_response);
    }

    for (index, value) in request.constraints.required_elements.iter().enumerate() {
        push(format!("$.constraints.required_elements[{index}]"), value);
    }

    if let Some(camera) = &request.camera {
        push("$.camera.field_of_view".to_owned(), &camera.field_of_view);
        push("$.camera.perspective".to_owned(), &camera.perspective);
        push("$.camera.depth_of_field".to_owned(), &camera.depth_of_field);
        push("$.camera.focus".to_owned(), &camera.focus);
        push(
            "$.camera.motion_rendering".to_owned(),
            &camera.motion_rendering,
        );
    }

    if let Some(consistency) = &request.consistency {
        for (index, dimension) in consistency.dimensions.iter().enumerate() {
            push(
                format!("$.consistency.dimensions[{index}]"),
                dimension.as_str(),
            );
        }
        for (index, value) in consistency.shared_anchors.iter().enumerate() {
            push(format!("$.consistency.shared_anchors[{index}]"), value);
        }
        if let Some(sequence) = &consistency.sequence {
            for (index, value) in sequence.steps.iter().enumerate() {
                push(format!("$.consistency.sequence.steps[{index}]"), value);
            }
        }
    }

    if let Some(storyboard) = &request.storyboard {
        push("$.storyboard.layout".to_owned(), &storyboard.layout);
        push(
            "$.storyboard.screen_direction".to_owned(),
            &storyboard.screen_direction,
        );
        for (index, value) in storyboard.identity_anchors.iter().enumerate() {
            push(format!("$.storyboard.identity_anchors[{index}]"), value);
        }
        for (index, panel) in storyboard.panels.iter().enumerate() {
            let base = format!("$.storyboard.panels[{index}]");
            push(format!("{base}.id"), &panel.id);
            push(format!("{base}.shot_size"), &panel.shot_size);
            push(format!("{base}.camera_angle"), &panel.camera_angle);
            push(format!("{base}.camera_move"), &panel.camera_move);
            push(format!("{base}.action"), &panel.action);
            push(format!("{base}.body_facing"), &panel.body_facing);
            push(format!("{base}.gaze_target"), &panel.gaze_target);
            push(format!("{base}.emotional_beat"), &panel.emotional_beat);
        }
    }

    fields
}
