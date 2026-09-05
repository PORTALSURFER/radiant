use super::super::AppBridge;
use crate::{
    application::{IntoView, UiUpdateContext},
    gui::{focus::FocusSurface, input::KeyPress, shortcuts::ShortcutResolution},
    runtime::{Command, ScrollUpdate, UiSurface},
};
use std::sync::Arc;

#[cfg(test)]
mod tests;

impl<State: 'static, Message: 'static, Project, Update, View>
    AppBridge<State, Message, Project, Update, View>
where
    Project: FnMut(&State) -> View + 'static,
    Update: FnMut(&mut State, Message, &mut UiUpdateContext<Message>) + 'static,
    View: IntoView<Message> + 'static,
{
    pub(super) fn application_environment_for_refresh(
        &mut self,
    ) -> Option<crate::application::ApplicationEnvironment> {
        self.application_environment_source
            .as_ref()
            .map(|source| source(&self.state))
    }

    fn application_environment_for_projection(
        &mut self,
    ) -> Option<crate::application::ApplicationEnvironment> {
        self.application_environment_source
            .as_ref()
            .map(|source| source(&self.state))
    }

    pub(super) fn project_surface_arc(&mut self) -> Arc<UiSurface<Message>> {
        Arc::new(self.pull_surface_owned())
    }

    pub(super) fn pull_surface_owned(&mut self) -> UiSurface<Message> {
        use crate::application::{
            ApplicationProjectionContext, view_node::reconciliation::ApplicationProjectionRecorder,
        };
        self.projection_producer.invalidate_external_projection();
        let mut recorder = ApplicationProjectionRecorder::new(None);
        let mut context = ApplicationProjectionContext::new(&mut recorder);
        let projection = (self.project)(&self.state).into_application_projection(&mut context);
        let (receipt, _) = context.finish();
        let (surface, scene) = projection.into_parts();
        scene.apply(&mut self.lifecycle);
        let surface = match self.application_environment_for_projection() {
            Some(environment) => surface.with_application_environment(environment),
            None => surface,
        };
        self.projection_producer
            .commit_startup(std::rc::Rc::new(receipt));
        surface
    }

    pub(super) fn pull_application_update(
        &mut self,
        request: crate::runtime::SurfaceRefreshRequest,
    ) -> crate::runtime::SurfaceUpdate<Message> {
        use crate::{
            application::{
                ApplicationProjectionContext,
                launch::projection_producer::{Candidate, ProducerRequest, StageDecision},
                view_node::reconciliation::ApplicationProjectionRecorder,
            },
            runtime::{ExactChangedRoots, SurfaceUpdate},
        };
        let environment = self.application_environment_for_projection();
        let transaction = self
            .projection_producer
            .begin_request(ProducerRequest::from_runtime(request, environment.clone()));
        let project = &mut self.project;
        let state = &self.state;
        let projected = transaction.project(|baseline| {
            let mut recorder = ApplicationProjectionRecorder::new(baseline);
            let mut context = ApplicationProjectionContext::new(&mut recorder);
            let projection = project(state).into_application_projection(&mut context);
            let (receipt, comparison) = context.finish();
            let projection = match environment {
                Some(environment) => projection.with_application_environment(environment),
                None => projection,
            };
            Candidate::new(projection, receipt, comparison)
        });
        let (candidate, decision) = projected.stage();
        let (surface, scene) = candidate.payload.into_parts();
        scene.apply(&mut self.lifecycle);
        match decision {
            StageDecision::Full => SurfaceUpdate::Full(surface),
            StageDecision::Exact {
                provider_authority,
                changed_roots,
            } => SurfaceUpdate::ExactChangedRoots(ExactChangedRoots {
                surface,
                runtime_identity: request.runtime_identity,
                request_revision: request.request_revision,
                active_surface_generation: request.active_surface_generation,
                viewport: request.viewport,
                window_environment: request.window_environment,
                provider_authority: Some(provider_authority),
                changed_roots,
            }),
        }
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
