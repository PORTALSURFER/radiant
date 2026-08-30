use radiant::{
    gui::automation::{AutomationNodeSemantics, AutomationRole},
    gui::types::Rgba8,
    layout::Rect,
    runtime::{PaintPrimitive, SurfacePaintPlan},
    theme::ThemeTokens,
    widgets::{
        PointerButton, PointerCapturePolicy, Widget, WidgetCapabilities, WidgetCapabilitiesV2,
        WidgetCommon, WidgetCursor, WidgetHitTest, WidgetHitTestResult, WidgetHitTestRevision,
        WidgetInput, WidgetKey, WidgetOutput, WidgetPointerMotion, WidgetPointerMotionRevision,
        WidgetSemantics, WidgetSemanticsRevision, WidgetSizing,
    },
};
use std::{cell::Cell, rc::Rc};

#[derive(Clone, Debug, PartialEq)]
pub(super) enum DemoMessage {
    Rename(String),
    SetActive(bool),
}

#[derive(Default)]
pub(super) struct DemoState {
    pub(super) name: String,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum CustomWidgetMessage {
    Activated,
}

#[derive(Clone)]
pub(super) struct CustomStatusWidget {
    pub(super) common: WidgetCommon,
    label: &'static str,
    pub(super) activation_count: usize,
}

#[derive(Clone)]
pub(super) struct LegacyHooksWidget {
    pub(super) common: WidgetCommon,
    unsupported_v2: bool,
}

impl LegacyHooksWidget {
    pub(super) fn new(id: u64) -> Self {
        Self {
            common: WidgetCommon::new(
                id,
                WidgetSizing::fixed(radiant::layout::Vector2::new(80.0, 24.0)),
            )
            .with_pointer_focus(),
            unsupported_v2: false,
        }
    }

    pub(super) fn with_unsupported_v2(id: u64) -> Self {
        Self {
            unsupported_v2: true,
            ..Self::new(id)
        }
    }
}

#[derive(Clone)]
pub(super) struct DescriptorPrecedenceWidget {
    pub(super) common: WidgetCommon,
    pub(super) moves: Rc<Cell<u32>>,
}

impl DescriptorPrecedenceWidget {
    pub(super) fn with_moves(id: u64, moves: Rc<Cell<u32>>) -> Self {
        Self {
            common: WidgetCommon::new(
                id,
                WidgetSizing::fixed(radiant::layout::Vector2::new(80.0, 24.0)),
            )
            .with_pointer_focus(),
            moves,
        }
    }
}

#[derive(Clone)]
pub(super) struct DirectAutomationWidget {
    pub(super) common: WidgetCommon,
}

impl DirectAutomationWidget {
    pub(super) fn new(id: u64) -> Self {
        Self {
            common: WidgetCommon::new(
                id,
                WidgetSizing::fixed(radiant::layout::Vector2::new(80.0, 24.0)),
            )
            .with_keyboard_focus(),
        }
    }
}

impl CustomStatusWidget {
    pub(super) fn new(id: u64) -> Self {
        let mut common = WidgetCommon::new(
            id,
            WidgetSizing::fixed(radiant::layout::Vector2::new(120.0, 28.0)),
        );
        common.focus = radiant::widgets::FocusBehavior::Keyboard;
        Self {
            common,
            label: "custom",
            activation_count: 0,
        }
    }
}

impl WidgetSemantics for CustomStatusWidget {
    fn revision(&self) -> WidgetSemanticsRevision {
        WidgetSemanticsRevision::exact(self.label)
    }

    fn automation_role(&self) -> radiant::gui::automation::AutomationRole {
        radiant::gui::automation::AutomationRole::Button
    }

    fn automation_label(&self) -> Option<String> {
        Some(self.label.to_owned())
    }

    fn automation_available_actions(&self) -> Option<Vec<String>> {
        Some(vec![
            radiant::gui::automation::AUTOMATION_ACTION_PRESS.to_owned(),
        ])
    }
}

impl WidgetPointerMotion for CustomStatusWidget {
    fn revision(&self) -> WidgetPointerMotionRevision {
        WidgetPointerMotionRevision::exact(true)
    }
}

impl WidgetHitTest for CustomStatusWidget {
    fn revision(&self) -> WidgetHitTestRevision {
        WidgetHitTestRevision::exact(())
    }
}

impl Widget for CustomStatusWidget {
    fn common(&self) -> &WidgetCommon {
        &self.common
    }

