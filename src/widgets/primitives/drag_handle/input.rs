//! Drag-handle pointer interaction behavior.

use crate::gui::types::Rect;
use crate::widgets::interaction::{
    DragHandleMessage, DragHandleMetadata, PointerButton, WidgetInput,
};
use crate::widgets::primitives::drag_handle::DragHandleWidget;

pub(super) fn handle_drag_handle_input(
    handle: &mut DragHandleWidget,
    bounds: Rect,
    input: WidgetInput,
) -> Option<DragHandleMessage> {
    if handle.common.state.disabled {
        return None;
    }

    match input {
        WidgetInput::PointerMove {
            position,
            modifiers,
            timestamp,
            sequence_range,
        } => {
            let contains_pointer = bounds.contains(position);
            if handle.hover_suppressed_until_exit {
                handle.common.state.hovered = false;
                handle.hover_started_at = None;
                handle.hover_highlight_revealed = false;
                if !contains_pointer {
                    handle.hover_suppressed_until_exit = false;
                }
            } else {
                if contains_pointer && !handle.common.state.hovered {
                    handle.hover_started_at = Some(std::time::Instant::now());
                    handle.hover_highlight_revealed = handle.hover_highlight_delay.is_zero();
                } else if !contains_pointer {
                    handle.hover_started_at = None;
                    handle.hover_highlight_revealed = false;
                }
                handle.common.state.hovered = contains_pointer;
            }
            handle
                .common
                .state
                .pressed
                .then_some(DragHandleMessage::moved_with_metadata(
                    position,
                    DragHandleMetadata {
                        modifiers,
                        timestamp,
                        sequence_range,
                    },
                ))
        }
        WidgetInput::PointerPress {
            position,
            button: PointerButton::Primary,
            modifiers,
            timestamp,
        } if bounds.contains(position) => {
            handle.hover_suppressed_until_exit = false;
            handle.hover_started_at = None;
            handle.hover_highlight_revealed = false;
            handle.common.state.pressed = true;
            handle.common.state.active = true;
            Some(DragHandleMessage::started_with_metadata(
                position,
                position,
                DragHandleMetadata {
                    modifiers,
                    timestamp,
                    sequence_range: None,
                },
            ))
        }
        WidgetInput::PointerDoubleClick {
            position,
            button: PointerButton::Primary,
            modifiers,
            timestamp,
        } if bounds.contains(position) => {
            handle.hover_suppressed_until_exit = false;
            handle.hover_started_at = None;
            handle.hover_highlight_revealed = true;
            handle.common.state.hovered = true;
            handle.common.state.pressed = false;
            handle.common.state.active = false;
            Some(DragHandleMessage::double_activate_with_metadata(
                position,
                DragHandleMetadata {
                    modifiers,
                    timestamp,
                    sequence_range: None,
                },
            ))
        }
        WidgetInput::PointerRelease {
            position,
            button: PointerButton::Primary,
            modifiers,
            timestamp,
        } => {
            handle.common.state.pressed = false;
            handle.common.state.active = false;
            if handle.trailing_rail_width.is_some() {
                handle.common.state.hovered = false;
                handle.hover_suppressed_until_exit = bounds.contains(position);
            }
            handle.hover_started_at = None;
            handle.hover_highlight_revealed = false;
            Some(DragHandleMessage::ended_with_metadata(
                position,
                DragHandleMetadata {
                    modifiers,
                    timestamp,
                    sequence_range: None,
                },
            ))
        }
        WidgetInput::FocusChanged(focused) => {
            let cancel_drag = !focused && handle.common.state.active;
            handle.common.state.focused = focused;
            if cancel_drag {
                handle.common.state.pressed = false;
                handle.common.state.active = false;
                handle.hover_started_at = handle.common.state.hovered.then(std::time::Instant::now);
                handle.hover_highlight_revealed = handle.hover_highlight_delay.is_zero();
                return Some(DragHandleMessage::Cancelled {
                    position: bounds.center(),
                });
            }
            None
        }
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        gui::input::{InputSequence, InputSequenceRange, InputTimestamp},
        gui::types::{Point, Rect, Vector2},
        widgets::WidgetSizing,
        widgets::interaction::PointerModifiers,
    };

    #[test]
    fn drag_handle_double_click_emits_double_activate() {
        let mut handle = DragHandleWidget::new(7, WidgetSizing::fixed(Vector2::new(24.0, 16.0)));
        let bounds = Rect::from_size(24.0, 16.0);
        let position = Point::new(8.0, 6.0);

        let message = handle_drag_handle_input(
            &mut handle,
            bounds,
            WidgetInput::primary_double_click(position),
        );

        assert_eq!(
            message,
            Some(DragHandleMessage::DoubleActivate {
                position,
                metadata: DragHandleMetadata::empty(),
            })
        );
        assert!(!handle.common.state.pressed);
        assert!(!handle.common.state.active);
    }

    #[test]
    fn drag_handle_preserves_native_metadata_and_only_moves_carry_sequences() {
        let mut handle = DragHandleWidget::new(9, WidgetSizing::fixed(Vector2::new(24.0, 16.0)));
        let bounds = Rect::from_size(24.0, 16.0);
        let press_position = Point::new(8.0, 6.0);
        let move_position = Point::new(12.0, 9.0);
        let release_position = Point::new(14.0, 10.0);
        let press_modifiers = PointerModifiers {
            command: true,
            shift: false,
            alt: true,
        };
        let move_modifiers = PointerModifiers {
            command: false,
            shift: true,
            alt: true,
        };
        let release_modifiers = PointerModifiers::default();
        let press_timestamp = InputTimestamp::capture();
        let move_timestamp = InputTimestamp::capture();
        let release_timestamp = InputTimestamp::capture();
        let move_sequence = InputSequenceRange::singleton(InputSequence::from_runtime_value(42));

        let started = handle
            .handle_input(
                bounds,
                WidgetInput::pointer_press_with_timestamp(
                    press_position,
                    PointerButton::Primary,
                    press_modifiers,
                    Some(press_timestamp),
                ),
            )
            .expect("native press should start a drag");
        assert_eq!(
            started.input_metadata(),
            DragHandleMetadata {
                modifiers: press_modifiers,
                timestamp: Some(press_timestamp),
                sequence_range: None,
            }
        );

        let moved = handle
            .handle_input(
                bounds,
                WidgetInput::pointer_move_with_metadata(
                    move_position,
                    move_modifiers,
                    Some(move_timestamp),
                    Some(move_sequence),
                ),
            )
            .expect("native move should remain active");
        assert_eq!(
            moved.input_metadata(),
            DragHandleMetadata {
                modifiers: move_modifiers,
                timestamp: Some(move_timestamp),
                sequence_range: Some(move_sequence),
            }
        );

        let ended = handle
            .handle_input(
                bounds,
                WidgetInput::pointer_release_with_timestamp(
                    release_position,
                    PointerButton::Primary,
                    release_modifiers,
                    Some(release_timestamp),
                ),
            )
            .expect("native release should end a drag");
        assert_eq!(
            ended.input_metadata(),
            DragHandleMetadata {
                modifiers: release_modifiers,
                timestamp: Some(release_timestamp),
                sequence_range: None,
            }
        );

        let double_modifiers = PointerModifiers {
            command: true,
            shift: true,
            alt: false,
        };
        let double_timestamp = InputTimestamp::capture();
        let double_activated = handle
            .handle_input(
                bounds,
                WidgetInput::pointer_double_click_with_timestamp(
                    release_position,
                    PointerButton::Primary,
                    double_modifiers,
                    Some(double_timestamp),
                ),
            )
            .expect("native double-click should activate the handle");
        assert_eq!(
            double_activated.input_metadata(),
            DragHandleMetadata {
                modifiers: double_modifiers,
                timestamp: Some(double_timestamp),
                sequence_range: None,
            }
        );
    }

    #[test]
    fn drag_handle_focus_loss_cancels_active_drag() {
        let mut handle = DragHandleWidget::new(8, WidgetSizing::fixed(Vector2::new(24.0, 16.0)));
        let bounds = Rect::from_size(24.0, 16.0);

        assert_eq!(
            handle_drag_handle_input(
                &mut handle,
                bounds,
                WidgetInput::primary_press(Point::new(8.0, 6.0)),
            ),
            Some(DragHandleMessage::started(Point::new(8.0, 6.0)))
        );
        let cancelled =
            handle_drag_handle_input(&mut handle, bounds, WidgetInput::FocusChanged(false))
                .expect("focus loss should cancel the active drag");
        assert_eq!(
            cancelled,
            DragHandleMessage::Cancelled {
                position: bounds.center()
            }
        );
        assert_eq!(cancelled.input_metadata(), DragHandleMetadata::empty());
        assert!(!handle.common.state.pressed);
        assert!(!handle.common.state.active);
    }
}
