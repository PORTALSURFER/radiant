//! Host-message mapping for surface widget leaves.

use crate::widgets::WidgetOutput;
use std::sync::Arc;

/// Shared mapper type that turns widget-specific payloads into host-defined messages.
pub type MessageMapper<Input, Message> = Arc<dyn Fn(Input) -> Message + Send + Sync>;

/// Message bindings that turn widget output payloads into host-defined messages.
#[derive(Default)]
pub struct WidgetMessageMapper<Message> {
    map: Option<Arc<dyn Fn(WidgetOutput) -> Option<Message> + Send + Sync>>,
}

impl<Message> Clone for WidgetMessageMapper<Message> {
    fn clone(&self) -> Self {
        Self {
            map: self.map.as_ref().map(Arc::clone),
        }
    }
}

impl<Message> WidgetMessageMapper<Message> {
    /// Build a mapper that does not emit host-defined messages.
    pub fn none() -> Self {
        Self { map: None }
    }

    /// Build a mapper for any typed widget output payload.
    pub fn typed<Output>(map: impl Fn(Output) -> Message + Send + Sync + 'static) -> Self
    where
        Output: Clone + Send + Sync + 'static,
    {
        Self::dynamic(move |output| output.typed_cloned::<Output>().map(&map))
    }

    /// Build a dynamic output mapper for custom widgets.
    pub fn dynamic(map: impl Fn(WidgetOutput) -> Option<Message> + Send + Sync + 'static) -> Self {
        Self {
            map: Some(Arc::new(map)),
        }
    }

    pub(super) fn maps_any_output(&self) -> bool {
        self.map.is_some()
    }

    pub(super) fn map_output(&self, output: WidgetOutput) -> Option<Message> {
        self.map.as_ref().and_then(|map| map(output))
    }
}
