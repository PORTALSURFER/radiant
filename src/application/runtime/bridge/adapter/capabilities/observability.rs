use super::super::super::AppBridge;
use crate::{
    application::{IntoView, UiUpdateContext},
    runtime::{
        RuntimeDiagnostics, RuntimeDiagnosticsHost, RuntimeFrameDiagnosticsHost,
        RuntimeLifecycleHost,
    },
};

impl<State, Message, Project, Update, View> RuntimeFrameDiagnosticsHost
    for AppBridge<State, Message, Project, Update, View>
where
    Project: FnMut(&State) -> View + 'static,
    Update: FnMut(&mut State, Message, &mut UiUpdateContext<Message>) + 'static,
    View: IntoView<Message> + 'static,
    Message: Send + 'static,
    State: 'static,
{
    fn observe_frame_diagnostics(&mut self, diagnostics: crate::runtime::NativeFrameDiagnostics) {
        if let Some(observer) = self.lifecycle.native_frame_diagnostics.as_mut() {
            observer(&mut self.state, diagnostics);
        }
    }
}

impl<State, Message, Project, Update, View> RuntimeDiagnosticsHost
    for AppBridge<State, Message, Project, Update, View>
where
    Project: FnMut(&State) -> View + 'static,
    Update: FnMut(&mut State, Message, &mut UiUpdateContext<Message>) + 'static,
    View: IntoView<Message> + 'static,
    Message: Send + 'static,
    State: 'static,
{
    fn runtime_diagnostics(&self) -> RuntimeDiagnostics {
        self.runtime.diagnostics_snapshot()
    }
}

impl<State, Message, Project, Update, View> RuntimeLifecycleHost
    for AppBridge<State, Message, Project, Update, View>
where
    Project: FnMut(&State) -> View + 'static,
    Update: FnMut(&mut State, Message, &mut UiUpdateContext<Message>) + 'static,
    View: IntoView<Message> + 'static,
    Message: Send + 'static,
    State: 'static,
{
    fn on_runtime_exit(&mut self) -> Option<serde_json::Value> {
        self.runtime_exit_artifact()
    }

    fn close_requested(&mut self) -> bool {
        self.allow_close_requested()
    }
}
