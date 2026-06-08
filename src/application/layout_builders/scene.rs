//! Declarative root scene builder for base content plus transient layers.

use crate::{
    application::{
        FrameClock, Layer, LayerInputPolicy, Presentation, TransientOverlay, ViewNode,
        ViewNodeKind, pointer_shield,
    },
    gui::{
        input::KeyPress,
        shortcuts::{ShortcutCatalog, ShortcutResolution},
    },
    runtime::LayerKind,
    widgets::PointerShieldMessage,
};
use std::any::Any;

/// Declarative root scene builder.
///
/// A scene keeps normal base layout separate from typed transient UI layers so
/// Radiant owns the generic root composition and layer z-order.
pub struct Scene<Message> {
    base: ViewNode<Message>,
    layers: Vec<Layer<Message>>,
    presentation: Option<Box<dyn Any>>,
    shortcuts: Option<Box<dyn Fn(KeyPress) -> ShortcutResolution<Message>>>,
}

/// Build a root scene around base application content.
pub fn scene<Message>(base: ViewNode<Message>) -> Scene<Message> {
    Scene {
        base,
        layers: Vec::new(),
        presentation: None,
        shortcuts: None,
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

    /// Declare scene-scoped shortcut layers.
    pub fn shortcuts(mut self, catalog: ShortcutCatalog<Message>) -> Self
    where
        Message: Clone,
    {
        self.shortcuts = Some(catalog.into_resolver());
        self
    }

    /// Declare optional scene-scoped shortcut layers.
    pub fn shortcuts_opt(self, catalog: Option<ShortcutCatalog<Message>>) -> Self
    where
        Message: Clone,
    {
        match catalog {
            Some(catalog) => self.shortcuts(catalog),
            None => self,
        }
    }

    /// Add one scene-scoped frame clock.
    pub fn frame_clock<State: 'static>(mut self, clock: FrameClock<State, Message>) -> Self {
        self.update_presentation(|presentation| presentation.frame_clock(clock));
        self
    }

    /// Add one optional scene-scoped frame clock.
    pub fn frame_clock_opt<State: 'static>(
        self,
        clock: Option<FrameClock<State, Message>>,
    ) -> Self {
        match clock {
            Some(clock) => self.frame_clock(clock),
            None => self,
        }
    }

    /// Add one scene-scoped paint-only transient overlay.
    pub fn overlay<State: 'static>(mut self, overlay: TransientOverlay<State>) -> Self {
        self.update_presentation(|presentation| presentation.transient_overlay(overlay));
        self
    }

    /// Add one optional scene-scoped paint-only transient overlay.
    pub fn overlay_opt<State: 'static>(self, overlay: Option<TransientOverlay<State>>) -> Self {
        match overlay {
            Some(overlay) => self.overlay(overlay),
            None => self,
        }
    }

    /// Lower this scene into a Radiant view node.
    pub fn into_view(self) -> ViewNode<Message> {
        let has_reserved_descendant_identity = self.base.has_reserved_identity_in_subtree()
            || self
                .layers
                .iter()
                .any(Layer::has_reserved_identity_in_subtree);
        ViewNode::new(ViewNodeKind::Scene {
            base: Box::new(self.base),
            layers: self.layers,
            presentation: self.presentation,
            shortcuts: self.shortcuts,
        })
        .with_reserved_descendant_identity(has_reserved_descendant_identity)
    }

    fn update_presentation<State: 'static>(
        &mut self,
        update: impl FnOnce(Presentation<State, Message>) -> Presentation<State, Message>,
    ) {
        let Some(presentation) = self.presentation.take() else {
            self.presentation = Some(Box::new(update(Presentation::default())));
            return;
        };
        let presentation = match presentation.downcast::<Presentation<State, Message>>() {
            Ok(presentation) => *presentation,
            Err(presentation) => {
                self.presentation = Some(presentation);
                return;
            }
        };
        self.presentation = Some(Box::new(update(presentation)));
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

    /// Keep this layer from adding any synthesized scene input surface.
    pub fn pass_through(mut self) -> Self {
        self.input_policy = LayerInputPolicy::PassThrough;
        self.input = None;
        self
    }

    /// Consume pointer and wheel input over the full scene behind this layer.
    pub fn block_input(mut self) -> Self
    where
        Message: 'static,
    {
        self.input_policy = LayerInputPolicy::BlockInput;
        self.input = Some(pointer_shield(true).consume());
        self
    }

    /// Emit a message on outside pointer activation and block wheel input
    /// behind this layer.
    pub fn dismiss_on_outside_click(mut self, message: Message) -> Self
    where
        Message: Clone + Send + Sync + 'static,
    {
        self.input_policy = LayerInputPolicy::DismissOnOutsideClick;
        self.input = Some(pointer_shield(true).filter_map(
            move |shield_message| match shield_message {
                PointerShieldMessage::PointerPress { .. }
                | PointerShieldMessage::PointerDrop { .. } => Some(message.clone()),
                PointerShieldMessage::PointerMove { .. }
                | PointerShieldMessage::PointerRelease { .. }
                | PointerShieldMessage::Wheel { .. } => None,
            },
        ));
        self
    }

    /// Return this layer's declared input behavior.
    pub const fn input_policy(&self) -> LayerInputPolicy {
        self.input_policy
    }
}

#[cfg(test)]
mod tests {
    use crate::{
        application::{IntoView, Layer, LayerInputPolicy, scene, text},
        layout::{LayoutNode, Vector2},
        runtime::LayerKind,
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
}
