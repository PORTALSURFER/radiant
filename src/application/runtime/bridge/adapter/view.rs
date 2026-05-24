use super::super::AppBridge;
use crate::{
    application::{IntoView, UpdateContext},
    gui::{focus::FocusSurface, input::KeyPress, shortcuts::ShortcutResolution},
    runtime::{Command, ScrollUpdate, UiSurface},
};
use std::sync::Arc;

impl<State, Message, Project, Update, View> AppBridge<State, Message, Project, Update, View>
where
    Project: FnMut(&mut State) -> View + 'static,
    Update: FnMut(&mut State, Message, &mut UpdateContext<Message>) + 'static,
    View: IntoView<Message> + 'static,
{
    pub(super) fn project_surface_arc(&mut self) -> Arc<UiSurface<Message>> {
        Arc::new((self.project)(&mut self.state).into_surface())
    }

    pub(super) fn pull_surface_owned(&mut self) -> UiSurface<Message> {
        (self.project)(&mut self.state).into_surface()
    }

    pub(super) fn update_message(&mut self, message: Message) -> Command<Message> {
        self.run_update(message)
    }

    pub(super) fn scroll_updated_command(
        &mut self,
        update: ScrollUpdate,
    ) -> Option<Command<Message>> {
        let scroll = self.lifecycle.scroll.as_mut()?;
        let mut context = UpdateContext::default();
        scroll(&mut self.state, update, &mut context);
        Some(context.into_command())
    }

    pub(super) fn resolve_shortcut(
        &mut self,
        pending_chord: Option<KeyPress>,
        press: KeyPress,
        focus: FocusSurface,
    ) -> ShortcutResolution<Message> {
        self.lifecycle
            .shortcuts
            .as_mut()
            .map(|shortcuts| shortcuts(&mut self.state, pending_chord, press, focus))
            .unwrap_or_else(ShortcutResolution::unhandled)
    }
}
