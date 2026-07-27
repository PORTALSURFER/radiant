use super::{InteractiveRowActions, InteractiveRowLocalActions};
use crate::gui::types::Point;
use std::{rc::Rc, sync::Arc};

impl<Message> InteractiveRowActions<Message> {
    /// Emit a host message when pointer hover moves over this row.
    pub fn hover(mut self, message: impl Fn(Point) -> Message + Send + Sync + 'static) -> Self {
        self.router.hover = Some(Arc::new(message));
        self
    }

    /// Emit a hover message for one host-owned row key.
    pub fn hover_key<Key>(
        mut self,
        key: Key,
        message: impl Fn(Key, Point) -> Message + Send + Sync + 'static,
    ) -> Self
    where
        Key: Clone + Send + Sync + 'static,
    {
        self.router.hover = Some(Arc::new(move |position| message(key.clone(), position)));
        self
    }

    /// Emit a host message for secondary activation.
    pub fn secondary(mut self, message: impl Fn(Point) -> Message + Send + Sync + 'static) -> Self {
        self.router.secondary = Some(Arc::new(message));
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
        self.router.secondary = Some(Arc::new(move |position| message(key.clone(), position)));
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
        self.router.activate = Some(Arc::new(move |_| primary_message(primary_key.clone())));
        self.router.secondary = Some(Arc::new(move |position| {
            secondary_message(key.clone(), position)
        }));
        self
    }
}

impl<Message> InteractiveRowLocalActions<Message> {
    /// Emit a UI-local message for pointer hover.
    pub fn hover(mut self, message: impl Fn(Point) -> Message + 'static) -> Self {
        self.router.hover = Some(Rc::new(message));
        self
    }

    /// Emit a UI-local hover message for one row key.
    pub fn hover_key<Key>(
        mut self,
        key: Key,
        message: impl Fn(Key, Point) -> Message + 'static,
    ) -> Self
    where
        Key: Clone + 'static,
    {
        self.router.hover = Some(Rc::new(move |position| message(key.clone(), position)));
        self
    }

    /// Emit a UI-local message for secondary activation.
    pub fn secondary(mut self, message: impl Fn(Point) -> Message + 'static) -> Self {
        self.router.secondary = Some(Rc::new(message));
        self
    }

    /// Emit a UI-local secondary-activation message for one row key.
    pub fn secondary_key<Key>(
        mut self,
        key: Key,
        message: impl Fn(Key, Point) -> Message + 'static,
    ) -> Self
    where
        Key: Clone + 'static,
    {
        self.router.secondary = Some(Rc::new(move |position| message(key.clone(), position)));
        self
    }

    /// Emit UI-local primary and secondary activation messages for one row key.
    pub fn primary_secondary_key<Key>(
        mut self,
        key: Key,
        primary_message: impl Fn(Key) -> Message + 'static,
        secondary_message: impl Fn(Key, Point) -> Message + 'static,
    ) -> Self
    where
        Key: Clone + 'static,
    {
        let primary_key = key.clone();
        self.router.activate = Some(Rc::new(move |_| primary_message(primary_key.clone())));
        self.router.secondary = Some(Rc::new(move |position| {
            secondary_message(key.clone(), position)
        }));
        self
    }
}
