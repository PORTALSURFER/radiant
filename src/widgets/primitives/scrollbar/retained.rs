//! Runtime-owned scrollbar edit session; the public primitive stays source compatible.
use super::{ScrollbarAxis, ScrollbarWidget, input};
use crate::gui::types::{Point, Rect};
use crate::layout::LayoutOutput;
use crate::runtime::PaintPrimitive;
use crate::theme::ThemeTokens;
use crate::widgets::{
    EditEvent, InteractionProvenance, NumericAccessibilityAction, PointerButton,
    ScrollbarEditBatch, SemanticAction, SemanticActionSource, WheelPhase, WheelSample, Widget,
    WidgetActionCapabilities, WidgetCapabilities, WidgetCapabilitiesV2, WidgetCommon, WidgetInput,
    WidgetOutput, WidgetPointerMotion, WidgetPointerMotionRevision, WidgetSemanticActionResult,
    WidgetSemanticActions, WidgetSemantics,
};

#[derive(Clone, Copy, Debug, PartialEq)]
enum Owner {
    Pointer,
    Wheel,
}

#[derive(Clone, Debug, PartialEq)]
pub(crate) struct RetainedScrollbarWidget {
    scrollbar: ScrollbarWidget,
    active: Option<(Owner, EditEvent<f32>)>,
    // A replacement model retires the old edit without rolling back the new model.
    retired: Option<ScrollbarEditBatch>,
}

