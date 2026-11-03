use super::super::common::{PromptLanguage, label};
use super::super::lut::resolve_photo_lut;
use super::super::model::ImagePromptRequest;
use super::{EDITORIAL_TIER2_ASSERT, TIER2_TAIL, visual};
use crate::catalog::CatalogEntry;

pub(super) fn render_editorial_flat_prompt(
    request: &ImagePromptRequest,
    selected: &[&CatalogEntry],
) -> String {
    let tier_two = request.constraints.safety_tier == 2;
    let rich_budgets = if tier_two {
        EditorialBudgets {
            subject: 56,
            face: 30,
            hair: 28,
            style: 42,
            scene_pose: 38,
            wardrobe: 56,
            composition: 40,
            lighting: 46,
            texture: 30,
        }
    } else {
        EditorialBudgets {
            subject: 90,
            face: 54,
            hair: 48,
            style: 64,
            scene_pose: 80,
            wardrobe: 96,
            composition: 60,
            lighting: 68,
            texture: 58,
        }
    };
    let prompt = render_editorial_flat_with_budgets(request, selected, rich_budgets);
    if prompt.chars().count() <= 550 {
        return prompt;
    }

    let compact_budgets = EditorialBudgets {
        subject: 56,
        face: 24,
        hair: 22,
        style: 34,
        scene_pose: 30,
        wardrobe: 40,
        composition: 32,
        lighting: 36,
        texture: 24,
    };
    render_editorial_flat_with_budgets(request, selected, compact_budgets)
}

fn render_editorial_flat_with_budgets(
    request: &ImagePromptRequest,
    selected: &[&CatalogEntry],
    budgets: EditorialBudgets,
) -> String {
    let language = request.language;
    let subject = &request.subjects[0];
    let tier_two = request.constraints.safety_tier == 2;
    let mut clauses = Vec::new();

    if tier_two {
        clauses.push(EDITORIAL_TIER2_ASSERT.to_owned());
    } else {
        let description = compact_slot(&subject.description, budgets.subject);
        clauses.push(match language {
            PromptLanguage::Korean => format!(
                "{} 1인 단독, {}",
                description,
                if request.constraints.original_characters_only {
                    "실존 인물과 무관한 성인 오리지널 캐릭터"
                } else {
                    "성인 모델"
                }
            ),
            PromptLanguage::English => format!(
                "one adult subject only, {}, {}",
                description,
                if request.constraints.original_characters_only {
                    "an original fictional character unrelated to any real person"
                } else {
                    "an adult model"
                }
            ),
        });
    }

    if let Some(face) = &subject.face {
        clauses.push(compact_slot(face, budgets.face));
    }
    if let Some(hair) = &subject.hair {
        clauses.push(compact_slot(hair, budgets.hair));
    }

    clauses.push(match language {
        PromptLanguage::Korean => "클린 인물·라이프스타일 에디토리얼 이미지".to_owned(),
        PromptLanguage::English => "clean people-and-lifestyle editorial image".to_owned(),
    });

    for style in selected.iter().filter(|entry| {
        matches!(
            entry.family,
            crate::catalog::CatalogFamily::Editorial | crate::catalog::CatalogFamily::Look
        )
    }) {
        let Some(directive) = style.prompt_directives.first() else {
            continue;
        };
        let instruction = compact_directive(&directive.text, budgets.style);
        let public_name = match language {
            PromptLanguage::Korean => Some(style.name_ko.clone()),
            PromptLanguage::English => style.display_name_en(),
        };
        clauses.push(match (public_name, instruction) {
            (Some(name), Some(instruction)) => format!("{}: {instruction}", visual(&name)),
            (Some(name), None) => visual(&name),
            (None, Some(instruction)) => instruction,
            (None, None) => continue,
        });
    }

    clauses.push(compact_join(
        [
            request.scene.environment.as_str(),
            subject.pose.as_str(),
            subject.gaze.as_str(),
        ],
        budgets.scene_pose,
    ));

    if let Some(wardrobe) = &subject.appearance {
        clauses.push(compact_slot(wardrobe, budgets.wardrobe));
    }

    clauses.push(compact_join(
        [
            request.composition.framing.as_str(),
            request.composition.viewpoint.as_str(),
        ],
        budgets.composition,
    ));

    clauses.push(compact_join(
        [
            request.lighting.key_direction.as_str(),
            request.lighting.key_quality.as_str(),
            request.lighting.shadow_character.as_str(),
        ],
        budgets.lighting,
    ));

    clauses.push(format!(
        "{} {}",
        label(language, "팔레트", "palette"),
        request
            .color
            .palette
            .iter()
            .map(|entry| entry.hex.to_ascii_uppercase())
            .collect::<Vec<_>>()
            .join(" ")
    ));

    let mut texture_parts = subject
        .distinguishing_features
        .iter()
        .take(1)
        .map(String::as_str)
        .collect::<Vec<_>>();
    if let Some(surface) = request.surfaces.first() {
        texture_parts.extend([surface.material.as_str(), surface.micro_detail.as_str()]);
    }
    let mut texture = compact_join(texture_parts, budgets.texture);
    if let Some(selection) = &request.color.photo_lut
        && let Some(resolved) = resolve_photo_lut(selection)
    {
        let lut = format!("{} LUT {}%", resolved.preset, resolved.strength_percent);
        texture = compact_join([texture.as_str(), lut.as_str()], budgets.texture);
    }
    if texture.is_empty() {
        texture = match language {
            PromptLanguage::Korean => "자연스러운 피부와 의상 재질, 미세한 필름 그레인".to_owned(),
            PromptLanguage::English => {
                "natural skin and garment texture with fine film grain".to_owned()
            }
        };
    }
    clauses.push(texture);

    if tier_two {
        clauses.push(TIER2_TAIL.to_owned());
    }
    clauses.push(format!("AR {}", request.output.aspect_ratio_label()));

    clauses
        .into_iter()
        .map(|clause| clause.trim_matches([',', ';', ' ']).to_owned())
        .filter(|clause| !clause.is_empty())
        .collect::<Vec<_>>()
        .join(", ")
}

