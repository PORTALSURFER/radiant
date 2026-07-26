use super::*;
use crate::{
    application::{
        ContinuityKey, Layer, ROOT_KEY_SCOPE, column, floating_layer, grid, row, row_key,
        scoped_key_id, text,
    },
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
fn identity_modifiers_use_last_call_wins() {
    let keyed = text::<()>("Keyed").id(42).key("title");
    let identified = text::<()>("Identified").key("title").id(42);

    assert_eq!(
        keyed.resolved_id(ROOT_KEY_SCOPE),
        Some(scoped_key_id(ROOT_KEY_SCOPE, "title"))
    );
    assert_eq!(identified.resolved_id(ROOT_KEY_SCOPE), Some(42));
}

#[test]
fn continuity_key_preserves_string_compatibility_and_same_scope_identity() {
    let borrowed = ContinuityKey::from("title");
    let owned = ContinuityKey::new(String::from("title"));

    assert_eq!(borrowed.as_str(), "title");
    assert_eq!(borrowed, owned);
    assert_eq!(
        text::<()>("Borrowed")
            .key(borrowed)
            .resolved_id(ROOT_KEY_SCOPE),
        Some(scoped_key_id(ROOT_KEY_SCOPE, "title"))
    );
    assert_eq!(
        text::<()>("Owned").key(owned).resolved_id(ROOT_KEY_SCOPE),
        Some(scoped_key_id(ROOT_KEY_SCOPE, "title"))
    );
}

#[test]
fn view_node_key_accepts_previous_to_string_inputs() {
    let borrowed: &str = "borrowed";
    let owned = String::from("owned");

    assert_eq!(
        text::<()>("Borrowed")
            .key(borrowed)
            .resolved_id(ROOT_KEY_SCOPE),
        Some(scoped_key_id(ROOT_KEY_SCOPE, borrowed))
    );
    assert_eq!(
        text::<()>("Owned").key(owned).resolved_id(ROOT_KEY_SCOPE),
        Some(scoped_key_id(ROOT_KEY_SCOPE, "owned"))
    );
    assert_eq!(
        text::<()>("Numeric")
            .key(17_u32)
            .resolved_id(ROOT_KEY_SCOPE),
        Some(scoped_key_id(ROOT_KEY_SCOPE, "17"))
    );
}

#[test]
fn continuity_key_identity_is_isolated_by_parent_scope() {
    let first_parent = column([text::<()>("Child").key("same")]).key("first");
    let second_parent = column([text::<()>("Child").key("same")]).key("second");
    let first_parent_id = scoped_key_id(ROOT_KEY_SCOPE, "first");
    let second_parent_id = scoped_key_id(ROOT_KEY_SCOPE, "second");
    let mut first_ids = Vec::new();
    let mut second_ids = Vec::new();

    first_parent.collect_reserved_ids(ROOT_KEY_SCOPE, &mut first_ids);
    second_parent.collect_reserved_ids(ROOT_KEY_SCOPE, &mut second_ids);

    assert_eq!(first_ids[0], first_parent_id);
    assert_eq!(second_ids[0], second_parent_id);
    assert_eq!(
        first_ids[1],
        scoped_key_id(first_parent_id, "same"),
        "child key should be scoped by its parent"
    );
    assert_eq!(
        second_ids[1],
        scoped_key_id(second_parent_id, "same"),
        "child key should be scoped by its parent"
    );
    assert_ne!(first_ids[1], second_ids[1]);
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
fn view_node_overlay_reserved_id_collection_includes_foreground_identities() {
    let view: ViewNode<()> = text("owner")
        .overlays(crate::application::overlays().context_menu(text("menu").id(12_345)));
    let mut ids = Vec::new();

    view.collect_reserved_ids(ROOT_KEY_SCOPE, &mut ids);

    assert_eq!(ids, vec![12_345]);
}

#[test]
fn view_node_overlay_reserved_id_collection_includes_input_identities() {
    let mut layer = Layer::modal(text("modal").id(12_346));
    layer.input = Some(text("input").key("modal-input"));
    let view: ViewNode<()> = text("owner").overlays(crate::application::overlays().layer(layer));
    let mut ids = Vec::new();

    view.collect_reserved_ids(ROOT_KEY_SCOPE, &mut ids);

    assert_eq!(ids.len(), 2);
    assert!(ids.contains(&12_346));
}

#[test]
fn view_node_overlay_optional_none_does_not_reserve_identity() {
    let view: ViewNode<()> = text("owner").overlays(crate::application::overlays().layer_opt(None));
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
