//! Cinematic subject counts are per panel; receipt centers remain full-canvas.
use promptgen_core::image::{ImagePromptRequest, ImageRenderProfile};

use super::VisibleInstance;

pub(super) fn grid(request: &ImagePromptRequest) -> Option<(u8, u8)> {
    if request.render_profile != ImageRenderProfile::CinematicStoryboard {
        return None;
    }
    match request.storyboard.as_ref()?.layout.as_str() {
        "three_panel_strip" => Some((3, 1)),
        "two_by_two" => Some((2, 2)),
        "three_by_two" => Some((3, 2)),
        _ => None,
    }
}

pub(crate) fn expected_instances(request: &ImagePromptRequest, count: u16) -> Option<u16> {
    match grid(request) {
        Some((columns, rows)) => count.checked_mul(u16::from(columns) * u16::from(rows)),
        None => Some(count),
    }
}

pub(super) fn local_center(
    instance: &VisibleInstance,
    columns: u8,
    rows: u8,
) -> (usize, VisibleInstance) {
    let scaled_x = u16::from(instance.x_percent) * u16::from(columns);
    let scaled_y = u16::from(instance.y_percent) * u16::from(rows);
    let column = (scaled_x / 100).min(u16::from(columns - 1));
    let row = (scaled_y / 100).min(u16::from(rows - 1));
    (
        usize::from(row * u16::from(columns) + column),
        VisibleInstance {
            x_percent: (scaled_x - column * 100) as u8,
            y_percent: (scaled_y - row * 100) as u8,
            evidence: instance.evidence.clone(),
        },
    )
}