impl RetainedScrollbarWidget {
    pub(crate) fn new(scrollbar: ScrollbarWidget) -> Self {
        Self {
            scrollbar,
            active: None,
            retired: None,
        }
    }
    fn editable(&self) -> bool {
        let props = self.scrollbar.props;
        !self.scrollbar.common.state.disabled
            && !self.scrollbar.common.state.read_only
            && self.scrollbar.state.offset_fraction.is_finite()
            && (0.0..=1.0).contains(&self.scrollbar.state.offset_fraction)
            && props.viewport_fraction.is_finite()
            && (0.0..1.0).contains(&props.viewport_fraction)
            && props.step_fraction.is_finite()
            && props.step_fraction >= 0.0
    }
    fn cancel(&mut self) -> Option<ScrollbarEditBatch> {
        self.cancel_with_provenance(cancellation_provenance())
    }
    fn cancel_with_provenance(
        &mut self,
        provenance: InteractionProvenance,
    ) -> Option<ScrollbarEditBatch> {
        if let Some(retired) = self.retired.take() {
            return Some(retired);
        }
        let (_, event) = self.active.take()?;
        self.scrollbar.common.state.pressed = false;
        self.scrollbar.state.drag_grip_fraction = None;
        let changed = differs(self.scrollbar.state.offset_fraction, event.start_value);
        self.scrollbar.state.offset_fraction = event.start_value;
        ScrollbarEditBatch::new(
            &[event.cancel(provenance)?],
            changed.then_some(event.start_value),
        )
    }
    fn atomic(
        &mut self,
        candidate: f32,
        provenance: InteractionProvenance,
    ) -> Option<ScrollbarEditBatch> {
        if !self.editable() || self.active.is_some() || !candidate.is_finite() {
            return None;
        }
        let candidate = candidate.clamp(0.0, 1.0);
        let start = self.scrollbar.state.offset_fraction;
        if !differs(start, candidate) {
            return None;
        }
        let begin = EditEvent::begin(start, provenance);
        let update = begin.update(candidate, provenance)?;
        let commit = update.commit(candidate, provenance)?;
        self.scrollbar.state.offset_fraction = candidate;
        ScrollbarEditBatch::new(&[begin, update, commit], Some(candidate))
    }
    fn advance(
        &mut self,
        candidate: f32,
        provenance: InteractionProvenance,
        terminal: bool,
    ) -> Option<ScrollbarEditBatch> {
        let (owner, previous) = self.active?;
        if !candidate.is_finite() {
            return if terminal { self.cancel() } else { None };
        }
        let candidate = candidate.clamp(0.0, 1.0);
        let changed = differs(self.scrollbar.state.offset_fraction, candidate);
        let event = if changed {
            previous.update(candidate, provenance)?
        } else {
            previous
        };
        self.scrollbar.state.offset_fraction = candidate;
        if terminal {
            let commit = event.commit(candidate, provenance)?;
            self.active = None;
            self.scrollbar.common.state.pressed = false;
            self.scrollbar.state.drag_grip_fraction = None;
            if changed {
                ScrollbarEditBatch::new(&[event, commit], Some(candidate))
            } else {
                ScrollbarEditBatch::new(&[commit], None)
            }
        } else {
            self.active = Some((owner, event));
            changed
                .then(|| ScrollbarEditBatch::new(&[event], Some(candidate)))
                .flatten()
        }
    }
    fn pointer_candidate(&self, bounds: Rect, position: Point) -> Option<f32> {
        if !valid_geometry(bounds, position) {
            return None;
        }
        let axis = self.scrollbar.props.axis;
        let length = super::axis_length(axis, bounds);
        let thumb = length * self.scrollbar.thumb_fraction(length);
        let free = length - thumb;
        if free <= f32::EPSILON {
            return None;
        }
        let grip = self.scrollbar.state.drag_grip_fraction?;
        Some(
            (super::geometry::axis_position(axis, position)
                - super::axis_start(axis, bounds)
                - thumb * grip)
                / free,
        )
    }
    fn edit_input(&mut self, bounds: Rect, event: WidgetInput) -> Option<ScrollbarEditBatch> {
        if let WidgetInput::FocusChanged(focused) = &event {
            self.scrollbar.common.state.focused = *focused;
        }
        if let Some(retired) = self.retired.take() {
            return Some(retired);
        }
        if !self.editable() {
            return self.cancel();
        }
        match event {
            WidgetInput::FocusChanged(focused) => {
                self.scrollbar.common.state.focused = focused;
                if focused { None } else { self.cancel() }
            }
            WidgetInput::PointerPress {
                position,
                button: PointerButton::Primary,
                modifiers,
                timestamp,
            } if self.active.is_none()
                && valid_geometry(bounds, position)
                && bounds.contains(position) =>
            {
                let thumb = self.scrollbar.thumb_rect(bounds);
                let length = super::axis_length(self.scrollbar.props.axis, bounds);
                if self.scrollbar.thumb_fraction(length) >= 1.0 {
                    return None;
                }
                let provenance = InteractionProvenance::Pointer {
                    modifiers,
                    timestamp,
                    sequence_range: None,
                };
                let start = self.scrollbar.state.offset_fraction;
                let begin = EditEvent::begin(start, provenance);
                let on_thumb = thumb.contains(position);
                self.scrollbar.state.drag_grip_fraction = Some(if on_thumb {
                    input::pointer_grip_fraction(&self.scrollbar, thumb, position)
                } else {
                    0.5
                });
                self.scrollbar.common.state.pressed = true;
                self.scrollbar.common.state.focused = true;
                self.scrollbar.common.state.hovered = true;
                self.active = Some((Owner::Pointer, begin));
                let candidate = if on_thumb {
                    start
                } else {
                    self.pointer_candidate(bounds, position)?.clamp(0.0, 1.0)
                };
                if differs(start, candidate) {
                    let update = begin.update(candidate, provenance)?;
                    self.active = Some((Owner::Pointer, update));
                    self.scrollbar.state.offset_fraction = candidate;
                    ScrollbarEditBatch::new(&[begin, update], Some(candidate))
                } else {
                    ScrollbarEditBatch::new(&[begin], None)
                }
            }
            WidgetInput::PointerMove {
                position,
                modifiers,
                timestamp,
                sequence_range,
            } => {
                self.scrollbar.common.state.hovered = bounds.contains(position);
                if !matches!(self.active, Some((Owner::Pointer, _))) {
                    return None;
                }
                let candidate = self.pointer_candidate(bounds, position)?;
                self.advance(
                    candidate,
                    InteractionProvenance::Pointer {
                        modifiers,
                        timestamp,
                        sequence_range,
                    },
                    false,
                )
            }
            WidgetInput::PointerRelease {
                position,
                button: PointerButton::Primary,
                modifiers,
                timestamp,
            } => {
                if !matches!(self.active, Some((Owner::Pointer, _))) {
                    return None;
                }
                self.scrollbar.common.state.hovered = bounds.contains(position);
                let Some(candidate) = self.pointer_candidate(bounds, position) else {
                    return self.cancel();
                };
                self.advance(
                    candidate,
                    InteractionProvenance::Pointer {
                        modifiers,
                        timestamp,
                        sequence_range: None,
                    },
                    true,
                )
            }
            WidgetInput::KeyPress { key, timestamp, .. }
                if self.scrollbar.common.state.focused && self.active.is_none() =>
            {
                let previous = self.scrollbar.state.offset_fraction;
                input::handle_key_input(&mut self.scrollbar, key)?;
                let candidate = self.scrollbar.state.offset_fraction;
                self.scrollbar.state.offset_fraction = previous;
                self.atomic(candidate, InteractionProvenance::Keyboard { timestamp })
            }
            WidgetInput::Wheel {
                position,
                delta,
                modifiers,
                timestamp,
                sequence_range,
            } if bounds.contains(position) && self.scrollbar.common.state.focused => {
                let candidate = self.wheel_candidate(bounds, delta)?;
                self.atomic(
                    candidate,
                    InteractionProvenance::Pointer {
                        modifiers,
                        timestamp,
                        sequence_range,
                    },
                )
            }
            _ => None,
        }
    }
    fn wheel_candidate(&self, bounds: Rect, delta: crate::gui::types::Vector2) -> Option<f32> {
        if !valid_geometry(bounds, bounds.min) || !delta.x.is_finite() || !delta.y.is_finite() {
            return None;
        }
        let axis = self.scrollbar.props.axis;
        let length = super::axis_length(axis, bounds);
        // Logical scroll displacement is normalized by content overflow, not thumb travel.
        let viewport = self.scrollbar.props.viewport_fraction;
        if viewport <= 0.0 || viewport >= 1.0 {
            return None;
        }
        let displacement = match axis {
            ScrollbarAxis::Horizontal => delta.x,
            ScrollbarAxis::Vertical => delta.y,
        };
        let candidate = self.scrollbar.state.offset_fraction
            + displacement / length * viewport / (1.0 - viewport);
        candidate.is_finite().then_some(candidate)
    }
    fn wheel(
        &mut self,
        bounds: Rect,
        position: Point,
        sample: WheelSample,
    ) -> Option<ScrollbarEditBatch> {
        if let Some(retired) = self.retired.take() {
            return Some(retired);
        }
        if !self.editable() {
            return self.cancel();
        }
        if !self.scrollbar.common.state.focused || !sample.is_valid() {
            return None;
        }
        let provenance = InteractionProvenance::Pointer {
            modifiers: sample.modifiers(),
            timestamp: sample.timestamp(),
            sequence_range: sample.sequence_range(),
        };
        match sample.phase() {
            Some(WheelPhase::Cancelled) => {
                if matches!(self.active, Some((Owner::Wheel, _))) {
                    self.cancel_with_provenance(provenance)
                } else {
                    None
                }
            }
            Some(WheelPhase::Started) if self.active.is_none() && bounds.contains(position) => {
                let candidate = self
                    .wheel_candidate(bounds, sample.delta().to_logical_pixels()?)?
                    .clamp(0.0, 1.0);
                let start = self.scrollbar.state.offset_fraction;
                let begin = EditEvent::begin(start, provenance);
                self.active = Some((Owner::Wheel, begin));
                if differs(start, candidate) {
                    let update = begin.update(candidate, provenance)?;
                    self.active = Some((Owner::Wheel, update));
                    self.scrollbar.state.offset_fraction = candidate;
                    ScrollbarEditBatch::new(&[begin, update], Some(candidate))
                } else {
                    ScrollbarEditBatch::new(&[begin], None)
                }
            }
            Some(WheelPhase::Changed | WheelPhase::Ended)
                if matches!(self.active, Some((Owner::Wheel, _))) =>
            {
                let candidate =
                    self.wheel_candidate(bounds, sample.delta().to_logical_pixels()?)?;
                self.advance(
                    candidate,
                    provenance,
                    sample.phase() == Some(WheelPhase::Ended),
                )
            }
            None | Some(WheelPhase::Discrete) if bounds.contains(position) => {
                let candidate =
                    self.wheel_candidate(bounds, sample.delta().to_logical_pixels()?)?;
                self.atomic(candidate, provenance)
            }
            _ => None,
        }
    }
}

