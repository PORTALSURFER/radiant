use crate::{
    application::{
        AppBridge, AppBridgeLifecycle, AppUpdate, RepaintPolicy, Result, UpdateContext,
        launch::IntoView,
    },
    gui_runtime::NativeRunOptions,
    runtime::{Command, RuntimeBridge, run_native_vello_runtime},
};
use std::marker::PhantomData;

/// Runnable stateful app builder.
pub struct RunnableStatefulApp<State, Message, Project, Update, View> {
    pub(super) state: State,
    pub(super) options: NativeRunOptions,
    pub(super) project: Project,
    pub(super) update: Update,
    pub(super) lifecycle: AppBridgeLifecycle<State, Message>,
    pub(super) _message: PhantomData<Message>,
    pub(super) _view: PhantomData<View>,
}

impl<State, Message, Project, Update, View>
    RunnableStatefulApp<State, Message, Project, Update, View>
where
    Project: FnMut(&mut State) -> View + 'static,
    Update: FnMut(&mut State, Message, &mut UpdateContext<Message>) + 'static,
    View: IntoView<Message> + 'static,
    Message: Send + 'static,
    State: 'static,
{
    /// Run this app through the native Vello runtime.
    pub fn run(self) -> Result {
        let options = self.options.clone();
        run_native_vello_runtime(options, self.into_bridge())
    }

    /// Run this app and return native runtime artifacts.
    pub fn run_with_artifacts(self) -> crate::gui_runtime::NativeGenericRunReport {
        let options = self.options.clone();
        crate::runtime::run_native_vello_runtime_with_artifacts(options, self.into_bridge())
    }

    /// Lower this app into the existing runtime bridge without opening a window.
    pub fn into_bridge(self) -> impl RuntimeBridge<Message> {
        AppBridge::new(self.state, self.project, self.update, self.lifecycle)
    }

    /// Apply an automatic repaint policy after app messages are reduced.
    pub fn repaint_policy(
        self,
        policy: RepaintPolicy<Message>,
    ) -> RunnableStatefulApp<State, Message, Project, AppUpdate<State, Message>, View> {
        let mut update = self.update;
        RunnableStatefulApp {
            state: self.state,
            options: self.options,
            project: self.project,
            update: Box::new(move |state, message, context| {
                let repaint = policy.scope_for(&message);
                update(state, message, context);
                if let Some(repaint) = repaint {
                    context.command(Command::repaint(repaint));
                }
            }),
            lifecycle: self.lifecycle,
            _message: PhantomData,
            _view: PhantomData,
        }
    }
}
