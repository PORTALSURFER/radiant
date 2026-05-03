use super::{
    Constraints, ContainerKind, ContainerPolicy, CrossAlign, Insets, LayoutNode, MainAlign,
    OverflowPolicy, Rect, SizeModeCross, SizeModeMain, SlotChild, SlotParams, Vector2, layout_tree,
};

/// Compute fixed-width row item rects aligned to the start edge of `bounds`.
///
/// This helper is intended for compact toolbars and control strips that need
/// deterministic slot geometry without owning a host-specific layout adapter.
pub fn fixed_width_row_rects_start(
    bounds: Rect,
    gap: f32,
    widths: &[f32],
    row_id: u64,
    first_item_id: u64,
) -> Vec<Rect> {
    fixed_width_row_rects(bounds, gap, widths, row_id, None, first_item_id)
}

/// Compute fixed-width row item rects aligned to the end edge of `bounds`.
///
/// The `spacer_id` is used for the leading fill slot that pushes items to the
/// end of the row. Callers that already maintain stable layout IDs should pass
/// an ID from their own reserved range.
pub fn fixed_width_row_rects_end(
    bounds: Rect,
    gap: f32,
    widths: &[f32],
    row_id: u64,
    spacer_id: u64,
    first_item_id: u64,
) -> Vec<Rect> {
    fixed_width_row_rects(bounds, gap, widths, row_id, Some(spacer_id), first_item_id)
}

/// Return the suffix of `widths` that fits in `available_width`.
///
/// This preserves the rightmost items for compact action clusters and returns
/// widths in their original order.
pub fn visible_suffix_widths(widths: &[f32], available_width: f32, gap: f32) -> Vec<f32> {
    if available_width <= 0.0 || widths.is_empty() {
        return Vec::new();
    }
    let mut used = 0.0;
    let mut reversed = Vec::new();
    for (index, width) in widths.iter().rev().enumerate() {
        let candidate = used + width + if index > 0 { gap } else { 0.0 };
        if candidate >= available_width {
            break;
        }
        reversed.push(*width);
        used = candidate;
    }
    reversed.reverse();
    reversed
}

fn fixed_width_row_rects(
    bounds: Rect,
    gap: f32,
    widths: &[f32],
    row_id: u64,
    spacer_id: Option<u64>,
    first_item_id: u64,
) -> Vec<Rect> {
    if widths.is_empty() || bounds.width() <= 0.0 || bounds.height() <= 0.0 {
        return Vec::new();
    }
    let mut children = Vec::with_capacity(widths.len() + usize::from(spacer_id.is_some()));
    if let Some(spacer_id) = spacer_id {
        children.push(SlotChild {
            slot: SlotParams::fill(),
            child: LayoutNode::widget(spacer_id, Vector2::new(1.0, 1.0)),
        });
    }
    for (index, width) in widths.iter().enumerate() {
        children.push(fixed_width_child(
            first_item_id + index as u64,
            *width,
            if index == 0 { 0.0 } else { gap },
        ));
    }
    let tree = LayoutNode::container(
        row_id,
        ContainerPolicy {
            kind: ContainerKind::Row,
            spacing: 0.0,
            align_main: MainAlign::Start,
            align_cross: CrossAlign::Stretch,
            overflow: OverflowPolicy::Clip,
            ..ContainerPolicy::default()
        },
        children,
    );
    let output = layout_tree(&tree, bounds);
    widths
        .iter()
        .enumerate()
        .map(|(index, _)| {
            let id = first_item_id + index as u64;
            let rect = output
                .rects
                .get(&id)
                .copied()
                .unwrap_or(bounds.empty_at_min());
            rect.clamp_to(bounds)
        })
        .collect()
}

fn fixed_width_child(node_id: u64, width: f32, left_margin: f32) -> SlotChild {
    SlotChild {
        slot: SlotParams {
            size_main: SizeModeMain::Fixed(width.max(0.0)),
            size_cross: SizeModeCross::Fill,
            constraints: Constraints::new(width.max(0.0), width.max(0.0), 0.0, f32::INFINITY),
            margin: Insets {
                left: left_margin.max(0.0),
                ..Insets::default()
            },
            align_cross_override: None,
            allow_fixed_compress: false,
        },
        child: LayoutNode::widget(node_id, Vector2::new(width.max(1.0), 1.0)),
    }
}

#[cfg(test)]
mod tests {
    use super::{fixed_width_row_rects_end, fixed_width_row_rects_start, visible_suffix_widths};
    use crate::gui::types::{Point, Rect};

    #[test]
    fn fixed_width_row_rects_start_places_items_from_left_edge() {
        let bounds = Rect::from_min_max(Point::new(10.0, 20.0), Point::new(110.0, 40.0));
        let rects = fixed_width_row_rects_start(bounds, 4.0, &[20.0, 30.0], 1, 10);

        assert_eq!(rects.len(), 2);
        assert_eq!(rects[0].min.x, 10.0);
        assert_eq!(rects[0].max.x, 30.0);
        assert_eq!(rects[1].min.x, 34.0);
        assert_eq!(rects[1].max.x, 64.0);
    }

    #[test]
    fn fixed_width_row_rects_end_places_items_against_right_edge() {
        let bounds = Rect::from_min_max(Point::new(10.0, 20.0), Point::new(110.0, 40.0));
        let rects = fixed_width_row_rects_end(bounds, 4.0, &[20.0, 30.0], 1, 2, 10);

        assert_eq!(rects.len(), 2);
        assert_eq!(rects[0].min.x, 56.0);
        assert_eq!(rects[0].max.x, 76.0);
        assert_eq!(rects[1].min.x, 80.0);
        assert_eq!(rects[1].max.x, 110.0);
    }

    #[test]
    fn visible_suffix_widths_preserves_rightmost_items_that_fit() {
        assert_eq!(
            visible_suffix_widths(&[20.0, 30.0, 40.0], 80.0, 4.0),
            [30.0, 40.0]
        );
        assert!(visible_suffix_widths(&[20.0], 20.0, 4.0).is_empty());
        assert_eq!(visible_suffix_widths(&[20.0], 20.1, 4.0), [20.0]);
    }
}