fn cancellation_provenance() -> InteractionProvenance {
    InteractionProvenance::Pointer {
        modifiers: Default::default(),
        timestamp: None,
        sequence_range: None,
    }
}

fn differs(a: f32, b: f32) -> bool {
    (a - b).abs() > f32::EPSILON
}
fn valid_geometry(bounds: Rect, position: Point) -> bool {
    [
        bounds.min.x,
        bounds.min.y,
        bounds.max.x,
        bounds.max.y,
        position.x,
        position.y,
    ]
    .into_iter()
    .all(f32::is_finite)
        && bounds.width().is_finite()
        && bounds.height().is_finite()
        && bounds.width() > 0.0
        && bounds.height() > 0.0
}

impl WidgetPointerMotion for RetainedScrollbarWidget {
    fn revision(&self) -> WidgetPointerMotionRevision {
        WidgetPointerMotionRevision::exact(false)
    }
    fn accepts_pointer_move(&self) -> bool {
        false
    }
}
impl WidgetSemantics for RetainedScrollbarWidget {
    fn automation_available_actions(&self) -> Option<Vec<String>> {
        use crate::gui::automation::{
            AUTOMATION_ACTION_DECREMENT, AUTOMATION_ACTION_FOCUS, AUTOMATION_ACTION_INCREMENT,
            AUTOMATION_ACTION_SET_TEXT,
        };
        let mut actions = Vec::new();
        if !self.scrollbar.common.state.disabled {
            actions.push(AUTOMATION_ACTION_FOCUS.to_owned());
        }
        if self.editable() {
            actions.extend(
                [
                    AUTOMATION_ACTION_INCREMENT,
                    AUTOMATION_ACTION_DECREMENT,
                    AUTOMATION_ACTION_SET_TEXT,
                ]
                .map(str::to_owned),
            );
        }
        Some(actions)
    }
    fn automation_role(&self) -> crate::gui::automation::AutomationRole {
        crate::gui::automation::AutomationRole::Slider
    }
    fn automation_value_text(&self) -> Option<String> {
        Some(format!("{:.3}", self.scrollbar.state.offset_fraction))
    }
}
impl WidgetSemanticActions for RetainedScrollbarWidget {
    fn supports(&self, action: &SemanticAction) -> bool {
        matches!(action, SemanticAction::Numeric(_))
    }
    fn dispatch(
        &mut self,
        action: SemanticAction,
        source: SemanticActionSource,
    ) -> WidgetSemanticActionResult {
        if !self.supports(&action)
            || !self.editable()
            || self.active.is_some()
            || self.retired.is_some()
        {
            return WidgetSemanticActionResult::Unsupported;
        }
        let value = self.scrollbar.state.offset_fraction;
        let step = self.scrollbar.props.step_fraction;
        let candidate = match action {
            SemanticAction::Numeric(NumericAccessibilityAction::Increment) => value + step,
            SemanticAction::Numeric(NumericAccessibilityAction::Decrement) => value - step,
            SemanticAction::Numeric(NumericAccessibilityAction::SetValueText(text)) => {
                // Bound externally supplied numeric text before parsing.
                if text.len() > 128 {
                    return WidgetSemanticActionResult::Unsupported;
                }
                let Ok(value) = text.parse::<f32>() else {
                    return WidgetSemanticActionResult::Unsupported;
                };
                value
            }
            _ => return WidgetSemanticActionResult::Unsupported,
        };
        if !candidate.is_finite() {
            return WidgetSemanticActionResult::Unsupported;
        }
        WidgetSemanticActionResult::Accepted(
            self.atomic(candidate, source.provenance())
                .map(WidgetOutput::typed),
        )
    }
}
impl Widget for RetainedScrollbarWidget {
    fn common(&self) -> &WidgetCommon {
        &self.scrollbar.common
    }
    fn common_mut(&mut self) -> &mut WidgetCommon {
        &mut self.scrollbar.common
    }
    fn handle_input(&mut self, bounds: Rect, input: WidgetInput) -> Option<WidgetOutput> {
        self.edit_input(bounds, input).map(WidgetOutput::typed)
    }
    fn handle_pointer_capture_cancelled(&mut self, _bounds: Rect) -> Option<WidgetOutput> {
        self.cancel().map(WidgetOutput::typed)
    }
    fn accepts_wheel_input(&self) -> bool {
        self.editable() && self.scrollbar.props.viewport_fraction > 0.0
    }
    fn handle_wheel_sample(
        &mut self,
        bounds: Rect,
        position: Point,
        sample: WheelSample,
    ) -> Option<WidgetOutput> {
        self.wheel(bounds, position, sample)
            .map(WidgetOutput::typed)
    }
    fn retains_managed_wheel_sequence(&self) -> bool {
        matches!(self.active, Some((Owner::Wheel, _)))
    }
    fn capabilities(&self) -> WidgetCapabilities<'_> {
        WidgetCapabilities::new().semantics(self)
    }
    fn capabilities_v2(&self) -> WidgetCapabilitiesV2<'_> {
        WidgetCapabilitiesV2::new()
            .with_pointer_motion(self)
            .with_semantic_actions(self)
    }
    fn action_capabilities(&mut self) -> WidgetActionCapabilities<'_> {
        WidgetActionCapabilities::none().with_semantic_actions(self)
    }
    fn synchronize_from_previous(&mut self, previous: &dyn Widget) {
        let Some(previous) = previous.as_any().downcast_ref::<Self>() else {
            return;
        };
        self.scrollbar.common.state.hovered = previous.scrollbar.common.state.hovered;
        self.scrollbar.common.state.focused = previous.scrollbar.common.state.focused;
        self.retired = previous.retired;
        if self.editable()
            && self.scrollbar.props == previous.scrollbar.props
            && self.scrollbar.state.offset_fraction == previous.scrollbar.state.offset_fraction
        {
            self.active = previous.active;
            self.scrollbar.common.state.pressed = previous.scrollbar.common.state.pressed;
            self.scrollbar.state.drag_grip_fraction = previous.scrollbar.state.drag_grip_fraction;
        } else if let Some((_, event)) = previous.active {
            // Synchronization cannot dispatch. The next input/capture-loss boundary
            // publishes only cancellation, with no rollback into the replacement model.
            self.retired = event
                .cancel(cancellation_provenance())
                .and_then(|cancel| ScrollbarEditBatch::new(&[cancel], None));
            self.active = None;
            self.scrollbar.common.state.pressed = false;
            self.scrollbar.state.drag_grip_fraction = None;
        }
    }
    fn append_paint(
        &self,
        primitives: &mut Vec<PaintPrimitive>,
        bounds: Rect,
        layout: &LayoutOutput,
        theme: &ThemeTokens,
    ) {
        self.scrollbar
            .append_paint(primitives, bounds, layout, theme);
    }
}

#[cfg(test)]
mod tests;
