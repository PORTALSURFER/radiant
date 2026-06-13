use super::super::AppBridge;
use crate::application::{IntoView, UiUpdateContext};
use crate::runtime::{Command, NativeFileDrop};

impl<State, Message, Project, Update, View> AppBridge<State, Message, Project, Update, View>
where
    Project: FnMut(&mut State) -> View + 'static,
    Update: FnMut(&mut State, Message, &mut UiUpdateContext<Message>) + 'static,
    View: IntoView<Message> + 'static,
{
    pub(super) fn runtime_exit_artifact(&mut self) -> Option<serde_json::Value> {
        self.runtime.shutdown();
        self.lifecycle
            .shutdown
            .as_mut()
            .and_then(|shutdown| shutdown(&mut self.state))
    }

    pub(super) fn allow_close_requested(&mut self) -> bool {
        self.lifecycle
            .close_requested
            .as_mut()
            .is_none_or(|close_requested| close_requested(&mut self.state))
    }

    pub(super) fn native_file_drop_command(&mut self, drop: NativeFileDrop) -> Command<Message> {
        let Some(native_file_drop) = self.lifecycle.native_file_drop.as_mut() else {
            return Command::none();
        };
        let mut context = UiUpdateContext::default();
        native_file_drop(&mut self.state, drop, &mut context);
        context.into_command()
    }
}
