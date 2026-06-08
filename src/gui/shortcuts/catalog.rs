use super::{ShortcutLayer, ShortcutResolution, ShortcutStack};
use crate::gui::input::KeyPress;

/// Declarative root shortcut catalog for scene-scoped shortcut layers.
///
/// Use this when a root scene owns contextual shortcut layers and an optional
/// fallback resolver. App-builder `.shortcuts(...)` remains available for
/// lower-level hosts that need focus or pending-chord aware callbacks.
pub struct ShortcutCatalog<Action> {
    stack: ShortcutStack<Action>,
    fallback: Option<Box<dyn Fn(KeyPress) -> ShortcutResolution<Action>>>,
}

impl<Action> Default for ShortcutCatalog<Action> {
    fn default() -> Self {
        Self::new()
    }
}

impl<Action> ShortcutCatalog<Action> {
    /// Build an empty shortcut catalog.
    pub const fn new() -> Self {
        Self {
            stack: ShortcutStack::new(),
            fallback: None,
        }
    }

    /// Build a shortcut catalog from an existing stack.
    pub const fn stack(stack: ShortcutStack<Action>) -> Self {
        Self {
            stack,
            fallback: None,
        }
    }

    /// Add one shortcut layer after earlier, higher-priority layers.
    pub fn layer(mut self, layer: ShortcutLayer<Action>) -> Self {
        self.stack = self.stack.push(layer);
        self
    }

    /// Add one shortcut layer only when the supplied condition is true.
    pub fn layer_when(self, condition: bool, layer: ShortcutLayer<Action>) -> Self {
        if condition { self.layer(layer) } else { self }
    }

    /// Add a fallback resolver for keys not handled by any layer.
    pub fn fallback(
        mut self,
        fallback: impl Fn(KeyPress) -> ShortcutResolution<Action> + 'static,
    ) -> Self {
        self.fallback = Some(Box::new(fallback));
        self
    }

    /// Return the ordered shortcut stack.
    pub const fn layers(&self) -> &ShortcutStack<Action> {
        &self.stack
    }

    /// Resolve one keypress against this catalog.
    pub fn resolve(&self, press: KeyPress) -> ShortcutResolution<Action>
    where
        Action: Clone,
    {
        self.stack.resolve_or_else(press, || {
            self.fallback
                .as_ref()
                .map_or_else(ShortcutResolution::unhandled, |fallback| fallback(press))
        })
    }

    pub(crate) fn into_resolver(self) -> Box<dyn Fn(KeyPress) -> ShortcutResolution<Action>>
    where
        Action: Clone + 'static,
    {
        Box::new(move |press| self.resolve(press))
    }
}
