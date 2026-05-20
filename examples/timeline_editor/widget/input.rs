#[path = "input/clip_handles.rs"]
mod clip_handles;
#[path = "input/pointer.rs"]
mod pointer;

use super::super::model::TimelineSurfaceMessage;
use super::ArrangementTimelineWidget;
use radiant::layout::Rect;
use radiant::widgets::{PointerButton, WidgetInput, WidgetKey, WidgetOutput};

pub(super) fn handle_timeline_input(
    widget: &mut ArrangementTimelineWidget,
    bounds: Rect,
    input: WidgetInput,
) -> Option<WidgetOutput> {
    let geometry = widget.geometry(bounds);
    match input {
        WidgetInput::PointerMove { position } => {
            pointer::handle_pointer_move(widget, bounds, geometry, position)
        }
        WidgetInput::PointerPress {
            position,
            button: PointerButton::Primary,
            ..
        } if bounds.contains(position) => pointer::handle_primary_press(widget, geometry, position),
        WidgetInput::PointerRelease {
            position,
            button: PointerButton::Primary,
            ..
        } => pointer::handle_primary_release(widget, geometry, position),
        WidgetInput::FocusChanged(focused) => {
            widget.common.state.focused = focused;
            None
        }
        WidgetInput::KeyPress(WidgetKey::Space) if widget.common.state.focused => {
            Some(WidgetOutput::typed(TimelineSurfaceMessage::Seek {
                beat: widget.cursor.active_beat(widget.playhead_beat),
            }))
        }
        WidgetInput::KeyPress(WidgetKey::Delete | WidgetKey::Backspace)
            if widget.common.state.focused && widget.selected_clip.is_some() =>
        {
            Some(WidgetOutput::typed(TimelineSurfaceMessage::DeleteSelected))
        }
        _ => None,
    }
}
