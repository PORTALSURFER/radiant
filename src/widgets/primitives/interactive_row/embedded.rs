//! Custom widget contract for app-painted rows with embedded row input.

use super::{InteractiveRowActions, InteractiveRowLocalActions, InteractiveRowWidget};
use crate::{
    gui::types::Rect,
    layout::LayoutOutput,
    runtime::{PaintPrimitive, ResolvedEnvironment},
    theme::ThemeTokens,
    widgets::{
        DeclaredTextMetrics, TextScaleParticipation,
        contract::{
            Widget, WidgetCapabilities, WidgetPaintContext, WidgetPointerMotion,
            WidgetPointerMotionRevision, WidgetSemantics,
        },
        interaction::{InteractiveRowMessage, WidgetInput, WidgetOutput},
        primitives::support::WidgetCommon,
    },
};

impl<T> WidgetPointerMotion for T
where
    T: EmbeddedInteractiveRowWidget,
{
    fn revision(&self) -> WidgetPointerMotionRevision {
        WidgetPointerMotion::revision(self.interactive_row())
    }

    fn accepts_pointer_move(&self) -> bool {
        WidgetPointerMotion::accepts_pointer_move(self.interactive_row())
    }

    fn pointer_capture_policy(&self) -> crate::widgets::PointerCapturePolicy {
        WidgetPointerMotion::pointer_capture_policy(self.interactive_row())
    }

    fn prefers_pointer_move_paint_only(&self) -> bool {
        WidgetPointerMotion::prefers_pointer_move_paint_only(self.interactive_row())
    }

    fn pointer_move_overlay_is_valid(&self) -> bool {
        WidgetPointerMotion::pointer_move_overlay_is_valid(self.interactive_row())
    }
}

/// Custom widget contract for widgets built around an embedded interactive row.
///
/// Implement this trait when a custom-painted row needs Radiant's generic row
/// input, pointer-motion policy, retained state synchronization, and widget
/// contract delegation, but the host still owns the row's visual content and
/// message type. The blanket [`Widget`] implementation keeps application row
/// wrappers focused on domain action routing and paint.
pub trait EmbeddedInteractiveRowWidget: Clone + 'static {
    /// Host-specific message emitted by the custom row.
    type Message: 'static;

    /// Return the embedded generic interactive row.
    fn interactive_row(&self) -> &InteractiveRowWidget;

    /// Return the embedded generic interactive row mutably.
    fn interactive_row_mut(&mut self) -> &mut InteractiveRowWidget;

    /// Return common action routing for this embedded row, when applicable.
    fn interactive_row_actions(&self) -> Option<&InteractiveRowActions<Self::Message>> {
        None
    }

    /// Return UI-local action routing for this embedded row, when applicable.
    fn interactive_row_local_actions(&self) -> Option<&InteractiveRowLocalActions<Self::Message>> {
        None
    }

    /// Map a generic row interaction into this custom row's message type.
    fn map_interactive_row_message(&self, message: InteractiveRowMessage) -> Option<Self::Message> {
        self.interactive_row_actions()
            .and_then(|actions| actions.route(message))
            .or_else(|| {
                self.interactive_row_local_actions()
                    .and_then(|actions| actions.route(message))
            })
    }

    /// Return the optional declared text metrics for this embedded row.
    ///
    /// Custom rows remain on the legacy unscaled contract unless they provide
    /// an explicit immutable declaration.
    fn declared_interactive_row_text_metrics(&self) -> Option<DeclaredTextMetrics> {
        None
    }

    /// Append host-specific paint through the environment-aware context.
    ///
    /// The default delegates exactly once to the legacy paint callback.
    fn append_interactive_row_paint_with_context(&self, context: &mut WidgetPaintContext<'_>) {
        let bounds = context.bounds();
        let layout = context.layout();
        let theme = context.theme();
        let primitives = context.primitives();
        self.append_interactive_row_paint(primitives, bounds, layout, theme);
    }

    /// Return optional semantics for this embedded row.
    fn interactive_row_semantics(&self) -> Option<&dyn WidgetSemantics> {
        None
    }

    /// Append host-specific paint for this custom row.
    fn append_interactive_row_paint(
        &self,
        primitives: &mut Vec<PaintPrimitive>,
        bounds: Rect,
        layout: &LayoutOutput,
        theme: &ThemeTokens,
    );
}

impl<T> Widget for T
where
    T: EmbeddedInteractiveRowWidget,
{
    fn common(&self) -> &WidgetCommon {
        self.interactive_row().common()
    }

    fn common_mut(&mut self) -> &mut WidgetCommon {
        self.interactive_row_mut().common_mut()
    }

    fn text_scale_participation(&self) -> TextScaleParticipation {
        self.declared_interactive_row_text_metrics()
            .map_or(TextScaleParticipation::Unscaled, |_| {
                TextScaleParticipation::Scaled
            })
    }

    fn layout_node_with_environment(
        &self,
        environment: &ResolvedEnvironment,
    ) -> crate::layout::LayoutNode {
        let Some(declared) = self.declared_interactive_row_text_metrics() else {
            return self.interactive_row().common().layout_node();
        };
        crate::layout::LayoutNode::Widget(
            declared
                .resolve(environment, TextScaleParticipation::Scaled)
                .layout_node(self.interactive_row().id()),
        )
    }

    fn handle_input(&mut self, bounds: Rect, input: WidgetInput) -> Option<WidgetOutput> {
        let message = self.interactive_row_mut().handle_input(bounds, input)?;
        self.map_interactive_row_message(message)
            .map(WidgetOutput::typed)
    }

    fn capabilities(&self) -> WidgetCapabilities<'_> {
        self.interactive_row_semantics()
            .map_or_else(WidgetCapabilities::none, |semantics| {
                WidgetCapabilities::new().semantics(semantics)
            })
    }

    fn capabilities_v2(&self) -> crate::widgets::WidgetCapabilitiesV2<'_> {
        crate::widgets::WidgetCapabilitiesV2::new().with_pointer_motion(self)
    }

    fn synchronize_from_previous(&mut self, previous: &dyn Widget) {
        let Some(previous) = previous.as_any().downcast_ref::<T>() else {
            return;
        };
        self.interactive_row_mut()
            .synchronize_from_previous(previous.interactive_row());
    }

    fn append_paint(
        &self,
        primitives: &mut Vec<PaintPrimitive>,
        bounds: Rect,
        layout: &LayoutOutput,
        theme: &ThemeTokens,
    ) {
        self.append_interactive_row_paint(primitives, bounds, layout, theme);
    }

    fn append_paint_with_context(&self, context: &mut WidgetPaintContext<'_>) {
        self.append_interactive_row_paint_with_context(context);
    }
}
