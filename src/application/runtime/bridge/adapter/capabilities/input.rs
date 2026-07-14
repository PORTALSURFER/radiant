use super::super::super::AppBridge;
use crate::{
    application::{IntoView, UiUpdateContext},
    gui::{focus::FocusSurface, input::KeyPress, shortcuts::ShortcutResolution},
    runtime::{Command, NativeFileOpen, RuntimeInputHost},
};

impl<State, Message, Project, Update, View> RuntimeInputHost<Message>
    for AppBridge<State, Message, Project, Update, View>
where
    Project: FnMut(&State) -> View + 'static,
    Update: FnMut(&mut State, Message, &mut UiUpdateContext<Message>) + 'static,
    View: IntoView<Message> + 'static,
    Message: Send + 'static,
    State: 'static,
{
    fn scroll_updated(&mut self, update: crate::runtime::ScrollUpdate) -> Option<Command<Message>> {
        self.scroll_updated_command(update)
    }

    fn native_file_drop(&mut self, drop: crate::runtime::NativeFileDrop) -> Command<Message> {
        self.native_file_drop_command(drop)
    }

    fn native_file_open(&mut self, open: NativeFileOpen) -> Command<Message> {
        self.native_file_open_command(open)
    }

    fn resolve_key_press(
        &mut self,
        pending_chord: Option<KeyPress>,
        press: KeyPress,
        focus: FocusSurface,
    ) -> ShortcutResolution<Message> {
        self.resolve_shortcut(pending_chord, press, focus)
    }
}
