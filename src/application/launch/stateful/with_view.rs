use super::runnable::RunnableStatefulApp;
use crate::{
    application::{
        AppBridge, AppBridgeLifecycle, AppUpdate, Result, UiUpdateContext, launch::IntoView,
    },
    gui_runtime::NativeRunOptions,
    runtime::{RuntimeBridge, run_native_vello_runtime},
};
use std::marker::PhantomData;
use std::{cell::RefCell, rc::Rc};

/// Stateful app builder after a view projection has been supplied.
pub struct StatefulAppWithView<State, Message, Project, View> {
    pub(super) state: State,
    pub(super) options: NativeRunOptions,
    pub(super) project: Project,
    pub(super) lifecycle: AppBridgeLifecycle<State, Message>,
    pub(super) window_environment: Option<Rc<RefCell<crate::runtime::WindowEnvironment>>>,
    pub(super) application_environment_source:
        Option<crate::application::runtime::ApplicationEnvironmentSource<State>>,
    pub(super) _message: PhantomData<Message>,
    pub(super) _view: PhantomData<View>,
}

impl<State, Message, Project, View> StatefulAppWithView<State, Message, Project, View> {
    /// Attach a cheap application presentation snapshot source.
    ///
    /// The source is consulted before a refresh chooses its repaint scope, so
    /// application environment changes cannot be hidden by a paint-only
    /// request and unchanged snapshots do not force projection.
    pub fn application_environment<Source>(mut self, source: Source) -> Self
    where
        Source: Fn(&State) -> crate::application::ApplicationEnvironment + 'static,
    {
        self.application_environment_source = Some(Box::new(source));
        self
    }
}

impl<State, Project, View> StatefulAppWithView<State, (), Project, View>
where
    Project: FnMut(&State) -> View + 'static,
    View: IntoView<()> + 'static,
    State: 'static,
{
    /// Run this static-message app through the native Vello runtime.
    pub fn run(self) -> Result {
        let options = self.options.clone();
        run_native_vello_runtime(options, self.into_bridge())
    }

    /// Run this static-message app and return native runtime artifacts.
    pub fn run_with_artifacts(self) -> crate::gui_runtime::NativeGenericRunReport {
        let options = self.options.clone();
        crate::runtime::run_native_vello_runtime_with_artifacts(options, self.into_bridge())
    }

    /// Lower this static-message app into the runtime bridge without opening a window.
    pub fn into_bridge(self) -> impl RuntimeBridge<()> {
        AppBridge::new_with_window_environment(
            self.state,
            self.project,
            |_: &mut State, (): (), context: &mut UiUpdateContext<()>| {
                context.request_repaint();
            },
            self.lifecycle,
            self.window_environment,
            self.application_environment_source,
        )
    }
}

impl<State, Message, Project, View> StatefulAppWithView<State, Message, Project, View>
where
    Project: FnMut(&State) -> View + 'static,
    View: IntoView<Message> + 'static,
    Message: 'static,
    State: 'static,
{
    /// Attach a simple app message handler that mutates app state and requests a repaint.
    pub fn update<Update>(
        self,
        mut update: Update,
    ) -> RunnableStatefulApp<State, Message, Project, AppUpdate<State, Message>, View>
    where
        Update: FnMut(&mut State, Message) + 'static,
    {
        self.handle_message(Box::new(move |state, message, context| {
            update(state, message);
            context.request_repaint();
        }))
    }

    /// Attach an app message handler that can queue UI-safe runtime follow-up work.
    pub fn handle_message<Update>(
        self,
        update: Update,
    ) -> RunnableStatefulApp<State, Message, Project, Update, View>
    where
        Update: FnMut(&mut State, Message, &mut UiUpdateContext<Message>) + 'static,
    {
        RunnableStatefulApp {
            state: self.state,
            options: self.options,
            project: self.project,
            update,
            lifecycle: self.lifecycle,
            window_environment: self.window_environment,
            application_environment_source: self.application_environment_source,
            _message: PhantomData,
            _view: PhantomData,
        }
    }
}
