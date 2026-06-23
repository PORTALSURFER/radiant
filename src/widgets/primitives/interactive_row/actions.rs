//! Common host-action routing for interactive row messages.

use crate::{
    gui::types::Point,
    widgets::interaction::{DragHandleMessage, InteractiveRowMessage, PointerModifiers},
};
use std::sync::Arc;

/// Host callbacks for common interactive-row message routing.
///
/// Use this router when a row host only needs the standard activation,
/// secondary-click, drag, drop, and hover-drop interaction shapes translated
/// into its own message type.
#[derive(Clone)]
pub struct InteractiveRowActions<Message> {
    activate: Option<Arc<dyn Fn() -> Message + Send + Sync + 'static>>,
    activate_with_modifiers:
        Option<Arc<dyn Fn(PointerModifiers) -> Message + Send + Sync + 'static>>,
    double_activate: Option<Arc<dyn Fn() -> Message + Send + Sync + 'static>>,
    secondary: Option<Arc<dyn Fn(Point) -> Message + Send + Sync + 'static>>,
    drag: Option<Arc<dyn Fn(DragHandleMessage) -> Message + Send + Sync + 'static>>,
    drop: Option<Arc<dyn Fn() -> Message + Send + Sync + 'static>>,
    hover_drop: Option<Arc<dyn Fn(Point) -> Message + Send + Sync + 'static>>,
}

impl<Message> InteractiveRowActions<Message> {
    /// Build an empty row-action router.
    pub fn new() -> Self {
        Self {
            activate: None,
            activate_with_modifiers: None,
            double_activate: None,
            secondary: None,
            drag: None,
            drop: None,
            hover_drop: None,
        }
    }

    /// Emit a host message for single primary activation.
    pub fn activate(mut self, message: impl Fn() -> Message + Send + Sync + 'static) -> Self {
        self.activate = Some(Arc::new(message));
        self
    }

    /// Emit a single-activation message for one host-owned row key.
    pub fn activate_key<Key>(
        mut self,
        key: Key,
        message: impl Fn(Key) -> Message + Send + Sync + 'static,
    ) -> Self
    where
        Key: Clone + Send + Sync + 'static,
    {
        self.activate = Some(Arc::new(move || message(key.clone())));
        self
    }

    /// Emit a host message for single primary activation.
    pub fn primary(self, message: impl Fn() -> Message + Send + Sync + 'static) -> Self {
        self.activate(message)
    }

    /// Emit a primary-activation message for one host-owned row key.
    pub fn primary_key<Key>(
        self,
        key: Key,
        message: impl Fn(Key) -> Message + Send + Sync + 'static,
    ) -> Self
    where
        Key: Clone + Send + Sync + 'static,
    {
        self.activate_key(key, message)
    }

    /// Emit a host message for single primary activation with modifier state.
    pub fn activate_with_modifiers(
        mut self,
        message: impl Fn(PointerModifiers) -> Message + Send + Sync + 'static,
    ) -> Self {
        self.activate_with_modifiers = Some(Arc::new(message));
        self
    }

    /// Emit a modifier-aware activation message for one host-owned row key.
    pub fn activate_with_modifiers_key<Key>(
        mut self,
        key: Key,
        message: impl Fn(Key, PointerModifiers) -> Message + Send + Sync + 'static,
    ) -> Self
    where
        Key: Clone + Send + Sync + 'static,
    {
        self.activate_with_modifiers =
            Some(Arc::new(move |modifiers| message(key.clone(), modifiers)));
        self
    }

    /// Emit a host message for single primary activation with modifier state.
    pub fn primary_with_modifiers(
        self,
        message: impl Fn(PointerModifiers) -> Message + Send + Sync + 'static,
    ) -> Self {
        self.activate_with_modifiers(message)
    }

    /// Emit a modifier-aware primary-activation message for one host-owned row key.
    pub fn primary_with_modifiers_key<Key>(
        self,
        key: Key,
        message: impl Fn(Key, PointerModifiers) -> Message + Send + Sync + 'static,
    ) -> Self
    where
        Key: Clone + Send + Sync + 'static,
    {
        self.activate_with_modifiers_key(key, message)
    }

    /// Emit a host message for double primary activation.
    pub fn double_activate(
        mut self,
        message: impl Fn() -> Message + Send + Sync + 'static,
    ) -> Self {
        self.double_activate = Some(Arc::new(message));
        self
    }

    /// Emit a double-activation message for one host-owned row key.
    pub fn double_activate_key<Key>(
        mut self,
        key: Key,
        message: impl Fn(Key) -> Message + Send + Sync + 'static,
    ) -> Self
    where
        Key: Clone + Send + Sync + 'static,
    {
        self.double_activate = Some(Arc::new(move || message(key.clone())));
        self
    }

    /// Emit a host message for double primary activation.
    pub fn double(self, message: impl Fn() -> Message + Send + Sync + 'static) -> Self {
        self.double_activate(message)
    }

    /// Emit a double-activation message for one host-owned row key.
    pub fn double_key<Key>(
        self,
        key: Key,
        message: impl Fn(Key) -> Message + Send + Sync + 'static,
    ) -> Self
    where
        Key: Clone + Send + Sync + 'static,
    {
        self.double_activate_key(key, message)
    }

