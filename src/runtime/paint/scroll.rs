//! Scroll paint helpers for backend-neutral paint plans.

use crate::gui::types::{Point, Rect, Vector2};
use crate::layout::{LayoutOutput, NodeId, OverflowPolicy, ScrollbarVisibility};
use crate::theme::ThemeTokens;

use super::{PaintFillRect, PaintPrimitive};

#[cfg(test)]
#[path = "scroll/tests.rs"]
mod tests;

#[derive(Clone, Copy, Debug, PartialEq)]
pub(in crate::runtime) struct ScrollAffordance {
    pub(in crate::runtime) track: Rect,
    pub(in crate::runtime) thumb: Rect,
    pub(in crate::runtime) max_scroll: f32,
}

pub(in crate::runtime) fn push_scroll_affordance(
    primitives: &mut Vec<PaintPrimitive>,
    node_id: NodeId,
    content_id: NodeId,
    layout: &LayoutOutput,
    theme: &ThemeTokens,
    active: bool,
) {
    let Some(affordance) = resolve_scroll_affordance(node_id, content_id, layout) else {
        return;
    };

    primitives.push(PaintPrimitive::FillRect(PaintFillRect {
        widget_id: node_id,
        rect: affordance.thumb,
        color: if active {
            theme.accent_copper
        } else {
            theme.grid_strong
        },
    }));
}

/// Append a horizontal scrollbar thumb when a scroll viewport overflows.
pub(in crate::runtime) fn push_horizontal_scroll_affordance(
    primitives: &mut Vec<PaintPrimitive>,
    node_id: NodeId,
    content_id: NodeId,
    layout: &LayoutOutput,
    theme: &ThemeTokens,
    active: bool,
) {
    let Some(affordance) = resolve_horizontal_scroll_affordance(node_id, content_id, layout) else {
        return;
    };
    primitives.push(PaintPrimitive::FillRect(PaintFillRect {
        widget_id: node_id,
        rect: affordance.thumb,
        color: if active {
            theme.accent_copper
        } else {
            theme.grid_strong
        },
    }));
}

pub(in crate::runtime) fn resolve_scroll_affordance(
    node_id: NodeId,
    content_id: NodeId,
    layout: &LayoutOutput,
) -> Option<ScrollAffordance> {
    let (viewport, bar_bounds) = scrollbar_viewports(node_id, layout)?;
    let content = layout.rects.get(&content_id).copied()?;
    if !viewport.is_finite() || !content.is_finite() {
        return None;
    }
    let overflow = layout.overflow_flags.get(&node_id)?;
    if overflow.policy != OverflowPolicy::Scroll || !overflow.y {
        return None;
    }

    let viewport_h = viewport.height().max(0.0);
    let content_h = content.height().max(viewport_h);
    if viewport_h <= 0.0 || content_h <= viewport_h {
        return None;
    }

    let track_w = 3.0;
    let y_inset = 6.0;
    let track_x = bar_bounds.max.x - track_w;
    let track_max_y = bar_bounds.max.y
        - if layout
            .scrollbar_placements
            .get(&node_id)
            .is_some_and(|placement| {
                *placement == crate::gui::layout_core::ScrollbarPlacement::Reserved
            })
            && overflow.x
        {
            3.0
        } else {
            0.0
        };
    let track = Rect::from_min_max(
        Point::new(track_x, viewport.min.y + y_inset),
        Point::new(track_x + track_w, track_max_y - y_inset),
    );
    if track.height() <= 0.0 {
        return None;
    }
    let max_scroll = (content_h - viewport_h).max(1.0);
    let scroll_y = (viewport.min.y - content.min.y).clamp(0.0, max_scroll);
    let min_thumb_h = 24.0_f32.min(track.height());
    let thumb_h = ((viewport_h / content_h) * track.height()).clamp(min_thumb_h, track.height());
    let thumb_y = track.min.y + ((track.height() - thumb_h) * (scroll_y / max_scroll));
    let thumb = Rect::from_min_size(
        Point::new(track.min.x, thumb_y),
        Vector2::new(track.width(), thumb_h),
    );

    Some(ScrollAffordance {
        track,
        thumb,
        max_scroll,
    })
}

