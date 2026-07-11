use super::InteractiveRowActions;
use crate::widgets::interaction::PointerModifiers;
use std::sync::Arc;

impl<Message> InteractiveRowActions<Message> {
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

    /// Emit modifier-aware primary activation and a separate double-activation for one row key.
    ///
    /// Use this for dense rows where a single click preserves modifier state
    /// for host-owned selection policy while a double click maps to a distinct
    /// host action such as opening, auditioning, or confirming the row.
    pub fn primary_with_modifiers_and_double_key<Key>(
        mut self,
        key: Key,
        primary_message: impl Fn(Key, PointerModifiers) -> Message + Send + Sync + 'static,
        double_message: impl Fn(Key) -> Message + Send + Sync + 'static,
    ) -> Self
    where
        Key: Clone + Send + Sync + 'static,
    {
        let primary_key = key.clone();
        self.activate_with_modifiers = Some(Arc::new(move |modifiers| {
            primary_message(primary_key.clone(), modifiers)
        }));
        self.double_activate = Some(Arc::new(move || double_message(key.clone())));
        self
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
}
