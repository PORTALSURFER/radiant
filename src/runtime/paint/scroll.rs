//! Scroll paint helpers for backend-neutral paint plans.

use crate::gui::types::{Point, Rect, Vector2};
use crate::layout::{LayoutOutput, NodeId, OverflowPolicy};
use crate::theme::ThemeTokens;

use super::{PaintFillRect, PaintPrimitive};

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

pub(in crate::runtime) fn resolve_scroll_affordance(
    node_id: NodeId,
    content_id: NodeId,
    layout: &LayoutOutput,
) -> Option<ScrollAffordance> {
    let viewport = layout.rects.get(&node_id).copied()?;
    let content = layout.rects.get(&content_id).copied()?;
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
    let track_x = viewport.max.x - track_w;
    let track = Rect::from_min_max(
        Point::new(track_x, viewport.min.y + y_inset),
        Point::new(track_x + track_w, viewport.max.y - y_inset),
    );
    let max_scroll = (content_h - viewport_h).max(1.0);
    let scroll_y = (viewport.min.y - content.min.y).clamp(0.0, max_scroll);
    let thumb_h = ((viewport_h / content_h) * track.height()).clamp(24.0, track.height());
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

pub(in crate::runtime) fn scroll_content_clip_rect(
    node_id: NodeId,
    layout: &LayoutOutput,
    viewport: Rect,
) -> Rect {
    let Some(overflow) = layout.overflow_flags.get(&node_id) else {
        return viewport;
    };
    if overflow.policy != OverflowPolicy::Scroll || !overflow.y {
        return viewport;
    }
    viewport
}
