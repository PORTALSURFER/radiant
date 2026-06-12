use super::*;
use crate::{
    application::{button, column},
    layout::{ContainerKind, LayoutNode, SizeModeMain},
};

#[test]
fn list_row_id_applies_fixed_row_chrome_defaults() {
    let view = column([list_row_id(7, [button("Open").message(())])]);
    let layout = view.into_surface().layout_node();

    let LayoutNode::Container(column) = layout else {
        panic!("parent should lower to a column container");
    };
    let LayoutNode::Container(row) = &column.children[0].child else {
        panic!("list row should lower to a row container");
    };

    assert_eq!(row.id, 7);
    assert_eq!(row.policy.kind, ContainerKind::Row);
    assert_eq!(row.policy.spacing, 10.0);
    assert_eq!(row.policy.padding.left, 12.0);
    assert_eq!(row.policy.padding.top, 7.0);
    assert!(matches!(
        column.children[0].slot.size_main,
        SizeModeMain::Fixed(height) if height == 44.0
    ));
}
