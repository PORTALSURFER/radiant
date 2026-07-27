use super::{InteractiveRowActions, InteractiveRowLocalActions};
use crate::widgets::interaction::PointerModifiers;
use std::{rc::Rc, sync::Arc};

impl<Message> InteractiveRowActions<Message> {
    /// Emit a host message for single primary activation.
    pub fn activate(mut self, message: impl Fn() -> Message + Send + Sync + 'static) -> Self {
        self.router.activate = Some(Arc::new(move |_| message()));
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
        self.router.activate = Some(Arc::new(move |_| message(key.clone())));
        self
    }

    /// Alias for [`Self::activate`].
    pub fn primary(self, message: impl Fn() -> Message + Send + Sync + 'static) -> Self {
        self.activate(message)
    }

    /// Alias for [`Self::activate_key`].
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
        self.router.activate_with_modifiers = Some(Arc::new(message));
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
        self.router.activate_with_modifiers =
            Some(Arc::new(move |modifiers| message(key.clone(), modifiers)));
        self
    }

    /// Alias for [`Self::activate_with_modifiers`].
    pub fn primary_with_modifiers(
        self,
        message: impl Fn(PointerModifiers) -> Message + Send + Sync + 'static,
    ) -> Self {
        self.activate_with_modifiers(message)
    }

    /// Alias for [`Self::activate_with_modifiers_key`].
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
        self.router.activate_with_modifiers = Some(Arc::new(move |modifiers| {
            primary_message(primary_key.clone(), modifiers)
        }));
        self.router.double_activate = Some(Arc::new(move |_| double_message(key.clone())));
        self
    }

    /// Emit a host message for double primary activation.
    pub fn double_activate(
        mut self,
        message: impl Fn() -> Message + Send + Sync + 'static,
    ) -> Self {
        self.router.double_activate = Some(Arc::new(move |_| message()));
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
        self.router.double_activate = Some(Arc::new(move |_| message(key.clone())));
        self
    }

    /// Alias for [`Self::double_activate`].
    pub fn double(self, message: impl Fn() -> Message + Send + Sync + 'static) -> Self {
        self.double_activate(message)
    }

    /// Alias for [`Self::double_activate_key`].
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

impl<Message> InteractiveRowLocalActions<Message> {
    /// Emit a UI-local message for single primary activation.
    pub fn activate(mut self, message: impl Fn() -> Message + 'static) -> Self {
        self.router.activate = Some(Rc::new(move |_| message()));
        self
    }

    /// Emit a UI-local single-activation message for one row key.
    pub fn activate_key<Key>(mut self, key: Key, message: impl Fn(Key) -> Message + 'static) -> Self
    where
        Key: Clone + 'static,
    {
        self.router.activate = Some(Rc::new(move |_| message(key.clone())));
        self
    }

    /// Alias for [`Self::activate`].
    pub fn primary(self, message: impl Fn() -> Message + 'static) -> Self {
        self.activate(message)
    }

    /// Alias for [`Self::activate_key`].
    pub fn primary_key<Key>(self, key: Key, message: impl Fn(Key) -> Message + 'static) -> Self
    where
        Key: Clone + 'static,
    {
        self.activate_key(key, message)
    }

    /// Emit a UI-local modifier-aware primary activation message.
    pub fn activate_with_modifiers(
        mut self,
        message: impl Fn(PointerModifiers) -> Message + 'static,
    ) -> Self {
        self.router.activate_with_modifiers = Some(Rc::new(message));
        self
    }

    /// Emit a UI-local modifier-aware activation message for one row key.
    pub fn activate_with_modifiers_key<Key>(
        mut self,
        key: Key,
        message: impl Fn(Key, PointerModifiers) -> Message + 'static,
    ) -> Self
    where
        Key: Clone + 'static,
    {
        self.router.activate_with_modifiers =
            Some(Rc::new(move |modifiers| message(key.clone(), modifiers)));
        self
    }

    /// Alias for [`Self::activate_with_modifiers`].
    pub fn primary_with_modifiers(
        self,
        message: impl Fn(PointerModifiers) -> Message + 'static,
    ) -> Self {
        self.activate_with_modifiers(message)
    }

    /// Alias for [`Self::activate_with_modifiers_key`].
    pub fn primary_with_modifiers_key<Key>(
        self,
        key: Key,
        message: impl Fn(Key, PointerModifiers) -> Message + 'static,
    ) -> Self
    where
        Key: Clone + 'static,
    {
        self.activate_with_modifiers_key(key, message)
    }

    /// Emit modifier-aware primary and double-activation messages for one row key.
    pub fn primary_with_modifiers_and_double_key<Key>(
        mut self,
        key: Key,
        primary_message: impl Fn(Key, PointerModifiers) -> Message + 'static,
        double_message: impl Fn(Key) -> Message + 'static,
    ) -> Self
    where
        Key: Clone + 'static,
    {
        let primary_key = key.clone();
        self.router.activate_with_modifiers = Some(Rc::new(move |modifiers| {
            primary_message(primary_key.clone(), modifiers)
        }));
        self.router.double_activate = Some(Rc::new(move |_| double_message(key.clone())));
        self
    }

    /// Emit a UI-local message for double primary activation.
    pub fn double_activate(mut self, message: impl Fn() -> Message + 'static) -> Self {
        self.router.double_activate = Some(Rc::new(move |_| message()));
        self
    }

    /// Emit a UI-local double-activation message for one row key.
    pub fn double_activate_key<Key>(
        mut self,
        key: Key,
        message: impl Fn(Key) -> Message + 'static,
    ) -> Self
    where
        Key: Clone + 'static,
    {
        self.router.double_activate = Some(Rc::new(move |_| message(key.clone())));
        self
    }

    /// Alias for [`Self::double_activate`].
    pub fn double(self, message: impl Fn() -> Message + 'static) -> Self {
        self.double_activate(message)
    }

    /// Alias for [`Self::double_activate_key`].
    pub fn double_key<Key>(self, key: Key, message: impl Fn(Key) -> Message + 'static) -> Self
    where
        Key: Clone + 'static,
    {
        self.double_activate_key(key, message)
    }
}
