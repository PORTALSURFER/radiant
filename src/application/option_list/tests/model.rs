use super::super::{CompactOptionListItem, CompactOptionListParts, compact_option_list};
use crate::{
    application::{IntoView, column},
    layout::{SizeModeMain, Vector2},
};

#[test]
fn compact_option_list_caps_height_and_keeps_empty_lists_hidden() {
    let empty_parts = CompactOptionListParts::new(Vec::new(), 80.0);
    assert_eq!(empty_parts.height(), 0.0);
    let empty_frame = compact_option_list::<()>(Vec::new(), 80.0)
        .view_frame_at_size_with_default_theme(Vector2::new(120.0, 80.0));
    assert!(empty_frame.paint_plan.text_runs().next().is_none());

    let items = (0..12)
        .map(|index| {
            CompactOptionListItem::new(format!("Item {index}"))
                .secondary_label("Group")
                .selected(index == 1)
        })
        .collect::<Vec<_>>();
    let view = compact_option_list::<()>(items, 80.0);
    let layout = column([view]).into_surface().layout_node();
    let crate::layout::LayoutNode::Container(parent_column) = layout else {
        panic!("parent should lower to a column container");
    };
    assert!(matches!(
        parent_column.children[0].slot.size_main,
        SizeModeMain::Fixed(height) if (height - 114.0).abs() < 0.01
    ));
}

#[test]
fn compact_option_list_parts_exposes_capped_height() {
    let items = (0..12)
        .map(|index| CompactOptionListItem::new(format!("Item {index}")))
        .collect::<Vec<_>>();
    let parts = CompactOptionListParts::new(items, 80.0)
        .max_visible_rows(4)
        .row_height(20.0)
        .vertical_chrome(8.0);

    assert_eq!(parts.height(), 88.0);
}
