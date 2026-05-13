use super::super::AppBridge;
use crate::application::{IntoView, UpdateContext};

impl<State, Message, Project, Update, View> AppBridge<State, Message, Project, Update, View>
where
    Project: FnMut(&mut State) -> View + 'static,
    Update: FnMut(&mut State, Message, &mut UpdateContext<Message>) + 'static,
    View: IntoView<Message> + 'static,
{
    pub(super) fn runtime_exit_artifact(&mut self) -> Option<serde_json::Value> {
        self.runtime.shutdown();
        self.shutdown
            .as_mut()
            .and_then(|shutdown| shutdown(&mut self.state))
    }

    pub(super) fn allow_close_requested(&mut self) -> bool {
        self.close_requested
            .as_mut()
            .is_none_or(|close_requested| close_requested(&mut self.state))
    }
}
