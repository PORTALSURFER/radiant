use super::super::AppBridge;
use crate::{
    application::{IntoView, UiUpdateContext},
    gui::{focus::FocusSurface, input::KeyPress, shortcuts::ShortcutResolution},
    runtime::{Command, ScrollUpdate, UiSurface},
};
use std::sync::Arc;

impl<State: 'static, Message: 'static, Project, Update, View>
    AppBridge<State, Message, Project, Update, View>
where
    Project: FnMut(&State) -> View + 'static,
    Update: FnMut(&mut State, Message, &mut UiUpdateContext<Message>) + 'static,
    View: IntoView<Message> + 'static,
{
    pub(super) fn project_surface_arc(&mut self) -> Arc<UiSurface<Message>> {
        let projection = (self.project)(&self.state).into_projection();
        let (surface, scene) = projection.into_parts();
        scene.apply(&mut self.lifecycle);
        Arc::new(surface)
    }

    pub(super) fn pull_surface_owned(&mut self) -> UiSurface<Message> {
        let projection = (self.project)(&self.state).into_projection();
        let (surface, scene) = projection.into_parts();
        scene.apply(&mut self.lifecycle);
        surface
    }

    pub(super) fn update_message(&mut self, message: Message) -> Command<Message> {
        self.run_update(message)
    }

    pub(super) fn update_message_with_runtime(
        &mut self,
        message: Message,
        snapshot: crate::runtime::RuntimeUpdateSnapshot,
    ) -> Command<Message> {
        self.run_update_with_runtime(message, snapshot)
    }

    pub(super) fn scroll_updated_command(
        &mut self,
        update: ScrollUpdate,
    ) -> Option<Command<Message>> {
        let scroll = self.lifecycle.scroll.as_mut()?;
        let mut context = UiUpdateContext::default();
        scroll(&mut self.state, update, &mut context);
        Some(context.into_command())
    }

    pub(super) fn resolve_shortcut(
        &mut self,
        pending_chord: Option<KeyPress>,
        press: KeyPress,
        focus: FocusSurface,
    ) -> ShortcutResolution<Message> {
        if let Some(scene_shortcuts) = self.lifecycle.scene_shortcuts.as_ref() {
            let resolution = scene_shortcuts(press);
            if resolution.handled {
                return resolution;
            }
        }
        self.lifecycle
            .shortcuts
            .as_mut()
            .map(|shortcuts| shortcuts(&mut self.state, pending_chord, press, focus))
            .unwrap_or_else(ShortcutResolution::unhandled)
    }
}
