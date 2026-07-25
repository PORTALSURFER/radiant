use super::super::AppBridge;
use crate::{
    application::runtime::TransientOverlayBinding,
    application::{IntoView, UiUpdateContext},
    gui::{paint::PaintFrame as GuiPaintFrame, types::Rect},
    layout::Vector2,
    runtime::{PaintPrimitive, TransientOverlayContext},
    widgets::RetainedSurfaceDescriptor,
};

impl<State, Message, Project, Update, View> AppBridge<State, Message, Project, Update, View>
where
    Project: FnMut(&State) -> View + 'static,
    Update: FnMut(&mut State, Message, &mut UiUpdateContext<Message>) + 'static,
    View: IntoView<Message> + 'static,
{
    pub(super) fn render_app_retained_surface(
        &mut self,
        descriptor: RetainedSurfaceDescriptor,
        rect: Rect,
        viewport: Vector2,
    ) -> Option<GuiPaintFrame> {
        self.lifecycle
            .retained_painters
            .get_mut(&descriptor.key)
            .and_then(|paint| paint(&mut self.state, descriptor, rect, viewport))
    }

    pub(super) fn paint_app_transient_overlay(
        &mut self,
        context: TransientOverlayContext<'_>,
        primitives: &mut Vec<PaintPrimitive>,
    ) {
        if let Some(paint) = self.lifecycle.transient_overlay.as_mut() {
            paint(&mut self.state, context, primitives);
        }
        paint_active_overlays(
            &mut self.lifecycle.transient_overlays,
            &mut self.state,
            context,
            primitives,
        );
        paint_active_overlays(
            &mut self.lifecycle.scene_transient_overlays,
            &mut self.state,
            context,
            primitives,
        );
    }
}

fn paint_active_overlays<State>(
    overlays: &mut [TransientOverlayBinding<State>],
    state: &mut State,
    context: TransientOverlayContext<'_>,
    primitives: &mut Vec<PaintPrimitive>,
) {
    for overlay in overlays {
        let eligible = overlay
            .pending_paint_eligibility
            .take()
            .unwrap_or_else(|| (overlay.activity)(state).needs_animation());
        if !eligible {
            continue;
        }
        if let Some(paint) = overlay.painter.as_mut() {
            paint(state, context, primitives);
        }
    }
}
