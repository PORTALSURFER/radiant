//! Reusable radial knob primitive with explicit host-automation gestures.

mod builders;
mod input;
mod retained;

pub(crate) use retained::RetainedKnobWidget;

use std::f32::consts::TAU;

use crate::gui::{
    input::InputTimestamp,
    types::{Point, Rect, Vector2},
};
use crate::layout::LayoutOutput;
use crate::runtime::{PaintPrimitive, PaintStrokePolyline};
use crate::theme::ThemeTokens;
use crate::widgets::contract::{
    FocusBehavior, PaintBounds, Widget, WidgetCapabilities, WidgetId, WidgetSemantics, WidgetSizing,
};
use crate::widgets::interaction::{
    KnobKeyboardGesture, KnobKeyboardMetadata, KnobMessage, KnobPointerMetadata, KnobWheelGesture,
    KnobWheelMetadata, PointerButton, WidgetInput, WidgetKey, WidgetOutput,
};

use super::support::{WidgetCommon, clamp_fraction, push_automation_active_marker};

const DEFAULT_DIAMETER: f32 = 40.0;
const DEFAULT_SENSITIVITY: f32 = 0.006;
const WHEEL_STEP: f32 = 0.05;
const WHEEL_FINE_STEP: f32 = 0.002;
const ARC_START: f32 = -5.0 * std::f32::consts::PI / 4.0;
const ARC_SWEEP: f32 = 3.0 * std::f32::consts::PI / 2.0;
const ARC_SEGMENTS: usize = 40;

/// Immutable radial knob configuration.
#[derive(Clone, Debug, PartialEq)]
pub struct KnobProps {
    /// Normalized value restored by a reset gesture.
    pub default_value: f32,
    /// Normalized value delta per logical vertical pointer pixel.
    pub sensitivity: f32,
    /// Whether a primary double-click resets to `default_value`.
    pub reset_on_double_click: bool,
}

/// Mutable radial knob interaction state.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct KnobState {
    /// Current normalized value.
    pub value: f32,
    /// Pointer position at the beginning of the active gesture.
    pub gesture_origin: Option<Point>,
    /// Whether Shift fine-adjustment mode is latched for the active gesture.
    pub fine_adjustment: bool,
}

/// Named construction fields for [`KnobWidget`].
#[derive(Clone, Debug, PartialEq)]
pub struct KnobWidgetParts {
    /// Stable widget identity.
    pub id: WidgetId,
    /// Initial normalized value.
    pub value: f32,
    /// Intrinsic knob sizing contract.
    pub sizing: WidgetSizing,
}

/// Public reusable radial knob descriptor.
#[derive(Clone, Debug, PartialEq)]
pub struct KnobWidget {
    /// Shared widget contract.
    pub common: WidgetCommon,
    /// Immutable knob configuration.
    pub props: KnobProps,
    /// Mutable knob interaction state.
    pub state: KnobState,
}

impl KnobWidget {
    /// Build a knob with normalized value and keyboard focus semantics.
    pub fn from_parts(parts: KnobWidgetParts) -> Self {
        let mut common = WidgetCommon::new(parts.id, parts.sizing);
        common.focus = FocusBehavior::Keyboard;
        common.paint.bounds = PaintBounds::ClipToRect;
        Self {
            common,
            props: KnobProps {
                default_value: clamp_fraction(parts.value),
                sensitivity: DEFAULT_SENSITIVITY,
                reset_on_double_click: true,
            },
            state: KnobState {
                value: clamp_fraction(parts.value),
                gesture_origin: None,
                fine_adjustment: false,
            },
        }
    }

    /// Build a knob using a fixed 40px diameter sizing contract.
    pub fn new(id: WidgetId, value: f32) -> Self {
        Self::from_parts(KnobWidgetParts {
            id,
            value,
            sizing: WidgetSizing::fixed(Vector2::new(DEFAULT_DIAMETER, DEFAULT_DIAMETER)),
        })
    }

    /// Set the value restored by reset gestures.
    pub fn with_default_value(mut self, value: f32) -> Self {
        self.props.default_value = clamp_fraction(value);
        self
    }

    /// Set vertical pointer sensitivity in normalized units per pixel.
    pub fn with_sensitivity(mut self, sensitivity: f32) -> Self {
        self.props.sensitivity = sensitivity.max(0.0001);
        self
    }

    /// Enable or disable primary double-click reset behavior.
    pub fn with_reset_on_double_click(mut self, enabled: bool) -> Self {
        self.props.reset_on_double_click = enabled;
        self
    }

