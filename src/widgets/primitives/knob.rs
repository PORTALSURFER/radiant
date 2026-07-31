//! Reusable radial knob primitive with explicit host-automation gestures.

use std::f32::consts::TAU;

use crate::gui::types::{Point, Rect, Vector2};
use crate::layout::LayoutOutput;
use crate::runtime::{PaintPrimitive, PaintStrokePolyline};
use crate::theme::ThemeTokens;
use crate::widgets::contract::{
    FocusBehavior, PaintBounds, Widget, WidgetCapabilities, WidgetId, WidgetSemantics, WidgetSizing,
};
use crate::widgets::interaction::{
    KnobKeyboardGesture, KnobMessage, KnobWheelGesture, PointerButton, WidgetInput, WidgetKey,
    WidgetOutput,
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
                ..
            }
            | WidgetInput::PointerDrop {
                button: PointerButton::Primary,
                ..
            } => {
                return self.finish_terminal_gesture(false);
            }
            WidgetInput::FocusChanged(false) => {
                return self.finish_terminal_gesture(true);
            }
            _ => {}
        }
        if self.common.state.disabled {
            return None;
        }
        match input {
            WidgetInput::PointerModifiersChanged { modifiers } => {
                if self.common.state.pressed {
                    self.state.fine_adjustment = modifiers.shift;
                }
                None
            }
            WidgetInput::PointerPress {
                position,
                button: PointerButton::Primary,
                modifiers,
            } if bounds.contains(position) => {
                self.common.state.hovered = true;
                self.common.state.pressed = true;
                self.common.state.focused = true;
                self.state.fine_adjustment = modifiers.shift;
                self.state.gesture_origin = Some(position);
                Some(KnobMessage::GestureStarted {
                    value: self.state.value,
                })
            }
            WidgetInput::PointerMove { position } => {
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
                    .map(|value| KnobMessage::ValueChanged { value })
            }
            WidgetInput::Wheel {
                position,
                delta,
                modifiers,
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
                Some(KnobMessage::WheelGesture(KnobWheelGesture::new(
                    start_value,
                    final_value,
                )))
            }
            WidgetInput::PointerDoubleClick {
                position,
                button: PointerButton::Primary,
                ..
            } if self.props.reset_on_double_click && bounds.contains(position) => {
                self.common.state.pressed = false;
                self.state.fine_adjustment = false;
                self.state.gesture_origin = None;
                self.state.value = self.props.default_value;
                Some(KnobMessage::Reset {
                    value: self.state.value,
                })
            }
            WidgetInput::FocusChanged(focused) => {
                self.common.state.focused = focused;
                None
            }
            WidgetInput::KeyPress(key) if self.common.state.focused => match key {
                WidgetKey::ArrowLeft | WidgetKey::ArrowDown => {
                    self.keyboard_gesture(self.state.value - self.props.sensitivity * 16.0)
                }
                WidgetKey::ArrowRight | WidgetKey::ArrowUp => {
                    self.keyboard_gesture(self.state.value + self.props.sensitivity * 16.0)
                }
                WidgetKey::Home => self.keyboard_gesture(0.0),
                WidgetKey::End => self.keyboard_gesture(1.0),
                _ => None,
            },
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

    fn keyboard_gesture(&mut self, value: f32) -> Option<KnobMessage> {
        let start_value = self.state.value;
        let final_value = self.set_value(value)?;
        Some(KnobMessage::KeyboardGesture(KnobKeyboardGesture::new(
            start_value,
            final_value,
        )))
    }

    fn finish_terminal_gesture(&mut self, focus_lost: bool) -> Option<KnobMessage> {
        let had_active_gesture = self.state.gesture_origin.take().is_some();
        self.common.state.pressed = false;
        self.state.fine_adjustment = false;
        if focus_lost {
            self.common.state.focused = false;
        }
        had_active_gesture.then_some(KnobMessage::GestureEnded {
            value: self.state.value,
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
mod tests {
    use super::*;
    use crate::{
        gui::types::Point,
        runtime::PaintPrimitive,
        widgets::interaction::PointerModifiers,
        widgets::{KnobAutomationEvent, WidgetState, WidgetVisualCue},
    };

    #[test]
    fn knob_emits_gesture_start_value_end_and_reset() {
        let bounds = Rect::from_min_size(Point::new(0.0, 0.0), Vector2::new(40.0, 40.0));
        let mut knob = KnobWidget::new(1, 0.5).with_default_value(0.25);
        assert_eq!(
            knob.handle_input(bounds, WidgetInput::primary_press(Point::new(20.0, 20.0))),
            Some(KnobMessage::GestureStarted { value: 0.5 })
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
            Some(KnobMessage::Reset { value: 0.25 })
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
            Some(KnobMessage::GestureStarted { value: 0.5 })
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
            Some(KnobMessage::GestureEnded { value }) if (value - 0.71).abs() < 0.0001
        ));
        assert!(knob.common.state.active);
    }

    #[test]
    fn knob_keyboard_gesture_batch_requires_focus_and_preserves_clamped_order() {
        let bounds = Rect::from_min_size(Point::new(0.0, 0.0), Vector2::new(40.0, 40.0));
        let mut knob = KnobWidget::new(1, 0.98).with_sensitivity(0.1);
        assert_eq!(
            knob.handle_input(bounds, WidgetInput::KeyPress(WidgetKey::ArrowRight)),
            None
        );
        knob.handle_input(bounds, WidgetInput::FocusChanged(true));
        let Some(KnobMessage::KeyboardGesture(batch)) =
            knob.handle_input(bounds, WidgetInput::KeyPress(WidgetKey::ArrowRight))
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

        knob.common.state.disabled = true;
        assert_eq!(
            knob.handle_input(bounds, WidgetInput::KeyPress(WidgetKey::ArrowLeft)),
            None
        );
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
    fn knob_double_click_reset_ignores_outside_and_disabled_inputs() {
        let bounds = Rect::from_min_size(Point::new(0.0, 0.0), Vector2::new(40.0, 40.0));
        let mut knob = KnobWidget::new(1, 0.8).with_default_value(0.2);
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
            Some(KnobMessage::GestureStarted { value: 0.5 })
        ));
        assert_eq!(
            knob.handle_input(bounds, WidgetInput::FocusChanged(false)),
            Some(KnobMessage::GestureEnded { value: 0.5 })
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
            Some(KnobMessage::GestureEnded { value: 0.5 })
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
