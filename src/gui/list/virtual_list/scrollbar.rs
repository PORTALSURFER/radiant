use crate::gui::types::{Point, Rect};

/// Request used to resolve one generic virtual-list scrollbar.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct VirtualListScrollbarRequest {
    /// Scrollbar track in window coordinates.
    pub track: Rect,
    /// Total logical item count.
    pub total_items: usize,
    /// Visible viewport item count.
    pub viewport_len: usize,
    /// Current viewport start.
    pub viewport_start: usize,
    /// Minimum thumb extent on the scrolling axis.
    pub min_thumb_extent: f32,
}

/// Resolved scrollbar geometry for one virtual-list viewport.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct VirtualListScrollbar {
    /// Scrollbar track in window coordinates.
    pub track: Rect,
    /// Scrollbar thumb in window coordinates.
    pub thumb: Rect,
}

/// Resolve vertical scrollbar geometry for a virtual list.
pub fn resolve_virtual_list_scrollbar(
    request: VirtualListScrollbarRequest,
) -> Option<VirtualListScrollbar> {
    if request.total_items == 0
        || request.viewport_len == 0
        || request.total_items <= request.viewport_len
        || request.track.height() <= 1.0
        || request.track.width() <= 0.0
    {
        return None;
    }

    let viewport_len = request.viewport_len.min(request.total_items);
    let max_viewport_start = request.total_items.saturating_sub(viewport_len);
    let thumb_extent = (request.track.height()
        * (viewport_len as f32 / request.total_items as f32))
        .round()
        .clamp(request.min_thumb_extent.max(1.0), request.track.height());
    let travel = (request.track.height() - thumb_extent).max(0.0);
    let start_ratio = if max_viewport_start == 0 {
        0.0
    } else {
        request.viewport_start.min(max_viewport_start) as f32 / max_viewport_start as f32
    };
    let thumb_min_y = (request.track.min.y + travel * start_ratio).round();
    let thumb = Rect::from_min_max(
        Point::new(request.track.min.x, thumb_min_y),
        Point::new(
            request.track.max.x,
            (thumb_min_y + thumb_extent).min(request.track.max.y),
        ),
    );

    Some(VirtualListScrollbar {
        track: request.track,
        thumb,
    })
}

/// Resolve a virtual-list viewport start from a dragged scrollbar thumb.
pub fn virtual_list_scrollbar_view_start_for_pointer(
    scrollbar: VirtualListScrollbar,
    viewport_len: usize,
    total_items: usize,
    pointer_y: f32,
    thumb_pointer_offset_y: f32,
) -> Option<usize> {
    if viewport_len == 0 || total_items <= viewport_len {
        return None;
    }
    let max_viewport_start = total_items.saturating_sub(viewport_len);
    let thumb_extent = scrollbar.thumb.height().max(1.0);
    let travel = (scrollbar.track.height() - thumb_extent).max(0.0);
    if travel <= f32::EPSILON || max_viewport_start == 0 {
        return Some(0);
    }

    let thumb_min_y = (pointer_y - thumb_pointer_offset_y)
        .clamp(scrollbar.track.min.y, scrollbar.track.max.y - thumb_extent);
    let start_ratio = ((thumb_min_y - scrollbar.track.min.y) / travel).clamp(0.0, 1.0);
    Some(((start_ratio * max_viewport_start as f32).round() as usize).min(max_viewport_start))
}
