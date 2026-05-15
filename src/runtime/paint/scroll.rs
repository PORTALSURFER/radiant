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
    let track_x = viewport.max.x - track_w;
    let track = Rect::from_min_max(
        Point::new(track_x, viewport.min.y + y_inset),
        Point::new(track_x + track_w, viewport.max.y - y_inset),
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::layout::OverflowInfo;

    #[test]
    fn scroll_affordance_clamps_thumb_to_cramped_track() {
        let mut layout = LayoutOutput::default();
        layout.rects.insert(
            1,
            Rect::from_min_size(Point::new(0.0, 0.0), Vector2::new(120.0, 20.0)),
        );
        layout.rects.insert(
            2,
            Rect::from_min_size(Point::new(0.0, -20.0), Vector2::new(120.0, 80.0)),
        );
        layout.overflow_flags.insert(
            1,
            OverflowInfo {
                x: false,
                y: true,
                policy: OverflowPolicy::Scroll,
            },
        );

        let affordance = resolve_scroll_affordance(1, 2, &layout)
            .expect("cramped overflowing scroll area should still resolve a thumb");

        assert_eq!(affordance.track.height(), 8.0);
        assert_eq!(affordance.thumb.height(), affordance.track.height());
        assert_eq!(affordance.thumb.min.y, affordance.track.min.y);
        assert_eq!(affordance.thumb.max.y, affordance.track.max.y);
    }

    #[test]
    fn scroll_affordance_omits_degenerate_track() {
        let mut layout = LayoutOutput::default();
        layout.rects.insert(
            1,
            Rect::from_min_size(Point::new(0.0, 0.0), Vector2::new(120.0, 12.0)),
        );
        layout.rects.insert(
            2,
            Rect::from_min_size(Point::new(0.0, -20.0), Vector2::new(120.0, 80.0)),
        );
        layout.overflow_flags.insert(
            1,
            OverflowInfo {
                x: false,
                y: true,
                policy: OverflowPolicy::Scroll,
            },
        );

        assert_eq!(resolve_scroll_affordance(1, 2, &layout), None);
    }

    #[test]
    fn scroll_affordance_rejects_nonfinite_layout_rects() {
        let mut layout = LayoutOutput::default();
        layout.rects.insert(
            1,
            Rect::from_min_max(Point::new(0.0, 0.0), Point::new(f32::NAN, 80.0)),
        );
        layout.rects.insert(
            2,
            Rect::from_min_size(Point::new(0.0, -20.0), Vector2::new(120.0, 160.0)),
        );
        layout.overflow_flags.insert(
            1,
            OverflowInfo {
                x: false,
                y: true,
                policy: OverflowPolicy::Scroll,
            },
        );

        assert_eq!(resolve_scroll_affordance(1, 2, &layout), None);

        layout.rects.insert(
            1,
            Rect::from_min_size(Point::new(0.0, 0.0), Vector2::new(120.0, 80.0)),
        );
        layout.rects.insert(
            2,
            Rect::from_min_max(Point::new(0.0, f32::INFINITY), Point::new(120.0, 160.0)),
        );

        assert_eq!(resolve_scroll_affordance(1, 2, &layout), None);
    }
}