    /// Route backend-neutral input and emit explicit automation messages.
    pub fn handle_input(&mut self, bounds: Rect, input: WidgetInput) -> Option<KnobMessage> {
        match &input {
            WidgetInput::PointerRelease {
                button: PointerButton::Primary,
                modifiers,
                timestamp,
                ..
            }
            | WidgetInput::PointerDrop {
                button: PointerButton::Primary,
                modifiers,
                timestamp,
                ..
            } => {
                return self.finish_terminal_gesture(
                    false,
                    KnobPointerMetadata {
                        modifiers: *modifiers,
                        timestamp: *timestamp,
                        sequence_range: None,
                    },
                );
            }
            WidgetInput::FocusChanged(false) => {
                return self.finish_terminal_gesture(true, KnobPointerMetadata::empty());
            }
            _ => {}
        }
        if self.common.state.disabled {
            return None;
        }
        match input {
            WidgetInput::PointerModifiersChanged {
                modifiers,
                timestamp: _,
            } => {
                if self.common.state.pressed {
                    self.state.fine_adjustment = modifiers.shift;
                }
                None
            }
            WidgetInput::PointerPress {
                position,
                button: PointerButton::Primary,
                modifiers,
                timestamp,
            } if bounds.contains(position) => {
                self.common.state.hovered = true;
                self.common.state.pressed = true;
                self.common.state.focused = true;
                self.state.fine_adjustment = modifiers.shift;
                self.state.gesture_origin = Some(position);
                Some(KnobMessage::GestureStarted {
                    value: self.state.value,
                    metadata: KnobPointerMetadata {
                        modifiers,
                        timestamp,
                        sequence_range: None,
                    },
                })
            }
            WidgetInput::PointerMove {
                position,
                modifiers,
                timestamp,
                sequence_range,
            } => {
                self.common.state.hovered = bounds.contains(position);
                if !self.common.state.pressed {
                    return None;
                }
                let origin = self.state.gesture_origin.unwrap_or(position);
                self.state.gesture_origin = Some(position);
                let sensitivity = if self.state.fine_adjustment {
                    self.props.sensitivity * 0.1
                } else {
                    self.props.sensitivity
                };
                self.set_value(self.state.value + (origin.y - position.y) * sensitivity)
                    .map(|value| KnobMessage::ValueChanged {
                        value,
                        metadata: KnobPointerMetadata {
                            modifiers,
                            timestamp,
                            sequence_range,
                        },
                    })
            }
            WidgetInput::Wheel {
                position,
                delta,
                modifiers,
                timestamp,
                sequence_range,
            } => {
                // Wheel input is an independent hover gesture. Do not let it
                // alter or terminate an active captured pointer drag.
                if self.common.state.pressed
                    || self.state.gesture_origin.is_some()
                    || !bounds.contains(position)
                {
                    return None;
                }
                let direction = if delta.y > 0.0 {
                    1.0
                } else if delta.y < 0.0 {
                    -1.0
                } else {
                    // Horizontal-only, zero, and unsigned vertical deltas
                    // remain available to the surrounding scroll surface.
                    return None;
                };
                let step = if modifiers.shift {
                    WHEEL_FINE_STEP
                } else {
                    WHEEL_STEP
                };
                let start_value = self.state.value;
                let final_value = self.set_value(start_value + direction * step)?;
                Some(KnobMessage::WheelGesture(
                    KnobWheelGesture::new_with_metadata(
                        start_value,
                        final_value,
                        KnobWheelMetadata {
                            modifiers,
                            timestamp,
                            sequence_range,
                        },
                    ),
                ))
            }
            WidgetInput::PointerDoubleClick {
                position,
                button: PointerButton::Primary,
                modifiers,
                timestamp,
            } if self.props.reset_on_double_click && bounds.contains(position) => {
                self.common.state.pressed = false;
                self.state.fine_adjustment = false;
                self.state.gesture_origin = None;
                self.state.value = self.props.default_value;
                Some(KnobMessage::Reset {
                    value: self.state.value,
                    metadata: KnobPointerMetadata {
                        modifiers,
                        timestamp,
                        sequence_range: None,
                    },
                })
            }
            WidgetInput::FocusChanged(focused) => {
                self.common.state.focused = focused;
                None
            }
            WidgetInput::KeyPress { key, timestamp, .. } if self.common.state.focused => {
                match key {
                    WidgetKey::ArrowLeft | WidgetKey::ArrowDown => self.keyboard_gesture(
                        self.state.value - self.props.sensitivity * 16.0,
                        timestamp,
                    ),
                    WidgetKey::ArrowRight | WidgetKey::ArrowUp => self.keyboard_gesture(
                        self.state.value + self.props.sensitivity * 16.0,
                        timestamp,
                    ),
                    WidgetKey::Home => self.keyboard_gesture(0.0, timestamp),
                    WidgetKey::End => self.keyboard_gesture(1.0, timestamp),
                    _ => None,
                }
            }
            _ => None,
        }
    }

    fn set_value(&mut self, value: f32) -> Option<f32> {
        let value = clamp_fraction(value);
        if (self.state.value - value).abs() <= f32::EPSILON {
            return None;
        }
        self.state.value = value;
        Some(value)
    }

    pub(super) fn is_editable(&self) -> bool {
        !self.common.state.disabled && !self.common.state.read_only
    }

    fn keyboard_gesture(
        &mut self,
        value: f32,
        timestamp: Option<InputTimestamp>,
    ) -> Option<KnobMessage> {
        let start_value = self.state.value;
        let final_value = self.set_value(value)?;
        Some(KnobMessage::KeyboardGesture(
            KnobKeyboardGesture::new_with_metadata(
                start_value,
                final_value,
                KnobKeyboardMetadata { timestamp },
            ),
        ))
    }

    fn finish_terminal_gesture(
        &mut self,
        focus_lost: bool,
        metadata: KnobPointerMetadata,
    ) -> Option<KnobMessage> {
        let had_active_gesture = self.state.gesture_origin.take().is_some();
        self.common.state.pressed = false;
        self.state.fine_adjustment = false;
        if focus_lost {
            self.common.state.focused = false;
        }
        had_active_gesture.then_some(KnobMessage::GestureEnded {
            value: self.state.value,
            metadata,
        })
    }
}

impl WidgetSemantics for KnobWidget {
    fn automation_role(&self) -> crate::gui::automation::AutomationRole {
        crate::gui::automation::AutomationRole::Slider
    }

    fn automation_value_text(&self) -> Option<String> {
        Some(format!("{:.3}", self.state.value))
    }
}

impl Widget for KnobWidget {
    fn common(&self) -> &WidgetCommon {
        &self.common
    }

    fn common_mut(&mut self) -> &mut WidgetCommon {
        &mut self.common
    }

    fn handle_input(&mut self, bounds: Rect, input: WidgetInput) -> Option<WidgetOutput> {
        KnobWidget::handle_input(self, bounds, input).map(WidgetOutput::typed)
    }

    fn synchronize_from_previous(&mut self, previous: &dyn Widget) {
        let Some(previous) = previous.as_any().downcast_ref::<Self>() else {
            return;
        };
        // The fresh projection remains authoritative for value, default,
        // sizing, style, and semantic state. Only runtime-owned interaction
        // details survive a refresh.
        self.common.state.hovered = previous.common.state.hovered;
        self.common.state.focused = previous.common.state.focused;
        self.common.state.pressed = previous.common.state.pressed;
        self.state.fine_adjustment = previous.state.fine_adjustment;
        self.state.gesture_origin = previous.state.gesture_origin;
    }

    fn accepts_pointer_move(&self) -> bool {
        true
    }

    fn accepts_wheel_input(&self) -> bool {
        true
    }

