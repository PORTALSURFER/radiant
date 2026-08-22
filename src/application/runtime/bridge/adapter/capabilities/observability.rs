use super::super::super::AppBridge;
use crate::{
    application::{IntoView, UiUpdateContext},
    runtime::{
        FrameGpuTimingSample, FrameProfile, RuntimeDiagnostics, RuntimeDiagnosticsHost,
        RuntimeFrameDiagnosticsHost, RuntimeFrameGpuTimingHost, RuntimeFrameProfileHost,
        RuntimeLifecycleHost,
    },
};

impl<State, Message, Project, Update, View> RuntimeFrameDiagnosticsHost
    for AppBridge<State, Message, Project, Update, View>
where
    Project: FnMut(&State) -> View + 'static,
    Update: FnMut(&mut State, Message, &mut UiUpdateContext<Message>) + 'static,
    View: IntoView<Message> + 'static,
    Message: 'static,
    State: 'static,
{
    fn observe_frame_diagnostics(&mut self, diagnostics: crate::runtime::NativeFrameDiagnostics) {
        if let Some(observer) = self.lifecycle.native_frame_diagnostics.as_mut() {
            observer(&mut self.state, diagnostics);
        }
    }
}

impl<State, Message, Project, Update, View> RuntimeFrameProfileHost
    for AppBridge<State, Message, Project, Update, View>
where
    Project: FnMut(&State) -> View + 'static,
    Update: FnMut(&mut State, Message, &mut UiUpdateContext<Message>) + 'static,
    View: IntoView<Message> + 'static,
    Message: 'static,
    State: 'static,
{
    fn observe_frame_profile(&mut self, profile: FrameProfile) {
        if let Some(observer) = self.lifecycle.native_frame_profile.as_mut() {
            observer(&mut self.state, profile);
        }
    }
}

impl<State, Message, Project, Update, View> RuntimeFrameGpuTimingHost
    for AppBridge<State, Message, Project, Update, View>
where
    Project: FnMut(&State) -> View + 'static,
    Update: FnMut(&mut State, Message, &mut UiUpdateContext<Message>) + 'static,
    View: IntoView<Message> + 'static,
    Message: 'static,
    State: 'static,
{
    fn observe_frame_gpu_timing(&mut self, sample: FrameGpuTimingSample) {
        if let Some(observer) = self.lifecycle.native_frame_gpu_timing.as_mut() {
            observer(&mut self.state, sample);
        }
    }
}

impl<State, Message, Project, Update, View> RuntimeDiagnosticsHost
    for AppBridge<State, Message, Project, Update, View>
where
    Project: FnMut(&State) -> View + 'static,
    Update: FnMut(&mut State, Message, &mut UiUpdateContext<Message>) + 'static,
    View: IntoView<Message> + 'static,
    Message: 'static,
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
    Message: 'static,
    State: 'static,
{
    fn on_runtime_closing(&mut self) {
        self.runtime_begin_closing();
    }

    fn on_runtime_exit(&mut self) -> Option<serde_json::Value> {
        self.runtime_exit_artifact()
    }

    fn close_requested(&mut self) -> bool {
        self.allow_close_requested()
    }
}
