use super::with_view::StatefulAppWithView;
use crate::{
    application::AppBridgeLifecycle,
    application::launch::IntoView,
    gui_runtime::{EmbeddedFont, NativePopupOptions, NativeRunOptions, ProfilingOptions},
    runtime::DevtoolsOverlayOptions,
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

    /// Configure fixed-cost native frame profiling for this app window.
    pub fn profiling(mut self, profiling: ProfilingOptions) -> Self {
        self.options.frame.profiling = profiling;
        self
    }

    /// Configure the runtime-local devtools inspector overlay for this app window.
    pub fn devtools_overlay(mut self, options: DevtoolsOverlayOptions) -> Self {
        self.options.frame.devtools = options;
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
            application_environment_source: None,
            component_environment_source: None,
            _message: PhantomData,
            _view: PhantomData,
        }
    }

    /// Attach an opt-in view with bounded reuse of pure component projections.
    ///
    /// The environment source must be a pure function of state. It supplies
    /// the same application snapshot to component functions and the runtime;
    /// window changes also invalidate component reuse automatically. Each
    /// component passes its state/theme/resource dependencies as exact inputs
    /// to [`crate::application::ComponentProjectionContext::project`].
    /// The ordinary application view and runtime refresh still execute.
    pub fn view_with_components<Message: 'static, Environment, Project>(
        self,
        environment_source: Environment,
        mut project: Project,
    ) -> StatefulAppWithView<
        State,
        Message,
        impl FnMut(&State) -> crate::application::View<Message>,
        crate::application::View<Message>,
    >
    where
        Environment: Fn(&State) -> crate::application::ApplicationEnvironment + 'static,
        Project: FnMut(
            &State,
            &mut crate::application::ComponentProjectionContext<'_, Message>,
        ) -> crate::application::View<Message>,
    {
        let source: Rc<dyn Fn(&State) -> crate::application::ApplicationEnvironment> =
            Rc::new(environment_source);
        let shared_source = Rc::new(RefCell::new(Rc::clone(&source)));
        let projection_source = Rc::clone(&shared_source);
        let mut cache =
            crate::application::view_node::components::ComponentProjectionCache::default();
        let mut builder = self.view_with_context(move |state, window| {
            let environment = crate::runtime::ResolvedEnvironment::from_snapshots(
                *window,
                std::sync::Arc::new((projection_source.borrow())(state)),
            );
            let mut context = cache.begin(environment);
            let view = project(state, &mut context);
            context.finish();
            view
        });
        builder.application_environment_source = Some(source);
        builder.component_environment_source = Some(shared_source);
        builder
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
            application_environment_source: None,
            component_environment_source: None,
            _message: PhantomData,
            _view: PhantomData,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        application::{
            ApplicationEnvironment, IntoView, LocaleId, TextCatalog, TextKey, TextScale,
            WritingDirection, text,
        },
        gui::shortcuts::{
            ShortcutDisplaySpec, ShortcutGesture, ShortcutKeyLabel, ShortcutPlatform,
            ShortcutPresenter,
        },
        runtime::{RepaintScope, SurfaceInvalidation, SurfaceRuntime, WindowColorScheme},
        theme::DpiScale,
    };
    use std::{
        cell::Cell,
        rc::Rc,
        sync::{Arc, Mutex},
    };

    #[test]
    fn devtools_overlay_builder_preserves_default_and_enabled_option() {
        let default = StatefulAppBuilder::new(());
        assert_eq!(
            default.options.frame.devtools,
            DevtoolsOverlayOptions::disabled()
        );

        let enabled =
            StatefulAppBuilder::new(()).devtools_overlay(DevtoolsOverlayOptions::enabled());
        assert_eq!(
            enabled.options.frame.devtools,
            DevtoolsOverlayOptions::enabled()
        );
    }

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

    #[test]
    fn app_bridge_projects_locale_snapshot_and_promotes_weak_refresh_scope() {
        let french = Rc::new(Cell::new(false));
        let projection_count = Rc::new(Cell::new(0));
        let projection_count_for_projection = Rc::clone(&projection_count);
        let french_for_projection = Rc::clone(&french);
        let french_for_environment = Rc::clone(&french);
        let app = StatefulAppBuilder::new(())
            .view(move |_| {
                projection_count_for_projection.set(projection_count_for_projection.get() + 1);
                let fr = LocaleId::new("fr").expect("valid locale");
                let locale = if french_for_projection.get() {
                    fr.clone()
                } else {
                    LocaleId::english()
                };
                let key = TextKey::new("save", "Save");
                let catalog = TextCatalog::default().insert(fr, key.clone(), "Enregistrer");
                let environment = ApplicationEnvironment::new(locale)
                    .with_catalog(Arc::new(catalog))
                    .with_text_scale(TextScale::new(1.0).expect("valid scale"))
                    .with_writing_direction(WritingDirection::Ltr);
                let localized = environment.localized(&key);
                let visible = localized.to_content();
                assert_eq!(localized.as_str(), visible.as_str());
                text::<()>(visible)
                    .into_projection()
                    .with_application_environment(environment)
            })
            .application_environment(move |_| {
                let fr = LocaleId::new("fr").expect("valid locale");
                let locale = if french_for_environment.get() {
                    fr.clone()
                } else {
                    LocaleId::english()
                };
                let key = TextKey::new("save", "Save");
                let catalog = TextCatalog::default().insert(fr, key, "Enregistrer");
                ApplicationEnvironment::new(locale).with_catalog(Arc::new(catalog))
            })
            .into_bridge();
        let mut runtime = SurfaceRuntime::new(app, crate::gui::types::Vector2::new(100.0, 60.0));
        assert_eq!(projection_count.get(), 1);

        french.set(true);
        runtime.refresh_with_scope(RepaintScope::PaintOnly);

        assert_eq!(projection_count.get(), 2);
        let paint = runtime.paint_plan(&crate::theme::ThemeTokens::default());
        assert!(paint.first_text_run("Enregistrer").is_some());
        fn has_label(node: &crate::gui::automation::AutomationNodeSnapshot, label: &str) -> bool {
            node.label.as_deref() == Some(label)
                || node.children.iter().any(|child| has_label(child, label))
        }
        assert!(has_label(
            &runtime.automation_snapshot().root,
            "Enregistrer"
        ));

        assert_eq!(
            runtime.last_refresh_diagnostics().invalidation,
            SurfaceInvalidation::Surface
        );
        assert_eq!(
            runtime.context().application_environment().fallback_chain()[0].as_str(),
            "fr"
        );

        let shortcut =
            ShortcutPresenter::new(ShortcutPlatform::Mac).present(&ShortcutDisplaySpec::new(
                ShortcutGesture::with_command(crate::gui::input::KeyCode::S),
                ShortcutKeyLabel::character('S'),
            ));
        assert_eq!(shortcut.compact_text(), "⌘S");
        assert_eq!(shortcut.spoken_text(), "Command+S");

        runtime.refresh_with_scope(RepaintScope::PaintOnly);
        assert_eq!(projection_count.get(), 2);
    }

    #[test]
    fn app_bridge_same_locale_catalog_replacement_promotes_paint_only_refresh() {
        fn environment(updated: bool) -> ApplicationEnvironment {
            let key = TextKey::new("status", "Initial");
            let catalog = TextCatalog::default().insert(
                LocaleId::english(),
                key,
                if updated { "Updated" } else { "Initial" },
            );
            ApplicationEnvironment::new(LocaleId::english()).with_catalog(Arc::new(catalog))
        }

        let updated = Rc::new(Cell::new(false));
        let projection_count = Rc::new(Cell::new(0));
        let updated_for_view = Rc::clone(&updated);
        let updated_for_environment = Rc::clone(&updated);
        let projection_count_for_view = Rc::clone(&projection_count);
        let app = StatefulAppBuilder::new(())
            .view(move |_| {
                projection_count_for_view.set(projection_count_for_view.get() + 1);
                let key = TextKey::new("status", "Initial");
                let localized = environment(updated_for_view.get()).localized(&key);
                text::<()>(localized.to_content())
            })
            .application_environment(move |_| environment(updated_for_environment.get()))
            .into_bridge();
        let mut runtime = SurfaceRuntime::new(app, crate::gui::types::Vector2::new(160.0, 40.0));
        let initial_counters = runtime.refresh_counters();
        assert_eq!(projection_count.get(), 1);
        assert_eq!(initial_counters.application_projection, 1);
        assert!(
            runtime
                .paint_plan(&crate::theme::ThemeTokens::default())
                .first_text_run("Initial")
                .is_some()
        );

        updated.set(true);
        runtime.refresh_with_scope(RepaintScope::PaintOnly);

        let changed_counters = runtime.refresh_counters();
        assert_eq!(projection_count.get(), 2);
        assert_eq!(
            changed_counters.application_projection,
            initial_counters.application_projection + 1
        );
        assert_eq!(
            runtime.last_refresh_diagnostics().invalidation,
            SurfaceInvalidation::Surface
        );
        assert!(
            runtime
                .paint_plan(&crate::theme::ThemeTokens::default())
                .first_text_run("Updated")
                .is_some()
        );
        fn has_label(node: &crate::gui::automation::AutomationNodeSnapshot, label: &str) -> bool {
            node.label.as_deref() == Some(label)
                || node.children.iter().any(|child| has_label(child, label))
        }
        assert!(has_label(&runtime.automation_snapshot().root, "Updated"));

        runtime.refresh_with_scope(RepaintScope::PaintOnly);
        assert_eq!(projection_count.get(), 2);
        assert_eq!(runtime.refresh_counters(), changed_counters);
        assert_eq!(
            runtime.last_refresh_diagnostics().invalidation,
            SurfaceInvalidation::PaintOnly
        );
    }
}
