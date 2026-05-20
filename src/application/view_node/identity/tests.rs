use super::*;
use crate::{
    application::{ROOT_KEY_SCOPE, column, grid, row, row_key, text},
    runtime::{SurfaceNode, WidgetMessageMapper},
    widgets::{ButtonWidget, WidgetSizing},
};

#[test]
fn reserved_id_collection_presizes_for_large_child_groups() {
    let view = column((0..64).map(|index| {
        row_key(
            format!("row-{index}"),
            Vec::<crate::application::ViewNode<()>>::new(),
        )
    }));
    let mut ids = Vec::new();

    view.collect_reserved_ids(ROOT_KEY_SCOPE, &mut ids);

    assert_eq!(ids.len(), 64);
    assert!(ids.capacity() >= 64);
}

#[test]
fn reserved_id_collection_skips_unreserved_descendants() {
    let view: ViewNode<()> = row_key("row", [text("unreserved child")]);
    let mut ids = Vec::new();

    view.collect_reserved_ids(ROOT_KEY_SCOPE, &mut ids);

    assert_eq!(ids.len(), 1);
    assert!(ids.contains(&view.resolved_id(ROOT_KEY_SCOPE).unwrap()));
}

#[test]
fn reserved_id_collection_presizes_for_nested_child_identities() {
    let view: ViewNode<()> = column(
        (0..64).map(|index| row_key(format!("row-{index}"), [text("action").id(10_000 + index)])),
    );
    let mut ids = Vec::new();

    view.collect_reserved_ids(ROOT_KEY_SCOPE, &mut ids);

    assert_eq!(ids.len(), 128);
    assert!(ids.capacity() >= 128);
}

#[test]
fn reserved_id_collection_includes_grid_child_identities() {
    let view: ViewNode<()> = grid(
        (0..16).map(|index| row_key(format!("tile-{index}"), [text("action").id(10_000 + index)])),
        4,
    );
    let mut ids = Vec::new();

    view.collect_reserved_ids(ROOT_KEY_SCOPE, &mut ids);

    assert_eq!(ids.len(), 32);
    for id in 10_000..10_016 {
        assert!(ids.contains(&id));
    }
    assert!(ids.capacity() >= 32);
}

#[test]
fn reserved_id_collection_presizes_wrapped_runtime_identities() {
    let runtime = SurfaceNode::widget(
        ButtonWidget::new(
            80,
            "Runtime",
            WidgetSizing::fixed(crate::layout::Vector2::new(80.0, 24.0)),
        ),
        WidgetMessageMapper::none(),
    );
    let view: ViewNode<()> = row([ViewNode::from(runtime).id(90)]);
    let mut ids = Vec::new();

    view.collect_reserved_ids(ROOT_KEY_SCOPE, &mut ids);

    assert_eq!(ids, vec![90, 80]);
    assert!(ids.capacity() >= 2);
}