    fn capabilities(&self) -> WidgetCapabilities<'_> {
        WidgetCapabilities::new().semantics(self)
    }

    fn append_paint(
        &self,
        primitives: &mut Vec<PaintPrimitive>,
        bounds: Rect,
        _layout: &LayoutOutput,
        theme: &ThemeTokens,
    ) {
        let tokens = crate::widgets::resolve_widget_visual_tokens(
            theme,
            self.common.style,
            self.common.state,
        );
        let center = Point::new(
            (bounds.min.x + bounds.max.x) * 0.5,
            (bounds.min.y + bounds.max.y) * 0.5,
        );
        let radius = bounds.width().min(bounds.height()) * 0.5 - 2.0;
        let ring = arc_points(center, radius.max(1.0), ARC_START, ARC_SWEEP, ARC_SEGMENTS);
        primitives.push(PaintPrimitive::StrokePolyline(PaintStrokePolyline {
            widget_id: self.common.id,
            points: ring.into(),
            color: tokens.border,
            width: if tokens.cue == crate::widgets::WidgetVisualCue::Focused {
                2.0
            } else {
                1.0
            },
        }));
        let value_arc = arc_points(
            center,
            (radius - 1.0).max(1.0),
            ARC_START,
            ARC_SWEEP * self.state.value.clamp(0.0, 1.0),
            ARC_SEGMENTS,
        );
        primitives.push(PaintPrimitive::StrokePolyline(PaintStrokePolyline {
            widget_id: self.common.id,
            points: value_arc.into(),
            color: tokens.emphasis,
            width: 3.0,
        }));
        if self.common.state.selected && !self.common.state.disabled {
            // Selection gets a shape cue independent from the resolved color
            // precedence: a short top tick remains visible alongside focus or
            // automation markers.
            let marker_angle = -TAU * 0.25;
            let outer = Point::new(
                center.x + radius * marker_angle.cos(),
                center.y + radius * marker_angle.sin(),
            );
            let inner = Point::new(
                center.x + (radius - 4.0) * marker_angle.cos(),
                center.y + (radius - 4.0) * marker_angle.sin(),
            );
            primitives.push(PaintPrimitive::StrokePolyline(PaintStrokePolyline {
                widget_id: self.common.id,
                points: [inner, outer].into(),
                color: tokens.emphasis,
                width: 2.0,
            }));
        }
        push_automation_active_marker(
            primitives,
            self.common.id,
            bounds,
            self.common.state,
            tokens.emphasis,
        );
        if self.common.state.focused && self.common.paint.paints_focus {
            let focus_ring = circle_points(center, (radius + 2.0).max(1.0), 40);
            primitives.push(PaintPrimitive::StrokePolyline(PaintStrokePolyline {
                widget_id: self.common.id,
                points: focus_ring.into(),
                color: tokens.emphasis,
                width: 1.0,
            }));
        }
    }
}

fn circle_points(center: Point, radius: f32, segments: usize) -> Vec<Point> {
    (0..=segments)
        .map(|index| {
            let angle = TAU * index as f32 / segments as f32;
            Point::new(
                center.x + radius * angle.cos(),
                center.y + radius * angle.sin(),
            )
        })
        .collect()
}

fn arc_points(center: Point, radius: f32, start: f32, sweep: f32, segments: usize) -> Vec<Point> {
    (0..=segments)
        .map(|index| {
            let fraction = index as f32 / segments as f32;
            let angle = start + sweep * fraction;
            Point::new(
                center.x + radius * angle.cos(),
                center.y + radius * angle.sin(),
            )
        })
        .collect()
}

