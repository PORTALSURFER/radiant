use crate::{gui_runtime::NativeRunOptions, runtime::UiSurface};
use std::sync::Arc;

/// One auxiliary OS window surface projected by an application runtime.
pub struct AuxiliaryWindow<Message> {
    /// Stable application-owned window key.
    pub key: String,
    /// Native launch and ownership options for the auxiliary window.
    pub options: NativeRunOptions,
    /// Current declarative surface for this window.
    pub surface: Arc<UiSurface<Message>>,
    /// Message dispatched when the operating system asks to close the window.
    pub close_message: Option<Message>,
}

impl<Message> AuxiliaryWindow<Message> {
    /// Construct one auxiliary window projection.
    pub fn new(
        key: impl Into<String>,
        options: NativeRunOptions,
        surface: Arc<UiSurface<Message>>,
    ) -> Self {
        Self {
            key: key.into(),
            options,
            surface,
            close_message: None,
        }
    }

    /// Dispatch the given message when the operating system closes this window.
    pub fn on_close(mut self, message: Message) -> Self {
        self.close_message = Some(message);
        self
    }
}
