use super::super::AppBridge;
use crate::{
    application::{IntoView, UiUpdateContext, ViewNode},
    gui::{focus::FocusSurface, input::KeyPress, shortcuts::ShortcutResolution},
    runtime::{Command, ScrollUpdate, UiSurface},
};
use std::{any::Any, sync::Arc};

impl<State: 'static, Message: 'static, Project, Update, View>
    AppBridge<State, Message, Project, Update, View>
where
    Project: FnMut(&mut State) -> View + 'static,
    Update: FnMut(&mut State, Message, &mut UiUpdateContext<Message>) + 'static,
    View: IntoView<Message> + 'static,
{
    pub(super) fn project_surface_arc(&mut self) -> Arc<UiSurface<Message>> {
        let mut view = (self.project)(&mut self.state);
        self.apply_view_scene_presentation(&mut view);
        Arc::new(view.into_surface())
    }

    pub(super) fn pull_surface_owned(&mut self) -> UiSurface<Message> {
        let mut view = (self.project)(&mut self.state);
        self.apply_view_scene_presentation(&mut view);
        view.into_surface()
    }

    fn apply_view_scene_presentation(&mut self, view: &mut View) {
        self.lifecycle.clear_scene_presentation();
        let Some(view) = (view as &mut dyn Any).downcast_mut::<ViewNode<Message>>() else {
            return;
        };
        view.apply_scene_presentation(&mut self.lifecycle);
    }

    pub(super) fn update_message(&mut self, message: Message) -> Command<Message> {
        self.run_update(message)
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
