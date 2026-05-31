use super::{ShortcutLayer, ShortcutResolution};
use crate::gui::input::KeyPress;

/// Priority-ordered shortcut layers for host-owned command catalogs.
///
/// Layers are resolved in insertion order. The first handled layer wins, which
/// lets hosts express modal overlays, contextual editing layers, and default
/// application shortcuts without hand-writing resolution chains.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct ShortcutStack<Action> {
    layers: Vec<ShortcutLayer<Action>>,
}

impl<Action> ShortcutStack<Action> {
    /// Build an empty shortcut stack.
    pub const fn new() -> Self {
        Self { layers: Vec::new() }
    }

    /// Add one layer to the end of the priority stack.
    pub fn push(mut self, layer: ShortcutLayer<Action>) -> Self {
        self.layers.push(layer);
        self
    }

    /// Add one layer only when the supplied condition is true.
    pub fn push_when(self, condition: bool, layer: ShortcutLayer<Action>) -> Self {
        if condition { self.push(layer) } else { self }
    }

    /// Return the ordered layers in this stack.
    pub fn layers(&self) -> &[ShortcutLayer<Action>] {
        &self.layers
    }

    /// Resolve `press` against the priority stack.
    pub fn resolve(&self, press: KeyPress) -> ShortcutResolution<Action>
    where
        Action: Clone,
    {
        self.resolve_or_else(press, ShortcutResolution::unhandled)
    }

    /// Resolve `press`, calling `fallback` only when every layer passes through.
    pub fn resolve_or_else(
        &self,
        press: KeyPress,
        fallback: impl FnOnce() -> ShortcutResolution<Action>,
    ) -> ShortcutResolution<Action>
    where
        Action: Clone,
    {
        self.layers
            .iter()
            .map(|layer| layer.resolve(press))
            .find(|resolution| resolution.handled)
            .unwrap_or_else(fallback)
    }
}
