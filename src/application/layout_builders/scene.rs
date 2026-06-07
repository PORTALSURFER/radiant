//! Declarative root scene builder for base content plus transient layers.

use crate::{
    application::{Layer, ViewNode, ViewNodeKind},
    runtime::LayerKind,
};

/// Declarative root scene builder.
///
/// A scene keeps normal base layout separate from typed transient UI layers so
/// Radiant owns the generic root composition and layer z-order.
pub struct Scene<Message> {
    base: ViewNode<Message>,
    layers: Vec<Layer<Message>>,
}

/// Build a root scene around base application content.
pub fn scene<Message>(base: ViewNode<Message>) -> Scene<Message> {
    Scene {
        base,
        layers: Vec::new(),
    }
}

impl<Message: 'static> Scene<Message> {
    /// Add one typed transient layer.
    pub fn layer(mut self, layer: Layer<Message>) -> Self {
        self.layers.push(layer);
        self
    }

    /// Add one optional typed transient layer.
    pub fn layer_opt(self, layer: Option<Layer<Message>>) -> Self {
        match layer {
            Some(layer) => self.layer(layer),
            None => self,
        }
    }

    /// Add typed transient layers in declaration order.
    pub fn layers(mut self, layers: impl IntoIterator<Item = Layer<Message>>) -> Self {
        self.layers.extend(layers);
        self
    }

    /// Lower this scene into a Radiant view node.
    pub fn into_view(self) -> ViewNode<Message> {
        let has_reserved_descendant_identity = self.base.has_reserved_identity_in_subtree()
            || self
                .layers
                .iter()
                .any(|layer| layer.view.has_reserved_identity_in_subtree());
        ViewNode::new(ViewNodeKind::Scene {
            base: Box::new(self.base),
            layers: self.layers,
        })
        .with_reserved_descendant_identity(has_reserved_descendant_identity)
    }
}

impl<Message> Layer<Message> {
    /// Build a generic floating layer above base content.
    pub fn floating(view: ViewNode<Message>) -> Self {
        Self::new(LayerKind::Floating, view)
    }

    /// Build a popover layer above generic floating layers.
    pub fn popover(view: ViewNode<Message>) -> Self {
        Self::new(LayerKind::Popover, view)
    }

    /// Build a modal layer above popovers.
    pub fn modal(view: ViewNode<Message>) -> Self {
        Self::new(LayerKind::Modal, view)
    }

    /// Build a context menu layer above modals.
    pub fn context_menu(view: ViewNode<Message>) -> Self {
        Self::new(LayerKind::ContextMenu, view)
    }

    /// Build a tooltip layer above context menus.
    pub fn tooltip(view: ViewNode<Message>) -> Self {
        Self::new(LayerKind::Tooltip, view)
    }

    /// Build a drag preview layer above every other transient category.
    pub fn drag_preview(view: ViewNode<Message>) -> Self {
        Self::new(LayerKind::DragPreview, view)
    }
}

#[cfg(test)]
mod tests {
    use crate::{
        application::{IntoView, Layer, scene, text},
        layout::{ContainerKind, LayoutNode, Vector2},
        runtime::LayerKind,
    };

    #[test]
    fn scene_with_only_base_projects_scene_root() {
        let layout = scene(text::<()>("Base"))
            .into_view()
            .into_surface()
            .layout_node();

        let LayoutNode::Container(container) = layout else {
            panic!("scene should project to a root stack container");
        };
        assert_eq!(container.policy.kind, ContainerKind::Stack);
        assert_eq!(container.children.len(), 1);
    }

    #[test]
    fn scene_omits_none_layers() {
        let layout = scene(text::<()>("Base"))
            .layer_opt(None)
            .into_view()
            .into_surface()
            .layout_node();

        let LayoutNode::Container(container) = layout else {
            panic!("scene should project to a root stack container");
        };
        assert_eq!(container.children.len(), 1);
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
    fn layer_kind_order_is_stable() {
        assert_eq!(LayerKind::ORDER.map(LayerKind::z_order), [0, 1, 2, 3, 4, 5]);
    }
}
