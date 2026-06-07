//! Declarative root layer host for transient application UI.

use crate::application::{ViewNode, stack_layers};

/// Declarative root composition host for base content plus transient layers.
///
/// `LayerHost` lets applications describe their active floating surfaces each
/// frame while Radiant owns the generic category z-order and stack assembly.
pub struct LayerHost<Message> {
    base: ViewNode<Message>,
    floating: Vec<ViewNode<Message>>,
    popovers: Vec<ViewNode<Message>>,
    modals: Vec<ViewNode<Message>>,
    context_menus: Vec<ViewNode<Message>>,
    tooltips: Vec<ViewNode<Message>>,
    drag_previews: Vec<ViewNode<Message>>,
}

/// Build a root layer host around base application content.
pub fn layer_host<Message>(base: ViewNode<Message>) -> LayerHost<Message> {
    LayerHost {
        base,
        floating: Vec::new(),
        popovers: Vec::new(),
        modals: Vec::new(),
        context_menus: Vec::new(),
        tooltips: Vec::new(),
        drag_previews: Vec::new(),
    }
}

impl<Message: 'static> LayerHost<Message> {
    /// Add a generic floating layer above base content.
    pub fn floating(mut self, layer: ViewNode<Message>) -> Self {
        self.floating.push(layer);
        self
    }

    /// Add an optional generic floating layer above base content.
    pub fn floating_opt(self, layer: Option<ViewNode<Message>>) -> Self {
        self.add_opt_layer(layer, Self::floating)
    }

    /// Add a popover layer above generic floating layers.
    pub fn popover(mut self, layer: ViewNode<Message>) -> Self {
        self.popovers.push(layer);
        self
    }

    /// Add an optional popover layer above generic floating layers.
    pub fn popover_opt(self, layer: Option<ViewNode<Message>>) -> Self {
        self.add_opt_layer(layer, Self::popover)
    }

    /// Add a modal layer above popovers.
    pub fn modal(mut self, layer: ViewNode<Message>) -> Self {
        self.modals.push(layer);
        self
    }

    /// Add an optional modal layer above popovers.
    pub fn modal_opt(self, layer: Option<ViewNode<Message>>) -> Self {
        self.add_opt_layer(layer, Self::modal)
    }

    /// Add a context menu layer above modals.
    pub fn context_menu(mut self, layer: ViewNode<Message>) -> Self {
        self.context_menus.push(layer);
        self
    }

    /// Add an optional context menu layer above modals.
    pub fn context_menu_opt(self, layer: Option<ViewNode<Message>>) -> Self {
        self.add_opt_layer(layer, Self::context_menu)
    }

    /// Add a tooltip layer above context menus.
    pub fn tooltip(mut self, layer: ViewNode<Message>) -> Self {
        self.tooltips.push(layer);
        self
    }

    /// Add an optional tooltip layer above context menus.
    pub fn tooltip_opt(self, layer: Option<ViewNode<Message>>) -> Self {
        self.add_opt_layer(layer, Self::tooltip)
    }

    /// Add a drag-preview layer above every other transient category.
    pub fn drag_preview(mut self, layer: ViewNode<Message>) -> Self {
        self.drag_previews.push(layer);
        self
    }

    /// Add an optional drag-preview layer above every other transient category.
    pub fn drag_preview_opt(self, layer: Option<ViewNode<Message>>) -> Self {
        self.add_opt_layer(layer, Self::drag_preview)
    }

    /// Lower the layer host into a normal Radiant view stack.
    pub fn into_view(self) -> ViewNode<Message> {
        let mut layers = Vec::with_capacity(
            1 + self.floating.len()
                + self.popovers.len()
                + self.modals.len()
                + self.context_menus.len()
                + self.tooltips.len()
                + self.drag_previews.len(),
        );
        layers.push(self.base);
        layers.extend(self.floating);
        layers.extend(self.popovers);
        layers.extend(self.modals);
        layers.extend(self.context_menus);
        layers.extend(self.tooltips);
        layers.extend(self.drag_previews);

        stack_layers(layers)
    }

    fn add_opt_layer(
        self,
        layer: Option<ViewNode<Message>>,
        add_layer: impl FnOnce(Self, ViewNode<Message>) -> Self,
    ) -> Self {
        match layer {
            Some(layer) => add_layer(self, layer),
            None => self,
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::{
        application::{IntoView, layer_host, text},
        layout::{ContainerKind, LayoutNode, Vector2},
    };

    #[test]
    fn layer_host_with_only_base_returns_base() {
        let layout = layer_host(text::<()>("Base"))
            .into_view()
            .into_surface()
            .layout_node();

        assert!(
            matches!(layout, LayoutNode::Widget(_)),
            "single base layer should not allocate a stack container"
        );
    }

    #[test]
    fn layer_host_omits_none_layers() {
        let layout = layer_host(text::<()>("Base"))
            .floating_opt(None)
            .popover_opt(None)
            .modal_opt(None)
            .context_menu_opt(None)
            .tooltip_opt(None)
            .drag_preview_opt(None)
            .into_view()
            .into_surface()
            .layout_node();

        assert!(
            matches!(layout, LayoutNode::Widget(_)),
            "None layers should be omitted before stack assembly"
        );
    }

    #[test]
    fn layer_host_preserves_declared_order_within_each_category() {
        let labels = layer_host(text::<()>("Base"))
            .modal(text("First modal"))
            .modal(text("Second modal"))
            .context_menu(text("First menu"))
            .context_menu(text("Second menu"))
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
    fn layer_host_applies_fixed_category_z_order() {
        let labels = layer_host(text::<()>("Base"))
            .tooltip(text("Tooltip"))
            .modal(text("Modal"))
            .floating(text("Floating"))
            .drag_preview(text("Drag preview"))
            .context_menu(text("Context menu"))
            .popover(text("Popover"))
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
    fn layer_host_includes_reserved_child_identities_from_transient_layers() {
        let layout = layer_host(text::<()>("Base"))
            .floating(text("Floating").key("floating-layer"))
            .modal(text("Modal").key("modal-layer"))
            .into_view()
            .into_surface()
            .layout_node();

        let LayoutNode::Container(container) = layout else {
            panic!("transient layers should lower to a stack container");
        };
        assert_eq!(container.policy.kind, ContainerKind::Stack);
        assert_eq!(container.children.len(), 3);
    }
}
