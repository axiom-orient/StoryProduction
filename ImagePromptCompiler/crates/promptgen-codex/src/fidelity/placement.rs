use std::collections::{BTreeMap, BTreeSet};

use promptgen_core::image::{CanvasPlacement, CanvasZone, ImagePromptRequest};

use super::panels::{expected_instances, grid, local_center};
use super::{PLACEMENT_MEASUREMENT_TOLERANCE_PERCENT, SubjectCountCheck, VisibleInstance};

pub(super) fn subject_instances_match_request(
    checks: &[SubjectCountCheck],
    request: &ImagePromptRequest,
) -> bool {
    counts_are_well_formed(checks, request)
        && request.subjects.iter().all(|subject| {
            checks
                .iter()
                .find(|check| check.id == subject.id)
                .is_some_and(|check| {
                    Some(check.observed) == expected_instances(request, subject.count)
                        && if let Some((columns, rows)) = grid(request) {
                            let mut per_panel = vec![0usize; usize::from(columns * rows)];
                            check.instances.iter().all(|instance| {
                                let (panel, local) = local_center(instance, columns, rows);
                                per_panel[panel] += 1;
                                placement_contains(&subject.placement, &local)
                            }) && per_panel
                                .iter()
                                .all(|count| *count == usize::from(subject.count))
                        } else {
                            check
                                .instances
                                .iter()
                                .all(|instance| placement_contains(&subject.placement, instance))
                        }
                })
        })
}

pub(crate) fn placement_failure_details(
    checks: &[SubjectCountCheck],
    request: &ImagePromptRequest,
) -> Option<(String, String, String)> {
    if let Some((columns, rows)) = grid(request) {
        if subject_instances_match_request(checks, request) {
            return None;
        }
        let summary = format!(
            "cinematic geometry requires {columns} columns by {rows} rows with the typed subject count and local placement in every panel"
        );
        let repair = format!(
            "Geometry-only correction: use exactly {columns} columns by {rows} rows, ordered left-to-right then top-to-bottom. Do not retain a transposed or stacked layout that disagrees with this grid. Show each typed subject's count separately in EVERY cell, with its typed placement measured inside that cell. Preserve the panel actions, shot sizes, character identity, lighting, palette, props, and every other visual property."
        );
        return Some((summary.clone(), summary, repair));
    }
    for subject in &request.subjects {
        let check = checks.iter().find(|check| check.id == subject.id)?;
        let outside = check
            .instances
            .iter()
            .filter(|instance| !placement_contains(&subject.placement, instance))
            .collect::<Vec<_>>();
        if outside.is_empty() {
            continue;
        }
        let centers = outside
            .iter()
            .map(|instance| format!("({}, {})", instance.x_percent, instance.y_percent))
            .collect::<Vec<_>>()
            .join(", ");
        let target = placement_label(&subject.placement);
        let (x, y, width, height) = placement_bounds(&subject.placement);
        let target_centers = suggested_layout_centers(&subject.placement, subject.count)
            .iter()
            .map(|(target_x, target_y)| format!("({target_x}, {target_y})"))
            .collect::<Vec<_>>()
            .join(", ");
        let summary = format!(
            "subject {:?} has {} instance center(s) outside {}: {}",
            subject.id,
            outside.len(),
            target,
            centers
        );
        let repair = format!(
            "Placement-only correction: re-layout all exactly {} visible instance(s) of {} as one compact, non-overlapping group. Use these explicit full-canvas center targets: {}. Every center must be inside {} with strict center bounds x={}..{}% and y={}..{}%; the currently outside centers are {}. Do not add or remove any instance. Preserve everything except this subject's placement.",
            subject.count,
            subject.description,
            target_centers,
            target,
            x,
            x.saturating_add(width).min(100),
            y,
            y.saturating_add(height).min(100),
            centers,
        );
        return Some((summary.clone(), summary, repair));
    }
    None
}

