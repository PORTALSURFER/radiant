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
    Message: 'static,
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

    fn native_focus_regained(&mut self) -> Command<Message> {
        self.native_focus_regained_command()
    }

    fn resolve_command(
        &mut self,
        request: crate::application::CommandRequest<'_>,
        focus: crate::application::CommandFocus,
    ) -> crate::application::CommandDispatch<Message> {
        self.lifecycle
            .command_router
            .as_ref()
            .map_or_else(crate::application::CommandDispatch::unhandled, |router| {
                router(&self.state, request, focus)
            })
    }

    fn command_service(&self) -> Option<crate::application::CommandService<Message>> {
        let router = self.lifecycle.declarative_command_router.as_ref()?;
        let keymap = self
            .lifecycle
            .command_keymap
            .as_ref()
            .map(|project| project(&self.state))
            .unwrap_or_default();
        Some(crate::application::CommandService::from_resolver(
            std::rc::Rc::clone(router),
            keymap,
        ))
    }

    fn resolve_command_with_scopes(
        &mut self,
        request: crate::application::CommandRequest<'_>,
        focus: crate::application::CommandFocus,
        scopes: crate::application::CommandScopeProjection<'_>,
    ) -> crate::application::CommandDispatch<Message> {
        if let Some(router) = &self.lifecycle.declarative_command_router {
            let keymap = self
                .lifecycle
                .command_keymap
                .as_ref()
                .map(|project| project(&self.state))
                .unwrap_or_default();
            router(request, scopes, &keymap)
        } else {
            self.resolve_command(request, focus)
        }
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