#[derive(Clone, Copy)]
struct EditorialBudgets {
    subject: usize,
    face: usize,
    hair: usize,
    style: usize,
    scene_pose: usize,
    wardrobe: usize,
    composition: usize,
    lighting: usize,
    texture: usize,
}

fn compact_join<'a>(values: impl IntoIterator<Item = &'a str>, max_chars: usize) -> String {
    let joined = values
        .into_iter()
        .map(visual)
        .filter(|value| !value.is_empty())
        .collect::<Vec<_>>()
        .join("; ");
    compact_slot(&joined, max_chars)
}

/// Compacts a catalog directive, which is a model instruction rather than a
/// descriptive slot. A word-level cut ("Use generous asymmetrical negative")
/// silently changes what the instruction asks for, so only whole clauses are
/// emitted. The budget is advisory: the first clause is always kept, because a
/// style whose instruction is missing entirely is weaker than a slightly longer
/// line, and the outer two-pass budget already compacts descriptive slots to
/// hold the whole prompt near its length target.
fn compact_directive(value: &str, max_chars: usize) -> Option<String> {
    let normalized = visual(value);
    if normalized.chars().count() <= max_chars {
        return (!normalized.is_empty()).then_some(normalized);
    }
    let mut selected: Vec<&str> = Vec::new();
    let mut used = 0usize;
    for segment in clause_segments(&normalized) {
        let segment_len = segment.chars().count();
        let separator_len = usize::from(!selected.is_empty()) * 2;
        if !selected.is_empty() && used + separator_len + segment_len > max_chars {
            break;
        }
        used += separator_len + segment_len;
        selected.push(segment);
    }
    (!selected.is_empty()).then(|| selected.join(", "))
}

pub(super) fn clause_segments(value: &str) -> impl Iterator<Item = &str> {
    value
        .split([',', ';', '\n'])
        .map(str::trim)
        .filter(|segment| !segment.is_empty())
}

fn compact_slot(value: &str, max_chars: usize) -> String {
    let normalized = visual(value);
    if normalized.chars().count() <= max_chars {
        return normalized;
    }

    // Format B must remain a sequence of complete visual claims. Cutting at an
    // arbitrary character can leave dangling particles or numeric fragments
    // (for example, "캔버스 높이의" or "3"), which is more ambiguous than a
    // slightly shorter slot. Prefer whole comma/semicolon clauses and retain
    // the first complete clause even when it alone exceeds the advisory budget.
    let segments = clause_segments(&normalized).collect::<Vec<_>>();
    if segments.is_empty() {
        return normalized;
    }

    let mut selected = Vec::new();
    let mut used = 0usize;
    for segment in segments {
        let segment_len = segment.chars().count();
        let separator_len = usize::from(!selected.is_empty()) * 2;
        if selected.is_empty() && segment_len > max_chars {
            return compact_words(segment, max_chars);
        }
        if used + separator_len + segment_len <= max_chars {
            used += separator_len + segment_len;
            selected.push(segment);
        } else {
            break;
        }
    }
    selected.join(", ")
}

fn compact_words(value: &str, max_chars: usize) -> String {
    let mut output = String::new();
    // The running character count is carried rather than recounted per word: the
    // budget is measured in characters, and `output.chars().count()` inside the loop
    // makes the walk quadratic in the emitted length for no benefit.
    let mut used = 0usize;
    for word in value.split_whitespace() {
        let separator = usize::from(!output.is_empty());
        let word_chars = word.chars().count();
        if used + separator + word_chars > max_chars {
            break;
        }
        if !output.is_empty() {
            output.push(' ');
        }
        output.push_str(word);
        used += separator + word_chars;
    }
    if !output.is_empty() {
        return output;
    }
    value.chars().take(max_chars).collect()
}
