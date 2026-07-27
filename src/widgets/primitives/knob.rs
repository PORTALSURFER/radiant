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
    KnobKeyboardGesture, KnobMessage, PointerButton, WidgetInput, WidgetKey, WidgetOutput,
};

use super::support::{WidgetCommon, clamp_fraction, push_automation_active_marker};

const DEFAULT_DIAMETER: f32 = 40.0;
const DEFAULT_SENSITIVITY: f32 = 0.006;
const ARC_START: f32 = -5.0 * std::f32::consts::PI / 4.0;
const ARC_SWEEP: f32 = 3.0 * std::f32::consts::PI / 2.0;

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
        if self.common.state.disabled {
            return None;
        }
        match input {
            WidgetInput::PointerPress {
                position,
                button: PointerButton::Primary,
                ..
            } if bounds.contains(position) => {
                self.common.state.hovered = true;
                self.common.state.pressed = true;
                self.common.state.focused = true;
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
                self.set_value(self.state.value + (origin.y - position.y) * self.props.sensitivity)
                    .map(|value| KnobMessage::ValueChanged { value })
            }
            WidgetInput::PointerRelease {
                position: _,
                button: PointerButton::Primary,
                ..
            } => {
                if !self.common.state.pressed {
                    return None;
                }
                self.common.state.pressed = false;
                self.state.gesture_origin = None;
                self.set_value(self.state.value)
                    .or(Some(self.state.value))
                    .map(|value| KnobMessage::GestureEnded { value })
            }
            WidgetInput::PointerDoubleClick {
                position,
                button: PointerButton::Primary,
                ..
            } if self.props.reset_on_double_click && bounds.contains(position) => {
                self.common.state.pressed = false;
                self.state.gesture_origin = None;
                self.state.value = self.props.default_value;
                Some(KnobMessage::Reset {
                    value: self.state.value,
                })
            }
            WidgetInput::FocusChanged(focused) => {
                self.common.state.focused = focused;
                if !focused {
                    let had_active_gesture = self.state.gesture_origin.is_some();
                    self.common.state.pressed = false;
                    self.state.gesture_origin = None;
                    if had_active_gesture {
                        return Some(KnobMessage::GestureEnded {
                            value: self.state.value,
                        });
                    }
                }
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
        self.state.gesture_origin = previous.state.gesture_origin;
        if self.common.state.disabled {
            self.common.state.pressed = false;
            self.state.gesture_origin = None;
        }
    }

    fn accepts_pointer_move(&self) -> bool {
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
        let ring = circle_points(center, radius.max(1.0), 40);
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
        let value_angle = ARC_START + ARC_SWEEP * self.state.value.clamp(0.0, 1.0);
        let indicator_end = Point::new(
            center.x + (radius - 4.0) * value_angle.cos(),
            center.y + (radius - 4.0) * value_angle.sin(),
        );
        primitives.push(PaintPrimitive::StrokePolyline(PaintStrokePolyline {
            widget_id: self.common.id,
            points: [center, indicator_end].into(),
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        gui::types::Point,
        runtime::PaintPrimitive,
        widgets::{WidgetState, WidgetVisualCue},
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
