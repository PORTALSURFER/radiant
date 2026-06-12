use crate::{
    application::{IntoView, Layer, column, overlays, scene, text},
    layout::{LayoutNode, Vector2},
};

#[test]
fn scene_with_only_base_returns_base_layout() {
    let layout = scene(text::<()>("Base"))
        .into_view()
        .into_surface()
        .layout_node();

    assert!(
        matches!(layout, LayoutNode::Widget(_)),
        "single base scene should not allocate a stack container"
    );
}

#[test]
fn scene_omits_none_layers() {
    let layout = scene(text::<()>("Base"))
        .layer_opt(None)
        .into_view()
        .into_surface()
        .layout_node();

    assert!(
        matches!(layout, LayoutNode::Widget(_)),
        "None layers should not allocate a stack container"
    );
}

#[test]
fn scene_preserves_declared_order_within_each_kind() {
    let labels = scene(text::<()>("Base"))
        .layer(Layer::modal(text("First modal")))
        .layer(Layer::modal(text("Second modal")))
        .layer(Layer::context_menu(text("First menu")))
        .layer(Layer::context_menu(text("Second menu")))
        .into_view()
        .view_frame_at_size_with_default_theme(Vector2::new(240.0, 160.0))
        .paint_plan
        .text_label_strings();

    assert_eq!(
        labels,
        [
            "Base",
            "First modal",
            "Second modal",
            "First menu",
            "Second menu"
        ]
    );
}

#[test]
fn scene_applies_fixed_layer_kind_z_order() {
    let labels = scene(text::<()>("Base"))
        .layer(Layer::tooltip(text("Tooltip")))
        .layer(Layer::modal(text("Modal")))
        .layer(Layer::floating(text("Floating")))
        .layer(Layer::drag_preview(text("Drag preview")))
        .layer(Layer::context_menu(text("Context menu")))
        .layer(Layer::popover(text("Popover")))
        .into_view()
        .view_frame_at_size_with_default_theme(Vector2::new(240.0, 160.0))
        .paint_plan
        .text_label_strings();

    assert_eq!(
        labels,
        [
            "Base",
            "Floating",
            "Popover",
            "Modal",
            "Context menu",
            "Tooltip",
            "Drag preview"
        ]
    );
}

#[test]
fn scene_paint_order_matches_layer_kind_order() {
    let labels = scene(text::<()>("Base"))
        .layers([
            Layer::drag_preview(text("Drag")),
            Layer::floating(text("Floating")),
            Layer::tooltip(text("Tooltip")),
        ])
        .into_view()
        .view_frame_at_size_with_default_theme(Vector2::new(240.0, 160.0))
        .paint_plan
        .text_label_strings();

    assert_eq!(labels, ["Base", "Floating", "Tooltip", "Drag"]);
}

#[test]
fn scene_overlay_layers_project_from_base_descendants() {
    let labels = scene::<()>(column([
        text("Status").overlays(overlays().popover(text("Job details"))),
        text("Browser").overlays(overlays().context_menu(text("Context menu"))),
        text("Editor").overlays(overlays().floating(text("Completion"))),
    ]))
    .into_view()
    .view_frame_at_size_with_default_theme(Vector2::new(320.0, 180.0))
    .paint_plan
    .text_label_strings();

    assert_eq!(
        labels,
        [
            "Status",
            "Browser",
            "Editor",
            "Completion",
            "Job details",
            "Context menu"
        ]
    );
}
