use super::{lifecycle::StatefulLifecycle, with_view::StatefulAppWithView};
use crate::{
    application::launch::IntoView,
    gui_runtime::{EmbeddedFont, NativePopupOptions, NativeRunOptions},
};
use std::{marker::PhantomData, path::PathBuf};

/// Initial builder for simple stateful Radiant apps.
pub struct StatefulAppBuilder<State> {
    pub(super) state: State,
    pub(super) options: NativeRunOptions,
}

impl<State> StatefulAppBuilder<State> {
    pub(in crate::application::launch) fn new(state: State) -> Self {
        Self {
            state,
            options: NativeRunOptions::default(),
        }
    }

    /// Set the native window title.
    pub fn title(mut self, title: impl Into<String>) -> Self {
        self.options.title = title.into();
        self
    }

    /// Set the initial logical window size.
    pub fn size(self, width: u32, height: u32) -> Self {
        self.logical_size(width as f32, height as f32)
    }

    /// Set the initial logical window size using floating-point logical pixels.
    pub fn logical_size(mut self, width: f32, height: f32) -> Self {
        self.options.inner_size = Some([width, height]);
        self
    }

    /// Set the minimum logical window size.
    pub fn min_size(self, width: u32, height: u32) -> Self {
        self.min_logical_size(width as f32, height as f32)
    }

    /// Set the minimum logical window size using floating-point logical pixels.
    pub fn min_logical_size(mut self, width: f32, height: f32) -> Self {
        self.options.min_inner_size = Some([width, height]);
        self
    }

    /// Set the full native runtime options for apps that need explicit launch control.
    pub fn options(mut self, options: NativeRunOptions) -> Self {
        self.options = options;
        self
    }

    /// Configure this app window as a borderless floating popup.
    pub fn floating_popup(mut self) -> Self {
        self.options = self.options.floating_popup();
        self
    }

    /// Configure this app window as a floating popup with explicit policy.
    pub fn popup_policy(mut self, popup: NativePopupOptions) -> Self {
        self.options = self.options.popup_policy(popup);
        self
    }

    /// Set the initial popup position in logical screen coordinates.
    pub fn popup_position(mut self, x: f32, y: f32) -> Self {
        self.options = self.options.popup_position(x, y);
        self
    }

    /// Add embedded TTF/OTF font bytes checked before file and native fallback fonts.
    pub fn embedded_font(mut self, font: impl Into<EmbeddedFont>) -> Self {
        self.options.text.embedded_fonts.push(font.into());
        self
    }

    /// Add a preferred font file checked after embedded fonts and before native fallbacks.
    pub fn font_path(mut self, path: impl Into<PathBuf>) -> Self {
        self.options.text.font_paths.push(path.into());
        self
    }

    /// Attach a state projection closure.
    pub fn view<Message, Project, View>(
        self,
        project: Project,
    ) -> StatefulAppWithView<State, Message, Project, View>
    where
        Project: FnMut(&mut State) -> View,
        View: IntoView<Message>,
    {
        StatefulAppWithView {
            state: self.state,
            options: self.options,
            project,
            lifecycle: StatefulLifecycle::default(),
            _message: PhantomData,
            _view: PhantomData,
        }
    }
}
