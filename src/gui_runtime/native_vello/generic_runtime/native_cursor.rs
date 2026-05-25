//! Native cursor mapping for backend-neutral widget cursor requests.

use super::GenericNativeVelloRunner;
use crate::{runtime::RuntimeBridge, widgets::WidgetCursor};
use winit::window::CursorIcon;

impl<Bridge, Message> GenericNativeVelloRunner<Bridge, Message>
where
    Bridge: RuntimeBridge<Message>,
{
    pub(super) fn update_native_cursor_at_last_position(&mut self) {
        let Some(position) = self.input.last_cursor else {
            self.set_native_cursor(WidgetCursor::Default);
            return;
        };
        let cursor = self.core.runtime.cursor_at(position);
        self.set_native_cursor(cursor);
    }

    pub(super) fn set_native_cursor(&mut self, cursor: WidgetCursor) {
        if self.input.native_cursor == Some(cursor) {
            return;
        }
        self.input.native_cursor = Some(cursor);
        if let Some(window) = self.window.window.as_ref() {
            window.set_cursor(cursor_icon_for_widget_cursor(cursor));
        }
    }
}

fn cursor_icon_for_widget_cursor(cursor: WidgetCursor) -> CursorIcon {
    match cursor {
        WidgetCursor::Default => CursorIcon::Default,
        WidgetCursor::Pointer => CursorIcon::Pointer,
        WidgetCursor::Text => CursorIcon::Text,
        WidgetCursor::Crosshair => CursorIcon::Crosshair,
        WidgetCursor::Grab => CursorIcon::Grab,
        WidgetCursor::Grabbing => CursorIcon::Grabbing,
        WidgetCursor::Move => CursorIcon::Move,
        WidgetCursor::ResizeHorizontal => CursorIcon::EwResize,
        WidgetCursor::ResizeLeft => CursorIcon::WResize,
        WidgetCursor::ResizeRight => CursorIcon::EResize,
        WidgetCursor::ResizeVertical => CursorIcon::NsResize,
        WidgetCursor::NotAllowed => CursorIcon::NotAllowed,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn widget_resize_edge_cursors_map_to_directional_native_icons() {
        assert_eq!(
            cursor_icon_for_widget_cursor(WidgetCursor::ResizeLeft),
            CursorIcon::WResize
        );
        assert_eq!(
            cursor_icon_for_widget_cursor(WidgetCursor::ResizeRight),
            CursorIcon::EResize
        );
    }
}