    fn common_mut(&mut self) -> &mut WidgetCommon {
        &mut self.common
    }

    fn capabilities(&self) -> WidgetCapabilities<'_> {
        WidgetCapabilities::new().semantics(self)
    }

    fn capabilities_v2(&self) -> WidgetCapabilitiesV2<'_> {
        WidgetCapabilitiesV2::new()
            .with_hit_test(self)
            .with_pointer_motion(self)
    }

    fn handle_input(&mut self, bounds: Rect, input: WidgetInput) -> Option<WidgetOutput> {
        match input {
            WidgetInput::PointerMove { position, .. } => {
                self.common.state.hovered = bounds.contains(position);
                None
            }
            WidgetInput::PointerRelease {
                position,
                button: PointerButton::Primary,
                ..
            } if bounds.contains(position) => {
                self.activation_count += 1;
                Some(WidgetOutput::custom(CustomWidgetMessage::Activated))
            }
            WidgetInput::KeyPress {
                key: WidgetKey::Enter,
                ..
            } if self.common.state.focused => {
                self.activation_count += 1;
                Some(WidgetOutput::custom(CustomWidgetMessage::Activated))
            }
            WidgetInput::FocusChanged(focused) => {
                self.common.state.focused = focused;
                None
            }
            _ => None,
        }
    }

    fn synchronize_from_previous(&mut self, previous: &dyn Widget) {
        if let Some(previous) = previous.as_any().downcast_ref::<CustomStatusWidget>() {
            self.activation_count = previous.activation_count;
        }
    }

    fn append_paint(
        &self,
        primitives: &mut Vec<PaintPrimitive>,
        bounds: Rect,
        _layout: &radiant::layout::LayoutOutput,
        theme: &ThemeTokens,
    ) {
        primitives.push(PaintPrimitive::FillRect(radiant::runtime::PaintFillRect {
            widget_id: self.common.id,
            rect: bounds,
            color: if self.common.state.hovered {
                theme.accent_danger
            } else {
                theme.surface_base
            },
        }));
        primitives.push(PaintPrimitive::Text(radiant::runtime::PaintTextRun {
            widget_id: self.common.id,
            text: self.label.into(),
            rect: bounds,
            font_size: 13.0,
            baseline: Some(18.0),
            color: theme.text_primary,
            align: radiant::runtime::PaintTextAlign::Center,
            wrap: radiant::widgets::TextWrap::None,
        }));
    }
}

impl Widget for LegacyHooksWidget {
    fn accepts_pointer_move(&self) -> bool {
        false
    }

    fn accepts_pointer_input(&self, _input: &WidgetInput) -> bool {
        true
    }

    fn allows_captured_pointer_pass_through(&self) -> bool {
        false
    }

    fn cursor_for_point(
        &self,
        _bounds: Rect,
        _point: radiant::layout::Point,
    ) -> Option<WidgetCursor> {
        Some(WidgetCursor::ResizeLeft)
    }

    fn prefers_pointer_move_paint_only(&self) -> bool {
        true
    }

    fn capabilities_v2(&self) -> WidgetCapabilitiesV2<'_> {
        if self.unsupported_v2 {
            WidgetCapabilitiesV2::new().with_contract_version(99)
        } else {
            WidgetCapabilitiesV2::none()
        }
    }

    fn common(&self) -> &WidgetCommon {
        &self.common
    }

    fn common_mut(&mut self) -> &mut WidgetCommon {
        &mut self.common
    }

    fn handle_input(&mut self, _bounds: Rect, _input: WidgetInput) -> Option<WidgetOutput> {
        None
    }

    fn append_paint(
        &self,
        _primitives: &mut Vec<PaintPrimitive>,
        _bounds: Rect,
        _layout: &radiant::layout::LayoutOutput,
        _theme: &ThemeTokens,
    ) {
    }
}

