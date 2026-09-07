use crate::{
    application::{IntoView, Layer, LayerInputPolicy, column, scene, text},
    layout::{OverlayAnchor, Vector2},
    runtime::LayerKind,
};

#[test]
fn scene_layer_input_policy_preserves_layer_kind_z_order() {
    let labels = scene(text::<()>("Base"))
        .layer(Layer::tooltip(text("Tooltip")).pass_through())
        .layer(Layer::modal(text("Modal")).block_input())
        .layer(Layer::floating(text("Floating")).pass_through())
        .layer(Layer::context_menu(text("Context menu")).dismiss_on_outside_click(()))
        .into_view()
        .view_frame_at_size_with_default_theme(Vector2::new(240.0, 160.0))
        .paint_plan
        .text_label_strings();

    assert_eq!(
        labels,
        ["Base", "Floating", "Modal", "Context menu", "Tooltip"]
    );
}

#[test]
fn layer_kind_order_is_stable() {
    assert_eq!(LayerKind::ORDER.map(LayerKind::z_order), [0, 1, 2, 3, 4, 5]);
}

#[test]
fn layer_input_policy_defaults_to_pass_through() {
    let layer = Layer::modal(text::<()>("Modal"));

    assert_eq!(layer.input_policy(), LayerInputPolicy::PassThrough);
}

#[test]
fn layer_policy_methods_report_policy() {
    assert_eq!(
        Layer::tooltip(text::<()>("Tooltip"))
            .pass_through()
            .input_policy(),
        LayerInputPolicy::PassThrough
    );
    assert_eq!(
        Layer::modal(text::<()>("Modal"))
            .block_input()
            .input_policy(),
        LayerInputPolicy::BlockInput
    );
    assert_eq!(
        Layer::context_menu(text("Menu"))
            .dismiss_on_outside_click(())
            .input_policy(),
        LayerInputPolicy::DismissOnOutsideClick
    );
}

#[test]
fn nested_anchored_layers_keep_their_grouped_foreground_and_input_order() {
    let target = 71_u64;
    let outer_anchor = OverlayAnchor::below(target, Vector2::new(120.0, 80.0));
    let inner_anchor = OverlayAnchor::below(72, Vector2::new(80.0, 30.0));
    let nested = scene(column([
        text::<()>("Outer foreground"),
        text("Inner target").id(72),
    ]))
    .layer(Layer::tooltip(text("Nested foreground").key("nested-anchor")).anchored_to(inner_anchor))
    .into_view()
    .key("outer-anchor");

    let labels = scene(text("Anchor target").id(target))
        .layer(
            Layer::popover(nested)
                .block_input()
                .anchored_to(outer_anchor),
        )
        .into_view()
        .view_frame_at_size_with_default_theme(Vector2::new(320.0, 180.0))
        .paint_plan
        .text_label_strings();

    assert_eq!(
        labels,
        [
            "Anchor target",
            "Outer foreground",
            "Inner target",
            "Nested foreground",
        ]
    );
}
