use std::collections::BTreeSet;

use crate::diagnostic::Diagnostic;
use crate::image::model::{
    ImagePromptRequest, ImageQuality, ImageReferenceRole, ImageRenderProfile, ImageTaskMode,
};
use crate::image::specificity::inspect_description;

use super::fields::common::validate_nonempty;

pub(super) fn validate_task_contract(
    request: &ImagePromptRequest,
    diagnostics: &mut Vec<Diagnostic>,
) {
    let mut seen_indices = BTreeSet::new();
    let mut base_count = 0usize;
    let mut has_anchor = false;
    let mut has_pose = false;
    let mut has_generate_forbidden_role = false;
    for (position, reference) in request.references.iter().enumerate() {
        let base = format!("$.references[{position}]");
        match reference.role {
            ImageReferenceRole::Base => {
                base_count += 1;
                has_anchor = true;
                has_generate_forbidden_role = true;
            }
            ImageReferenceRole::Pose => {
                has_pose = true;
                has_anchor = true;
                has_generate_forbidden_role = true;
            }
            ImageReferenceRole::Layout => has_anchor = true,
            ImageReferenceRole::Mask => has_generate_forbidden_role = true,
            _ => {}
        }
        if reference.index == 0 {
            diagnostics.push(Diagnostic::error(
                "IMG_REFERENCE_INDEX_ZERO",
                format!("{base}.index"),
                "reference indices start at 1",
            ));
        }
        let expected = u8::try_from(position + 1).unwrap_or(u8::MAX);
        if reference.index != expected {
            diagnostics.push(Diagnostic::error(
                "IMG_REFERENCE_ORDER",
                format!("{base}.index"),
                format!(
                    "references must be ordered and contiguous as 1..N; expected {expected}, found {}",
                    reference.index
                ),
            ));
        }
        if !seen_indices.insert(reference.index) {
            diagnostics.push(Diagnostic::error(
                "IMG_REFERENCE_DUPLICATE",
                format!("{base}.index"),
                format!("reference index {} is duplicated", reference.index),
            ));
        }
        validate_nonempty(
            &format!("{base}.description"),
            &reference.description,
            diagnostics,
        );
        inspect_description(
            &format!("{base}.description"),
            &reference.description,
            diagnostics,
        );
        if reference.use_for.is_empty() {
            diagnostics.push(Diagnostic::error(
                "IMG_REFERENCE_USE_EMPTY",
                format!("{base}.use_for"),
                "declare exactly what this reference controls",
            ));
        }
        let mut use_keys = BTreeSet::new();
        for (index, value) in reference.use_for.iter().enumerate() {
            let path = format!("{base}.use_for[{index}]");
            validate_nonempty(&path, value, diagnostics);
            inspect_description(&path, value, diagnostics);
            let key = value.trim().to_lowercase();
            if !use_keys.insert(key) {
                diagnostics.push(Diagnostic::error(
                    "IMG_REFERENCE_USE_DUPLICATE",
                    path,
                    "use_for contains a duplicate responsibility",
                ));
            }
        }
    }

    let mut change_keys = BTreeSet::new();
    for (index, value) in request.change_contract.change_only.iter().enumerate() {
        let path = format!("$.change_contract.change_only[{index}]");
        validate_nonempty(&path, value, diagnostics);
        inspect_description(&path, value, diagnostics);
        let key = value.trim().to_lowercase();
        if !change_keys.insert(key) {
            diagnostics.push(Diagnostic::error(
                "IMG_CHANGE_DUPLICATE",
                path,
                "change_only contains a duplicate instruction",
            ));
        }
    }
    let mut preserve_keys = BTreeSet::new();
    for (index, value) in request.change_contract.preserve.iter().enumerate() {
        let path = format!("$.change_contract.preserve[{index}]");
        validate_nonempty(&path, value, diagnostics);
        inspect_description(&path, value, diagnostics);
        let key = value.trim().to_lowercase();
        if !preserve_keys.insert(key.clone()) {
            diagnostics.push(Diagnostic::error(
                "IMG_PRESERVE_DUPLICATE",
                path.clone(),
                "preserve contains a duplicate invariant",
            ));
        }
        if change_keys.contains(&key) {
            diagnostics.push(Diagnostic::error(
                "IMG_CHANGE_PRESERVE_CONFLICT",
                path,
                "the same instruction cannot be both changed and preserved",
            ));
        }
    }
    let has_base = base_count > 0;

    match request.task_mode {
        ImageTaskMode::Generate => {
            if has_generate_forbidden_role {
                diagnostics.push(Diagnostic::error(
                    "IMG_GENERATE_EDIT_REFERENCE_ROLE",
                    "$.references",
                    "generate mode may use subject, style, product, layout, or palette references; base and mask roles require edit or composite mode",
                ));
            }
            if !request.change_contract.change_only.is_empty() {
                diagnostics.push(Diagnostic::error(
                    "IMG_GENERATE_CHANGE_ONLY",
                    "$.change_contract.change_only",
                    "generate mode describes the desired result directly; change_only is reserved for edit and composite modes",
                ));
            }
            if request.references.is_empty() && !request.change_contract.preserve.is_empty() {
                diagnostics.push(Diagnostic::error(
                    "IMG_GENERATE_PRESERVE_WITHOUT_REFERENCE",
                    "$.change_contract.preserve",
                    "preserve invariants require at least one declared reference image",
                ));
            }
        }
        ImageTaskMode::Edit => {
            if request.references.is_empty() {
                diagnostics.push(Diagnostic::error(
                    "IMG_EDIT_REFERENCE_REQUIRED",
                    "$.references",
                    "edit mode requires at least one reference image",
                ));
            }
            if !has_base {
                diagnostics.push(Diagnostic::error(
                    "IMG_EDIT_BASE_REQUIRED",
                    "$.references",
                    "edit mode requires exactly one reference with role=base",
                ));
            } else if base_count != 1 {
                diagnostics.push(Diagnostic::error(
                    "IMG_EDIT_BASE_AMBIGUOUS",
                    "$.references",
                    "edit mode requires exactly one base image; classify other references by their actual role",
                ));
            }
            require_change_contract(request, diagnostics);
        }
        ImageTaskMode::Composite => {
            if base_count > 1 {
                diagnostics.push(Diagnostic::error(
                    "IMG_COMPOSITE_BASE_AMBIGUOUS",
                    "$.references",
                    "composite mode permits at most one base image; classify other inputs by product, subject, layout, style, palette, or mask role",
                ));
            }
            if request.references.len() < 2 {
                diagnostics.push(Diagnostic::error(
                    "IMG_COMPOSITE_REFERENCES_REQUIRED",
                    "$.references",
                    "composite mode requires at least two reference images",
                ));
            }
            if !has_anchor {
                diagnostics.push(Diagnostic::error(
                    "IMG_COMPOSITE_ANCHOR_REQUIRED",
                    "$.references",
                    "composite mode requires a base or layout reference to anchor the result",
                ));
            }
            require_change_contract(request, diagnostics);
        }
    }

    if has_pose && request.render_profile != ImageRenderProfile::PoseTransfer {
        diagnostics.push(Diagnostic::error(
            "IMG_POSE_REFERENCE_PROFILE_REQUIRED",
            "$.render_profile",
            "role=pose is reserved for render_profile=pose_transfer so pose authority cannot be applied accidentally",
        ));
    }

    if request.change_contract.change_only.len() > 3 {
        diagnostics.push(
            Diagnostic::warning(
                "IMG_EDIT_CHANGE_SCOPE",
                "$.change_contract.change_only",
                "more than three independent changes increase edit drift and make failures hard to attribute",
            )
            .with_hint("split the edit into smaller iterations and repeat the preserve invariants each time"),
        );
    }
    if request.task_mode != ImageTaskMode::Generate && request.output.quality == ImageQuality::Low {
        diagnostics.push(Diagnostic::warning(
            "IMG_EDIT_QUALITY",
            "$.output.quality",
            "identity-sensitive edits and composites should use medium or high quality",
        ));
    }
}

fn require_change_contract(request: &ImagePromptRequest, diagnostics: &mut Vec<Diagnostic>) {
    if request.change_contract.change_only.is_empty() {
        diagnostics.push(Diagnostic::error(
            "IMG_CHANGE_ONLY_REQUIRED",
            "$.change_contract.change_only",
            "edit and composite modes require at least one explicit change",
        ));
    }
    if request.change_contract.preserve.is_empty() {
        diagnostics.push(Diagnostic::error(
            "IMG_PRESERVE_REQUIRED",
            "$.change_contract.preserve",
            "edit and composite modes require explicit invariants to preserve",
        ));
    }
}
