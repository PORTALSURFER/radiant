use crate::gui::types::Rect;

use super::{NormalizedPixelSnap, NormalizedViewport};

pub(super) fn local_ratio(viewport: NormalizedViewport, absolute_ratio: f64) -> f32 {
    if !absolute_ratio.is_finite() || viewport.width_ratio <= f64::EPSILON {
        return 0.0;
    }
    ((absolute_ratio.clamp(0.0, 1.0) - viewport.start_ratio) / viewport.width_ratio).clamp(0.0, 1.0)
        as f32
}

pub(super) fn x_for_ratio(
    viewport: NormalizedViewport,
    rect: Rect,
    absolute_ratio: f64,
    snap: NormalizedPixelSnap,
) -> f32 {
    let Some((min_x, max_x)) = finite_ordered_x_bounds(rect) else {
        return 0.0;
    };
    if max_x <= min_x {
        return min_x;
    }
    let raw_x = rect.min.x + (rect.width() * local_ratio(viewport, absolute_ratio));
    match snap {
        NormalizedPixelSnap::None => raw_x,
        NormalizedPixelSnap::Nearest => raw_x.round(),
    }
    .clamp(min_x, max_x)
}

fn finite_ordered_x_bounds(rect: Rect) -> Option<(f32, f32)> {
    if !rect.min.x.is_finite() || !rect.max.x.is_finite() {
        return None;
    }
    Some(if rect.min.x <= rect.max.x {
        (rect.min.x, rect.max.x)
    } else {
        (rect.max.x, rect.min.x)
    })
}
