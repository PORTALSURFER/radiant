use crate::{gui_runtime::NativeRunOptions, runtime::UiSurface};
use std::sync::Arc;

/// Native close handling policy for an auxiliary window.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum AuxiliaryWindowClosePolicy {
    /// Destroy the auxiliary window runner when the OS close button is used.
    #[default]
    Destroy,
    /// Hide the auxiliary window runner so a later projection with the same key
    /// can reuse the already-created native window and renderer state.
    Hide,
}

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
    /// Native close handling policy for this auxiliary window.
    pub close_policy: AuxiliaryWindowClosePolicy,
}

impl<Message> AuxiliaryWindow<Message> {
    /// Construct a common secondary utility-window projection.
    ///
    /// Use [`Self::new`] with explicit [`NativeRunOptions`] when the window
    /// requires advanced native configuration.
    pub fn utility(
        key: impl Into<String>,
        title: impl Into<String>,
        width: f32,
        height: f32,
        surface: Arc<UiSurface<Message>>,
    ) -> Self {
        Self::new(
            key,
            NativeRunOptions::utility_window(title, width, height),
            surface,
        )
    }

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
            close_policy: AuxiliaryWindowClosePolicy::default(),
        }
    }

    /// Dispatch the given message when the operating system closes this window.
    pub fn on_close(mut self, message: Message) -> Self {
        self.close_message = Some(message);
        self
    }

    /// Hide and retain the native window when the operating system closes it.
    ///
    /// This is useful for settings panels, inspectors, and other frequently
    /// reopened secondary windows where startup cost should be paid once.
    pub fn cache_on_close(mut self) -> Self {
        self.close_policy = AuxiliaryWindowClosePolicy::Hide;
        self
    }

    /// Return whether this auxiliary window should be hidden instead of
    /// destroyed when the operating system asks to close it.
    pub fn caches_on_close(&self) -> bool {
        self.close_policy == AuxiliaryWindowClosePolicy::Hide
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{application::empty, prelude::IntoView};

    fn empty_surface() -> Arc<UiSurface<()>> {
        Arc::new(empty().into_surface())
    }

    #[test]
    fn auxiliary_windows_destroy_on_close_by_default() {
        let window: AuxiliaryWindow<()> =
            AuxiliaryWindow::new("settings", NativeRunOptions::default(), empty_surface());

        assert_eq!(window.close_policy, AuxiliaryWindowClosePolicy::Destroy);
        assert!(!window.caches_on_close());
    }

    #[test]
    fn auxiliary_windows_can_cache_on_close() {
        let window: AuxiliaryWindow<()> =
            AuxiliaryWindow::new("settings", NativeRunOptions::default(), empty_surface())
                .cache_on_close();

        assert_eq!(window.close_policy, AuxiliaryWindowClosePolicy::Hide);
        assert!(window.caches_on_close());
    }
}
