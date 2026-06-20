use super::super::{ScrollUpdate, SurfaceRuntime};
use crate::{
    gui::types::{Point, Vector2},
    layout::NodeId,
    runtime::RuntimeBridge,
};

impl<Bridge, Message> SurfaceRuntime<Bridge, Message>
where
    Bridge: RuntimeBridge<Message>,
{
    pub(super) fn scroll_to_offset(&mut self, node_id: NodeId, offset: Vector2) {
        let previous_offset = self.layout_state.scroll_offset(node_id);
        self.layout_state.scroll_offsets.insert(node_id, offset);
        self.relayout_current_surface();
        let offset = self.layout_state.scroll_offset(node_id);
        if offset == previous_offset {
            return;
        }
        let viewport = self
            .layout
            .rects
            .get(&node_id)
            .map(|rect| Vector2::new(rect.width(), rect.height()))
            .unwrap_or_default();
        self.report_scroll_update(ScrollUpdate {
            node_id,
            position: Point::new(0.0, 0.0),
            delta: Vector2::new(offset.x - previous_offset.x, offset.y - previous_offset.y),
            previous_offset,
            offset,
            viewport,
        });
    }

    pub(super) fn scroll_into_view_offset(
        &self,
        node_id: NodeId,
        target_y: f32,
        target_height: f32,
        margin_top: f32,
        margin_bottom: f32,
        snap_y: Option<f32>,
    ) -> Option<Vector2> {
        let viewport = self.layout.rects.get(&node_id)?;
        let current = self.layout_state.scroll_offset(node_id);
        let viewport_height = viewport.height().max(0.0);
        if viewport_height <= 0.0 {
            return None;
        }
        let target_top = target_y.max(0.0);
        let target_bottom = target_top + target_height.max(0.0);
        let margin_top = margin_top.max(0.0);
        let margin_bottom = margin_bottom.max(0.0);
        let top_limit = target_top.saturating_sub_f32(margin_top);
        let bottom_limit = (target_bottom + margin_bottom - viewport_height).max(0.0);
        let target_offset_y = if current.y > top_limit {
            top_limit
        } else if current.y < bottom_limit {
            bottom_limit
        } else {
            current.y
        };
        let target_offset_y = snap_scroll_offset_y(current.y, target_offset_y, snap_y);
        Some(Vector2::new(current.x, target_offset_y))
    }

    pub(super) fn scroll_fixed_row_into_view_offset(
        &self,
        node_id: NodeId,
        row_index: usize,
        row_stride: f32,
        leading_context_rows: usize,
        trailing_context_rows: usize,
        direction: i32,
    ) -> Option<Vector2> {
        let viewport = self.layout.rects.get(&node_id)?;
        let current = self.layout_state.scroll_offset(node_id);
        if !row_stride.is_finite() || row_stride <= f32::EPSILON {
            return None;
        }
        let visible_rows = (viewport.height().max(0.0) / row_stride).ceil().max(1.0) as usize;
        let target_offset_y = if direction < 0 {
            let top_limit = row_index.saturating_sub(leading_context_rows);
            let top_limit_y = top_limit as f32 * row_stride;
            if current.y > top_limit_y {
                top_limit_y
            } else {
                current.y
            }
        } else if direction > 0 {
            let bottom_limit = row_index
                .saturating_add(trailing_context_rows)
                .saturating_add(1)
                .saturating_sub(visible_rows);
            let bottom_limit_y = bottom_limit as f32 * row_stride;
            if current.y < bottom_limit_y {
                bottom_limit_y
            } else {
                current.y
            }
        } else {
            current.y
        };
        Some(Vector2::new(current.x, target_offset_y))
    }
}

fn snap_scroll_offset_y(current_y: f32, target_y: f32, snap_y: Option<f32>) -> f32 {
    let Some(snap_y) = snap_y.filter(|snap_y| snap_y.is_finite() && *snap_y > 0.0) else {
        return target_y;
    };
    if target_y == current_y {
        target_y
    } else {
        ((target_y / snap_y).round() * snap_y).max(0.0)
    }
}

trait SaturatingSubF32 {
    fn saturating_sub_f32(self, rhs: f32) -> f32;
}

impl SaturatingSubF32 for f32 {
    fn saturating_sub_f32(self, rhs: f32) -> f32 {
        (self - rhs).max(0.0)
    }
}
