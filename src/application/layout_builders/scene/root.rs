use crate::{
    application::{FrameClock, Layer, Presentation, TransientOverlay, ViewNode, ViewNodeKind},
    gui::{
        input::KeyPress,
        shortcuts::{ShortcutCatalog, ShortcutResolution},
    },
};
use std::any::Any;

/// Declarative root scene builder.
///
/// A scene keeps normal base layout separate from typed overlay layers so
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
    /// Add one typed scene overlay layer.
    pub fn layer(mut self, layer: Layer<Message>) -> Self {
        self.layers.push(layer);
        self
    }

    /// Add one optional typed scene overlay layer.
    pub fn layer_opt(self, layer: Option<Layer<Message>>) -> Self {
        match layer {
            Some(layer) => self.layer(layer),
            None => self,
        }
    }

    /// Add typed scene overlay layers in declaration order.
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
