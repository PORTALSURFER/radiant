use super::runnable::RunnableStatefulApp;
use crate::{
    application::{
        AppBridge, AppBridgeLifecycle, AppUpdate, Result, StateAction, UpdateContext,
        launch::IntoView,
    },
    gui_runtime::NativeRunOptions,
    runtime::{Command, RuntimeBridge, run_native_vello_runtime},
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
            context.command(Command::request_repaint());
        }))
    }

    /// Attach an app message handler that can queue runtime-visible work through an update context.
    pub fn handle_message<Update>(
        self,
        update: Update,
    ) -> RunnableStatefulApp<State, Message, Project, Update, View>
    where
        Update: FnMut(&mut State, Message, &mut UpdateContext<Message>) + 'static,
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

    /// Compatibility alias for [`Self::handle_message`].
    ///
    /// Prefer [`Self::handle_message`] in app-facing code. The reducer name is
    /// still public for existing callers and tests that intentionally use the
    /// state-machine vocabulary.
    pub fn reducer<Update>(
        self,
        update: Update,
    ) -> RunnableStatefulApp<State, Message, Project, Update, View>
    where
        Update: FnMut(&mut State, Message, &mut UpdateContext<Message>) + 'static,
    {
        self.handle_message(update)
    }

    /// Advanced compatibility alias for [`Self::handle_message`].
    ///
    /// Prefer [`Self::handle_message`] for app-facing message handlers. This older name
    /// remains public for existing code and examples that intentionally model
    /// the lower-level update/context hook.
    pub fn update_with<Update>(
        self,
        update: Update,
    ) -> RunnableStatefulApp<State, Message, Project, Update, View>
    where
        Update: FnMut(&mut State, Message, &mut UpdateContext<Message>) + 'static,
    {
        self.handle_message(update)
    }

    /// Attach a simple app message handler that returns runtime-visible commands.
    pub fn update_command<Update>(
        self,
        mut update: Update,
    ) -> RunnableStatefulApp<State, Message, Project, AppUpdate<State, Message>, View>
    where
        Update: FnMut(&mut State, Message) -> Command<Message> + 'static,
    {
        self.handle_message(Box::new(move |state, message, context| {
            context.command(update(state, message));
        }))
    }
}

impl<State, Project, View> StatefulAppWithView<State, StateAction<State>, Project, View>
where
    Project: FnMut(&mut State) -> View + 'static,
    View: IntoView<StateAction<State>> + 'static,
    State: 'static,
{
    /// Run this direct-callback app through the native Vello runtime.
    pub fn run(self) -> Result {
        let options = self.options.clone();
        run_native_vello_runtime(options, self.into_bridge())
    }

    /// Run this app and return native runtime artifacts.
    pub fn run_with_artifacts(self) -> crate::gui_runtime::NativeGenericRunReport {
        let options = self.options.clone();
        crate::runtime::run_native_vello_runtime_with_artifacts(options, self.into_bridge())
    }

    /// Lower this direct-callback app into the existing runtime bridge without opening a window.
    pub fn into_bridge(self) -> impl RuntimeBridge<StateAction<State>> {
        AppBridge::new(
            self.state,
            self.project,
            |state: &mut State,
             action: StateAction<State>,
             context: &mut UpdateContext<StateAction<State>>| {
                action.run(state);
                context.request_repaint();
            },
            self.lifecycle,
        )
    }
}
