use super::InteractiveRowActions;
use crate::{gui::types::Point, widgets::interaction::DragHandleMessage};
use std::sync::Arc;

impl<Message> InteractiveRowActions<Message> {
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

    /// Emit a host message when a tracked drop target should be cleared.
    pub fn clear_drop(
        mut self,
        message: impl Fn(Point) -> Message + Send + Sync + 'static,
    ) -> Self {
        self.clear_drop = Some(Arc::new(message));
        self
    }

    /// Emit a drop-target clear message for one host-owned row key.
    pub fn clear_drop_key<Key>(
        mut self,
        key: Key,
        message: impl Fn(Key, Point) -> Message + Send + Sync + 'static,
    ) -> Self
    where
        Key: Clone + Send + Sync + 'static,
    {
        self.clear_drop = Some(Arc::new(move |position| message(key.clone(), position)));
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

    /// Emit drop, hover, and tracked-target clear messages for one target key.
    ///
    /// Use this with tracked drop-candidate rows when the generic row primitive
    /// owns the hover lifecycle and the host only needs domain messages for
    /// target entry, target clearing, and final drop.
    pub fn tracked_drop_candidate_key<Key>(
        mut self,
        key: Key,
        drop_message: impl Fn(Key) -> Message + Send + Sync + 'static,
        hover_drop_message: impl Fn(Key, Point) -> Message + Send + Sync + 'static,
        clear_drop_message: impl Fn(Key, Point) -> Message + Send + Sync + 'static,
    ) -> Self
    where
        Key: Clone + Send + Sync + 'static,
    {
        let drop_key = key.clone();
        let hover_key = key.clone();
        self.drop = Some(Arc::new(move || drop_message(drop_key.clone())));
        self.hover_drop = Some(Arc::new(move |position| {
            hover_drop_message(hover_key.clone(), position)
        }));
        self.clear_drop = Some(Arc::new(move |position| {
            clear_drop_message(key.clone(), position)
        }));
        self
    }
}