fn placement_contains(placement: &CanvasPlacement, instance: &VisibleInstance) -> bool {
    let (x, y, width, height) = placement_bounds(placement);
    percent_inside(instance.x_percent, x, width) && percent_inside(instance.y_percent, y, height)
}

fn placement_bounds(placement: &CanvasPlacement) -> (u8, u8, u8, u8) {
    match placement {
        CanvasPlacement::Custom {
            x_percent,
            y_percent,
            width_percent,
            height_percent,
        } => (*x_percent, *y_percent, *width_percent, *height_percent),
        CanvasPlacement::Zone(zone) => zone_box(*zone),
    }
}

fn suggested_layout_centers(placement: &CanvasPlacement, count: u16) -> Vec<(u8, u8)> {
    let count = usize::from(count);
    if count == 0 {
        return Vec::new();
    }
    let mut columns = 1usize;
    while columns * columns < count {
        columns += 1;
    }
    let rows = count.div_ceil(columns);
    let (x, y, width, height) = placement_bounds(placement);
    (0..count)
        .map(|index| {
            let column = index % columns;
            let row = index / columns;
            let target_x = u16::from(x)
                + u16::from(width) * u16::try_from(column + 1).expect("column fits")
                    / u16::try_from(columns + 1).expect("column count fits");
            let target_y = u16::from(y)
                + u16::from(height) * u16::try_from(row + 1).expect("row fits")
                    / u16::try_from(rows + 1).expect("row count fits");
            (
                u8::try_from(target_x.min(100)).expect("percentage fits"),
                u8::try_from(target_y.min(100)).expect("percentage fits"),
            )
        })
        .collect()
}

fn percent_inside(value: u8, start: u8, length: u8) -> bool {
    let minimum = start.saturating_sub(PLACEMENT_MEASUREMENT_TOLERANCE_PERCENT);
    let maximum = start
        .saturating_add(length)
        .saturating_add(PLACEMENT_MEASUREMENT_TOLERANCE_PERCENT)
        .min(100);
    (minimum..=maximum).contains(&value)
}

fn zone_box(zone: CanvasZone) -> (u8, u8, u8, u8) {
    match zone {
        CanvasZone::TopLeft => (0, 0, 34, 34),
        CanvasZone::TopCenter => (33, 0, 34, 34),
        CanvasZone::TopRight => (66, 0, 34, 34),
        CanvasZone::MiddleLeft => (0, 33, 34, 34),
        CanvasZone::Center => (33, 33, 34, 34),
        CanvasZone::MiddleRight => (66, 33, 34, 34),
        CanvasZone::BottomLeft => (0, 66, 34, 34),
        CanvasZone::BottomCenter => (33, 66, 34, 34),
        CanvasZone::BottomRight => (66, 66, 34, 34),
    }
}

pub(super) fn placement_label(placement: &CanvasPlacement) -> String {
    match placement {
        CanvasPlacement::Custom {
            x_percent,
            y_percent,
            width_percent,
            height_percent,
        } => format!(
            "custom_box(x={x_percent}%, y={y_percent}%, width={width_percent}%, height={height_percent}%)"
        ),
        CanvasPlacement::Zone(zone) => format!("the {} canvas zone", zone.as_str()),
    }
}

pub(super) fn counts_are_well_formed(
    checks: &[SubjectCountCheck],
    request: &ImagePromptRequest,
) -> bool {
    let mut found = BTreeMap::new();
    for check in checks {
        if found.insert(check.id.as_str(), check).is_some() {
            return false;
        }
    }
    found.len() == request.subjects.len()
        && request.subjects.iter().all(|subject| {
            found.get(subject.id.as_str()).is_some_and(|check| {
                Some(check.expected) == expected_instances(request, subject.count)
                    && usize::from(check.observed) == check.instances.len()
                    && instance_centers_are_unique(&check.instances)
            })
        })
}

fn instance_centers_are_unique(instances: &[VisibleInstance]) -> bool {
    let mut centers = BTreeSet::new();
    instances
        .iter()
        .all(|instance| centers.insert((instance.x_percent, instance.y_percent)))
}