    /// Emit a host message for secondary activation.
    pub fn secondary(mut self, message: impl Fn(Point) -> Message + Send + Sync + 'static) -> Self {
        self.secondary = Some(Arc::new(message));
        self
    }

    /// Emit a secondary-activation message for one host-owned row key.
    pub fn secondary_key<Key>(
        mut self,
        key: Key,
        message: impl Fn(Key, Point) -> Message + Send + Sync + 'static,
    ) -> Self
    where
        Key: Clone + Send + Sync + 'static,
    {
        self.secondary = Some(Arc::new(move |position| message(key.clone(), position)));
        self
    }

    /// Emit primary and secondary activation messages for one host-owned row key.
    ///
    /// Use this when the same row, badge, chip, or tree item key routes normal
    /// activation and context-menu activation to separate host message shapes.
    pub fn primary_secondary_key<Key>(
        mut self,
        key: Key,
        primary_message: impl Fn(Key) -> Message + Send + Sync + 'static,
        secondary_message: impl Fn(Key, Point) -> Message + Send + Sync + 'static,
    ) -> Self
    where
        Key: Clone + Send + Sync + 'static,
    {
        let primary_key = key.clone();
        self.activate = Some(Arc::new(move || primary_message(primary_key.clone())));
        self.secondary = Some(Arc::new(move |position| {
            secondary_message(key.clone(), position)
        }));
        self
    }

    /// Emit a host message for drag lifecycle updates.
    pub fn drag(
        mut self,
        message: impl Fn(DragHandleMessage) -> Message + Send + Sync + 'static,
    ) -> Self {
        self.drag = Some(Arc::new(message));
        self
    }

    /// Emit drag lifecycle messages for one host-owned row key.
    pub fn drag_key<Key>(
        mut self,
        key: Key,
        message: impl Fn(Key, DragHandleMessage) -> Message + Send + Sync + 'static,
    ) -> Self
    where
        Key: Clone + Send + Sync + 'static,
    {
        self.drag = Some(Arc::new(move |drag| message(key.clone(), drag)));
        self
    }

    /// Emit a host message when a drop lands on the row.
    pub fn drop(mut self, message: impl Fn() -> Message + Send + Sync + 'static) -> Self {
        self.drop = Some(Arc::new(message));
        self
    }

    /// Emit a drop message for one host-owned row key.
    pub fn drop_key<Key>(
        mut self,
        key: Key,
        message: impl Fn(Key) -> Message + Send + Sync + 'static,
    ) -> Self
    where
        Key: Clone + Send + Sync + 'static,
    {
        self.drop = Some(Arc::new(move || message(key.clone())));
        self
    }

    /// Emit a host message when another row drag hovers this drop target.
    pub fn hover_drop(
        mut self,
        message: impl Fn(Point) -> Message + Send + Sync + 'static,
    ) -> Self {
        self.hover_drop = Some(Arc::new(message));
        self
    }

    /// Emit a hover-drop message for one host-owned row key.
    pub fn hover_drop_key<Key>(
        mut self,
        key: Key,
        message: impl Fn(Key, Point) -> Message + Send + Sync + 'static,
    ) -> Self
    where
        Key: Clone + Send + Sync + 'static,
    {
        self.hover_drop = Some(Arc::new(move |position| message(key.clone(), position)));
        self
    }

    /// Emit drop and hover-drop messages for one host-owned target key.
    ///
    /// Use this when a row, chip, tree item, lane, or layer routes both the
    /// eventual drop and live hover-target update through the same host-owned
    /// identity.
    pub fn drop_target_key<Key>(
        mut self,
        key: Key,
        drop_message: impl Fn(Key) -> Message + Send + Sync + 'static,
        hover_drop_message: impl Fn(Key, Point) -> Message + Send + Sync + 'static,
    ) -> Self
    where
        Key: Clone + Send + Sync + 'static,
    {
        let drop_key = key.clone();
        self.drop = Some(Arc::new(move || drop_message(drop_key.clone())));
        self.hover_drop = Some(Arc::new(move |position| {
            hover_drop_message(key.clone(), position)
        }));
        self
    }

    /// Route a generic row interaction into the configured host action.
    pub fn route(&self, message: InteractiveRowMessage) -> Option<Message> {
        if let Some(position) = message.secondary_position() {
            return self.secondary.as_ref().map(|callback| callback(position));
        }
        if let Some(drag) = message.drag_message() {
            return self.drag.as_ref().map(|callback| callback(drag));
        }
        if message.is_drop() {
            return self.drop.as_ref().map(|callback| callback());
        }
        if let Some(position) = message.hover_drop_position() {
            return self.hover_drop.as_ref().map(|callback| callback(position));
        }
        if message.is_double_activation() {
            return self.double_activate.as_ref().map(|callback| callback());
        }
        if let Some(modifiers) = message.single_activation_modifiers() {
            if let Some(callback) = &self.activate_with_modifiers {
                return Some(callback(modifiers));
            }
            return self.activate.as_ref().map(|callback| callback());
        }
        None
    }
}

impl<Message> Default for InteractiveRowActions<Message> {
    fn default() -> Self {
        Self::new()
    }
}

impl<Message> std::fmt::Debug for InteractiveRowActions<Message> {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("InteractiveRowActions")
            .finish_non_exhaustive()
    }
}