/// Resolve the horizontal scrollbar using the same committed geometry used by
/// paint and hit testing. Keeping this beside the vertical resolver prevents
/// drag math from developing a second, subtly different track definition.
pub(in crate::runtime) fn resolve_horizontal_scroll_affordance(
    node_id: NodeId,
    content_id: NodeId,
    layout: &LayoutOutput,
) -> Option<ScrollAffordance> {
    let (viewport, bar_bounds) = scrollbar_viewports(node_id, layout)?;
    let content = layout.rects.get(&content_id).copied()?;
    if !viewport.is_finite() || !content.is_finite() {
        return None;
    }
    let overflow = layout.overflow_flags.get(&node_id)?;
    if overflow.policy != OverflowPolicy::Scroll || !overflow.x {
        return None;
    }
    let viewport_w = viewport.width().max(0.0);
    let content_w = content.width().max(viewport_w);
    if viewport_w <= 0.0 || content_w <= viewport_w {
        return None;
    }
    let track_h = 3.0;
    let track_max_x = bar_bounds.max.x
        - if layout
            .scrollbar_placements
            .get(&node_id)
            .is_some_and(|placement| {
                *placement == crate::gui::layout_core::ScrollbarPlacement::Reserved
            })
            && overflow.y
        {
            3.0
        } else {
            0.0
        };
    let track = Rect::from_min_max(
        Point::new(viewport.min.x + 6.0, bar_bounds.max.y - track_h),
        Point::new(track_max_x - 6.0, bar_bounds.max.y),
    );
    if track.width() <= 0.0 {
        return None;
    }
    let max_scroll = (content_w - viewport_w).max(1.0);
    let scroll_x = (viewport.min.x - content.min.x).clamp(0.0, max_scroll);
    let min_thumb_w = 24.0_f32.min(track.width());
    let thumb_w = ((viewport_w / content_w) * track.width()).clamp(min_thumb_w, track.width());
    let thumb_x = track.min.x + (track.width() - thumb_w) * (scroll_x / max_scroll);
    Some(ScrollAffordance {
        track,
        thumb: Rect::from_min_size(
            Point::new(thumb_x, track.min.y),
            Vector2::new(thumb_w, track.height()),
        ),
        max_scroll,
    })
}

pub(in crate::runtime) fn scrollbar_viewport(
    node_id: NodeId,
    layout: &LayoutOutput,
) -> Option<Rect> {
    layout.rects.get(&node_id).copied()
}

fn scrollbar_viewports(node_id: NodeId, layout: &LayoutOutput) -> Option<(Rect, Rect)> {
    let bar_bounds = layout.rects.get(&node_id).copied()?;
    let viewport = match layout.scrollbar_placements.get(&node_id).copied() {
        Some(crate::gui::layout_core::ScrollbarPlacement::Reserved) => layout
            .viewport_bounds
            .get(&node_id)
            .copied()
            .unwrap_or(bar_bounds),
        _ => bar_bounds,
    };
    Some((viewport, bar_bounds))
}

pub(in crate::runtime) fn scroll_content_clip_rect(
    node_id: NodeId,
    layout: &LayoutOutput,
    viewport: Rect,
) -> Rect {
    let Some(overflow) = layout.overflow_flags.get(&node_id) else {
        return viewport;
    };
    if overflow.policy != OverflowPolicy::Scroll {
        return viewport;
    }
    match layout.scrollbar_placements.get(&node_id).copied() {
        Some(crate::gui::layout_core::ScrollbarPlacement::Reserved) => {
            scrollbar_viewports(node_id, layout)
                .map_or(viewport, |(committed_viewport, _)| committed_viewport)
        }
        _ => viewport,
    }
}

/// Resolve whether a scrollbar affordance is visible at the private paint
/// and pointer-routing boundaries.
pub(in crate::runtime) fn scrollbar_visibility_allows(
    visibility: ScrollbarVisibility,
    node_id: NodeId,
    auto_visible: &[NodeId],
) -> bool {
    match visibility {
        ScrollbarVisibility::Hidden => false,
        ScrollbarVisibility::Always => true,
        ScrollbarVisibility::Auto => auto_visible.binary_search(&node_id).is_ok(),
    }
}