#[cfg(test)]
#[path = "knob/typed_tests.rs"]
mod typed_tests;

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        gui::{
            input::{InputSequence, InputSequenceRange, InputTimestamp},
            types::Point,
        },
        runtime::PaintPrimitive,
        widgets::interaction::PointerModifiers,
        widgets::{
            KnobAutomationEvent, KnobKeyboardGesture, KnobKeyboardMetadata, KnobPointerMetadata,
            KnobWheelGesture, KnobWheelMetadata, WidgetState, WidgetVisualCue,
        },
    };

    #[test]
    fn knob_emits_gesture_start_value_end_and_reset() {
        let bounds = Rect::from_min_size(Point::new(0.0, 0.0), Vector2::new(40.0, 40.0));
        let mut knob = KnobWidget::new(1, 0.5).with_default_value(0.25);
        assert_eq!(
            knob.handle_input(bounds, WidgetInput::primary_press(Point::new(20.0, 20.0))),
            Some(KnobMessage::GestureStarted {
                value: 0.5,
                metadata: KnobPointerMetadata::default(),
            })
        );
        assert!(matches!(
            knob.handle_input(bounds, WidgetInput::pointer_move(Point::new(20.0, 0.0))),
            Some(KnobMessage::ValueChanged { .. })
        ));
        assert!(matches!(
            knob.handle_input(bounds, WidgetInput::primary_release(Point::new(20.0, 0.0))),
            Some(KnobMessage::GestureEnded { .. })
        ));
        assert_eq!(
            knob.handle_input(
                bounds,
                WidgetInput::primary_double_click(Point::new(80.0, 80.0))
            ),
            None
        );
        assert_ne!(knob.state.value, 0.25);
        assert_eq!(
            knob.handle_input(
                bounds,
                WidgetInput::primary_double_click(Point::new(20.0, 20.0))
            ),
            Some(KnobMessage::Reset {
                value: 0.25,
                metadata: KnobPointerMetadata::default(),
            })
        );
    }

    #[test]
    fn knob_reset_forwards_native_metadata_and_cleans_pointer_state() {
        let bounds = Rect::from_min_size(Point::new(0.0, 0.0), Vector2::new(40.0, 40.0));
        let mut knob = KnobWidget::new(1, 0.5).with_default_value(0.25);
        assert!(matches!(
            knob.handle_input(
                bounds,
                WidgetInput::pointer_press(
                    Point::new(20.0, 20.0),
                    PointerButton::Primary,
                    PointerModifiers {
                        shift: true,
                        ..PointerModifiers::default()
                    },
                ),
            ),
            Some(KnobMessage::GestureStarted { .. })
        ));
        assert!(knob.common.state.pressed);
        assert!(knob.state.fine_adjustment);
        assert_eq!(knob.state.gesture_origin, Some(Point::new(20.0, 20.0)));

        let modifiers = PointerModifiers {
            command: true,
            alt: true,
            ..PointerModifiers::default()
        };
        let timestamp = InputTimestamp::capture();
        let metadata = KnobPointerMetadata {
            modifiers,
            timestamp: Some(timestamp),
            sequence_range: None,
        };
        let reset = knob.handle_input(
            bounds,
            WidgetInput::pointer_double_click_with_timestamp(
                Point::new(20.0, 20.0),
                PointerButton::Primary,
                modifiers,
                Some(timestamp),
            ),
        );
        assert_eq!(
            reset,
            Some(KnobMessage::Reset {
                value: 0.25,
                metadata,
            })
        );
        assert_eq!(
            reset
                .as_ref()
                .and_then(KnobMessage::pointer_gesture_metadata),
            Some(metadata)
        );
        assert_eq!(knob.state.value, 0.25);
        assert!(!knob.common.state.pressed);
        assert!(!knob.state.fine_adjustment);
        assert_eq!(knob.state.gesture_origin, None);
        assert!(knob.common.state.focused);
        assert_eq!(
            knob.handle_input(bounds, WidgetInput::primary_release(Point::new(20.0, 20.0))),
            None
        );
    }

    #[test]
    fn knob_reset_emits_once_when_value_already_equals_default() {
        let bounds = Rect::from_min_size(Point::new(0.0, 0.0), Vector2::new(40.0, 40.0));
        let mut knob = KnobWidget::new(1, 0.25).with_default_value(0.25);
        assert_eq!(
            knob.handle_input(
                bounds,
                WidgetInput::primary_double_click(Point::new(20.0, 20.0)),
            ),
            Some(KnobMessage::Reset {
                value: 0.25,
                metadata: KnobPointerMetadata::default(),
            })
        );
        assert_eq!(
            knob.handle_input(bounds, WidgetInput::primary_release(Point::new(20.0, 20.0))),
            None
        );
        assert_eq!(knob.state.value, 0.25);
    }

    #[test]
    fn knob_pointer_gesture_forwards_native_metadata_by_phase() {
        let bounds = Rect::from_min_size(Point::new(0.0, 0.0), Vector2::new(40.0, 40.0));
        let mut knob = KnobWidget::new(1, 0.5).with_sensitivity(0.01);
        let press_position = Point::new(20.0, 20.0);
        let move_position = Point::new(20.0, 10.0);
        let press_modifiers = PointerModifiers {
            command: true,
            alt: true,
            ..PointerModifiers::default()
        };
        let move_modifiers = PointerModifiers {
            shift: true,
            alt: true,
            ..PointerModifiers::default()
        };
        let release_modifiers = PointerModifiers {
            command: true,
            shift: true,
            ..PointerModifiers::default()
        };
        let press_timestamp = InputTimestamp::capture();
        let move_timestamp = InputTimestamp::capture();
        let release_timestamp = InputTimestamp::capture();
        let mut move_sequence =
            InputSequenceRange::singleton(InputSequence::from_runtime_value(101));
        move_sequence.extend_end(InputSequence::from_runtime_value(104));

        let started = knob.handle_input(
            bounds,
            WidgetInput::pointer_press_with_timestamp(
                press_position,
                PointerButton::Primary,
                press_modifiers,
                Some(press_timestamp),
            ),
        );
        let started_metadata = KnobPointerMetadata {
            modifiers: press_modifiers,
            timestamp: Some(press_timestamp),
            sequence_range: None,
        };
        assert_eq!(
            started,
            Some(KnobMessage::GestureStarted {
                value: 0.5,
                metadata: started_metadata,
            })
        );
        assert_eq!(
            started
                .as_ref()
                .and_then(KnobMessage::pointer_gesture_metadata),
            Some(started_metadata)
        );

        let moved = knob.handle_input(
            bounds,
            WidgetInput::pointer_move_with_metadata(
                move_position,
                move_modifiers,
                Some(move_timestamp),
                Some(move_sequence),
            ),
        );
        let moved_metadata = KnobPointerMetadata {
            modifiers: move_modifiers,
            timestamp: Some(move_timestamp),
            sequence_range: Some(move_sequence),
        };
        assert!(matches!(
            moved,
            Some(KnobMessage::ValueChanged {
                value: 0.6,
                metadata,
            }) if metadata == moved_metadata
        ));
        assert_eq!(
            moved
                .as_ref()
                .and_then(KnobMessage::pointer_gesture_metadata),
            Some(moved_metadata)
        );

        assert_eq!(
            knob.handle_input(
                bounds,
                WidgetInput::pointer_move_with_metadata(
                    move_position,
                    PointerModifiers::default(),
                    None,
                    None,
                ),
            ),
            None
        );

        let ended = knob.handle_input(
            bounds,
            WidgetInput::pointer_release_with_timestamp(
                move_position,
                PointerButton::Primary,
                release_modifiers,
                Some(release_timestamp),
            ),
        );
        let ended_metadata = KnobPointerMetadata {
            modifiers: release_modifiers,
            timestamp: Some(release_timestamp),
            sequence_range: None,
        };
        assert_eq!(
            ended,
            Some(KnobMessage::GestureEnded {
                value: 0.6,
                metadata: ended_metadata,
            })
        );
        assert_eq!(
            ended
                .as_ref()
                .and_then(KnobMessage::pointer_gesture_metadata),
            Some(ended_metadata)
        );
    }

    #[test]
    fn knob_pointer_gesture_uses_empty_metadata_for_synthetic_and_focus_loss() {
        let bounds = Rect::from_min_size(Point::new(0.0, 0.0), Vector2::new(40.0, 40.0));
        let mut knob = KnobWidget::new(1, 0.5);
        assert_eq!(KnobPointerMetadata::empty(), KnobPointerMetadata::default());
        assert_eq!(
            knob.handle_input(bounds, WidgetInput::primary_press(Point::new(20.0, 20.0))),
            Some(KnobMessage::GestureStarted {
                value: 0.5,
                metadata: KnobPointerMetadata::default(),
            })
        );
        assert_eq!(
            knob.handle_input(bounds, WidgetInput::FocusChanged(false)),
            Some(KnobMessage::GestureEnded {
                value: 0.5,
                metadata: KnobPointerMetadata::empty(),
            })
        );
        assert_eq!(
            knob.handle_input(bounds, WidgetInput::primary_release(Point::new(20.0, 20.0))),
            None
        );
    }

    #[test]
    fn knob_pointer_gesture_metadata_is_not_reported_for_keyboard_or_wheel() {
        let metadata = KnobPointerMetadata {
            modifiers: PointerModifiers {
                shift: true,
                ..PointerModifiers::default()
            },
            ..KnobPointerMetadata::default()
        };
        assert_eq!(
            KnobMessage::GestureStarted {
                value: 0.25,
                metadata,
            }
            .pointer_gesture_metadata(),
            Some(metadata)
        );
        assert_eq!(
            KnobMessage::Reset {
                value: 0.25,
                metadata,
            }
            .pointer_gesture_metadata(),
            Some(metadata)
        );
        assert_eq!(
            KnobMessage::KeyboardGesture(KnobKeyboardGesture::new(0.25, 0.35))
                .pointer_gesture_metadata(),
            None
        );
        assert_eq!(
            KnobMessage::WheelGesture(KnobWheelGesture::new(0.25, 0.3)).pointer_gesture_metadata(),
            None
        );
    }

    #[test]
    fn knob_pointer_gesture_omits_clamped_noop_moves() {
        let bounds = Rect::from_min_size(Point::new(0.0, 0.0), Vector2::new(40.0, 40.0));
        let mut knob = KnobWidget::new(1, 0.0);
        assert!(matches!(
            knob.handle_input(bounds, WidgetInput::primary_press(Point::new(20.0, 20.0))),
            Some(KnobMessage::GestureStarted { .. })
        ));
        assert_eq!(
            knob.handle_input(
                bounds,
                WidgetInput::pointer_move_with_metadata(
                    Point::new(20.0, 40.0),
                    PointerModifiers::default(),
                    None,
                    None,
                ),
            ),
            None
        );
        assert_eq!(knob.state.value, 0.0);
        assert!(matches!(
            knob.handle_input(bounds, WidgetInput::primary_release(Point::new(20.0, 40.0))),
            Some(KnobMessage::GestureEnded { value: 0.0, .. })
        ));
    }

    #[test]
    fn knob_pointer_drop_forwards_terminal_metadata() {
        let bounds = Rect::from_min_size(Point::new(0.0, 0.0), Vector2::new(40.0, 40.0));
        let mut knob = KnobWidget::new(1, 0.5);
        assert!(matches!(
            knob.handle_input(bounds, WidgetInput::primary_press(Point::new(20.0, 20.0))),
            Some(KnobMessage::GestureStarted { .. })
        ));
        let modifiers = PointerModifiers {
            alt: true,
            ..PointerModifiers::default()
        };
        let timestamp = InputTimestamp::capture();
        assert_eq!(
            knob.handle_input(
                bounds,
                WidgetInput::pointer_drop_with_timestamp(
                    Point::new(80.0, 80.0),
                    PointerButton::Primary,
                    modifiers,
                    Some(timestamp),
                ),
            ),
            Some(KnobMessage::GestureEnded {
                value: 0.5,
                metadata: KnobPointerMetadata {
                    modifiers,
                    timestamp: Some(timestamp),
                    sequence_range: None,
                },
            })
        );
        assert_eq!(
            knob.handle_input(
                bounds,
                WidgetInput::pointer_drop_with_timestamp(
                    Point::new(80.0, 80.0),
                    PointerButton::Primary,
                    modifiers,
                    Some(timestamp),
                ),
            ),
            None
        );
    }

    #[test]
    fn knob_shift_fine_drag_tracks_modifier_changes_without_restarting_gesture() {
        let bounds = Rect::from_min_size(Point::new(0.0, 0.0), Vector2::new(40.0, 40.0));
        let mut knob = KnobWidget::new(1, 0.5).with_sensitivity(0.01);
        knob.common.state.active = true;
        assert_eq!(
            knob.handle_input(
                bounds,
                WidgetInput::pointer_press(
                    Point::new(20.0, 20.0),
                    PointerButton::Primary,
                    PointerModifiers::default(),
                ),
            ),
            Some(KnobMessage::GestureStarted {
                value: 0.5,
                metadata: KnobPointerMetadata::default(),
            })
        );
        assert!(knob.common.state.active);

        assert!(matches!(
            knob.handle_input(bounds, WidgetInput::pointer_move(Point::new(20.0, 10.0))),
            Some(KnobMessage::ValueChanged { .. })
        ));
        assert!((knob.state.value - 0.6).abs() < 0.0001);

        assert_eq!(
            knob.handle_input(
                bounds,
                WidgetInput::PointerModifiersChanged {
                    modifiers: PointerModifiers {
                        shift: true,
                        ..PointerModifiers::default()
                    },
                    timestamp: None,
                },
            ),
            None
        );
        assert!(matches!(
            knob.handle_input(bounds, WidgetInput::pointer_move(Point::new(20.0, 0.0))),
            Some(KnobMessage::ValueChanged { .. })
        ));
        assert!((knob.state.value - 0.61).abs() < 0.0001);
        assert!(knob.common.state.active);

        let mut refreshed = KnobWidget::new(1, knob.state.value).with_sensitivity(0.01);
        refreshed.common.state.active = true;
        refreshed.synchronize_from_previous(&knob);
        assert!(refreshed.common.state.active);
        assert!(refreshed.state.fine_adjustment);
        assert!(refreshed.state.gesture_origin.is_some());
        knob = refreshed;

        assert_eq!(
            knob.handle_input(
                bounds,
                WidgetInput::PointerModifiersChanged {
                    modifiers: PointerModifiers::default(),
                    timestamp: None,
                },
            ),
            None
        );
        assert!(matches!(
            knob.handle_input(bounds, WidgetInput::pointer_move(Point::new(20.0, -10.0))),
            Some(KnobMessage::ValueChanged { .. })
        ));
        assert!((knob.state.value - 0.71).abs() < 0.0001);
        assert!(knob.common.state.active);
        assert!(matches!(
            knob.handle_input(bounds, WidgetInput::primary_release(Point::new(20.0, -10.0))),
            Some(KnobMessage::GestureEnded { value, .. }) if (value - 0.71).abs() < 0.0001
        ));
        assert!(knob.common.state.active);
    }

    #[test]
    fn knob_keyboard_gesture_batch_requires_focus_and_preserves_clamped_order() {
        let bounds = Rect::from_min_size(Point::new(0.0, 0.0), Vector2::new(40.0, 40.0));
        let mut knob = KnobWidget::new(1, 0.98).with_sensitivity(0.1);
        assert_eq!(
            knob.handle_input(bounds, WidgetInput::key_press(WidgetKey::ArrowRight)),
            None
        );
        knob.handle_input(bounds, WidgetInput::FocusChanged(true));
        let Some(KnobMessage::KeyboardGesture(batch)) =
            knob.handle_input(bounds, WidgetInput::key_press(WidgetKey::ArrowRight))
        else {
            panic!("focused keyboard edit should emit a lifecycle batch");
        };
        assert_eq!(
            batch.events,
            [
                crate::widgets::KnobAutomationEvent::GestureStarted { value: 0.98 },
                crate::widgets::KnobAutomationEvent::ValueChanged { value: 1.0 },
                crate::widgets::KnobAutomationEvent::GestureEnded { value: 1.0 },
            ]
        );
        assert_eq!(batch.input_metadata(), KnobKeyboardMetadata::default());

        knob.common.state.disabled = true;
        assert_eq!(
            knob.handle_input(bounds, WidgetInput::key_press(WidgetKey::ArrowLeft)),
            None
        );
    }

    #[test]
    fn knob_keyboard_gesture_preserves_timestamp_for_an_accepted_value_change() {
        let bounds = Rect::from_min_size(Point::new(0.0, 0.0), Vector2::new(40.0, 40.0));
        let mut knob = KnobWidget::new(1, 0.5).with_sensitivity(0.01);
        knob.handle_input(bounds, WidgetInput::FocusChanged(true));
        let timestamp = InputTimestamp::capture();

        let Some(KnobMessage::KeyboardGesture(batch)) = knob.handle_input(
            bounds,
            WidgetInput::key_press_with_timestamp(WidgetKey::ArrowRight, Some(timestamp)),
        ) else {
            panic!("focused keyboard edit should emit a lifecycle batch");
        };

        assert_eq!(
            batch.events[0],
            KnobAutomationEvent::GestureStarted { value: 0.5 }
        );
        assert!(matches!(
            batch.events[1],
            KnobAutomationEvent::ValueChanged { value } if (value - 0.66).abs() < 0.00001
        ));
        assert!(matches!(
            batch.events[2],
            KnobAutomationEvent::GestureEnded { value } if (value - 0.66).abs() < 0.00001
        ));
        assert_eq!(
            batch.input_metadata(),
            KnobKeyboardMetadata {
                timestamp: Some(timestamp),
            }
        );
    }

    #[test]
    fn knob_keyboard_gesture_vetoes_unfocused_disabled_unsupported_and_noop_keys() {
        let bounds = Rect::from_min_size(Point::new(0.0, 0.0), Vector2::new(40.0, 40.0));
        let timestamp = Some(InputTimestamp::capture());
        let mut knob = KnobWidget::new(1, 0.5);

        assert_eq!(
            knob.handle_input(
                bounds,
                WidgetInput::key_press_with_timestamp(WidgetKey::ArrowRight, timestamp),
            ),
            None
        );
        knob.handle_input(bounds, WidgetInput::FocusChanged(true));
        assert_eq!(
            knob.handle_input(
                bounds,
                WidgetInput::key_press_with_timestamp(WidgetKey::Enter, timestamp),
            ),
            None
        );

        knob.state.value = 1.0;
        assert_eq!(
            knob.handle_input(
                bounds,
                WidgetInput::key_press_with_timestamp(WidgetKey::ArrowRight, timestamp),
            ),
            None
        );

        knob.common.state.disabled = true;
        assert_eq!(
            knob.handle_input(
                bounds,
                WidgetInput::key_press_with_timestamp(WidgetKey::ArrowLeft, timestamp),
            ),
            None
        );
        assert_eq!(knob.state.value, 1.0);
    }

    #[test]
    fn knob_wheel_gesture_uses_logical_vertical_sign_and_shift_fine_step() {
        let bounds = Rect::from_min_size(Point::new(0.0, 0.0), Vector2::new(40.0, 40.0));
        let mut knob = KnobWidget::new(1, 0.5);

        let Some(KnobMessage::WheelGesture(batch)) = knob.handle_input(
            bounds,
            WidgetInput::plain_wheel(Point::new(20.0, 20.0), Vector2::new(18.0, 120.0)),
        ) else {
            panic!("positive logical vertical wheel should emit a wheel lifecycle batch");
        };
        assert_eq!(
            batch.events[0],
            KnobAutomationEvent::GestureStarted { value: 0.5 }
        );
        assert_eq!(
            batch.events[1],
            KnobAutomationEvent::ValueChanged { value: 0.55 }
        );
        assert_eq!(
            batch.events[2],
            KnobAutomationEvent::GestureEnded { value: 0.55 }
        );

        let Some(KnobMessage::WheelGesture(batch)) = knob.handle_input(
            bounds,
            WidgetInput::wheel(
                Point::new(20.0, 20.0),
                Vector2::new(-32.0, -900.0),
                PointerModifiers {
                    shift: true,
                    command: true,
                    alt: true,
                },
            ),
        ) else {
            panic!("negative logical vertical wheel should emit a wheel lifecycle batch");
        };
        assert_eq!(
            batch.events[0],
            KnobAutomationEvent::GestureStarted { value: 0.55 }
        );
        assert!(matches!(
            batch.events[1],
            KnobAutomationEvent::ValueChanged { value } if (value - 0.548).abs() < 0.00001
        ));
        assert!(matches!(
            batch.events[2],
            KnobAutomationEvent::GestureEnded { value } if (value - 0.548).abs() < 0.00001
        ));
        assert!((knob.state.value - 0.548).abs() < 0.00001);
        assert_eq!(
            batch.input_metadata(),
            KnobWheelMetadata {
                modifiers: PointerModifiers {
                    shift: true,
                    command: true,
                    alt: true,
                },
                ..KnobWheelMetadata::default()
            }
        );
    }

    #[test]
    fn knob_wheel_gesture_preserves_native_metadata_and_sequence_ranges() {
        let bounds = Rect::from_min_size(Point::new(0.0, 0.0), Vector2::new(40.0, 40.0));
        let mut knob = KnobWidget::new(1, 0.5);
        let positive_modifiers = PointerModifiers {
            command: true,
            alt: true,
            ..PointerModifiers::default()
        };
        let positive_timestamp = InputTimestamp::capture();
        let mut positive_range =
            InputSequenceRange::singleton(InputSequence::from_runtime_value(21));
        positive_range.extend_end(InputSequence::from_runtime_value(24));

        let Some(KnobMessage::WheelGesture(positive)) = knob.handle_input(
            bounds,
            WidgetInput::wheel_with_metadata(
                Point::new(20.0, 20.0),
                Vector2::new(6.0, 120.0),
                positive_modifiers,
                Some(positive_timestamp),
                Some(positive_range),
            ),
        ) else {
            panic!("metadata-bearing positive wheel should emit a gesture");
        };
        assert_eq!(
            positive.events,
            [
                KnobAutomationEvent::GestureStarted { value: 0.5 },
                KnobAutomationEvent::ValueChanged { value: 0.55 },
                KnobAutomationEvent::GestureEnded { value: 0.55 },
            ]
        );
        assert_eq!(
            positive.input_metadata(),
            KnobWheelMetadata {
                modifiers: positive_modifiers,
                timestamp: Some(positive_timestamp),
                sequence_range: Some(positive_range),
            }
        );

        let negative_modifiers = PointerModifiers {
            shift: true,
            command: true,
            ..PointerModifiers::default()
        };
        let negative_timestamp = InputTimestamp::capture();
        let mut negative_range =
            InputSequenceRange::singleton(InputSequence::from_runtime_value(31));
        negative_range.extend_end(InputSequence::from_runtime_value(35));

        let Some(KnobMessage::WheelGesture(negative)) = knob.handle_input(
            bounds,
            WidgetInput::wheel_with_metadata(
                Point::new(20.0, 20.0),
                Vector2::new(-8.0, -120.0),
                negative_modifiers,
                Some(negative_timestamp),
                Some(negative_range),
            ),
        ) else {
            panic!("metadata-bearing negative wheel should emit a gesture");
        };
        assert_eq!(
            negative.events[0],
            KnobAutomationEvent::GestureStarted { value: 0.55 }
        );
        assert!(matches!(
            negative.events[1],
            KnobAutomationEvent::ValueChanged { value } if (value - 0.548).abs() < 0.00001
        ));
        assert!(matches!(
            negative.events[2],
            KnobAutomationEvent::GestureEnded { value } if (value - 0.548).abs() < 0.00001
        ));
        assert_eq!(
            negative.input_metadata(),
            KnobWheelMetadata {
                modifiers: negative_modifiers,
                timestamp: Some(negative_timestamp),
                sequence_range: Some(negative_range),
            }
        );
    }

    #[test]
    fn knob_wheel_synthetic_and_missing_metadata_use_default_options() {
        let bounds = Rect::from_min_size(Point::new(0.0, 0.0), Vector2::new(40.0, 40.0));
        let inside = Point::new(20.0, 20.0);
        let mut knob = KnobWidget::new(1, 0.2);

        let Some(KnobMessage::WheelGesture(plain)) = knob.handle_input(
            bounds,
            WidgetInput::plain_wheel(inside, Vector2::new(0.0, 120.0)),
        ) else {
            panic!("plain wheel should emit a gesture");
        };
        assert_eq!(plain.input_metadata(), KnobWheelMetadata::default());

        let modifiers = PointerModifiers {
            alt: true,
            ..PointerModifiers::default()
        };
        let Some(KnobMessage::WheelGesture(public)) = knob.handle_input(
            bounds,
            WidgetInput::wheel(inside, Vector2::new(0.0, -120.0), modifiers),
        ) else {
            panic!("public wheel should emit a gesture");
        };
        assert_eq!(
            public.input_metadata(),
            KnobWheelMetadata {
                modifiers,
                ..KnobWheelMetadata::default()
            }
        );

        let Some(KnobMessage::WheelGesture(missing)) = knob.handle_input(
            bounds,
            WidgetInput::wheel_with_metadata(
                inside,
                Vector2::new(0.0, 120.0),
                modifiers,
                None,
                None,
            ),
        ) else {
            panic!("wheel without optional metadata should emit a gesture");
        };
        assert_eq!(
            missing.input_metadata(),
            KnobWheelMetadata {
                modifiers,
                ..KnobWheelMetadata::default()
            }
        );
    }

    #[test]
    fn knob_ignores_ineffective_wheel_inputs_and_preserves_pointer_drag() {
        let bounds = Rect::from_min_size(Point::new(0.0, 0.0), Vector2::new(40.0, 40.0));
        let outside = Point::new(80.0, 20.0);
        let inside = Point::new(20.0, 20.0);
        let mut knob = KnobWidget::new(1, 0.0);

        for input in [
            WidgetInput::plain_wheel(inside, Vector2::new(0.0, 0.0)),
            WidgetInput::plain_wheel(inside, Vector2::new(20.0, 0.0)),
            WidgetInput::plain_wheel(outside, Vector2::new(0.0, -120.0)),
        ] {
            assert_eq!(knob.handle_input(bounds, input), None);
            assert_eq!(knob.state.value, 0.0);
        }
        knob.common.state.disabled = true;
        assert_eq!(
            knob.handle_input(
                bounds,
                WidgetInput::plain_wheel(inside, Vector2::new(0.0, 120.0)),
            ),
            None
        );
        assert_eq!(knob.state.value, 0.0);
        knob.common.state.disabled = false;
        assert_eq!(
            knob.handle_input(
                bounds,
                WidgetInput::wheel(
                    inside,
                    Vector2::new(0.0, -120.0),
                    PointerModifiers::default()
                )
            ),
            None,
        );
        assert_eq!(knob.state.value, 0.0);

        knob.state.value = 1.0;
        assert_eq!(
            knob.handle_input(
                bounds,
                WidgetInput::plain_wheel(inside, Vector2::new(0.0, 120.0)),
            ),
            None,
        );
        assert_eq!(knob.state.value, 1.0);

        knob.state.value = 0.5;
        assert!(matches!(
            knob.handle_input(bounds, WidgetInput::primary_press(inside)),
            Some(KnobMessage::GestureStarted { .. })
        ));
        assert_eq!(
            knob.handle_input(
                bounds,
                WidgetInput::plain_wheel(inside, Vector2::new(0.0, -120.0)),
            ),
            None,
        );
        assert_eq!(knob.state.value, 0.5);
        assert!(knob.common.state.pressed);
        assert!(knob.state.gesture_origin.is_some());
    }

    #[test]
    fn knob_reset_ignores_outside_disabled_and_disabled_inputs() {
        let bounds = Rect::from_min_size(Point::new(0.0, 0.0), Vector2::new(40.0, 40.0));
        let mut knob = KnobWidget::new(1, 0.8).with_default_value(0.2);
        assert_eq!(
            knob.handle_input(
                bounds,
                WidgetInput::primary_double_click(Point::new(80.0, 80.0))
            ),
            None
        );
        assert_eq!(knob.state.value, 0.8);

        knob = knob.with_reset_on_double_click(false);
        assert_eq!(
            knob.handle_input(
                bounds,
                WidgetInput::primary_double_click(Point::new(20.0, 20.0))
            ),
            None
        );
        assert_eq!(knob.state.value, 0.8);

        knob = knob.with_reset_on_double_click(true);
        knob.common.state.disabled = true;
        assert_eq!(
            knob.handle_input(
                bounds,
                WidgetInput::primary_double_click(Point::new(20.0, 20.0))
            ),
            None
        );
        assert_eq!(knob.state.value, 0.8);
    }

    #[test]
    fn knob_focus_loss_ends_active_pointer_gesture_once() {
        let bounds = Rect::from_min_size(Point::new(0.0, 0.0), Vector2::new(40.0, 40.0));
        let mut knob = KnobWidget::new(1, 0.5);
        assert!(matches!(
            knob.handle_input(bounds, WidgetInput::primary_press(Point::new(20.0, 20.0))),
            Some(KnobMessage::GestureStarted {
                value: 0.5,
                metadata,
            })
                if metadata == KnobPointerMetadata::default()
        ));
        assert_eq!(
            knob.handle_input(bounds, WidgetInput::FocusChanged(false)),
            Some(KnobMessage::GestureEnded {
                value: 0.5,
                metadata: KnobPointerMetadata::default(),
            })
        );
        assert_eq!(
            knob.handle_input(bounds, WidgetInput::primary_release(Point::new(20.0, 20.0))),
            None
        );

        assert_eq!(
            knob.handle_input(bounds, WidgetInput::FocusChanged(true)),
            None
        );
        assert_eq!(
            knob.handle_input(bounds, WidgetInput::FocusChanged(false)),
            None
        );
    }

    #[test]
    fn secondary_terminal_inputs_do_not_cancel_primary_knob_gesture() {
        let bounds = Rect::from_min_size(Point::new(0.0, 0.0), Vector2::new(40.0, 40.0));
        let mut knob = KnobWidget::new(1, 0.5);
        assert!(matches!(
            knob.handle_input(bounds, WidgetInput::primary_press(Point::new(20.0, 20.0))),
            Some(KnobMessage::GestureStarted { .. })
        ));
        assert_eq!(
            knob.handle_input(
                bounds,
                WidgetInput::pointer_release(
                    Point::new(20.0, 20.0),
                    PointerButton::Secondary,
                    Default::default(),
                )
            ),
            None
        );
        assert_eq!(
            knob.handle_input(
                bounds,
                WidgetInput::pointer_drop(
                    Point::new(20.0, 20.0),
                    PointerButton::Secondary,
                    Default::default(),
                )
            ),
            None
        );
        assert!(knob.common.state.pressed);
        assert!(knob.state.gesture_origin.is_some());
        assert_eq!(
            knob.handle_input(bounds, WidgetInput::primary_release(Point::new(20.0, 20.0))),
            Some(KnobMessage::GestureEnded {
                value: 0.5,
                metadata: KnobPointerMetadata::default(),
            })
        );
    }

    #[test]
    fn knob_paints_all_state_variants_with_non_color_automation_marker() {
        let bounds = Rect::from_min_size(Point::new(0.0, 0.0), Vector2::new(40.0, 40.0));
        for state in [
            WidgetState::default(),
            WidgetState {
                hovered: true,
                ..WidgetState::default()
            },
            WidgetState {
                pressed: true,
                ..WidgetState::default()
            },
            WidgetState {
                focused: true,
                ..WidgetState::default()
            },
            WidgetState {
                selected: true,
                ..WidgetState::default()
            },
            WidgetState {
                disabled: true,
                ..WidgetState::default()
            },
            WidgetState {
                automation_active: true,
                ..WidgetState::default()
            },
        ] {
            let mut knob = KnobWidget::new(1, 0.5);
            knob.common.state = state;
            let mut primitives = Vec::new();
            knob.append_paint(
                &mut primitives,
                bounds,
                &LayoutOutput::default(),
                &ThemeTokens::default(),
            );
            assert!(
                primitives
                    .iter()
                    .any(|p| matches!(p, PaintPrimitive::StrokePolyline(_)))
            );
            if state.automation_active {
                assert_eq!(
                    crate::widgets::resolve_widget_visual_tokens(
                        &ThemeTokens::default(),
                        knob.common.style,
                        state
                    )
                    .cue,
                    WidgetVisualCue::AutomationActive
                );
                assert!(primitives.len() >= 3);
            }
        }
    }

    #[test]
    fn focused_automation_knob_keeps_focus_ring_and_automation_marker() {
        let bounds = Rect::from_min_size(Point::new(0.0, 0.0), Vector2::new(40.0, 40.0));
        let mut knob = KnobWidget::new(1, 0.5);
        knob.common.state.focused = true;
        knob.common.state.automation_active = true;
        let mut primitives = Vec::new();

        knob.append_paint(
            &mut primitives,
            bounds,
            &LayoutOutput::default(),
            &ThemeTokens::default(),
        );

        // Base ring + value indicator + automation tick + focus ring.
        assert_eq!(primitives.len(), 4);
        assert_eq!(
            crate::widgets::resolve_widget_visual_tokens(
                &ThemeTokens::default(),
                knob.common.style,
                knob.common.state,
            )
            .cue,
            WidgetVisualCue::Focused
        );
    }

    #[test]
    fn knob_paints_270_degree_track_and_value_arc() {
        let bounds = Rect::from_min_size(Point::new(0.0, 0.0), Vector2::new(40.0, 40.0));
        let knob = KnobWidget::new(1, 0.5);
        let mut primitives = Vec::new();

        knob.append_paint(
            &mut primitives,
            bounds,
            &LayoutOutput::default(),
            &ThemeTokens::default(),
        );

        let polylines: Vec<_> = primitives
            .iter()
            .filter_map(|primitive| match primitive {
                PaintPrimitive::StrokePolyline(polyline) => Some(polyline),
                _ => None,
            })
            .collect();
        assert_eq!(polylines.len(), 2);
        assert_eq!(polylines[0].points.len(), ARC_SEGMENTS + 1);
        assert_eq!(polylines[1].points.len(), ARC_SEGMENTS + 1);
        assert_eq!(polylines[0].width, 1.0);
        assert_eq!(polylines[1].width, 3.0);

        let center = Point::new(20.0, 20.0);
        let track_start = polylines[0].points.first().expect("track has a start");
        let track_end = polylines[0].points.last().expect("track has an end");
        assert!((track_start.x - (center.x - 18.0 * 2.0_f32.sqrt() / 2.0)).abs() < 0.001);
        assert!((track_start.y - (center.y + 18.0 * 2.0_f32.sqrt() / 2.0)).abs() < 0.001);
        assert!((track_end.x - (center.x + 18.0 * 2.0_f32.sqrt() / 2.0)).abs() < 0.001);
        assert!((track_end.y - (center.y + 18.0 * 2.0_f32.sqrt() / 2.0)).abs() < 0.001);

        let value_end = polylines[1].points.last().expect("value arc has an end");
        assert!((value_end.x - center.x).abs() < 0.001);
        assert!((value_end.y - 3.0).abs() < 0.001);

        let tokens = crate::widgets::resolve_widget_visual_tokens(
            &ThemeTokens::default(),
            knob.common.style,
            knob.common.state,
        );
        assert_eq!(polylines[0].color, tokens.border);
        assert_eq!(polylines[1].color, tokens.emphasis);
    }

    #[test]
    fn selected_knob_has_distinct_structure_from_default_neutral_knob() {
        let bounds = Rect::from_min_size(Point::new(0.0, 0.0), Vector2::new(40.0, 40.0));
        let default_knob = KnobWidget::new(1, 0.5);
        let mut selected_knob = KnobWidget::new(1, 0.5);
        selected_knob.common.state.selected = true;
        let mut default_primitives = Vec::new();
        let mut selected_primitives = Vec::new();

        default_knob.append_paint(
            &mut default_primitives,
            bounds,
            &LayoutOutput::default(),
            &ThemeTokens::default(),
        );
        selected_knob.append_paint(
            &mut selected_primitives,
            bounds,
            &LayoutOutput::default(),
            &ThemeTokens::default(),
        );

        assert_eq!(default_primitives.len(), 2);
        assert_eq!(selected_primitives.len(), 3);
        assert_ne!(default_primitives, selected_primitives);
    }
}
