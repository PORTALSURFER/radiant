use super::super::super::AppBridge;
use crate::{
    application::{IntoView, UiUpdateContext},
    runtime::{
        PaintPrimitive, RuntimeRetainedSurfaceHost, RuntimeTransientOverlayHost, RuntimeWindowHost,
        TransientOverlayContext,
    },
};

impl<State, Message, Project, Update, View> RuntimeWindowHost<Message>
    for AppBridge<State, Message, Project, Update, View>
where
    Project: FnMut(&State) -> View + 'static,
    Update: FnMut(&mut State, Message, &mut UiUpdateContext<Message>) + 'static,
    View: IntoView<Message> + 'static,
    Message: Send + 'static,
    State: 'static,
{
    fn project_auxiliary_windows(&mut self) -> Vec<crate::runtime::AuxiliaryWindow<Message>> {
        self.project_app_auxiliary_windows()
    }
}

impl<State, Message, Project, Update, View> RuntimeRetainedSurfaceHost
    for AppBridge<State, Message, Project, Update, View>
where
    Project: FnMut(&State) -> View + 'static,
    Update: FnMut(&mut State, Message, &mut UiUpdateContext<Message>) + 'static,
    View: IntoView<Message> + 'static,
    Message: Send + 'static,
    State: 'static,
{
    fn render_retained_surface(
        &mut self,
        descriptor: crate::widgets::RetainedSurfaceDescriptor,
        rect: crate::gui::types::Rect,
        viewport: crate::layout::Vector2,
    ) -> Option<crate::gui::paint::PaintFrame> {
        self.render_app_retained_surface(descriptor, rect, viewport)
    }
}

impl<State, Message, Project, Update, View> RuntimeTransientOverlayHost
    for AppBridge<State, Message, Project, Update, View>
where
    Project: FnMut(&State) -> View + 'static,
    Update: FnMut(&mut State, Message, &mut UiUpdateContext<Message>) + 'static,
    View: IntoView<Message> + 'static,
    Message: Send + 'static,
    State: 'static,
{
    fn paint_transient_overlay(
        &mut self,
        context: TransientOverlayContext<'_>,
        primitives: &mut Vec<PaintPrimitive>,
    ) {
        self.paint_app_transient_overlay(context, primitives);
    }
}
