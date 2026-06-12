use super::*;
use crate::{
    application::{ViewNode, button, column},
    layout::{ContainerKind, LayoutNode, SizeModeMain},
    widgets::{WidgetProminence, WidgetStyle, WidgetTone},
};

#[test]
fn bounded_scroll_column_hides_empty_rows() {
    let view = bounded_scroll_column(Vec::<ViewNode<()>>::new(), 6, 18.0, 6.0);
    let layout = column([view]).into_surface().layout_node();

    let LayoutNode::Container(column) = layout else {
        panic!("parent should lower to a column container");
    };
    assert!(matches!(
        column.children[0].slot.size_main,
        SizeModeMain::Fixed(height) if height == 0.0
    ));
}

#[test]
fn bounded_scroll_column_caps_visible_row_height() {
    let rows = (0..12)
        .map(|index| button(format!("Row {index}")).message(()).height(18.0))
        .collect::<Vec<_>>();
    let view = bounded_scroll_column_from_parts(
        BoundedScrollColumnParts::new(rows, 6, 18.0, 6.0)
            .style(WidgetStyle::new(
                WidgetTone::Neutral,
                WidgetProminence::Subtle,
            ))
            .padding(3.0),
    );

    let layout = column([view]).into_surface().layout_node();
    let LayoutNode::Container(scroll) = layout else {
        panic!("parent should lower to a column container");
    };
    assert!(matches!(
        scroll.children[0].slot.size_main,
        SizeModeMain::Fixed(height) if (height - 114.0).abs() < 0.01
    ));
    let LayoutNode::Container(scroll) = &scroll.children[0].child else {
        panic!("bounded list should lower to a scroll container");
    };
    assert_eq!(scroll.policy.kind, ContainerKind::ScrollView);
}
