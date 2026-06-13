use crate::{
    application::{
        AppBridge, AppBridgeLifecycle, RepaintPolicy, Result, UiUpdateContext, launch::IntoView,
    },
    gui_runtime::NativeRunOptions,
    runtime::{RuntimeBridge, run_native_vello_runtime},
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
    Update: FnMut(&mut State, Message, &mut UiUpdateContext<Message>) + 'static,
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

    /// Apply an automatic repaint policy to ordinary app messages.
    ///
    /// Frame-clock messages use their frame-clock repaint scope first, so apps
    /// do not need to exclude frame messages from ordinary repaint policy.
    pub fn repaint_policy(mut self, policy: RepaintPolicy<Message>) -> Self {
        self.lifecycle.repaint_policy = Some(policy);
        self
    }
}
