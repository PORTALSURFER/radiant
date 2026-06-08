use super::*;
use crate::{
    application::{Layer, ROOT_KEY_SCOPE, column, floating_layer, grid, row, row_key, text},
    gui::types::Point,
    layout::Vector2,
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
fn reserved_id_collection_includes_floating_layer_child_identities() {
    let view: ViewNode<()> = row([floating_layer(
        Point::new(0.0, -24.0),
        Vector2::new(120.0, 24.0),
        column([
            text("floating").key("floating-label"),
            text("fixed").id(12_345),
        ])
        .key("floating-content"),
    )
    .key("floating-layer")]);
    let mut ids = Vec::new();

    view.collect_reserved_ids(ROOT_KEY_SCOPE, &mut ids);

    assert_eq!(ids.len(), 4);
    assert!(ids.contains(&12_345));
}

#[test]
fn view_node_transient_reserved_id_collection_includes_foreground_identities() {
    let view: ViewNode<()> = text("owner").context_menu_layer(text("menu").id(12_345));
    let mut ids = Vec::new();

    view.collect_reserved_ids(ROOT_KEY_SCOPE, &mut ids);

    assert_eq!(ids, vec![12_345]);
}

#[test]
fn view_node_transient_reserved_id_collection_includes_input_identities() {
    let mut layer = Layer::modal(text("modal").id(12_346));
    layer.input = Some(text("input").key("modal-input"));
    let view: ViewNode<()> = text("owner").transient_layer(layer);
    let mut ids = Vec::new();

    view.collect_reserved_ids(ROOT_KEY_SCOPE, &mut ids);

    assert_eq!(ids.len(), 2);
    assert!(ids.contains(&12_346));
}

#[test]
fn view_node_transient_optional_none_does_not_reserve_identity() {
    let view: ViewNode<()> = text("owner").transient_layer_opt(None);
    let mut ids = Vec::new();

    view.collect_reserved_ids(ROOT_KEY_SCOPE, &mut ids);

    assert!(ids.is_empty());
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
