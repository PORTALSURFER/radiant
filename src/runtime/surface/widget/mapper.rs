//! Host-message mapping for surface widget leaves.

use crate::{
    runtime::{NativeFileDrop, ScrollUpdate},
    widgets::WidgetOutput,
};
use std::sync::Arc;

/// Shared mapper type that turns widget-specific payloads into host-defined messages.
pub type MessageMapper<Input, Message> = Arc<dyn Fn(Input) -> Message + Send + Sync>;

/// Shared mapper type that turns scroll movement into optional host-defined messages.
///
/// Scroll containers may update local runtime offset for sub-row or otherwise
/// unchanged movement without asking the host to reproject the surface.
pub type ScrollMessageMapper<Message> = Arc<dyn Fn(ScrollUpdate) -> Option<Message> + Send + Sync>;

/// Shared mapper type that turns native file-drop events into host-defined messages.
pub type NativeFileDropMessageMapper<Message> = MessageMapper<NativeFileDrop, Message>;

/// Message bindings that turn widget output payloads into host-defined messages.
#[derive(Default)]
pub struct WidgetMessageMapper<Message> {
    map: Option<Arc<dyn Fn(WidgetOutput) -> Option<Message> + Send + Sync>>,
    native_file_drop: Option<NativeFileDropMessageMapper<Message>>,
}

impl<Message> Clone for WidgetMessageMapper<Message> {
    fn clone(&self) -> Self {
        Self {
            map: self.map.as_ref().map(Arc::clone),
            native_file_drop: self.native_file_drop.as_ref().map(Arc::clone),
        }
    }
}

impl<Message> WidgetMessageMapper<Message> {
    /// Build a mapper that does not emit host-defined messages.
    pub fn none() -> Self {
        Self {
            map: None,
            native_file_drop: None,
        }
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
            native_file_drop: None,
        }
    }

    /// Return this mapper with native file-drop events mapped to host messages.
    pub fn with_native_file_drop(
        mut self,
        map: impl Fn(NativeFileDrop) -> Message + Send + Sync + 'static,
    ) -> Self {
        self.native_file_drop = Some(Arc::new(map));
        self
    }

    pub(super) fn maps_any_output(&self) -> bool {
        self.map.is_some()
    }

    pub(super) fn map_output(&self, output: WidgetOutput) -> Option<Message> {
        self.map.as_ref().and_then(|map| map(output))
    }

    pub(super) fn map_native_file_drop(&self, drop: NativeFileDrop) -> Option<Message> {
        self.native_file_drop.as_ref().map(|map| map(drop))
    }
}