impl WidgetHitTest for DescriptorPrecedenceWidget {
    fn revision(&self) -> WidgetHitTestRevision {
        WidgetHitTestRevision::exact(())
    }

    fn hit_test(
        &self,
        _bounds: Rect,
        _point: radiant::layout::Point,
        _input: &WidgetInput,
    ) -> WidgetHitTestResult {
        WidgetHitTestResult::Opaque
    }

    fn cursor_for_point(
        &self,
        _bounds: Rect,
        _point: radiant::layout::Point,
    ) -> Option<WidgetCursor> {
        Some(WidgetCursor::ResizeRight)
    }
}

impl WidgetPointerMotion for DescriptorPrecedenceWidget {
    fn revision(&self) -> WidgetPointerMotionRevision {
        WidgetPointerMotionRevision::exact(())
    }

    fn accepts_pointer_move(&self) -> bool {
        false
    }

    fn pointer_capture_policy(&self) -> PointerCapturePolicy {
        PointerCapturePolicy::Exclusive
    }

    fn prefers_pointer_move_paint_only(&self) -> bool {
        true
    }

    fn pointer_move_overlay_is_valid(&self) -> bool {
        true
    }
}

impl Widget for DescriptorPrecedenceWidget {
    fn accepts_pointer_move(&self) -> bool {
        true
    }

    fn accepts_pointer_input(&self, _input: &WidgetInput) -> bool {
        true
    }

    fn allows_captured_pointer_pass_through(&self) -> bool {
        true
    }

    fn cursor_for_point(
        &self,
        _bounds: Rect,
        _point: radiant::layout::Point,
    ) -> Option<WidgetCursor> {
        Some(WidgetCursor::ResizeLeft)
    }

    fn prefers_pointer_move_paint_only(&self) -> bool {
        false
    }

    fn common(&self) -> &WidgetCommon {
        &self.common
    }

    fn common_mut(&mut self) -> &mut WidgetCommon {
        &mut self.common
    }

    fn capabilities(&self) -> WidgetCapabilities<'_> {
        WidgetCapabilities::none()
    }

    fn capabilities_v2(&self) -> WidgetCapabilitiesV2<'_> {
        WidgetCapabilitiesV2::new()
            .with_hit_test(self)
            .with_pointer_motion(self)
    }

    fn handle_input(&mut self, _bounds: Rect, _input: WidgetInput) -> Option<WidgetOutput> {
        if matches!(_input, WidgetInput::PointerMove { .. }) {
            self.moves.set(self.moves.get().saturating_add(1));
        }
        None
    }

    fn append_paint(
        &self,
        _primitives: &mut Vec<PaintPrimitive>,
        _bounds: Rect,
        _layout: &radiant::layout::LayoutOutput,
        _theme: &ThemeTokens,
    ) {
    }
}

impl Widget for DirectAutomationWidget {
    fn automation_semantics(&self) -> AutomationNodeSemantics {
        AutomationNodeSemantics::new(AutomationRole::Readout).with_label("direct override")
    }

    fn automation_available_actions(&self) -> Option<Vec<String>> {
        Some(vec![String::from("direct-action")])
    }

    fn common(&self) -> &WidgetCommon {
        &self.common
    }

    fn common_mut(&mut self) -> &mut WidgetCommon {
        &mut self.common
    }

    fn handle_input(&mut self, _bounds: Rect, _input: WidgetInput) -> Option<WidgetOutput> {
        None
    }

    fn append_paint(
        &self,
        _primitives: &mut Vec<PaintPrimitive>,
        _bounds: Rect,
        _layout: &radiant::layout::LayoutOutput,
        _theme: &ThemeTokens,
    ) {
    }
}

pub(super) fn widget_fill_color(plan: &SurfacePaintPlan, widget_id: u64) -> Option<Rgba8> {
    plan.primitives
        .iter()
        .find_map(|primitive| match primitive {
            PaintPrimitive::FillRect(fill) if fill.widget_id == widget_id => Some(fill.color),
            _ => None,
        })
}
