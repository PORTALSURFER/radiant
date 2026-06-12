//! Custom widget contract for app-painted rows with embedded row input.

use super::{InteractiveRowActions, InteractiveRowWidget};
use crate::{
    gui::types::Rect,
    layout::LayoutOutput,
    runtime::PaintPrimitive,
    theme::ThemeTokens,
    widgets::{
        contract::Widget,
        interaction::{InteractiveRowMessage, WidgetInput, WidgetOutput},
        primitives::support::WidgetCommon,
    },
};

/// Custom widget contract for widgets built around an embedded interactive row.
///
/// Implement this trait when a custom-painted row needs Radiant's generic row
/// input, pointer-motion policy, retained state synchronization, and widget
/// contract delegation, but the host still owns the row's visual content and
/// message type. The blanket [`Widget`] implementation keeps application row
/// wrappers focused on domain action routing and paint.
pub trait EmbeddedInteractiveRowWidget: Clone + Send + Sync + 'static {
    /// Host-specific message emitted by the custom row.
    type Message: Send + Sync + 'static;

    /// Return the embedded generic interactive row.
    fn interactive_row(&self) -> &InteractiveRowWidget;

    /// Return the embedded generic interactive row mutably.
    fn interactive_row_mut(&mut self) -> &mut InteractiveRowWidget;

    /// Return common action routing for this embedded row, when applicable.
    fn interactive_row_actions(&self) -> Option<&InteractiveRowActions<Self::Message>> {
        None
    }

    /// Map a generic row interaction into this custom row's message type.
    fn map_interactive_row_message(&self, message: InteractiveRowMessage) -> Option<Self::Message> {
        self.interactive_row_actions()?.route(message)
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

    fn handle_input(&mut self, bounds: Rect, input: WidgetInput) -> Option<WidgetOutput> {
        let message = self.interactive_row_mut().handle_input(bounds, input)?;
        self.map_interactive_row_message(message)
            .map(WidgetOutput::typed)
    }

    fn accepts_pointer_move(&self) -> bool {
        self.interactive_row().accepts_pointer_move()
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
}
