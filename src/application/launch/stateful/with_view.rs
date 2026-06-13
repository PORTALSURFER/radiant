use super::runnable::RunnableStatefulApp;
use crate::{
    application::{
        AppBridge, AppBridgeLifecycle, AppUpdate, Result, UiUpdateContext, launch::IntoView,
    },
    gui_runtime::NativeRunOptions,
    runtime::{RuntimeBridge, run_native_vello_runtime},
};
use std::marker::PhantomData;

/// Stateful app builder after a view projection has been supplied.
pub struct StatefulAppWithView<State, Message, Project, View> {
    pub(super) state: State,
    pub(super) options: NativeRunOptions,
    pub(super) project: Project,
    pub(super) lifecycle: AppBridgeLifecycle<State, Message>,
    pub(super) _message: PhantomData<Message>,
    pub(super) _view: PhantomData<View>,
}

impl<State, Project, View> StatefulAppWithView<State, (), Project, View>
where
    Project: FnMut(&mut State) -> View + 'static,
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
        AppBridge::new(
            self.state,
            self.project,
            |_: &mut State, (): (), context: &mut UiUpdateContext<()>| {
                context.request_repaint();
            },
            self.lifecycle,
        )
    }
}

impl<State, Message, Project, View> StatefulAppWithView<State, Message, Project, View>
where
    Project: FnMut(&mut State) -> View + 'static,
    View: IntoView<Message> + 'static,
    Message: Send + 'static,
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
            _message: PhantomData,
            _view: PhantomData,
        }
    }
}
