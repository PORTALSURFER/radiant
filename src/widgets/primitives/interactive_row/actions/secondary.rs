use super::InteractiveRowActions;
use crate::gui::types::Point;
use std::sync::Arc;

impl<Message> InteractiveRowActions<Message> {
    /// Emit a host message when pointer hover moves over this row.
    pub fn hover(mut self, message: impl Fn(Point) -> Message + Send + Sync + 'static) -> Self {
        self.hover = Some(Arc::new(message));
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
        self.hover = Some(Arc::new(move |position| message(key.clone(), position)));
        self
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
}
