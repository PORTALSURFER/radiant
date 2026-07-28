use super::with_view::StatefulAppWithView;
use crate::{
    application::AppBridgeLifecycle,
    application::launch::IntoView,
    gui_runtime::{EmbeddedFont, NativePopupOptions, NativeRunOptions},
};
use std::{cell::RefCell, rc::Rc};
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
        self.options.window.title = title.into();
        self
    }

    /// Set the initial logical window size.
    pub fn size(self, width: u32, height: u32) -> Self {
        self.logical_size(width as f32, height as f32)
    }

    /// Set the initial logical window size using floating-point logical pixels.
    pub fn logical_size(mut self, width: f32, height: f32) -> Self {
        self.options.window.geometry.inner_size = Some([width, height]);
        self
    }

    /// Set the minimum logical window size.
    pub fn min_size(self, width: u32, height: u32) -> Self {
        self.min_logical_size(width as f32, height as f32)
    }

    /// Set the minimum logical window size using floating-point logical pixels.
    pub fn min_logical_size(mut self, width: f32, height: f32) -> Self {
        self.options.window.geometry.min_inner_size = Some([width, height]);
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

    /// Configure this app window as a prewarmed floating popup.
    pub fn prewarmed_popup(mut self, x: f32, y: f32) -> Self {
        self.options = self
            .options
            .popup_policy(NativePopupOptions::prewarmed_at(x, y));
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

    /// Attach an immutable state projection closure.
    pub fn view<Message, Project, View>(
        self,
        project: Project,
    ) -> StatefulAppWithView<State, Message, Project, View>
    where
        Project: FnMut(&State) -> View,
        View: IntoView<Message>,
    {
        StatefulAppWithView {
            state: self.state,
            options: self.options,
            project,
            lifecycle: AppBridgeLifecycle::default(),
            window_environment: None,
            _message: PhantomData,
            _view: PhantomData,
        }
    }

    /// Attach a projection that can opt into the current window environment.
    ///
    /// The existing [`Self::view`] closure remains unchanged; this additive
    /// method is for applications whose main surface needs scale, appearance,
    /// contrast, or motion preference during projection.
    pub fn view_with_context<Message, Project, View>(
        self,
        mut project: Project,
    ) -> StatefulAppWithView<State, Message, impl FnMut(&State) -> View, View>
    where
        Project: FnMut(&State, &crate::runtime::WindowEnvironment) -> View,
        View: IntoView<Message>,
    {
        let environment = Rc::new(RefCell::new(crate::runtime::WindowEnvironment::default()));
        let projection_environment = Rc::clone(&environment);
        let project = move |state: &State| {
            let environment = *projection_environment.borrow();
            project(state, &environment)
        };
        StatefulAppWithView {
            state: self.state,
            options: self.options,
            project,
            lifecycle: AppBridgeLifecycle::default(),
            window_environment: Some(environment),
            _message: PhantomData,
            _view: PhantomData,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        application::text,
        runtime::{SurfaceRuntime, WindowColorScheme},
        theme::DpiScale,
    };
    use std::sync::{Arc, Mutex};

    #[test]
    fn context_aware_projection_receives_updated_window_environment() {
        let seen = Arc::new(Mutex::new(Vec::new()));
        let seen_by_projection = Arc::clone(&seen);
        let app = StatefulAppBuilder::new(())
            .view_with_context(move |_, environment| {
                seen_by_projection
                    .lock()
                    .unwrap()
                    .push(environment.display_scale().factor());
                text::<()>("environment")
            })
            .into_bridge();
        let mut runtime = SurfaceRuntime::new(app, crate::gui::types::Vector2::new(100.0, 60.0));
        let environment = crate::runtime::WindowEnvironment::new(
            DpiScale::new(2.0),
            Some(WindowColorScheme::Dark),
            false,
            true,
        );
        assert!(runtime.set_window_environment(environment));
        runtime.refresh();

        assert_eq!(*seen.lock().unwrap(), vec![1.0, 2.0]);
    }
}
