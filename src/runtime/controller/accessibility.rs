//! Runtime-owned admission and dispatch for neutral numeric accessibility actions.

use super::{SurfaceRuntime, focus::FocusTransition};
use crate::{
    gui::automation::{AutomationNodeId, AutomationTarget},
    runtime::{
        NumericAccessibilityDispatchResult, NumericAccessibilityRequest,
        NumericAccessibilityUnavailableReason, RuntimeBridge, WidgetDispatchResult,
    },
    widgets::{
        NumericAccessibilityAction, NumericAccessibilityBlockOwner,
        NumericAccessibilityRejectedReason, WidgetId,
    },
};

impl<Bridge, Message> SurfaceRuntime<Bridge, Message>
where
    Bridge: RuntimeBridge<Message>,
{
    /// Execute one discrete neutral numeric accessibility request after
    /// current identity, focus, capability, and ownership checks.
    pub fn dispatch_numeric_accessibility_action(
        &mut self,
        request: NumericAccessibilityRequest,
    ) -> NumericAccessibilityDispatchResult {
        if !self.lifecycle_accepts_accessibility_work() {
            return NumericAccessibilityDispatchResult::Unavailable {
                reason: NumericAccessibilityUnavailableReason::StaleTarget,
            };
        }

        let widget_id = match self.resolve_accessibility_target(&request.target) {
            Ok(widget_id) => widget_id,
            Err(reason) => {
                return NumericAccessibilityDispatchResult::Unavailable { reason };
            }
        };

        if let Some(owner) = self.accessibility_incumbent_owner(widget_id) {
            return NumericAccessibilityDispatchResult::Blocked { owner };
        }

        if let Some(reason) =
            self.accessibility_capability_rejection(widget_id, &request.action, false)
        {
            return NumericAccessibilityDispatchResult::Rejected { reason };
        }

        if !self.is_live_focus_target(widget_id) {
            return NumericAccessibilityDispatchResult::Rejected {
                reason: NumericAccessibilityRejectedReason::NotFocusable,
            };
        }

        match self.request_focus(widget_id) {
            FocusTransition::Changed | FocusTransition::Unchanged => {}
            FocusTransition::Vetoed => {
                return NumericAccessibilityDispatchResult::Rejected {
                    reason: NumericAccessibilityRejectedReason::FocusDenied,
                };
            }
            FocusTransition::InvalidTarget => {
                return NumericAccessibilityDispatchResult::Unavailable {
                    reason: self
                        .resolve_accessibility_target(&request.target)
                        .err()
                        .unwrap_or(NumericAccessibilityUnavailableReason::StaleTarget),
                };
            }
        }

        let Ok(current_widget_id) = self.resolve_accessibility_target(&request.target) else {
            return NumericAccessibilityDispatchResult::Unavailable {
                reason: NumericAccessibilityUnavailableReason::StaleTarget,
            };
        };
        if current_widget_id != widget_id || !self.is_authoritative_focus_target(widget_id) {
            return NumericAccessibilityDispatchResult::Rejected {
                reason: NumericAccessibilityRejectedReason::FocusDenied,
            };
        }

        if let Some(owner) = self.accessibility_incumbent_owner(widget_id) {
            return NumericAccessibilityDispatchResult::Blocked { owner };
        }

        if let Some(reason) =
            self.accessibility_capability_rejection(widget_id, &request.action, true)
        {
            return NumericAccessibilityDispatchResult::Rejected { reason };
        }

        let Some((output, dispatch)) = self
            .surface_widget_mut(widget_id)
            .and_then(|widget| widget.dispatch_accessibility_action(widget_id, request.action))
        else {
            return NumericAccessibilityDispatchResult::Rejected {
                reason: NumericAccessibilityRejectedReason::UnsupportedAction,
            };
        };

        match dispatch {
            WidgetDispatchResult::Message(message) => {
                let outcome = self.dispatch_message(message);
                self.pending_input_command_outcome.merge(outcome);
            }
            WidgetDispatchResult::UnmappedOutput => self.relayout(),
            WidgetDispatchResult::NoOutput => {}
        }

        NumericAccessibilityDispatchResult::Accepted { widget_id, output }
    }

    fn accessibility_incumbent_owner(
        &self,
        target_widget_id: WidgetId,
    ) -> Option<NumericAccessibilityBlockOwner> {
        let focused_widget_id = self.focused_widget();
        let incumbent_widget_id = focused_widget_id.filter(|id| *id != target_widget_id);
        incumbent_widget_id
            .and_then(|widget_id| self.surface_widget(widget_id))
            .and_then(|widget| widget.accessibility_action_owner())
            .or_else(|| {
                self.surface_widget(target_widget_id)
                    .and_then(|widget| widget.accessibility_action_owner())
            })
    }

    fn lifecycle_accepts_accessibility_work(&self) -> bool {
        self.lifecycle.accepts_work()
    }

    fn resolve_accessibility_target(
        &self,
        target: &AutomationTarget,
    ) -> Result<WidgetId, NumericAccessibilityUnavailableReason> {
        let Some(authority) = target.authority else {
            return if self.current_automation_target(&target.id).is_some() {
                Err(NumericAccessibilityUnavailableReason::StaleTarget)
            } else {
                Err(NumericAccessibilityUnavailableReason::UnknownTarget)
            };
        };
        if !authority.materialized {
            return Err(NumericAccessibilityUnavailableReason::UnmaterializedTarget);
        }

        let widget_id = target
            .id
            .0
            .parse::<WidgetId>()
            .map_err(|_| NumericAccessibilityUnavailableReason::UnknownTarget)?;
        let Some(current) = self.current_automation_target(&target.id) else {
            return Err(NumericAccessibilityUnavailableReason::RemovedTarget);
        };
        if current.authority != Some(authority)
            || current.path != target.path
            || current.role != target.role
        {
            return Err(NumericAccessibilityUnavailableReason::StaleTarget);
        }
        Ok(widget_id)
    }

    fn current_automation_target(&self, id: &AutomationNodeId) -> Option<AutomationTarget> {
        self.automation_target_snapshot()
            .targets
            .into_iter()
            .find(|target| target.id == *id)
    }

    fn accessibility_capability_rejection(
        &self,
        widget_id: WidgetId,
        action: &NumericAccessibilityAction,
        require_focused: bool,
    ) -> Option<NumericAccessibilityRejectedReason> {
        let Some(widget) = self.surface_widget(widget_id) else {
            return Some(NumericAccessibilityRejectedReason::UnsupportedAction);
        };
        let common = widget.widget_object().common();
        if common.state.disabled {
            return Some(NumericAccessibilityRejectedReason::Disabled);
        }
        if common.state.read_only {
            return Some(NumericAccessibilityRejectedReason::ReadOnly);
        }
        if !widget.supports_accessibility_action(action) {
            return Some(NumericAccessibilityRejectedReason::UnsupportedAction);
        }
        if require_focused && !common.state.focused {
            return Some(NumericAccessibilityRejectedReason::NotFocusable);
        }
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        gui::{
            automation::AutomationTarget,
            types::{Rect, Vector2},
        },
        layout::LayoutOutput,
        runtime::{
            Command, PaintPrimitive, RuntimeBridge, SurfaceNode, SurfaceRuntime, UiSurface,
            WidgetMessageMapper,
        },
        theme::ThemeTokens,
        widgets::{
            NumericAccessibilityBlockOwner, NumericAccessibilityOutcome, NumericAdjustment,
            NumericCodec, NumericInputInteractionBatch, NumericInputWidget, NumericParseResult,
            NumericStep, NumericStepDirection, Widget, WidgetCommon, WidgetInput, WidgetOutput,
            WidgetSizing,
        },
    };
    use std::{cell::Cell, rc::Rc, sync::Arc};

    #[derive(Clone, Copy)]
    struct TestCodec;

    impl NumericCodec<f32> for TestCodec {
        type Error = ();

        fn parse(&self, text: &str) -> NumericParseResult<f32> {
            if text.is_empty() || text == "-" {
                return NumericParseResult::Incomplete;
            }
            text.parse::<f32>()
                .map(NumericParseResult::Valid)
                .unwrap_or(NumericParseResult::Invalid)
        }

        fn format_editable(&self, value: &f32, output: &mut dyn std::fmt::Write) -> Result<(), ()> {
            write!(output, "{value}").map_err(|_| ())
        }
    }

    #[derive(Clone, Copy)]
    struct TestAdjustment;

    impl NumericAdjustment<f32> for TestAdjustment {
        type Error = ();

        fn normalized_to_value(&self, normalized: f32) -> Result<f32, ()> {
            Ok(normalized)
        }

        fn value_to_normalized(&self, value: &f32) -> Result<f32, ()> {
            Ok(*value)
        }

        fn step(
            &self,
            value: &f32,
            direction: NumericStepDirection,
            _step: NumericStep,
        ) -> Result<f32, ()> {
            Ok(*value
                + match direction {
                    NumericStepDirection::Increase => 1.0,
                    NumericStepDirection::Decrease => -1.0,
                })
        }

        fn scrub(&self, value: &f32, _delta: f32, _step: NumericStep) -> Result<f32, ()> {
            Ok(*value)
        }

        fn wheel(&self, value: &f32, _delta: f32, _step: NumericStep) -> Result<f32, ()> {
            Ok(*value)
        }
    }

    #[derive(Clone, Copy, PartialEq, Eq)]
    enum SurfaceMode {
        Numeric,
        NumericAndVeto,
        OwnedVeto,
        Empty,
    }

    struct TestBridge {
        mode: Rc<Cell<SurfaceMode>>,
        reductions: Rc<Cell<usize>>,
        mapped_accessibility: Rc<Cell<usize>>,
    }

    impl TestBridge {
        fn new(mode: SurfaceMode) -> Self {
            Self {
                mode: Rc::new(Cell::new(mode)),
                reductions: Rc::new(Cell::new(0)),
                mapped_accessibility: Rc::new(Cell::new(0)),
            }
        }

        fn surface(&self) -> UiSurface<usize> {
            match self.mode.get() {
                SurfaceMode::Empty => UiSurface::new(SurfaceNode::row(1, 0.0, Vec::new())),
                SurfaceMode::Numeric | SurfaceMode::NumericAndVeto | SurfaceMode::OwnedVeto => {
                    let mut input = NumericInputWidget::try_new(
                        0.0,
                        TestCodec,
                        TestAdjustment,
                        WidgetSizing::fixed(Vector2::new(120.0, 28.0)),
                    )
                    .expect("test numeric input should construct");
                    input.set_accessibility_action_mode();
                    let mapped_accessibility = Rc::clone(&self.mapped_accessibility);
                    let mapper = WidgetMessageMapper::typed(
                        |_: NumericInputInteractionBatch<f32, (), ()>| 0usize,
                    )
                    .with_accessibility_action(
                        move |_: NumericAccessibilityOutcome<f32, (), ()>| {
                            mapped_accessibility.set(mapped_accessibility.get().saturating_add(1));
                            1usize
                        },
                    );
                    let numeric = SurfaceNode::widget(input, mapper).with_id(42);
                    if matches!(
                        self.mode.get(),
                        SurfaceMode::NumericAndVeto | SurfaceMode::OwnedVeto
                    ) {
                        UiSurface::new(SurfaceNode::row(
                            1,
                            0.0,
                            vec![
                                crate::runtime::SurfaceChild::fill(numeric),
                                crate::runtime::SurfaceChild::fill(SurfaceNode::widget(
                                    if self.mode.get() == SurfaceMode::OwnedVeto {
                                        VetoWidget::with_owner(
                                            43,
                                            NumericAccessibilityBlockOwner::TextEdit,
                                        )
                                    } else {
                                        VetoWidget::new(43)
                                    },
                                    WidgetMessageMapper::none(),
                                )),
                            ],
                        ))
                    } else {
                        UiSurface::new(numeric)
                    }
                }
            }
        }
    }

    impl RuntimeBridge<usize> for TestBridge {
        fn project_surface(&mut self) -> Arc<UiSurface<usize>> {
            crate::runtime::test_arc_surface(self.surface())
        }

        fn reduce_message(&mut self, _message: usize) {
            self.reductions.set(self.reductions.get().saturating_add(1));
        }

        fn update(&mut self, message: usize) -> Command<usize> {
            self.reduce_message(message);
            Command::none()
        }
    }

    #[derive(Clone)]
    struct VetoWidget {
        common: WidgetCommon,
        accessibility_owner: Option<NumericAccessibilityBlockOwner>,
    }

    impl VetoWidget {
        fn new(id: u64) -> Self {
            Self {
                common: WidgetCommon::fixed(id, 120.0, 28.0).with_keyboard_focus(),
                accessibility_owner: None,
            }
        }

        fn with_owner(id: u64, owner: NumericAccessibilityBlockOwner) -> Self {
            Self {
                common: WidgetCommon::fixed(id, 120.0, 28.0).with_keyboard_focus(),
                accessibility_owner: Some(owner),
            }
        }
    }

    impl Widget for VetoWidget {
        fn common(&self) -> &WidgetCommon {
            &self.common
        }

        fn common_mut(&mut self) -> &mut WidgetCommon {
            &mut self.common
        }

        fn accessibility_action_owner(&self) -> Option<NumericAccessibilityBlockOwner> {
            self.accessibility_owner
        }

        fn prepare_focus_loss(&mut self) -> crate::widgets::FocusLossDecision {
            crate::widgets::FocusLossDecision::Veto
        }

        fn handle_input(&mut self, _bounds: Rect, _input: WidgetInput) -> Option<WidgetOutput> {
            None
        }

        fn append_paint(
            &self,
            _primitives: &mut Vec<PaintPrimitive>,
            _bounds: Rect,
            _layout: &LayoutOutput,
            _theme: &ThemeTokens,
        ) {
        }
    }

    fn runtime(mode: SurfaceMode) -> SurfaceRuntime<TestBridge, usize> {
        SurfaceRuntime::new(TestBridge::new(mode), Vector2::new(260.0, 48.0))
    }

    fn target(runtime: &SurfaceRuntime<TestBridge, usize>) -> AutomationTarget {
        runtime
            .automation_target_snapshot()
            .targets
            .into_iter()
            .find(|target| target.id.0 == "42")
            .expect("numeric target should be present")
    }

    #[test]
    fn accepted_action_preserves_typed_local_outcome_and_maps_once() {
        let mut runtime = runtime(SurfaceMode::Numeric);
        let captured = target(&runtime);
        assert_eq!(
            captured.available_actions,
            vec![
                crate::runtime::AUTOMATION_ACTION_FOCUS.to_owned(),
                crate::runtime::AUTOMATION_ACTION_INCREMENT.to_owned(),
                crate::runtime::AUTOMATION_ACTION_DECREMENT.to_owned(),
                crate::runtime::AUTOMATION_ACTION_SET_TEXT.to_owned(),
            ]
        );
        let result = runtime.dispatch_numeric_accessibility_action(
            NumericAccessibilityRequest::new(captured, NumericAccessibilityAction::Increment),
        );

        let NumericAccessibilityDispatchResult::Accepted { widget_id, output } = result else {
            panic!("numeric action should be accepted");
        };
        assert_eq!(widget_id, 42);
        let outcome = output
            .typed_ref::<NumericAccessibilityOutcome<f32, (), ()>>()
            .expect("accepted output should retain the typed outcome");
        assert!(matches!(outcome, NumericAccessibilityOutcome::Edit(_)));
        assert_eq!(runtime.bridge().reductions.get(), 1);
        assert_eq!(runtime.bridge().mapped_accessibility.get(), 1);
    }

    #[test]
    fn stale_authority_is_rejected_before_focus_or_widget_mutation() {
        let mut runtime = runtime(SurfaceMode::Numeric);
        let mut stale = target(&runtime);
        stale
            .authority
            .as_mut()
            .expect("runtime target carries authority")
            .runtime_generation = 0;

        let result = runtime.dispatch_numeric_accessibility_action(
            NumericAccessibilityRequest::new(stale, NumericAccessibilityAction::Increment),
        );

        assert_eq!(
            result,
            NumericAccessibilityDispatchResult::Unavailable {
                reason: NumericAccessibilityUnavailableReason::StaleTarget,
            }
        );
        assert_eq!(runtime.focused_widget(), None);
    }

    #[test]
    fn unmaterialized_authority_is_rejected_before_focus_or_widget_mutation() {
        let mut runtime = runtime(SurfaceMode::Numeric);
        let mut unmaterialized = target(&runtime);
        unmaterialized
            .authority
            .as_mut()
            .expect("runtime target carries authority")
            .materialized = false;

        let result = runtime.dispatch_numeric_accessibility_action(
            NumericAccessibilityRequest::new(unmaterialized, NumericAccessibilityAction::Increment),
        );

        assert_eq!(
            result,
            NumericAccessibilityDispatchResult::Unavailable {
                reason: NumericAccessibilityUnavailableReason::UnmaterializedTarget,
            }
        );
        assert_eq!(runtime.focused_widget(), None);
    }

    #[test]
    fn removed_materialized_target_is_classified_as_removed() {
        let mut runtime = runtime(SurfaceMode::Numeric);
        let captured = target(&runtime);
        runtime.bridge_mut().mode.set(SurfaceMode::Empty);
        runtime.refresh();

        let result = runtime.dispatch_numeric_accessibility_action(
            NumericAccessibilityRequest::new(captured, NumericAccessibilityAction::Increment),
        );

        assert_eq!(
            result,
            NumericAccessibilityDispatchResult::Unavailable {
                reason: NumericAccessibilityUnavailableReason::RemovedTarget,
            }
        );
    }

    #[test]
    fn incumbent_owner_blocks_before_focus_transfer() {
        let mut runtime = runtime(SurfaceMode::Numeric);
        let captured = target(&runtime);
        assert!(runtime.focus_widget(42));
        runtime.dispatch_input(
            42,
            WidgetInput::Character {
                character: '1',
                timestamp: None,
            },
        );

        let result = runtime.dispatch_numeric_accessibility_action(
            NumericAccessibilityRequest::new(captured, NumericAccessibilityAction::Increment),
        );

        assert_eq!(
            result,
            NumericAccessibilityDispatchResult::Blocked {
                owner: NumericAccessibilityBlockOwner::TextEdit,
            }
        );
        assert_eq!(runtime.focused_widget(), Some(42));
    }

    #[test]
    fn focused_different_widget_owner_blocks_before_focus_transfer() {
        let mut runtime = runtime(SurfaceMode::OwnedVeto);
        assert!(runtime.focus_widget(43));
        let captured = target(&runtime);

        let result = runtime.dispatch_numeric_accessibility_action(
            NumericAccessibilityRequest::new(captured, NumericAccessibilityAction::Increment),
        );

        assert_eq!(
            result,
            NumericAccessibilityDispatchResult::Blocked {
                owner: NumericAccessibilityBlockOwner::TextEdit,
            }
        );
        assert_eq!(runtime.focused_widget(), Some(43));
        assert_eq!(runtime.bridge().mapped_accessibility.get(), 0);
    }

    #[test]
    fn focus_veto_is_rejected_without_invoking_numeric_policy() {
        let mut runtime = runtime(SurfaceMode::NumericAndVeto);
        assert!(runtime.focus_widget(43));
        let captured = target(&runtime);

        let result = runtime.dispatch_numeric_accessibility_action(
            NumericAccessibilityRequest::new(captured, NumericAccessibilityAction::Increment),
        );

        assert_eq!(
            result,
            NumericAccessibilityDispatchResult::Rejected {
                reason: NumericAccessibilityRejectedReason::FocusDenied,
            }
        );
        assert_eq!(runtime.focused_widget(), Some(43));
        assert_eq!(runtime.bridge().mapped_accessibility.get(), 0);
    }
}
