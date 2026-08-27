use super::super::{FocusTraversal, focus::FocusTransition, interaction_state::RuntimeFocusOwner};
use super::*;
use crate::{
    gui::automation::AutomationRole,
    gui::input::{InputSequence, InputSequenceRange, InputTimestamp},
    gui::layout_core::SplitPaneRuntimeState,
    gui::types::{Point, Rect, Vector2},
    layout::{
        Constraints, ContainerKind, ContainerPolicy, ContainerStateDeclaration,
        LAYOUT_CAPABILITIES_CONTRACT_VERSION, LAYOUT_CAPABILITIES_PROJECTION_CONTRACT_VERSION,
        LayoutCapabilities, LayoutContainerStateContext, LayoutHitRegion, LayoutHitRegionId,
        LayoutInput, LayoutInteraction, LayoutInteractionRevision, LayoutOutput,
        LayoutTargetIdentity, OverflowPolicy, SizeModeCross, SizeModeMain, SlotParams,
        SplitPaneAxis, SplitPaneCollapsePolicy, SplitPanePolicy,
    },
    runtime::{
        Command, CommandOutcome, Event, PaintPrimitive, RepaintScope, SurfaceChild, SurfaceNode,
        UiSurface, WidgetMessageMapper,
    },
    theme::ThemeTokens,
    widgets::{
        ButtonWidget, DragHandleWidget, EditPhase, FocusBehavior, FocusLossDecision,
        InteractionSource, InteractiveRowWidget, KeyboardModifiers, NumericAdjustment, NumericStep,
        NumericStepDirection, PointerButton, PointerModifiers, PointerPressAdmission,
        PointerShieldMessage, PointerShieldWidget, RetainedSliderDomainWidget, SliderDomainError,
        SliderDomainMessage, SliderEditBatch, SliderWidget, TextInputWidget, TextWidget, Widget,
        WidgetCommon, WidgetInput, WidgetKey, WidgetOutput, WidgetSizing,
    },
};
use std::{
    cell::{Cell, RefCell},
    rc::Rc,
    sync::Arc,
};

struct FocusTestBridge;

impl RuntimeBridge<usize> for FocusTestBridge {
    fn project_surface(&mut self) -> Arc<UiSurface<usize>> {
        crate::runtime::test_arc_surface(UiSurface::new(SurfaceNode::column(
            1,
            0.0,
            vec![
                fixed_child(
                    28.0,
                    SurfaceNode::widget(
                        TextInputWidget::new(
                            10,
                            "tag",
                            WidgetSizing::fixed(Vector2::new(160.0, 28.0)),
                        ),
                        WidgetMessageMapper::none(),
                    ),
                ),
                fixed_child(
                    28.0,
                    SurfaceNode::widget(
                        non_focusable_interactive_row(20),
                        WidgetMessageMapper::none(),
                    ),
                ),
            ],
        )))
    }

    fn reduce_message(&mut self, _message: usize) {}
}

#[derive(Default)]
struct FocusLossOutputBridge {
    dispatched: Vec<usize>,
}

impl RuntimeBridge<usize> for FocusLossOutputBridge {
    fn project_surface(&mut self) -> Arc<UiSurface<usize>> {
        crate::runtime::test_arc_surface(UiSurface::new(SurfaceNode::widget(
            FocusLossOutputWidget::new(30),
            WidgetMessageMapper::typed(|message: usize| message),
        )))
    }

    fn reduce_message(&mut self, message: usize) {
        self.dispatched.push(message);
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum FocusDecisionEvent {
    Prepare(usize),
    Changed(usize, bool),
    Press(usize),
    DoubleClick(usize),
    HostOutput,
}

#[derive(Clone)]
struct FocusDecisionWidget {
    common: WidgetCommon,
    decision: Rc<Cell<FocusLossDecision>>,
    events: Rc<RefCell<Vec<FocusDecisionEvent>>>,
    emit_focus_loss_output: bool,
    pointer_press_admission: PointerPressAdmission,
    retains_managed_pointer_capture: bool,
}

impl FocusDecisionWidget {
    fn new(
        id: u64,
        decision: Rc<Cell<FocusLossDecision>>,
        events: Rc<RefCell<Vec<FocusDecisionEvent>>>,
        emit_focus_loss_output: bool,
        focusable: bool,
    ) -> Self {
        Self::new_with_size(
            id,
            decision,
            events,
            Vector2::new(160.0, 28.0),
            emit_focus_loss_output,
            focusable,
        )
    }

    fn new_with_size(
        id: u64,
        decision: Rc<Cell<FocusLossDecision>>,
        events: Rc<RefCell<Vec<FocusDecisionEvent>>>,
        size: Vector2,
        emit_focus_loss_output: bool,
        focusable: bool,
    ) -> Self {
        Self {
            common: WidgetCommon::fixed(id, size.x, size.y)
                .with_focus(if focusable {
                    FocusBehavior::Keyboard
                } else {
                    FocusBehavior::None
                })
                .without_default_chrome(),
            decision,
            events,
            emit_focus_loss_output,
            pointer_press_admission: PointerPressAdmission::Legacy,
            retains_managed_pointer_capture: false,
        }
    }

    fn with_pointer_press_admission(
        mut self,
        admission: PointerPressAdmission,
        retains_managed_pointer_capture: bool,
    ) -> Self {
        self.pointer_press_admission = admission;
        self.retains_managed_pointer_capture = retains_managed_pointer_capture;
        self
    }
}

impl Widget for FocusDecisionWidget {
    fn common(&self) -> &WidgetCommon {
        &self.common
    }

    fn common_mut(&mut self) -> &mut WidgetCommon {
        &mut self.common
    }

    fn prepare_focus_loss(&mut self) -> FocusLossDecision {
        self.events
            .borrow_mut()
            .push(FocusDecisionEvent::Prepare(self.common.id as usize));
        self.decision.get()
    }

    fn preflight_pointer_press(
        &self,
        _bounds: Rect,
        _input: &WidgetInput,
    ) -> PointerPressAdmission {
        self.pointer_press_admission
    }

    fn retains_managed_pointer_capture(&self) -> bool {
        self.retains_managed_pointer_capture
    }

    fn handle_input(&mut self, bounds: Rect, input: WidgetInput) -> Option<WidgetOutput> {
        match input {
            WidgetInput::FocusChanged(focused) => {
                self.common.state.focused = focused;
                self.events.borrow_mut().push(FocusDecisionEvent::Changed(
                    self.common.id as usize,
                    focused,
                ));
                if !focused && self.emit_focus_loss_output {
                    Some(WidgetOutput::typed(FocusDecisionEvent::HostOutput))
                } else {
                    None
                }
            }
            WidgetInput::PointerPress { position, .. } if bounds.contains(position) => {
                self.events
                    .borrow_mut()
                    .push(FocusDecisionEvent::Press(self.common.id as usize));
                None
            }
            WidgetInput::PointerDoubleClick { position, .. } if bounds.contains(position) => {
                self.events
                    .borrow_mut()
                    .push(FocusDecisionEvent::DoubleClick(self.common.id as usize));
                None
            }
            _ => None,
        }
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

#[derive(Clone)]
struct FocusDecisionBridge {
    old_decision: Rc<Cell<FocusLossDecision>>,
    target_decision: Rc<Cell<FocusLossDecision>>,
    events: Rc<RefCell<Vec<FocusDecisionEvent>>>,
    remove_old: bool,
    target_focusable: bool,
    scroll: bool,
    old_pointer_press_admission: PointerPressAdmission,
    old_retains_managed_pointer_capture: bool,
}

impl FocusDecisionBridge {
    fn new() -> Self {
        Self {
            old_decision: Rc::new(Cell::new(FocusLossDecision::Allow)),
            target_decision: Rc::new(Cell::new(FocusLossDecision::Allow)),
            events: Rc::new(RefCell::new(Vec::new())),
            remove_old: false,
            target_focusable: true,
            scroll: false,
            old_pointer_press_admission: PointerPressAdmission::Legacy,
            old_retains_managed_pointer_capture: false,
        }
    }

    fn with_target_focusable(mut self, focusable: bool) -> Self {
        self.target_focusable = focusable;
        self
    }

    fn with_scroll(mut self) -> Self {
        self.scroll = true;
        self
    }

    fn with_managed_capture(mut self) -> Self {
        self.old_pointer_press_admission = PointerPressAdmission::ManagedCapture;
        self.old_retains_managed_pointer_capture = true;
        self
    }

    fn row(&self) -> SurfaceNode<FocusDecisionEvent> {
        if self.scroll {
            return SurfaceNode::scroll_area(
                30,
                SurfaceNode::widget(
                    FocusDecisionWidget::new_with_size(
                        10,
                        Rc::clone(&self.old_decision),
                        Rc::clone(&self.events),
                        Vector2::new(300.0, 200.0),
                        true,
                        true,
                    )
                    .with_pointer_press_admission(
                        self.old_pointer_press_admission,
                        self.old_retains_managed_pointer_capture,
                    ),
                    WidgetMessageMapper::typed(|event: FocusDecisionEvent| event),
                ),
            );
        }

        let mut children = Vec::with_capacity(if self.remove_old { 1 } else { 2 });
        if !self.remove_old {
            children.push(fixed_child(
                28.0,
                SurfaceNode::widget(
                    FocusDecisionWidget::new(
                        10,
                        Rc::clone(&self.old_decision),
                        Rc::clone(&self.events),
                        true,
                        true,
                    )
                    .with_pointer_press_admission(
                        self.old_pointer_press_admission,
                        self.old_retains_managed_pointer_capture,
                    ),
                    WidgetMessageMapper::typed(|event: FocusDecisionEvent| event),
                ),
            ));
        }
        children.push(fixed_child(
            28.0,
            SurfaceNode::widget(
                FocusDecisionWidget::new(
                    20,
                    Rc::clone(&self.target_decision),
                    Rc::clone(&self.events),
                    false,
                    self.target_focusable,
                ),
                WidgetMessageMapper::typed(|event: FocusDecisionEvent| event),
            ),
        ));
        SurfaceNode::column(1, 0.0, children)
    }
}

impl Default for FocusDecisionBridge {
    fn default() -> Self {
        Self::new()
    }
}

impl RuntimeBridge<FocusDecisionEvent> for FocusDecisionBridge {
    fn project_surface(&mut self) -> Arc<UiSurface<FocusDecisionEvent>> {
        crate::runtime::test_arc_surface(UiSurface::new(self.row()))
    }

    fn reduce_message(&mut self, message: FocusDecisionEvent) {
        if message == FocusDecisionEvent::HostOutput {
            self.events.borrow_mut().push(message);
        }
    }
}

struct FocusDecisionSplitBridge {
    inner: FocusDecisionBridge,
    policy: SplitPanePolicy,
    remove_split_on_host_output: bool,
    change_split_geometry_on_host_output: bool,
    change_split_initial_ratio_on_host_output: bool,
    focus_widget_20_on_host_output: bool,
    collapse_policy: Option<SplitPaneCollapsePolicy>,
    split_present: bool,
    split_spacing: f32,
}

impl FocusDecisionSplitBridge {
    fn new() -> Self {
        Self {
            inner: FocusDecisionBridge::new(),
            policy: SplitPanePolicy {
                axis: SplitPaneAxis::Horizontal,
                initial_ratio: 0.25,
                divider_extent: 8.0,
                first_min_extent: 0.0,
                second_min_extent: 0.0,
            },
            remove_split_on_host_output: false,
            change_split_geometry_on_host_output: false,
            change_split_initial_ratio_on_host_output: false,
            focus_widget_20_on_host_output: false,
            collapse_policy: None,
            split_present: true,
            split_spacing: 0.0,
        }
    }

    fn with_split_removed_on_host_output(mut self) -> Self {
        self.remove_split_on_host_output = true;
        self
    }

    fn with_split_geometry_changed_on_host_output(mut self) -> Self {
        self.change_split_geometry_on_host_output = true;
        self
    }

    fn with_split_initial_ratio_changed_on_host_output(mut self) -> Self {
        self.change_split_initial_ratio_on_host_output = true;
        self
    }

    fn with_focus_widget_20_on_host_output(mut self) -> Self {
        self.focus_widget_20_on_host_output = true;
        self
    }

    fn with_collapse_policy(mut self, collapse_policy: SplitPaneCollapsePolicy) -> Self {
        self.collapse_policy = Some(collapse_policy);
        self
    }

    fn surface(&self) -> UiSurface<FocusDecisionEvent> {
        let split = SurfaceNode::container(
            1,
            ContainerPolicy {
                kind: ContainerKind::SplitPane,
                split_pane: self.policy,
                ..ContainerPolicy::default()
            },
            vec![
                SurfaceChild::fill(SurfaceNode::widget(
                    TextWidget::new(2, "first", WidgetSizing::fixed(Vector2::new(20.0, 20.0))),
                    WidgetMessageMapper::none(),
                )),
                SurfaceChild::fill(SurfaceNode::widget(
                    TextWidget::new(3, "second", WidgetSizing::fixed(Vector2::new(20.0, 20.0))),
                    WidgetMessageMapper::none(),
                )),
            ],
        )
        .with_split_pane_runtime_mode(Some(
            crate::gui::layout_core::SplitPaneRuntimeMode::RuntimeOwned {
                collapse_policy: self.collapse_policy,
            },
        ))
        .with_layout_capabilities(
            crate::gui::layout_core::runtime_owned_split_pane_capabilities(
                self.policy,
                self.collapse_policy,
            ),
        );

        let mut children = vec![fixed_child(
            28.0,
            SurfaceNode::widget(
                FocusDecisionWidget::new(
                    10,
                    Rc::clone(&self.inner.old_decision),
                    Rc::clone(&self.inner.events),
                    true,
                    true,
                ),
                WidgetMessageMapper::typed(|event: FocusDecisionEvent| event),
            ),
        )];
        if self.focus_widget_20_on_host_output {
            children.push(fixed_child(
                28.0,
                SurfaceNode::widget(
                    FocusDecisionWidget::new(
                        20,
                        Rc::clone(&self.inner.target_decision),
                        Rc::clone(&self.inner.events),
                        false,
                        true,
                    ),
                    WidgetMessageMapper::typed(|event: FocusDecisionEvent| event),
                ),
            ));
        }
        if self.split_present {
            children.push(SurfaceChild::fill(split));
        }
        UiSurface::new(SurfaceNode::column(99, self.split_spacing, children))
    }
}

impl RuntimeBridge<FocusDecisionEvent> for FocusDecisionSplitBridge {
    fn project_surface(&mut self) -> Arc<UiSurface<FocusDecisionEvent>> {
        crate::runtime::test_arc_surface(self.surface())
    }

    fn reduce_message(&mut self, message: FocusDecisionEvent) {
        self.inner.reduce_message(message);
        if message == FocusDecisionEvent::HostOutput && self.remove_split_on_host_output {
            self.split_present = false;
        }
        if message == FocusDecisionEvent::HostOutput && self.change_split_geometry_on_host_output {
            self.split_spacing = 8.0;
        }
        if message == FocusDecisionEvent::HostOutput
            && self.change_split_initial_ratio_on_host_output
        {
            self.policy.initial_ratio = 0.5;
        }
    }

    fn update(&mut self, message: FocusDecisionEvent) -> Command<FocusDecisionEvent> {
        self.reduce_message(message);
        if message == FocusDecisionEvent::HostOutput && self.focus_widget_20_on_host_output {
            Command::focus(20)
        } else {
            Command::none()
        }
    }
}

struct SplitFallbackBridge {
    events: Rc<RefCell<Vec<FocusDecisionEvent>>>,
}

impl SplitFallbackBridge {
    fn new(events: Rc<RefCell<Vec<FocusDecisionEvent>>>) -> Self {
        Self { events }
    }

    fn surface(&self) -> UiSurface<FocusDecisionEvent> {
        let policy = SplitPanePolicy {
            axis: SplitPaneAxis::Horizontal,
            initial_ratio: 0.25,
            divider_extent: 8.0,
            first_min_extent: 0.0,
            second_min_extent: 0.0,
        };
        let split = SurfaceNode::container(
            1,
            ContainerPolicy {
                kind: ContainerKind::SplitPane,
                split_pane: policy,
                ..ContainerPolicy::default()
            },
            vec![
                SurfaceChild::fill(SurfaceNode::widget(
                    TextWidget::new(2, "first", WidgetSizing::fixed(Vector2::new(20.0, 20.0))),
                    WidgetMessageMapper::none(),
                )),
                SurfaceChild::fill(SurfaceNode::widget(
                    TextWidget::new(3, "second", WidgetSizing::fixed(Vector2::new(20.0, 20.0))),
                    WidgetMessageMapper::none(),
                )),
            ],
        )
        .with_split_pane_runtime_mode(Some(
            crate::gui::layout_core::SplitPaneRuntimeMode::RuntimeOwned {
                collapse_policy: None,
            },
        ))
        .with_layout_capabilities(
            crate::gui::layout_core::runtime_owned_split_pane_capabilities(policy, None),
        );
        UiSurface::new(SurfaceNode::stack(
            99,
            vec![
                SurfaceChild::fill(SurfaceNode::widget(
                    FocusDecisionWidget::new_with_size(
                        40,
                        Rc::new(Cell::new(FocusLossDecision::Allow)),
                        Rc::clone(&self.events),
                        Vector2::new(200.0, 80.0),
                        false,
                        false,
                    ),
                    WidgetMessageMapper::typed(|event: FocusDecisionEvent| event),
                )),
                SurfaceChild::fill(split),
            ],
        ))
    }
}

impl RuntimeBridge<FocusDecisionEvent> for SplitFallbackBridge {
    fn project_surface(&mut self) -> Arc<UiSurface<FocusDecisionEvent>> {
        crate::runtime::test_arc_surface(self.surface())
    }

    fn reduce_message(&mut self, _message: FocusDecisionEvent) {}
}

#[derive(Default)]
struct SliderCaptureBridge {
    batches: Vec<SliderEditBatch>,
}

impl RuntimeBridge<SliderEditBatch> for SliderCaptureBridge {
    fn project_surface(&mut self) -> Arc<UiSurface<SliderEditBatch>> {
        crate::runtime::test_arc_surface(UiSurface::new(SurfaceNode::slider_edits_mapped(
            31,
            0.25,
            WidgetSizing::fixed(Vector2::new(120.0, 28.0)),
            |batch| batch,
        )))
    }

    fn reduce_message(&mut self, message: SliderEditBatch) {
        self.batches.push(message);
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum DomainAdjustmentError {
    Forward,
}

#[derive(Clone)]
struct FailingReleaseAdjustment {
    forward_calls: Rc<Cell<usize>>,
}

impl NumericAdjustment<f32> for FailingReleaseAdjustment {
    type Error = DomainAdjustmentError;

    fn normalized_to_value(&self, normalized: f32) -> Result<f32, Self::Error> {
        let call = self.forward_calls.get() + 1;
        self.forward_calls.set(call);
        if call > 1 {
            Err(DomainAdjustmentError::Forward)
        } else {
            Ok(10.0 + normalized * 100.0)
        }
    }

    fn value_to_normalized(&self, value: &f32) -> Result<f32, Self::Error> {
        Ok((*value - 10.0) / 100.0)
    }

    fn step(
        &self,
        _value: &f32,
        _direction: NumericStepDirection,
        _step: NumericStep,
    ) -> Result<f32, Self::Error> {
        panic!("domain slider test must not invoke adjustment steps")
    }

    fn scrub(
        &self,
        _value: &f32,
        _normalized_delta: f32,
        _step: NumericStep,
    ) -> Result<f32, Self::Error> {
        panic!("domain slider test must not invoke adjustment scrubbing")
    }

    fn wheel(&self, _value: &f32, _delta: f32, _step: NumericStep) -> Result<f32, Self::Error> {
        panic!("domain slider test must not invoke adjustment wheel changes")
    }
}

struct FailingDomainSliderBridge {
    messages: Vec<SliderDomainMessage<DomainAdjustmentError>>,
    domain_value: f32,
    forward_calls: Rc<Cell<usize>>,
}

impl FailingDomainSliderBridge {
    fn new() -> Self {
        Self {
            messages: Vec::new(),
            domain_value: 10.0,
            forward_calls: Rc::new(Cell::new(0)),
        }
    }
}

impl RuntimeBridge<SliderDomainMessage<DomainAdjustmentError>> for FailingDomainSliderBridge {
    fn project_surface(&mut self) -> Arc<UiSurface<SliderDomainMessage<DomainAdjustmentError>>> {
        let normalized_value = (self.domain_value - 10.0) / 100.0;
        let slider = RetainedSliderDomainWidget::new(
            SliderWidget::new(
                31,
                normalized_value,
                WidgetSizing::fixed(Vector2::new(120.0, 28.0)),
            ),
            Rc::new(FailingReleaseAdjustment {
                forward_calls: Rc::clone(&self.forward_calls),
            }),
            self.domain_value,
        );
        crate::runtime::test_arc_surface(UiSurface::new(SurfaceNode::widget(
            slider,
            WidgetMessageMapper::typed(|message: SliderDomainMessage<DomainAdjustmentError>| {
                message
            }),
        )))
    }

    fn reduce_message(&mut self, message: SliderDomainMessage<DomainAdjustmentError>) {
        if let SliderDomainMessage::ValueChanged { value } = &message {
            self.domain_value = *value;
        }
        self.messages.push(message);
    }
}

#[derive(Default)]
struct PointerSnapshotBridge {
    snapshots: Vec<Option<Point>>,
}

struct PointerPolicyStackBridge;

#[derive(Clone, Copy, Debug, PartialEq)]
enum DoubleClickTimestampMessage {
    DoubleClick(Option<InputTimestamp>),
    Press(Option<InputTimestamp>),
    Modifiers(Option<InputTimestamp>),
    Wheel {
        position: Point,
        delta: Vector2,
        modifiers: PointerModifiers,
        timestamp: Option<InputTimestamp>,
        sequence_range: Option<InputSequenceRange>,
    },
    Move {
        modifiers: PointerModifiers,
        timestamp: Option<InputTimestamp>,
        sequence_range: Option<InputSequenceRange>,
    },
}

#[derive(Clone)]
struct DoubleClickTimestampWidget {
    common: WidgetCommon,
    handles_double_click: bool,
}

impl DoubleClickTimestampWidget {
    fn new(id: u64, handles_double_click: bool) -> Self {
        Self {
            common: WidgetCommon::new(id, WidgetSizing::fixed(Vector2::new(120.0, 40.0))),
            handles_double_click,
        }
    }
}

impl Widget for DoubleClickTimestampWidget {
    fn common(&self) -> &WidgetCommon {
        &self.common
    }

    fn common_mut(&mut self) -> &mut WidgetCommon {
        &mut self.common
    }

    fn handle_input(&mut self, bounds: Rect, input: WidgetInput) -> Option<WidgetOutput> {
        match input {
            WidgetInput::PointerDoubleClick {
                position,
                timestamp,
                ..
            } if self.handles_double_click && bounds.contains(position) => Some(
                WidgetOutput::typed(DoubleClickTimestampMessage::DoubleClick(timestamp)),
            ),
            WidgetInput::PointerPress {
                position,
                timestamp,
                ..
            } if bounds.contains(position) => Some(WidgetOutput::typed(
                DoubleClickTimestampMessage::Press(timestamp),
            )),
            WidgetInput::PointerModifiersChanged { timestamp, .. } => Some(WidgetOutput::typed(
                DoubleClickTimestampMessage::Modifiers(timestamp),
            )),
            WidgetInput::Wheel {
                position,
                delta,
                modifiers,
                timestamp,
                sequence_range,
                ..
            } if bounds.contains(position) => {
                Some(WidgetOutput::typed(DoubleClickTimestampMessage::Wheel {
                    position,
                    delta,
                    modifiers,
                    timestamp,
                    sequence_range,
                }))
            }
            WidgetInput::PointerMove {
                position,
                modifiers,
                timestamp,
                sequence_range,
                ..
            } if bounds.contains(position) => {
                Some(WidgetOutput::typed(DoubleClickTimestampMessage::Move {
                    modifiers,
                    timestamp,
                    sequence_range,
                }))
            }
            _ => None,
        }
    }

    fn accepts_wheel_input(&self) -> bool {
        true
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

#[derive(Default)]
struct DoubleClickTimestampBridge {
    messages: Vec<DoubleClickTimestampMessage>,
    handles_double_click: bool,
}

impl RuntimeBridge<DoubleClickTimestampMessage> for DoubleClickTimestampBridge {
    fn project_surface(&mut self) -> Arc<UiSurface<DoubleClickTimestampMessage>> {
        crate::runtime::test_arc_surface(UiSurface::new(SurfaceNode::widget(
            DoubleClickTimestampWidget::new(40, self.handles_double_click),
            WidgetMessageMapper::typed(|message: DoubleClickTimestampMessage| message),
        )))
    }

    fn reduce_message(&mut self, message: DoubleClickTimestampMessage) {
        self.messages.push(message);
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum MoveFanoutMessage {
    Press,
    Move {
        widget_id: u64,
        modifiers: PointerModifiers,
        timestamp: Option<InputTimestamp>,
    },
}

#[derive(Clone)]
struct MoveFanoutWidget {
    common: WidgetCommon,
}

impl MoveFanoutWidget {
    fn new(id: u64) -> Self {
        Self {
            common: WidgetCommon::new(id, WidgetSizing::fixed(Vector2::new(160.0, 28.0))),
        }
    }
}

impl Widget for MoveFanoutWidget {
    fn common(&self) -> &WidgetCommon {
        &self.common
    }

    fn common_mut(&mut self) -> &mut WidgetCommon {
        &mut self.common
    }

    fn handle_input(&mut self, bounds: Rect, input: WidgetInput) -> Option<WidgetOutput> {
        match input {
            WidgetInput::PointerPress { position, .. } if bounds.contains(position) => {
                Some(WidgetOutput::typed(MoveFanoutMessage::Press))
            }
            WidgetInput::PointerMove {
                modifiers,
                timestamp,
                ..
            } => Some(WidgetOutput::typed(MoveFanoutMessage::Move {
                widget_id: self.common.id,
                modifiers,
                timestamp,
            })),
            _ => None,
        }
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

#[derive(Default)]
struct MoveFanoutBridge {
    samples: Vec<(u64, PointerModifiers, Option<InputTimestamp>)>,
}

impl RuntimeBridge<MoveFanoutMessage> for MoveFanoutBridge {
    fn project_surface(&mut self) -> Arc<UiSurface<MoveFanoutMessage>> {
        crate::runtime::test_arc_surface(UiSurface::new(SurfaceNode::column(
            1,
            0.0,
            vec![
                fixed_child(
                    28.0,
                    SurfaceNode::widget(
                        MoveFanoutWidget::new(50),
                        WidgetMessageMapper::typed(|message: MoveFanoutMessage| message),
                    ),
                ),
                fixed_child(
                    28.0,
                    SurfaceNode::widget(
                        MoveFanoutWidget::new(60),
                        WidgetMessageMapper::typed(|message: MoveFanoutMessage| message),
                    ),
                ),
            ],
        )))
    }

    fn reduce_message(&mut self, message: MoveFanoutMessage) {
        if let MoveFanoutMessage::Move {
            widget_id,
            modifiers,
            timestamp,
        } = message
        {
            self.samples.push((widget_id, modifiers, timestamp));
        }
    }
}

impl RuntimeBridge<u64> for PointerPolicyStackBridge {
    fn project_surface(&mut self) -> Arc<UiSurface<u64>> {
        crate::runtime::test_arc_surface(UiSurface::new(SurfaceNode::stack(
            1,
            vec![
                SurfaceChild::fill(SurfaceNode::widget(
                    PointerShieldWidget::new(10, WidgetSizing::fixed(Vector2::new(1.0, 1.0))),
                    WidgetMessageMapper::typed(|_: PointerShieldMessage| 10),
                )),
                SurfaceChild::fill(SurfaceNode::widget(
                    PointerShieldWidget::new(20, WidgetSizing::fixed(Vector2::new(1.0, 1.0)))
                        .with_pointer_press(false)
                        .with_pointer_release(false)
                        .with_pointer_drop(false)
                        .with_wheel(false),
                    WidgetMessageMapper::typed(|_: PointerShieldMessage| 20),
                )),
            ],
        )))
    }

    fn reduce_message(&mut self, _message: u64) {}
}

impl RuntimeBridge<()> for PointerSnapshotBridge {
    fn project_surface(&mut self) -> Arc<UiSurface<()>> {
        crate::runtime::test_arc_surface(UiSurface::new(SurfaceNode::container(
            1,
            Default::default(),
            Vec::new(),
        )))
    }

    fn update_with_runtime(
        &mut self,
        _message: (),
        snapshot: crate::runtime::RuntimeUpdateSnapshot,
    ) -> Command<()> {
        self.snapshots.push(snapshot.current_pointer_position());
        Command::none()
    }
}

#[derive(Clone)]
struct FocusLossOutputWidget {
    common: WidgetCommon,
}

impl FocusLossOutputWidget {
    fn new(id: u64) -> Self {
        let mut common = WidgetCommon::fixed(id, 160.0, 28.0).without_default_chrome();
        common.paint.suppresses_container_hover = true;
        Self { common }
    }
}

impl Widget for FocusLossOutputWidget {
    fn common(&self) -> &WidgetCommon {
        &self.common
    }

    fn common_mut(&mut self) -> &mut WidgetCommon {
        &mut self.common
    }

    fn handle_input(&mut self, bounds: Rect, input: WidgetInput) -> Option<WidgetOutput> {
        match input {
            WidgetInput::PointerPress {
                position,
                button: PointerButton::Primary,
                ..
            } if bounds.contains(position) => {
                self.common.state.pressed = true;
                None
            }
            WidgetInput::FocusChanged(false) => {
                self.common.state.pressed = false;
                Some(WidgetOutput::typed(99_usize))
            }
            _ => None,
        }
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

fn non_focusable_interactive_row(id: u64) -> InteractiveRowWidget {
    let mut row = InteractiveRowWidget::new(id, WidgetSizing::fixed(Vector2::new(160.0, 28.0)));
    row.common.focus = FocusBehavior::None;
    row.common.paint.suppresses_container_hover = true;
    row
}

fn fixed_child<Message>(height: f32, child: SurfaceNode<Message>) -> SurfaceChild<Message> {
    SurfaceChild::new(
        SlotParams {
            size_main: SizeModeMain::Fixed(height),
            size_cross: SizeModeCross::Fill,
            constraints: Constraints::unconstrained(),
            margin: Default::default(),
            align_cross_override: None,
            allow_fixed_compress: false,
        },
        child,
    )
}

fn fixed_width_child<Message>(width: f32, child: SurfaceNode<Message>) -> SurfaceChild<Message> {
    SurfaceChild::new(
        SlotParams {
            size_main: SizeModeMain::Fixed(width),
            size_cross: SizeModeCross::Fill,
            constraints: Constraints::unconstrained(),
            margin: Default::default(),
            align_cross_override: None,
            allow_fixed_compress: false,
        },
        child,
    )
}

#[derive(Clone, Copy)]
enum SplitInteractionMode {
    Static,
    RuntimeOwned,
    Controlled,
}

struct SplitInteractionBridge {
    mode: SplitInteractionMode,
    policy: SplitPanePolicy,
    collapse_policy: Option<SplitPaneCollapsePolicy>,
    ratio: f32,
    generation: u64,
    clipped: bool,
    mounted: bool,
    nested: bool,
}

impl SplitInteractionBridge {
    fn new(mode: SplitInteractionMode) -> Self {
        Self {
            mode,
            policy: SplitPanePolicy {
                axis: SplitPaneAxis::Horizontal,
                initial_ratio: 0.25,
                divider_extent: 8.0,
                first_min_extent: 0.0,
                second_min_extent: 0.0,
            },
            collapse_policy: None,
            ratio: 0.25,
            generation: 1,
            clipped: false,
            mounted: true,
            nested: false,
        }
    }

    fn with_axis(mut self, axis: SplitPaneAxis) -> Self {
        self.policy.axis = axis;
        self
    }

    fn with_nested(mut self) -> Self {
        self.nested = true;
        self
    }

    fn with_collapse_policy(mut self, collapse_policy: SplitPaneCollapsePolicy) -> Self {
        self.collapse_policy = Some(collapse_policy);
        self
    }

    fn surface(&self) -> UiSurface<()> {
        if !self.mounted {
            return UiSurface::new(SurfaceNode::widget(
                TextWidget::new(
                    90,
                    "unmounted",
                    WidgetSizing::fixed(Vector2::new(80.0, 20.0)),
                ),
                WidgetMessageMapper::none(),
            ));
        }
        let policy = self.policy;
        let child_size = if self.clipped {
            Vector2::new(160.0, 80.0)
        } else {
            Vector2::new(20.0, 20.0)
        };
        let first = if self.nested {
            let inner_policy = SplitPanePolicy {
                axis: SplitPaneAxis::Vertical,
                ..policy
            };
            let inner_first = SurfaceNode::widget(
                TextWidget::new(5, "inner-first", WidgetSizing::fixed(child_size)),
                WidgetMessageMapper::none(),
            );
            let inner_second = SurfaceNode::widget(
                TextWidget::new(6, "inner-second", WidgetSizing::fixed(child_size)),
                WidgetMessageMapper::none(),
            );
            let mut inner = SurfaceNode::container(
                4,
                ContainerPolicy {
                    kind: ContainerKind::SplitPane,
                    split_pane: inner_policy,
                    ..ContainerPolicy::default()
                },
                vec![
                    SurfaceChild::fill(inner_first),
                    SurfaceChild::fill(inner_second),
                ],
            );
            match self.mode {
                SplitInteractionMode::Static => {}
                SplitInteractionMode::RuntimeOwned => {
                    inner = inner
                        .with_split_pane_runtime_mode(Some(
                            crate::gui::layout_core::SplitPaneRuntimeMode::RuntimeOwned {
                                collapse_policy: self.collapse_policy,
                            },
                        ))
                        .with_layout_capabilities(
                            crate::gui::layout_core::runtime_owned_split_pane_capabilities(
                                inner_policy,
                                self.collapse_policy,
                            ),
                        );
                }
                SplitInteractionMode::Controlled => {
                    inner = inner.with_split_pane_runtime_mode(Some(
                        crate::gui::layout_core::SplitPaneRuntimeMode::Controlled(
                            crate::gui::layout_core::Controlled::new(self.ratio, self.generation),
                        ),
                    ));
                }
            }
            inner
        } else {
            SurfaceNode::widget(
                TextWidget::new(2, "first", WidgetSizing::fixed(child_size)),
                WidgetMessageMapper::none(),
            )
        };
        let second = SurfaceNode::widget(
            TextWidget::new(3, "second", WidgetSizing::fixed(child_size)),
            WidgetMessageMapper::none(),
        );
        let mut split = SurfaceNode::container(
            1,
            ContainerPolicy {
                kind: ContainerKind::SplitPane,
                split_pane: policy,
                ..ContainerPolicy::default()
            },
            vec![SurfaceChild::fill(first), SurfaceChild::fill(second)],
        );
        match self.mode {
            SplitInteractionMode::Static => {}
            SplitInteractionMode::RuntimeOwned => {
                split = split
                    .with_split_pane_runtime_mode(Some(
                        crate::gui::layout_core::SplitPaneRuntimeMode::RuntimeOwned {
                            collapse_policy: self.collapse_policy,
                        },
                    ))
                    .with_layout_capabilities(
                        crate::gui::layout_core::runtime_owned_split_pane_capabilities(
                            policy,
                            self.collapse_policy,
                        ),
                    );
            }
            SplitInteractionMode::Controlled => {
                split = split.with_split_pane_runtime_mode(Some(
                    crate::gui::layout_core::SplitPaneRuntimeMode::Controlled(
                        crate::gui::layout_core::Controlled::new(self.ratio, self.generation),
                    ),
                ));
            }
        }
        if self.clipped {
            UiSurface::new(SurfaceNode::container(
                99,
                ContainerPolicy {
                    kind: ContainerKind::ScrollView,
                    overflow: OverflowPolicy::Scroll,
                    ..ContainerPolicy::default()
                },
                vec![SurfaceChild::fill(split)],
            ))
        } else {
            UiSurface::new(split)
        }
    }
}

impl RuntimeBridge<()> for SplitInteractionBridge {
    fn project_surface(&mut self) -> Arc<UiSurface<()>> {
        crate::runtime::test_arc_surface(self.surface())
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
enum SplitSettledMessage {
    Settled { mapper: u8, ratio: f32 },
}

struct SettledSplitInteractionBridge {
    mode: SplitInteractionMode,
    policy: SplitPanePolicy,
    collapse_policy: Option<SplitPaneCollapsePolicy>,
    ratio: f32,
    generation: u64,
    mapper: u8,
    mounted: bool,
    messages: Rc<RefCell<Vec<SplitSettledMessage>>>,
}

impl SettledSplitInteractionBridge {
    fn new(mode: SplitInteractionMode, messages: Rc<RefCell<Vec<SplitSettledMessage>>>) -> Self {
        Self {
            mode,
            policy: SplitPanePolicy {
                axis: SplitPaneAxis::Horizontal,
                initial_ratio: 0.25,
                divider_extent: 8.0,
                first_min_extent: 0.0,
                second_min_extent: 0.0,
            },
            collapse_policy: None,
            ratio: 0.25,
            generation: 1,
            mapper: 1,
            mounted: true,
            messages,
        }
    }

    fn with_collapse_policy(mut self, collapse_policy: SplitPaneCollapsePolicy) -> Self {
        self.collapse_policy = Some(collapse_policy);
        self
    }

    fn with_axis(mut self, axis: SplitPaneAxis) -> Self {
        self.policy.axis = axis;
        self
    }

    fn surface(&self) -> UiSurface<SplitSettledMessage> {
        if !self.mounted {
            return UiSurface::new(SurfaceNode::widget(
                TextWidget::new(
                    90,
                    "unmounted",
                    WidgetSizing::fixed(Vector2::new(80.0, 20.0)),
                ),
                WidgetMessageMapper::none(),
            ));
        }
        let policy = self.policy;
        let first = SurfaceNode::widget(
            TextWidget::new(2, "first", WidgetSizing::fixed(Vector2::new(20.0, 20.0))),
            WidgetMessageMapper::none(),
        );
        let second = SurfaceNode::widget(
            TextWidget::new(3, "second", WidgetSizing::fixed(Vector2::new(20.0, 20.0))),
            WidgetMessageMapper::none(),
        );
        let mut split = SurfaceNode::container(
            1,
            ContainerPolicy {
                kind: ContainerKind::SplitPane,
                split_pane: policy,
                ..ContainerPolicy::default()
            },
            vec![SurfaceChild::fill(first), SurfaceChild::fill(second)],
        );
        match self.mode {
            SplitInteractionMode::Static => {}
            SplitInteractionMode::RuntimeOwned => {
                let mapper = self.mapper;
                split = split
                    .with_split_pane_runtime_mode(Some(
                        crate::gui::layout_core::SplitPaneRuntimeMode::RuntimeOwned {
                            collapse_policy: self.collapse_policy,
                        },
                    ))
                    .with_layout_capabilities(
                        crate::gui::layout_core::runtime_owned_split_pane_capabilities_with_ratio_settled(
                            policy,
                            self.collapse_policy,
                            Some(Rc::new(move |ratio| {
                                SplitSettledMessage::Settled { mapper, ratio }
                            })),
                        ),
                    );
            }
            SplitInteractionMode::Controlled => {
                split = split.with_split_pane_runtime_mode(Some(
                    crate::gui::layout_core::SplitPaneRuntimeMode::Controlled(
                        crate::gui::layout_core::Controlled::new(self.ratio, self.generation),
                    ),
                ));
            }
        }
        UiSurface::new(split)
    }
}

impl RuntimeBridge<SplitSettledMessage> for SettledSplitInteractionBridge {
    fn project_surface(&mut self) -> Arc<UiSurface<SplitSettledMessage>> {
        crate::runtime::test_arc_surface(self.surface())
    }

    fn reduce_message(&mut self, message: SplitSettledMessage) {
        self.messages.borrow_mut().push(message);
    }
}

#[derive(Clone, Copy)]
enum LayoutProbeRevision {
    Exact(&'static str),
    Conservative,
}

impl LayoutProbeRevision {
    fn evidence(self) -> LayoutInteractionRevision {
        match self {
            Self::Exact(value) => LayoutInteractionRevision::exact(value),
            Self::Conservative => LayoutInteractionRevision::conservative(),
        }
    }
}

#[derive(Default)]
struct LayoutProbeState {
    events: Vec<(LayoutTargetIdentity, LayoutInput)>,
    messages: Vec<u8>,
    handled: bool,
    capture_on_press: bool,
    capture_on_release: bool,
    emit_message_on_press: bool,
    emit_message_on_release: bool,
    repaint_on_move: bool,
    work_on_move: bool,
    widget_moves: usize,
}

struct LayoutProbeInteraction {
    state: Rc<RefCell<LayoutProbeState>>,
    revision: LayoutInteractionRevision,
    regions: Vec<LayoutHitRegion>,
}

impl LayoutInteraction<u8> for LayoutProbeInteraction {
    fn revision(&self) -> LayoutInteractionRevision {
        self.revision.clone()
    }

    fn visit_hit_regions(&self, _local_bounds: Rect, visitor: &mut dyn FnMut(LayoutHitRegion)) {
        for region in &self.regions {
            visitor(*region);
        }
    }

    fn handle_layout_input(
        &self,
        input: LayoutInput,
        context: &mut crate::layout::LayoutEventContext<u8>,
    ) {
        let (
            handled,
            capture_on_press,
            capture_on_release,
            emit_message_on_press,
            emit_message_on_release,
            repaint_on_move,
            work_on_move,
        ) = {
            let mut state = self.state.borrow_mut();
            state.events.push((context.target(), input));
            (
                state.handled,
                state.capture_on_press,
                state.capture_on_release,
                state.emit_message_on_press,
                state.emit_message_on_release,
                state.repaint_on_move,
                state.work_on_move,
            )
        };

        if handled {
            context.handle();
        }
        if capture_on_press && matches!(input, LayoutInput::PointerPress { .. }) {
            context.capture_pointer();
        }
        if capture_on_release && matches!(input, LayoutInput::PointerRelease { .. }) {
            context.capture_pointer();
        }
        if emit_message_on_press && matches!(input, LayoutInput::PointerPress { .. }) {
            assert!(context.emit_message(7));
        }
        if emit_message_on_release && matches!(input, LayoutInput::PointerRelease { .. }) {
            assert!(context.emit_message(7));
        }
        if matches!(input, LayoutInput::PointerMove { .. }) {
            if repaint_on_move {
                context.request_repaint();
            }
            if work_on_move {
                context.request_work();
            }
        }
    }
}

struct LayoutProbeBridge {
    state: Rc<RefCell<LayoutProbeState>>,
    revision: LayoutProbeRevision,
    contract_version: u16,
    visible: bool,
    scroll: bool,
    exclusive: bool,
    pass_through: bool,
    change_revision_on_message: bool,
}

impl LayoutProbeBridge {
    fn new(state: Rc<RefCell<LayoutProbeState>>) -> Self {
        Self {
            state,
            revision: LayoutProbeRevision::Exact("layout-probe"),
            contract_version: LAYOUT_CAPABILITIES_CONTRACT_VERSION,
            visible: true,
            scroll: false,
            exclusive: false,
            pass_through: false,
            change_revision_on_message: false,
        }
    }

    fn surface(&self) -> UiSurface<u8> {
        let regions = if self.visible {
            vec![
                LayoutHitRegion::new(
                    LayoutHitRegionId::new(1),
                    Rect::from_min_max(Point::new(0.5, 0.0), Point::new(1.0, 1.0)),
                )
                .expect("layout probe region should be valid"),
            ]
        } else {
            Vec::new()
        };
        let interaction = LayoutProbeInteraction {
            state: Rc::clone(&self.state),
            revision: self.revision.evidence(),
            regions,
        };
        let mut capabilities = LayoutCapabilities::new().interaction_local(interaction);
        capabilities.contract_version = self.contract_version;

        let root = if self.scroll {
            SurfaceNode::container(
                1,
                ContainerPolicy {
                    kind: ContainerKind::ScrollView,
                    overflow: OverflowPolicy::Scroll,
                    ..ContainerPolicy::default()
                },
                vec![SurfaceChild::fill(SurfaceNode::widget(
                    TextWidget::new(2, "wide", WidgetSizing::fixed(Vector2::new(300.0, 200.0))),
                    WidgetMessageMapper::none(),
                ))],
            )
        } else {
            let left_widget = if self.exclusive {
                SurfaceNode::widget(
                    DragHandleWidget::new(10, WidgetSizing::fixed(Vector2::new(100.0, 40.0))),
                    WidgetMessageMapper::none(),
                )
            } else if self.pass_through {
                SurfaceNode::widget(
                    PassThroughMoveWidget::new(10, Rc::clone(&self.state)),
                    WidgetMessageMapper::none(),
                )
            } else {
                SurfaceNode::widget(
                    TextInputWidget::new(
                        10,
                        "input",
                        WidgetSizing::fixed(Vector2::new(100.0, 40.0)),
                    ),
                    WidgetMessageMapper::none(),
                )
            };
            SurfaceNode::row(
                1,
                0.0,
                vec![
                    fixed_width_child(100.0, left_widget),
                    fixed_width_child(
                        100.0,
                        SurfaceNode::widget(
                            ButtonWidget::new(
                                20,
                                "button",
                                WidgetSizing::fixed(Vector2::new(100.0, 40.0)),
                            ),
                            WidgetMessageMapper::none(),
                        ),
                    ),
                ],
            )
        };
        UiSurface::new(root.with_layout_capabilities(capabilities))
    }
}

#[derive(Clone, Copy)]
enum LayoutStateShape {
    CellU32,
    TrackedU32,
    TrackedU64,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum LayoutStateTrace {
    Cancelled,
    DroppedU32,
    DroppedU64,
}

struct TrackedLayoutState {
    touches: u32,
    trace: Rc<RefCell<Vec<LayoutStateTrace>>>,
}

impl Drop for TrackedLayoutState {
    fn drop(&mut self) {
        self.trace.borrow_mut().push(LayoutStateTrace::DroppedU32);
    }
}

struct AlternateTrackedLayoutState {
    touches: u32,
    trace: Rc<RefCell<Vec<LayoutStateTrace>>>,
}

impl Drop for AlternateTrackedLayoutState {
    fn drop(&mut self) {
        self.trace.borrow_mut().push(LayoutStateTrace::DroppedU64);
    }
}

#[derive(Clone)]
struct LayoutStateProbeConfig {
    shape: LayoutStateShape,
    initialized: Rc<Cell<usize>>,
    cell_u32: Rc<Cell<u32>>,
    trace: Rc<RefCell<Vec<LayoutStateTrace>>>,
}

impl LayoutStateProbeConfig {
    fn declaration(&self, container_id: u64, schema_version: u16) -> ContainerStateDeclaration {
        match self.shape {
            LayoutStateShape::CellU32 => {
                let initialized = Rc::clone(&self.initialized);
                let value = Rc::clone(&self.cell_u32);
                ContainerStateDeclaration::new::<Rc<Cell<u32>>, _>(
                    container_id,
                    schema_version,
                    move || {
                        initialized.set(initialized.get().saturating_add(1));
                        Rc::clone(&value)
                    },
                )
            }
            LayoutStateShape::TrackedU32 => {
                let initialized = Rc::clone(&self.initialized);
                let trace = Rc::clone(&self.trace);
                ContainerStateDeclaration::new::<TrackedLayoutState, _>(
                    container_id,
                    schema_version,
                    move || {
                        initialized.set(initialized.get().saturating_add(1));
                        TrackedLayoutState {
                            touches: 0,
                            trace: Rc::clone(&trace),
                        }
                    },
                )
            }
            LayoutStateShape::TrackedU64 => {
                let initialized = Rc::clone(&self.initialized);
                let trace = Rc::clone(&self.trace);
                ContainerStateDeclaration::new::<AlternateTrackedLayoutState, _>(
                    container_id,
                    schema_version,
                    move || {
                        initialized.set(initialized.get().saturating_add(1));
                        AlternateTrackedLayoutState {
                            touches: 0,
                            trace: Rc::clone(&trace),
                        }
                    },
                )
            }
        }
    }

    fn touch(&self, context: &mut LayoutContainerStateContext<'_>) {
        match self.shape {
            LayoutStateShape::CellU32 => {
                let value = context
                    .state_mut::<Rc<Cell<u32>>>()
                    .expect("the state-aware callback should receive Rc<Cell<u32>>");
                value.set(value.get().saturating_add(1));
            }
            LayoutStateShape::TrackedU32 => {
                let state = context
                    .state_mut::<TrackedLayoutState>()
                    .expect("the state-aware callback should receive tracked u32 state");
                state.touches = state.touches.saturating_add(1);
            }
            LayoutStateShape::TrackedU64 => {
                let state = context
                    .state_mut::<AlternateTrackedLayoutState>()
                    .expect("the state-aware callback should receive tracked u64 state");
                state.touches = state.touches.saturating_add(1);
            }
        }
    }
}

struct LayoutStateProbeInteraction {
    events: Rc<RefCell<LayoutProbeState>>,
    config: LayoutStateProbeConfig,
    schema_version: u16,
    revision: LayoutInteractionRevision,
    declared_container_id: Option<crate::layout::NodeId>,
}

impl LayoutInteraction<u8> for LayoutStateProbeInteraction {
    fn revision(&self) -> LayoutInteractionRevision {
        self.revision.clone()
    }

    fn visit_hit_regions(&self, _local_bounds: Rect, visitor: &mut dyn FnMut(LayoutHitRegion)) {
        visitor(
            LayoutHitRegion::new(
                LayoutHitRegionId::new(1),
                Rect::from_min_max(Point::new(0.0, 0.0), Point::new(1.0, 1.0)),
            )
            .expect("state probe region should be valid"),
        );
    }

    fn state(&self, container_id: crate::layout::NodeId) -> Option<ContainerStateDeclaration> {
        Some(self.config.declaration(
            self.declared_container_id.unwrap_or(container_id),
            self.schema_version,
        ))
    }

    fn handle_layout_input_with_state(
        &self,
        input: LayoutInput,
        context: &mut crate::layout::LayoutEventContext<u8>,
        state: &mut LayoutContainerStateContext<'_>,
    ) {
        let (handled, capture_on_press) = {
            let mut events = self.events.borrow_mut();
            events.events.push((context.target(), input));
            (events.handled, events.capture_on_press)
        };
        if handled {
            context.handle();
        }
        if capture_on_press && matches!(input, LayoutInput::PointerPress { .. }) {
            context.capture_pointer();
        }
        if matches!(input, LayoutInput::PointerCaptureCancelled { .. }) {
            self.config
                .trace
                .borrow_mut()
                .push(LayoutStateTrace::Cancelled);
        }
        self.config.touch(state);
    }
}

struct LayoutStateProbeBridge {
    events: Rc<RefCell<LayoutProbeState>>,
    config: LayoutStateProbeConfig,
    schema_version: u16,
    revision: LayoutProbeRevision,
    contract_version: u16,
    mounted: bool,
}

impl LayoutStateProbeBridge {
    fn new(events: Rc<RefCell<LayoutProbeState>>, config: LayoutStateProbeConfig) -> Self {
        Self {
            events,
            config,
            schema_version: 1,
            revision: LayoutProbeRevision::Exact("state-probe"),
            contract_version: LAYOUT_CAPABILITIES_CONTRACT_VERSION,
            mounted: true,
        }
    }

    fn surface(&self) -> UiSurface<u8> {
        if !self.mounted {
            return UiSurface::new(SurfaceNode::widget(
                TextWidget::new(
                    99,
                    "unmounted",
                    WidgetSizing::fixed(Vector2::new(200.0, 40.0)),
                ),
                WidgetMessageMapper::none(),
            ));
        }
        let interaction = LayoutStateProbeInteraction {
            events: Rc::clone(&self.events),
            config: self.config.clone(),
            schema_version: self.schema_version,
            revision: self.revision.evidence(),
            declared_container_id: None,
        };
        let mut capabilities = LayoutCapabilities::new().interaction_local(interaction);
        capabilities.contract_version = self.contract_version;
        UiSurface::new(
            SurfaceNode::row(
                1,
                0.0,
                vec![fixed_width_child(
                    200.0,
                    SurfaceNode::widget(
                        TextWidget::new(
                            2,
                            "stateful layout probe",
                            WidgetSizing::fixed(Vector2::new(200.0, 40.0)),
                        ),
                        WidgetMessageMapper::none(),
                    ),
                )],
            )
            .with_layout_capabilities(capabilities),
        )
    }
}

impl RuntimeBridge<u8> for LayoutStateProbeBridge {
    fn project_surface(&mut self) -> Arc<UiSurface<u8>> {
        crate::runtime::test_arc_surface(self.surface())
    }

    fn reduce_message(&mut self, message: u8) {
        self.events.borrow_mut().messages.push(message);
    }
}

struct DualLayoutStateProbeBridge {
    left: LayoutStateProbeConfig,
    right: LayoutStateProbeConfig,
    left_events: Rc<RefCell<LayoutProbeState>>,
    right_events: Rc<RefCell<LayoutProbeState>>,
    foreign_state: bool,
}

impl DualLayoutStateProbeBridge {
    fn surface(&self) -> UiSurface<u8> {
        let (left_declared_container_id, right_declared_container_id) = if self.foreign_state {
            (Some(2), Some(1))
        } else {
            (None, None)
        };
        let left_interaction = LayoutStateProbeInteraction {
            events: Rc::clone(&self.left_events),
            config: self.left.clone(),
            schema_version: 1,
            revision: LayoutInteractionRevision::exact("left-state-probe"),
            declared_container_id: left_declared_container_id,
        };
        let right_interaction = LayoutStateProbeInteraction {
            events: Rc::clone(&self.right_events),
            config: self.right.clone(),
            schema_version: 1,
            revision: LayoutInteractionRevision::exact("right-state-probe"),
            declared_container_id: right_declared_container_id,
        };
        let left_capabilities = LayoutCapabilities::new().interaction_local(left_interaction);
        let right_capabilities = LayoutCapabilities::new().interaction_local(right_interaction);
        UiSurface::new(SurfaceNode::row(
            100,
            0.0,
            vec![
                fixed_width_child(
                    100.0,
                    SurfaceNode::container(
                        1,
                        ContainerPolicy::default(),
                        vec![SurfaceChild::fill(SurfaceNode::widget(
                            TextWidget::new(
                                101,
                                "left",
                                WidgetSizing::fixed(Vector2::new(100.0, 40.0)),
                            ),
                            WidgetMessageMapper::none(),
                        ))],
                    )
                    .with_layout_capabilities(left_capabilities),
                ),
                fixed_width_child(
                    100.0,
                    SurfaceNode::container(
                        2,
                        ContainerPolicy::default(),
                        vec![SurfaceChild::fill(SurfaceNode::widget(
                            TextWidget::new(
                                102,
                                "right",
                                WidgetSizing::fixed(Vector2::new(100.0, 40.0)),
                            ),
                            WidgetMessageMapper::none(),
                        ))],
                    )
                    .with_layout_capabilities(right_capabilities),
                ),
            ],
        ))
    }
}

impl RuntimeBridge<u8> for DualLayoutStateProbeBridge {
    fn project_surface(&mut self) -> Arc<UiSurface<u8>> {
        crate::runtime::test_arc_surface(self.surface())
    }
}

fn layout_state_probe_config(shape: LayoutStateShape) -> LayoutStateProbeConfig {
    LayoutStateProbeConfig {
        shape,
        initialized: Rc::new(Cell::new(0)),
        cell_u32: Rc::new(Cell::new(0)),
        trace: Rc::new(RefCell::new(Vec::new())),
    }
}

#[derive(Clone)]
struct PassThroughMoveWidget {
    common: WidgetCommon,
    state: Rc<RefCell<LayoutProbeState>>,
}

impl PassThroughMoveWidget {
    fn new(id: u64, state: Rc<RefCell<LayoutProbeState>>) -> Self {
        let mut common = WidgetCommon::fixed(id, 100.0, 40.0);
        common.focus = FocusBehavior::Pointer;
        Self { common, state }
    }
}

impl Widget for PassThroughMoveWidget {
    fn common(&self) -> &WidgetCommon {
        &self.common
    }

    fn common_mut(&mut self) -> &mut WidgetCommon {
        &mut self.common
    }

    fn handle_input(&mut self, _bounds: Rect, input: WidgetInput) -> Option<WidgetOutput> {
        match input {
            WidgetInput::PointerMove { .. } => {
                self.state.borrow_mut().widget_moves += 1;
                Some(WidgetOutput::typed(0))
            }
            WidgetInput::PointerPress { .. } | WidgetInput::PointerRelease { .. } => {
                Some(WidgetOutput::typed(0))
            }
            _ => None,
        }
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

impl RuntimeBridge<u8> for LayoutProbeBridge {
    fn project_surface(&mut self) -> Arc<UiSurface<u8>> {
        crate::runtime::test_arc_surface(self.surface())
    }

    fn pull_surface(&mut self) -> UiSurface<u8> {
        self.surface()
    }

    fn reduce_message(&mut self, message: u8) {
        self.state.borrow_mut().messages.push(message);
        if self.change_revision_on_message {
            self.revision = LayoutProbeRevision::Exact("changed-after-release");
        }
    }
}

#[test]
fn stateful_v4_layout_state_reuses_non_send_state_across_input_and_reprojection() {
    let events = Rc::new(RefCell::new(LayoutProbeState {
        handled: true,
        capture_on_press: true,
        ..LayoutProbeState::default()
    }));
    let config = layout_state_probe_config(LayoutStateShape::CellU32);
    let initialized = Rc::clone(&config.initialized);
    let value = Rc::clone(&config.cell_u32);
    let mut runtime = SurfaceRuntime::new(
        LayoutStateProbeBridge::new(Rc::clone(&events), config),
        Vector2::new(200.0, 40.0),
    );

    assert_eq!(runtime.layout_container_state_slot_count(), 1);
    assert_eq!(initialized.get(), 1);

    runtime.dispatch_event(Event::pointer_move(Point::new(150.0, 20.0)));
    assert_eq!(value.get(), 1);
    assert_eq!(runtime.layout_container_state_slot_count(), 1);

    runtime.dispatch_event(Event::primary_press(Point::new(150.0, 20.0)));
    assert_eq!(value.get(), 2);
    assert!(runtime.layout_pointer_capture().is_some());

    let counters = runtime.refresh_counters();
    let _ = runtime.take_repaint_requested();
    assert_eq!(
        runtime.take_pending_input_command_outcome(),
        CommandOutcome::default()
    );
    runtime.dispatch_event(Event::pointer_move(Point::new(240.0, 80.0)));
    assert_eq!(value.get(), 3);
    assert_eq!(runtime.refresh_counters(), counters);
    assert!(!runtime.repaint_requested());
    assert_eq!(
        runtime.take_pending_input_command_outcome(),
        CommandOutcome::default()
    );

    runtime.refresh();
    runtime.refresh_with_scope(RepaintScope::Projection);
    runtime.set_viewport(Vector2::new(240.0, 40.0));

    assert_eq!(initialized.get(), 1);
    assert_eq!(value.get(), 3);
    assert_eq!(runtime.layout_container_state_slot_count(), 1);
}

#[test]
fn same_state_type_and_schema_stay_independent_for_distinct_containers() {
    let left = layout_state_probe_config(LayoutStateShape::CellU32);
    let right = layout_state_probe_config(LayoutStateShape::CellU32);
    let left_value = Rc::clone(&left.cell_u32);
    let right_value = Rc::clone(&right.cell_u32);
    let left_events = Rc::new(RefCell::new(LayoutProbeState {
        handled: true,
        ..LayoutProbeState::default()
    }));
    let right_events = Rc::new(RefCell::new(LayoutProbeState {
        handled: true,
        ..LayoutProbeState::default()
    }));
    let mut runtime = SurfaceRuntime::new(
        DualLayoutStateProbeBridge {
            left,
            right,
            left_events,
            right_events,
            foreign_state: false,
        },
        Vector2::new(200.0, 40.0),
    );

    assert_eq!(runtime.layout_container_state_slot_count(), 2);
    runtime.dispatch_event(Event::pointer_move(Point::new(50.0, 20.0)));
    runtime.dispatch_event(Event::pointer_move(Point::new(150.0, 20.0)));

    assert_eq!(left_value.get(), 1);
    assert_eq!(right_value.get(), 1);
    assert_eq!(runtime.layout_container_state_slot_count(), 2);
}

#[test]
fn foreign_state_declarations_cannot_alias_or_retain_two_mounted_containers() {
    let left = layout_state_probe_config(LayoutStateShape::CellU32);
    let right = layout_state_probe_config(LayoutStateShape::CellU32);
    let left_initialized = Rc::clone(&left.initialized);
    let right_initialized = Rc::clone(&right.initialized);
    let left_events = Rc::new(RefCell::new(LayoutProbeState {
        handled: true,
        ..LayoutProbeState::default()
    }));
    let right_events = Rc::new(RefCell::new(LayoutProbeState {
        handled: true,
        ..LayoutProbeState::default()
    }));
    let mut runtime = SurfaceRuntime::new(
        DualLayoutStateProbeBridge {
            left,
            right,
            left_events,
            right_events,
            foreign_state: false,
        },
        Vector2::new(200.0, 40.0),
    );

    assert_eq!(left_initialized.get(), 1);
    assert_eq!(right_initialized.get(), 1);
    assert_eq!(runtime.layout_container_state_slot_count(), 2);

    runtime.bridge_mut().foreign_state = true;
    runtime.refresh();

    let diagnostics = runtime.last_refresh_diagnostics().layout_state;
    assert_eq!(diagnostics.foreign_declaration_count, 2);
    assert_eq!(diagnostics.dropped_count, 2);
    assert_eq!(diagnostics.initialized_count, 0);
    assert_eq!(diagnostics.replacement_count, 0);
    assert_eq!(runtime.layout_container_state_slot_count(), 0);
    assert!(
        runtime
            .traversal
            .containers
            .layout_targets
            .iter()
            .all(|target| target.state_id.is_none())
    );
    assert_eq!(left_initialized.get(), 1);
    assert_eq!(right_initialized.get(), 1);
}

#[test]
fn changed_state_schema_and_concrete_type_replace_once_with_bounded_evidence() {
    let events = Rc::new(RefCell::new(LayoutProbeState {
        handled: true,
        capture_on_press: true,
        ..LayoutProbeState::default()
    }));
    let config = layout_state_probe_config(LayoutStateShape::TrackedU32);
    let initialized = Rc::clone(&config.initialized);
    let trace = Rc::clone(&config.trace);
    let mut runtime = SurfaceRuntime::new(
        LayoutStateProbeBridge::new(Rc::clone(&events), config),
        Vector2::new(200.0, 40.0),
    );
    runtime.dispatch_event(Event::primary_press(Point::new(150.0, 20.0)));

    runtime.bridge_mut().schema_version = 2;
    runtime.refresh();
    assert_eq!(runtime.layout_pointer_capture(), None);
    assert_eq!(
        trace.borrow().as_slice(),
        &[LayoutStateTrace::Cancelled, LayoutStateTrace::DroppedU32]
    );
    assert_eq!(initialized.get(), 2);
    let schema_replacement = runtime.last_refresh_diagnostics().layout_state.replacements[0]
        .expect("schema replacement diagnostic");
    assert_eq!(schema_replacement.container_id, 1);
    assert_eq!(schema_replacement.previous.schema_version(), 1);
    assert_eq!(schema_replacement.current.schema_version(), 2);
    assert!(schema_replacement.previous.is::<TrackedLayoutState>());
    assert!(schema_replacement.current.is::<TrackedLayoutState>());

    runtime.refresh();
    assert_eq!(
        runtime
            .last_refresh_diagnostics()
            .layout_state
            .replacement_count,
        0
    );
    assert_eq!(
        runtime
            .last_refresh_diagnostics()
            .layout_state
            .initialized_count,
        0
    );
    assert_eq!(
        trace.borrow().as_slice(),
        &[LayoutStateTrace::Cancelled, LayoutStateTrace::DroppedU32]
    );

    runtime.bridge_mut().config.shape = LayoutStateShape::TrackedU64;
    runtime.refresh();
    assert_eq!(initialized.get(), 3);
    let type_replacement = runtime.last_refresh_diagnostics().layout_state.replacements[0]
        .expect("concrete type replacement diagnostic");
    assert!(type_replacement.previous.is::<TrackedLayoutState>());
    assert!(type_replacement.current.is::<AlternateTrackedLayoutState>());
    assert_eq!(
        trace.borrow().as_slice(),
        &[
            LayoutStateTrace::Cancelled,
            LayoutStateTrace::DroppedU32,
            LayoutStateTrace::DroppedU32,
        ]
    );

    runtime.bridge_mut().mounted = false;
    runtime.refresh();
    assert_eq!(runtime.layout_container_state_slot_count(), 0);
    assert_eq!(
        runtime
            .last_refresh_diagnostics()
            .layout_state
            .dropped_count,
        1
    );
    assert_eq!(
        trace.borrow().as_slice(),
        &[
            LayoutStateTrace::Cancelled,
            LayoutStateTrace::DroppedU32,
            LayoutStateTrace::DroppedU32,
            LayoutStateTrace::DroppedU64,
        ]
    );
}

#[test]
fn captured_container_removal_cancels_before_dropping_old_state_once() {
    let events = Rc::new(RefCell::new(LayoutProbeState {
        handled: true,
        capture_on_press: true,
        ..LayoutProbeState::default()
    }));
    let config = layout_state_probe_config(LayoutStateShape::TrackedU32);
    let trace = Rc::clone(&config.trace);
    let mut runtime = SurfaceRuntime::new(
        LayoutStateProbeBridge::new(events, config),
        Vector2::new(200.0, 40.0),
    );
    runtime.dispatch_event(Event::primary_press(Point::new(150.0, 20.0)));
    runtime.bridge_mut().mounted = false;
    runtime.refresh();

    assert_eq!(runtime.layout_pointer_capture(), None);
    assert_eq!(
        runtime
            .last_refresh_diagnostics()
            .layout_state
            .dropped_count,
        1
    );
    assert_eq!(
        trace.borrow().as_slice(),
        &[LayoutStateTrace::Cancelled, LayoutStateTrace::DroppedU32]
    );
    runtime.refresh();
    assert_eq!(
        trace.borrow().as_slice(),
        &[LayoutStateTrace::Cancelled, LayoutStateTrace::DroppedU32]
    );
}

#[test]
fn state_identity_and_contract_changes_cancel_captured_old_binding() {
    let schema_events = Rc::new(RefCell::new(LayoutProbeState {
        handled: true,
        capture_on_press: true,
        ..LayoutProbeState::default()
    }));
    let schema_config = layout_state_probe_config(LayoutStateShape::TrackedU32);
    let schema_trace = Rc::clone(&schema_config.trace);
    let mut schema_runtime = SurfaceRuntime::new(
        LayoutStateProbeBridge::new(schema_events, schema_config),
        Vector2::new(200.0, 40.0),
    );
    schema_runtime.dispatch_event(Event::primary_press(Point::new(150.0, 20.0)));
    schema_runtime.bridge_mut().schema_version = 2;
    schema_runtime.refresh();
    assert_eq!(schema_runtime.layout_pointer_capture(), None);
    assert_eq!(
        schema_trace.borrow().as_slice(),
        &[LayoutStateTrace::Cancelled, LayoutStateTrace::DroppedU32]
    );

    let contract_events = Rc::new(RefCell::new(LayoutProbeState {
        handled: true,
        capture_on_press: true,
        ..LayoutProbeState::default()
    }));
    let contract_config = layout_state_probe_config(LayoutStateShape::TrackedU32);
    let contract_trace = Rc::clone(&contract_config.trace);
    let mut contract_runtime = SurfaceRuntime::new(
        LayoutStateProbeBridge::new(contract_events, contract_config),
        Vector2::new(200.0, 40.0),
    );
    contract_runtime.dispatch_event(Event::primary_press(Point::new(150.0, 20.0)));
    contract_runtime.bridge_mut().contract_version = 3;
    contract_runtime.refresh();
    assert_eq!(contract_runtime.layout_pointer_capture(), None);
    assert_eq!(contract_runtime.layout_container_state_slot_count(), 0);
    assert_eq!(
        contract_trace.borrow().as_slice(),
        &[LayoutStateTrace::Cancelled, LayoutStateTrace::DroppedU32]
    );
}

#[test]
fn stateless_v4_and_legacy_v3_interactions_do_not_allocate_state() {
    let v4_state = Rc::new(RefCell::new(LayoutProbeState {
        handled: true,
        ..LayoutProbeState::default()
    }));
    let mut v4_runtime = SurfaceRuntime::new(
        LayoutProbeBridge::new(Rc::clone(&v4_state)),
        Vector2::new(200.0, 40.0),
    );
    assert_eq!(v4_runtime.layout_container_state_slot_count(), 0);
    v4_runtime.refresh();
    assert_eq!(v4_runtime.layout_container_state_slot_count(), 0);
    assert_eq!(
        v4_runtime.last_refresh_diagnostics().layout_state,
        Default::default()
    );

    let v3_state = Rc::new(RefCell::new(LayoutProbeState {
        handled: true,
        capture_on_press: true,
        ..LayoutProbeState::default()
    }));
    let mut v3_runtime = SurfaceRuntime::new(
        LayoutProbeBridge {
            contract_version: 3,
            ..LayoutProbeBridge::new(Rc::clone(&v3_state))
        },
        Vector2::new(200.0, 40.0),
    );
    v3_runtime.dispatch_event(Event::primary_press(Point::new(150.0, 20.0)));
    assert!(v3_runtime.layout_pointer_capture().is_some());
    assert_eq!(v3_runtime.layout_container_state_slot_count(), 0);
    assert!(
        v3_state
            .borrow()
            .events
            .iter()
            .any(|(_, input)| matches!(input, LayoutInput::PointerPress { .. }))
    );
}

#[test]
fn layout_input_is_offered_before_widget_and_unhandled_input_falls_back() {
    let state = Rc::new(RefCell::new(LayoutProbeState {
        handled: true,
        emit_message_on_press: true,
        ..LayoutProbeState::default()
    }));
    let mut runtime = SurfaceRuntime::new(
        LayoutProbeBridge::new(Rc::clone(&state)),
        Vector2::new(200.0, 40.0),
    );
    let left = Point::new(20.0, 20.0);
    let right = Point::new(150.0, 20.0);
    let timestamp = Some(InputTimestamp::capture());
    let modifiers = PointerModifiers {
        command: true,
        shift: true,
        alt: false,
    };

    assert_eq!(runtime.dispatch_primary_click(left).press_target, Some(10));
    assert_eq!(runtime.focused_widget(), Some(10));
    assert_eq!(
        runtime.dispatch_event(Event::pointer_press_with_timestamp(
            right,
            PointerButton::Primary,
            modifiers,
            timestamp,
        )),
        None,
        "handled layout input must prevent widget fallback"
    );
    assert_eq!(runtime.pointer_capture(), None);
    assert_eq!(runtime.layout_pointer_capture(), None);
    assert_eq!(runtime.focused_widget(), Some(10));
    assert_eq!(runtime.bridge().state.borrow().messages, vec![7]);
    assert_eq!(
        runtime.bridge().state.borrow().events,
        vec![(
            LayoutTargetIdentity::new(1, LayoutHitRegionId::new(1)),
            LayoutInput::PointerPress {
                position: right,
                button: PointerButton::Primary,
                modifiers,
                timestamp,
            },
        )]
    );

    runtime.bridge().state.borrow_mut().handled = false;
    assert_eq!(
        runtime.dispatch_event(Event::primary_press(right)),
        Some(20),
        "an unhandled layout event must preserve widget fallback"
    );
    assert_eq!(runtime.pointer_capture(), Some(20));
}

#[test]
fn layout_capture_is_exclusive_and_delivers_metadata_outside_bounds_without_refresh() {
    let state = Rc::new(RefCell::new(LayoutProbeState {
        handled: true,
        capture_on_press: true,
        ..LayoutProbeState::default()
    }));
    let mut runtime = SurfaceRuntime::new(
        LayoutProbeBridge::new(Rc::clone(&state)),
        Vector2::new(200.0, 40.0),
    );
    let target = LayoutTargetIdentity::new(1, LayoutHitRegionId::new(1));
    let press_timestamp = Some(InputTimestamp::capture());
    assert_eq!(
        runtime.dispatch_event(Event::pointer_press_with_timestamp(
            Point::new(150.0, 20.0),
            PointerButton::Primary,
            PointerModifiers::default(),
            press_timestamp,
        )),
        None
    );
    assert_eq!(runtime.layout_pointer_capture(), Some(target));
    assert_eq!(runtime.pointer_capture(), None);

    let counters = runtime.refresh_counters();
    let repaint_requested = runtime.repaint_requested();
    let modifiers = PointerModifiers {
        command: true,
        shift: false,
        alt: true,
    };
    let timestamp = Some(InputTimestamp::capture());
    let sequence_range = Some(InputSequenceRange::singleton(
        InputSequence::from_runtime_value(71),
    ));
    for position in [Point::new(-20.0, 20.0), Point::new(240.0, 80.0)] {
        assert_eq!(
            runtime.dispatch_event(Event::pointer_move_with_metadata(
                position,
                modifiers,
                timestamp,
                sequence_range,
            )),
            None
        );
    }
    assert_eq!(
        runtime.dispatch_event(Event::pointer_release_with_timestamp(
            Point::new(240.0, 80.0),
            PointerButton::Primary,
            modifiers,
            timestamp,
        )),
        None
    );

    let events = &runtime.bridge().state.borrow().events;
    assert!(matches!(events[0].1, LayoutInput::PointerPress { .. }));
    assert_eq!(
        events
            .iter()
            .filter_map(|(_, input)| match input {
                LayoutInput::PointerMove {
                    position,
                    modifiers: input_modifiers,
                    timestamp: input_timestamp,
                    sequence_range: input_sequence_range,
                } => Some((
                    *position,
                    *input_modifiers,
                    *input_timestamp,
                    *input_sequence_range
                )),
                _ => None,
            })
            .collect::<Vec<_>>(),
        vec![
            (
                Point::new(-20.0, 20.0),
                modifiers,
                timestamp,
                sequence_range
            ),
            (
                Point::new(240.0, 80.0),
                modifiers,
                timestamp,
                sequence_range
            ),
        ]
    );
    assert!(matches!(events[3].1, LayoutInput::PointerRelease { .. }));
    assert_eq!(runtime.layout_pointer_capture(), None);
    assert_eq!(runtime.refresh_counters(), counters);
    assert_eq!(runtime.repaint_requested(), repaint_requested);
}

#[test]
fn fresh_layout_release_cannot_start_a_new_capture() {
    let state = Rc::new(RefCell::new(LayoutProbeState {
        handled: true,
        capture_on_release: true,
        ..LayoutProbeState::default()
    }));
    let mut runtime = SurfaceRuntime::new(
        LayoutProbeBridge::new(Rc::clone(&state)),
        Vector2::new(200.0, 40.0),
    );

    assert_eq!(
        runtime.dispatch_event(Event::pointer_release(
            Point::new(150.0, 20.0),
            PointerButton::Primary,
            PointerModifiers::default(),
        )),
        None
    );
    assert_eq!(runtime.layout_pointer_capture(), None);
    assert!(matches!(
        state.borrow().events.as_slice(),
        [(_, LayoutInput::PointerRelease { .. })]
    ));
}

#[test]
fn captured_layout_release_clears_capture_before_refreshing_message() {
    let state = Rc::new(RefCell::new(LayoutProbeState {
        handled: true,
        capture_on_press: true,
        emit_message_on_release: true,
        ..LayoutProbeState::default()
    }));
    let mut runtime = SurfaceRuntime::new(
        LayoutProbeBridge {
            change_revision_on_message: true,
            ..LayoutProbeBridge::new(Rc::clone(&state))
        },
        Vector2::new(200.0, 40.0),
    );

    assert_eq!(
        runtime.dispatch_event(Event::primary_press(Point::new(150.0, 20.0))),
        None
    );
    assert!(runtime.layout_pointer_capture().is_some());

    assert_eq!(
        runtime.dispatch_event(Event::pointer_release(
            Point::new(240.0, 80.0),
            PointerButton::Primary,
            PointerModifiers::default(),
        )),
        None
    );
    assert_eq!(runtime.layout_pointer_capture(), None);
    let events = state.borrow().events.clone();
    assert_eq!(
        events
            .iter()
            .filter(|(_, input)| matches!(input, LayoutInput::PointerRelease { .. }))
            .count(),
        1
    );
    assert_eq!(
        events
            .iter()
            .filter(|(_, input)| matches!(input, LayoutInput::PointerCaptureCancelled { .. }))
            .count(),
        0
    );
    assert_eq!(state.borrow().messages, vec![7]);
}

#[test]
fn widget_capture_precedes_fresh_layout_and_scrollbar_capture_precedes_layout() {
    let state = Rc::new(RefCell::new(LayoutProbeState {
        handled: true,
        capture_on_press: true,
        ..LayoutProbeState::default()
    }));
    let mut runtime = SurfaceRuntime::new(
        LayoutProbeBridge {
            exclusive: true,
            ..LayoutProbeBridge::new(Rc::clone(&state))
        },
        Vector2::new(200.0, 40.0),
    );
    assert_eq!(runtime.widget_at(Point::new(20.0, 20.0)), Some(10));
    assert_eq!(
        runtime.dispatch_event(Event::primary_press(Point::new(20.0, 20.0))),
        Some(10)
    );
    assert_eq!(runtime.pointer_capture(), Some(10));
    assert_eq!(
        runtime.dispatch_event(Event::pointer_move(Point::new(150.0, 20.0))),
        Some(10)
    );
    assert_eq!(runtime.pointer_capture(), Some(10));
    assert_eq!(runtime.bridge().state.borrow().events, Vec::new());

    let pass_through_state = Rc::new(RefCell::new(LayoutProbeState {
        handled: true,
        capture_on_press: true,
        ..LayoutProbeState::default()
    }));
    let mut pass_through_runtime = SurfaceRuntime::new(
        LayoutProbeBridge {
            pass_through: true,
            ..LayoutProbeBridge::new(Rc::clone(&pass_through_state))
        },
        Vector2::new(200.0, 40.0),
    );
    assert_eq!(
        pass_through_runtime.dispatch_event(Event::primary_press(Point::new(20.0, 20.0))),
        Some(10)
    );
    assert_eq!(pass_through_runtime.pointer_capture(), Some(10));
    let _ = pass_through_runtime.dispatch_event(Event::pointer_move(Point::new(150.0, 20.0)));
    assert_eq!(pass_through_runtime.pointer_capture(), Some(10));
    assert_eq!(pass_through_state.borrow().widget_moves, 1);
    assert!(pass_through_state.borrow().events.is_empty());

    let press_state = Rc::new(RefCell::new(LayoutProbeState {
        handled: true,
        capture_on_press: true,
        ..LayoutProbeState::default()
    }));
    let mut press_runtime = SurfaceRuntime::new(
        LayoutProbeBridge {
            exclusive: true,
            ..LayoutProbeBridge::new(Rc::clone(&press_state))
        },
        Vector2::new(200.0, 40.0),
    );
    assert_eq!(
        press_runtime.dispatch_event(Event::primary_press(Point::new(20.0, 20.0))),
        Some(10)
    );
    assert_eq!(press_runtime.pointer_capture(), Some(10));
    let _ = press_runtime.dispatch_event(Event::primary_press(Point::new(150.0, 20.0)));
    assert_eq!(press_runtime.pointer_capture(), Some(10));
    assert!(press_state.borrow().events.is_empty());

    let double_click_state = Rc::new(RefCell::new(LayoutProbeState {
        handled: true,
        capture_on_press: true,
        ..LayoutProbeState::default()
    }));
    let mut double_click_runtime = SurfaceRuntime::new(
        LayoutProbeBridge {
            exclusive: true,
            ..LayoutProbeBridge::new(Rc::clone(&double_click_state))
        },
        Vector2::new(200.0, 40.0),
    );
    assert_eq!(
        double_click_runtime.dispatch_event(Event::primary_press(Point::new(20.0, 20.0))),
        Some(10)
    );
    assert_eq!(double_click_runtime.pointer_capture(), Some(10));
    let _ =
        double_click_runtime.dispatch_event(Event::primary_double_click(Point::new(150.0, 20.0)));
    assert_eq!(double_click_runtime.pointer_capture(), Some(10));
    assert!(double_click_state.borrow().events.is_empty());

    let scrollbar_state = Rc::new(RefCell::new(LayoutProbeState {
        handled: true,
        capture_on_press: true,
        ..LayoutProbeState::default()
    }));
    let mut scrollbar_runtime = SurfaceRuntime::new(
        LayoutProbeBridge {
            scroll: true,
            ..LayoutProbeBridge::new(Rc::clone(&scrollbar_state))
        },
        Vector2::new(100.0, 50.0),
    );
    let scrollbar_point = (0..100)
        .flat_map(|x| (0..50).map(move |y| Point::new(x as f32 + 0.5, y as f32 + 0.5)))
        .find(|point| scrollbar_runtime.scroll_affordance_at(*point).is_some())
        .expect("overflow surface should expose a scrollbar thumb");
    assert_eq!(
        scrollbar_runtime.dispatch_event(Event::primary_press(scrollbar_point)),
        None
    );
    assert!(scrollbar_runtime.scrollbar_drag_active());
    assert_eq!(scrollbar_runtime.layout_pointer_capture(), None);
    assert!(scrollbar_state.borrow().events.is_empty());
}

#[test]
fn layout_capture_rebinds_only_on_exact_revision_and_cancels_once_otherwise() {
    let state = Rc::new(RefCell::new(LayoutProbeState {
        handled: true,
        capture_on_press: true,
        ..LayoutProbeState::default()
    }));
    let mut runtime = SurfaceRuntime::new(
        LayoutProbeBridge::new(Rc::clone(&state)),
        Vector2::new(200.0, 40.0),
    );
    runtime.dispatch_event(Event::primary_press(Point::new(150.0, 20.0)));
    assert!(runtime.layout_pointer_capture().is_some());

    runtime.refresh();
    assert!(runtime.layout_pointer_capture().is_some());
    assert_eq!(
        state
            .borrow()
            .events
            .iter()
            .filter(|(_, input)| matches!(input, LayoutInput::PointerCaptureCancelled { .. }))
            .count(),
        0
    );

    runtime.bridge_mut().revision = LayoutProbeRevision::Exact("changed");
    runtime.refresh();
    assert_eq!(runtime.layout_pointer_capture(), None);
    assert_eq!(
        state
            .borrow()
            .events
            .iter()
            .filter(|(_, input)| matches!(input, LayoutInput::PointerCaptureCancelled { .. }))
            .count(),
        1
    );
    runtime.refresh();
    assert_eq!(
        state
            .borrow()
            .events
            .iter()
            .filter(|(_, input)| matches!(input, LayoutInput::PointerCaptureCancelled { .. }))
            .count(),
        1
    );

    let conservative_state = Rc::new(RefCell::new(LayoutProbeState {
        handled: true,
        capture_on_press: true,
        ..LayoutProbeState::default()
    }));
    let mut conservative_runtime = SurfaceRuntime::new(
        LayoutProbeBridge::new(Rc::clone(&conservative_state)),
        Vector2::new(200.0, 40.0),
    );
    conservative_runtime.dispatch_event(Event::primary_press(Point::new(150.0, 20.0)));
    conservative_runtime.bridge_mut().revision = LayoutProbeRevision::Conservative;
    conservative_runtime.refresh();
    assert_eq!(conservative_runtime.layout_pointer_capture(), None);
    assert_eq!(
        conservative_state
            .borrow()
            .events
            .iter()
            .filter(|(_, input)| matches!(input, LayoutInput::PointerCaptureCancelled { .. }))
            .count(),
        1
    );
}

#[test]
fn projection_only_version_two_remains_query_only_for_pointer_routing() {
    let state = Rc::new(RefCell::new(LayoutProbeState {
        handled: true,
        ..LayoutProbeState::default()
    }));
    let mut runtime = SurfaceRuntime::new(
        LayoutProbeBridge {
            contract_version: LAYOUT_CAPABILITIES_PROJECTION_CONTRACT_VERSION,
            ..LayoutProbeBridge::new(Rc::clone(&state))
        },
        Vector2::new(200.0, 40.0),
    );
    assert!(runtime.layout_target_at(Point::new(150.0, 20.0)).is_some());
    assert_eq!(
        runtime.dispatch_event(Event::primary_press(Point::new(150.0, 20.0))),
        Some(20)
    );
    assert!(state.borrow().events.is_empty());
    assert_eq!(runtime.layout_pointer_capture(), None);
}

#[test]
fn pointer_events_feed_latest_position_to_update_snapshot() {
    let mut runtime =
        SurfaceRuntime::new(PointerSnapshotBridge::default(), Vector2::new(200.0, 80.0));

    runtime.dispatch_event(Event::pointer_move(Point::new(3.0, 4.0)));
    runtime.dispatch_message(());
    runtime.dispatch_event(Event::primary_press(Point::new(9.0, 10.0)));
    runtime.dispatch_message(());
    runtime.dispatch_event(Event::scroll(
        Point::new(11.0, 12.0),
        Vector2::new(0.0, 16.0),
    ));
    runtime.dispatch_message(());

    assert_eq!(
        runtime.bridge().snapshots,
        vec![
            Some(Point::new(3.0, 4.0)),
            Some(Point::new(9.0, 10.0)),
            Some(Point::new(11.0, 12.0)),
        ]
    );
}

#[test]
fn pointer_press_skips_stacked_widgets_that_reject_press_input() {
    let mut runtime = SurfaceRuntime::new(PointerPolicyStackBridge, Vector2::new(200.0, 80.0));
    let point = Point::new(40.0, 30.0);

    assert_eq!(
        runtime.dispatch_event(Event::primary_press(point)),
        Some(10)
    );
    assert_eq!(runtime.pointer_capture(), Some(10));

    let mut double_click_runtime =
        SurfaceRuntime::new(PointerPolicyStackBridge, Vector2::new(200.0, 80.0));
    assert_eq!(
        double_click_runtime.dispatch_event(Event::primary_double_click(point)),
        Some(10)
    );
    assert_eq!(double_click_runtime.pointer_capture(), Some(10));
}

#[test]
fn synthetic_double_click_preserves_timestamp_through_widget_dispatch() {
    let timestamp = Some(InputTimestamp::capture());
    let point = Point::new(40.0, 20.0);
    let mut runtime = SurfaceRuntime::new(
        DoubleClickTimestampBridge {
            handles_double_click: true,
            ..DoubleClickTimestampBridge::default()
        },
        Vector2::new(120.0, 40.0),
    );

    assert_eq!(
        runtime.dispatch_event(Event::pointer_double_click_with_timestamp(
            point,
            PointerButton::Primary,
            PointerModifiers::default(),
            timestamp,
        )),
        Some(40)
    );
    assert_eq!(
        runtime.bridge().messages,
        vec![DoubleClickTimestampMessage::DoubleClick(timestamp)]
    );
}

#[test]
fn synthetic_double_click_fallback_preserves_timestamp_on_pointer_press() {
    let timestamp = Some(InputTimestamp::capture());
    let point = Point::new(40.0, 20.0);
    let mut runtime = SurfaceRuntime::new(
        DoubleClickTimestampBridge::default(),
        Vector2::new(120.0, 40.0),
    );

    assert_eq!(
        runtime.dispatch_event(Event::pointer_double_click_with_timestamp(
            point,
            PointerButton::Primary,
            PointerModifiers::default(),
            timestamp,
        )),
        Some(40)
    );
    assert_eq!(
        runtime.bridge().messages,
        vec![DoubleClickTimestampMessage::Press(timestamp)]
    );
}

#[test]
fn internal_modifier_timestamp_survives_event_to_widget_dispatch() {
    let timestamp = Some(InputTimestamp::capture());
    let point = Point::new(40.0, 20.0);
    let mut runtime = SurfaceRuntime::new(
        DoubleClickTimestampBridge::default(),
        Vector2::new(120.0, 40.0),
    );

    assert_eq!(
        runtime.dispatch_event(Event::primary_press(point)),
        Some(40)
    );
    assert_eq!(
        runtime.dispatch_event(Event::pointer_modifiers_changed_with_timestamp(
            PointerModifiers {
                shift: true,
                ..PointerModifiers::default()
            },
            timestamp,
        )),
        Some(40)
    );
    assert_eq!(
        runtime.bridge().messages,
        vec![
            DoubleClickTimestampMessage::Press(None),
            DoubleClickTimestampMessage::Modifiers(timestamp),
        ]
    );
}

#[test]
fn internal_pointer_move_metadata_survives_event_to_widget_dispatch() {
    let timestamp = Some(InputTimestamp::capture());
    let modifiers = PointerModifiers {
        command: true,
        shift: true,
        alt: true,
    };
    let point = Point::new(40.0, 20.0);
    let sequence_range = Some(InputSequenceRange::singleton(
        InputSequence::from_runtime_value(7),
    ));
    let mut runtime = SurfaceRuntime::new(
        DoubleClickTimestampBridge::default(),
        Vector2::new(120.0, 40.0),
    );

    assert_eq!(
        runtime.dispatch_event(Event::pointer_move_with_metadata(
            point,
            modifiers,
            timestamp,
            sequence_range,
        )),
        Some(40)
    );
    assert_eq!(
        runtime.bridge().messages,
        vec![DoubleClickTimestampMessage::Move {
            modifiers,
            timestamp,
            sequence_range,
        }]
    );
}

#[test]
fn internal_scroll_metadata_survives_event_to_widget_dispatch() {
    let timestamp = Some(InputTimestamp::capture());
    let modifiers = PointerModifiers {
        command: true,
        shift: true,
        alt: true,
    };
    let point = Point::new(40.0, 20.0);
    let delta = Vector2::new(0.0, -24.0);
    let sequence_range = Some(InputSequenceRange::singleton(
        InputSequence::from_runtime_value(11),
    ));
    let mut runtime = SurfaceRuntime::new(
        DoubleClickTimestampBridge::default(),
        Vector2::new(120.0, 40.0),
    );

    assert_eq!(
        runtime.dispatch_event(Event::scroll_with_metadata(
            point,
            delta,
            modifiers,
            timestamp,
            sequence_range,
        )),
        None
    );
    assert_eq!(
        runtime.bridge().messages,
        vec![DoubleClickTimestampMessage::Wheel {
            position: point,
            delta,
            modifiers,
            timestamp,
            sequence_range,
        }]
    );
}

#[test]
fn pointer_move_fanout_preserves_one_sample_metadata_for_hover_capture_and_pass_through() {
    let first_point = Point::new(8.0, 8.0);
    let second_point = Point::new(8.0, 36.0);
    let modifiers = PointerModifiers {
        command: true,
        shift: false,
        alt: true,
    };
    let timestamp = Some(InputTimestamp::capture());
    let mut runtime = SurfaceRuntime::new(MoveFanoutBridge::default(), Vector2::new(160.0, 56.0));

    assert_eq!(
        runtime
            .dispatch_pointer_move_with_outcome(first_point)
            .target,
        Some(50)
    );
    assert_eq!(runtime.widget_at(second_point), Some(60));
    runtime.bridge_mut().samples.clear();
    assert_eq!(
        runtime.dispatch_event(Event::primary_press(first_point)),
        Some(50)
    );

    assert_eq!(
        runtime.dispatch_event(Event::pointer_move_with_metadata(
            second_point,
            modifiers,
            timestamp,
            None,
        )),
        Some(50)
    );

    let samples = &runtime.bridge().samples;
    assert_eq!(samples.len(), 3);
    assert_eq!(
        samples
            .iter()
            .map(|(widget_id, ..)| *widget_id)
            .collect::<Vec<_>>(),
        vec![50, 60, 50]
    );
    assert!(
        samples
            .iter()
            .all(|(_, sample_modifiers, sample_timestamp)| {
                *sample_modifiers == modifiers && *sample_timestamp == timestamp
            })
    );
}

#[test]
fn focus_loss_veto_retains_owner_without_focus_events_or_host_output() {
    let mut runtime = SurfaceRuntime::new(FocusDecisionBridge::new(), Vector2::new(200.0, 80.0));

    assert!(runtime.focus_widget(10));
    runtime.take_repaint_requested();
    runtime.bridge().events.borrow_mut().clear();
    runtime
        .bridge_mut()
        .old_decision
        .set(FocusLossDecision::Veto);

    assert!(runtime.focus_widget(20));

    assert_eq!(runtime.focused_widget(), Some(10));
    assert_eq!(
        runtime.bridge().events.borrow().as_slice(),
        [FocusDecisionEvent::Prepare(10)]
    );
    assert!(runtime.repaint_requested());
    assert!(
        runtime
            .surface()
            .find_widget(10)
            .expect("vetoing widget")
            .widget()
            .common()
            .state
            .focused
    );
    assert!(
        !runtime
            .surface()
            .find_widget(20)
            .expect("focus target")
            .widget()
            .common()
            .state
            .focused
    );
}

#[test]
fn clear_focus_veto_retains_owner_without_focus_events() {
    let mut runtime = SurfaceRuntime::new(FocusDecisionBridge::new(), Vector2::new(200.0, 80.0));

    assert!(runtime.focus_widget(10));
    runtime.take_repaint_requested();
    runtime.bridge().events.borrow_mut().clear();
    runtime
        .bridge_mut()
        .old_decision
        .set(FocusLossDecision::Veto);

    runtime.clear_focus();

    assert_eq!(runtime.focused_widget(), Some(10));
    assert_eq!(
        runtime.bridge().events.borrow().as_slice(),
        [FocusDecisionEvent::Prepare(10)]
    );
    assert!(runtime.repaint_requested());
}

#[test]
fn focus_loss_allow_commits_owner_before_focus_loss_output_reprojection() {
    let mut runtime = SurfaceRuntime::new(FocusDecisionBridge::new(), Vector2::new(200.0, 80.0));

    assert!(runtime.focus_widget(10));
    runtime.bridge().events.borrow_mut().clear();

    assert!(runtime.focus_widget(20));

    assert_eq!(runtime.focused_widget(), Some(20));
    assert_eq!(
        runtime.bridge().events.borrow().as_slice(),
        [
            FocusDecisionEvent::Prepare(10),
            FocusDecisionEvent::Changed(10, false),
            FocusDecisionEvent::HostOutput,
            FocusDecisionEvent::Changed(20, true),
        ]
    );
    assert!(
        !runtime
            .surface()
            .find_widget(10)
            .expect("previous widget")
            .widget()
            .common()
            .state
            .focused
    );
    assert!(
        runtime
            .surface()
            .find_widget(20)
            .expect("new focus owner")
            .widget()
            .common()
            .state
            .focused
    );
}

#[test]
fn exact_current_runtime_owned_separator_admits_one_private_owner() {
    let mut runtime = SurfaceRuntime::new(
        SplitInteractionBridge::new(SplitInteractionMode::RuntimeOwned),
        Vector2::new(200.0, 80.0),
    );
    let projection = runtime
        .split_pane_separator_projections()
        .first()
        .copied()
        .expect("current runtime-owned separator projection");

    assert_eq!(
        runtime.request_split_pane_separator_focus(projection),
        FocusTransition::Changed
    );
    assert!(matches!(
        runtime.interaction.focus.owner,
        Some(RuntimeFocusOwner::SplitPaneSeparator(_))
    ));
    assert_eq!(runtime.focused_widget(), None);
    assert_eq!(
        runtime.request_split_pane_separator_focus(projection),
        FocusTransition::Unchanged
    );
}

#[test]
fn primary_separator_press_allows_transfer_before_layout_capture_and_keeps_public_focus_widget_only()
 {
    let mut runtime =
        SurfaceRuntime::new(FocusDecisionSplitBridge::new(), Vector2::new(200.0, 160.0));
    assert!(runtime.focus_widget(10));
    runtime.bridge().inner.events.borrow_mut().clear();
    runtime.take_repaint_requested();
    let projection = runtime
        .split_pane_separator_projections()
        .first()
        .copied()
        .expect("current runtime-owned separator projection");
    let position = projection.divider_bounds.center();

    assert_eq!(runtime.dispatch_event(Event::primary_press(position)), None);
    assert_eq!(runtime.layout_pointer_capture(), Some(projection.target));
    assert_eq!(runtime.pointer_capture(), None);
    assert_eq!(runtime.focused_widget(), None);
    assert_eq!(
        runtime.bridge().inner.events.borrow().as_slice(),
        [
            FocusDecisionEvent::Prepare(10),
            FocusDecisionEvent::Changed(10, false),
            FocusDecisionEvent::HostOutput,
        ]
    );
    assert_eq!(runtime.traversal.widgets.keyboard_focus.order(), &[10]);
    assert_eq!(runtime.traverse_focus(FocusTraversal::Forward), Some(10));
    assert_eq!(runtime.focused_widget(), Some(10));
}

#[test]
fn separator_pointer_focus_loss_command_preserves_independent_widget_owner() {
    let mut runtime = SurfaceRuntime::new(
        FocusDecisionSplitBridge::new().with_focus_widget_20_on_host_output(),
        Vector2::new(200.0, 160.0),
    );
    assert!(runtime.focus_widget(10));
    runtime.bridge().inner.events.borrow_mut().clear();
    runtime.take_repaint_requested();
    let before_projection = runtime
        .split_pane_separator_projections()
        .first()
        .copied()
        .expect("current runtime-owned separator projection");
    let before_state = runtime
        .interaction
        .layout_state
        .lookup_current_state_view(before_projection.mounted_state_id)
        .and_then(|read| read.downcast_ref::<SplitPaneRuntimeState>().copied())
        .expect("runtime-owned split state remains mounted");
    let before_layout = runtime.layout().rects.clone();
    let before_counters = runtime.refresh_counters();
    let position = before_projection.divider_bounds.center();

    assert_eq!(runtime.dispatch_event(Event::primary_press(position)), None);

    assert_eq!(
        runtime.interaction.focus.owner,
        Some(RuntimeFocusOwner::Widget(20))
    );
    assert_eq!(runtime.focused_widget(), Some(20));
    assert!(
        runtime
            .surface()
            .find_widget(20)
            .expect("independent focus target")
            .widget()
            .common()
            .state
            .focused
    );
    assert_eq!(runtime.pointer_capture(), None);
    assert_eq!(runtime.layout_pointer_capture(), None);
    assert_eq!(runtime.layout().rects, before_layout);
    assert_eq!(
        runtime.split_pane_separator_projections().first().copied(),
        Some(before_projection)
    );
    assert_eq!(
        runtime
            .interaction
            .layout_state
            .lookup_current_state_view(before_projection.mounted_state_id)
            .and_then(|read| read.downcast_ref::<SplitPaneRuntimeState>().copied()),
        Some(before_state)
    );
    assert_eq!(
        runtime.refresh_counters().layout,
        before_counters.layout + 1
    );
    assert_eq!(
        runtime.bridge().inner.events.borrow().as_slice(),
        [
            FocusDecisionEvent::Prepare(10),
            FocusDecisionEvent::Changed(10, false),
            FocusDecisionEvent::HostOutput,
            FocusDecisionEvent::Changed(20, true),
        ]
    );
}

#[test]
fn separator_pointer_double_click_focus_loss_command_preserves_actionable_split() {
    let mut runtime = SurfaceRuntime::new(
        FocusDecisionSplitBridge::new()
            .with_focus_widget_20_on_host_output()
            .with_collapse_policy(SplitPaneCollapsePolicy::FirstPane),
        Vector2::new(200.0, 160.0),
    );
    assert!(runtime.focus_widget(10));
    runtime.bridge().inner.events.borrow_mut().clear();
    runtime.take_repaint_requested();
    let before_projection = runtime
        .split_pane_separator_projections()
        .first()
        .copied()
        .expect("current runtime-owned separator projection");
    let before_state = runtime
        .interaction
        .layout_state
        .lookup_current_state_view(before_projection.mounted_state_id)
        .and_then(|read| read.downcast_ref::<SplitPaneRuntimeState>().copied())
        .expect("runtime-owned split state remains mounted");
    assert_eq!(
        before_state.policy_revision.collapse_policy,
        Some(SplitPaneCollapsePolicy::FirstPane)
    );
    let before_layout = runtime.layout().rects.clone();
    let before_counters = runtime.refresh_counters();
    let position = before_projection.divider_bounds.center();

    assert_eq!(
        runtime.dispatch_event(Event::primary_double_click(position)),
        None
    );

    assert_eq!(
        runtime.interaction.focus.owner,
        Some(RuntimeFocusOwner::Widget(20))
    );
    assert_eq!(runtime.focused_widget(), Some(20));
    assert!(
        runtime
            .surface()
            .find_widget(20)
            .expect("independent focus target")
            .widget()
            .common()
            .state
            .focused
    );
    assert_eq!(runtime.pointer_capture(), None);
    assert_eq!(runtime.layout_pointer_capture(), None);
    assert_eq!(runtime.layout().rects, before_layout);
    assert_eq!(
        runtime.split_pane_separator_projections().first().copied(),
        Some(before_projection)
    );
    assert_eq!(
        runtime
            .interaction
            .layout_state
            .lookup_current_state_view(before_projection.mounted_state_id)
            .and_then(|read| read.downcast_ref::<SplitPaneRuntimeState>().copied()),
        Some(before_state)
    );
    assert_eq!(
        runtime.refresh_counters().layout,
        before_counters.layout + 1
    );
    assert_eq!(
        runtime.bridge().inner.events.borrow().as_slice(),
        [
            FocusDecisionEvent::Prepare(10),
            FocusDecisionEvent::Changed(10, false),
            FocusDecisionEvent::HostOutput,
            FocusDecisionEvent::Changed(20, true),
        ]
    );
}

#[test]
fn separator_pointer_focus_veto_blocks_primary_and_double_click_without_mutation() {
    let mut runtime =
        SurfaceRuntime::new(FocusDecisionSplitBridge::new(), Vector2::new(200.0, 160.0));
    assert!(runtime.focus_widget(10));
    runtime.bridge().inner.events.borrow_mut().clear();
    runtime
        .bridge_mut()
        .inner
        .old_decision
        .set(FocusLossDecision::Veto);
    runtime.take_repaint_requested();
    let projection = runtime
        .split_pane_separator_projections()
        .first()
        .copied()
        .expect("current runtime-owned separator projection");
    let position = projection.divider_bounds.center();

    let before_layout = runtime.layout().rects.clone();
    let before_projection = projection;
    let before_state = runtime
        .interaction
        .layout_state
        .lookup_current_state_view(projection.mounted_state_id)
        .and_then(|read| read.downcast_ref::<SplitPaneRuntimeState>().copied())
        .expect("runtime-owned split state");
    let before_counters = runtime.refresh_counters();
    assert_eq!(runtime.dispatch_event(Event::primary_press(position)), None);
    assert_eq!(runtime.focused_widget(), Some(10));
    assert!(matches!(
        runtime.interaction.focus.owner,
        Some(RuntimeFocusOwner::Widget(10))
    ));
    assert_eq!(runtime.pointer_capture(), None);
    assert_eq!(runtime.layout_pointer_capture(), None);
    assert_eq!(runtime.layout().rects, before_layout);
    assert_eq!(
        runtime.split_pane_separator_projections().first().copied(),
        Some(before_projection)
    );
    assert_eq!(runtime.refresh_counters(), before_counters);
    assert_eq!(
        runtime
            .interaction
            .layout_state
            .lookup_current_state_view(projection.mounted_state_id)
            .and_then(|read| read.downcast_ref::<SplitPaneRuntimeState>().copied()),
        Some(before_state)
    );
    assert_eq!(
        runtime.bridge().inner.events.borrow().as_slice(),
        [FocusDecisionEvent::Prepare(10)]
    );
    assert!(runtime.repaint_requested());

    runtime.take_repaint_requested();
    runtime.bridge().inner.events.borrow_mut().clear();
    let before_layout = runtime.layout().rects.clone();
    let before_projection = runtime
        .split_pane_separator_projections()
        .first()
        .copied()
        .expect("separator remains current after veto");
    let before_state = runtime
        .interaction
        .layout_state
        .lookup_current_state_view(before_projection.mounted_state_id)
        .and_then(|read| read.downcast_ref::<SplitPaneRuntimeState>().copied())
        .expect("runtime-owned split state remains mounted");
    let before_counters = runtime.refresh_counters();
    assert_eq!(
        runtime.dispatch_event(Event::primary_double_click(position)),
        None
    );
    assert_eq!(runtime.focused_widget(), Some(10));
    assert_eq!(runtime.pointer_capture(), None);
    assert_eq!(runtime.layout_pointer_capture(), None);
    assert_eq!(runtime.layout().rects, before_layout);
    assert_eq!(
        runtime.split_pane_separator_projections().first().copied(),
        Some(before_projection)
    );
    assert_eq!(runtime.refresh_counters(), before_counters);
    assert_eq!(
        runtime
            .interaction
            .layout_state
            .lookup_current_state_view(before_projection.mounted_state_id)
            .and_then(|read| read.downcast_ref::<SplitPaneRuntimeState>().copied()),
        Some(before_state)
    );
    assert_eq!(
        runtime.bridge().inner.events.borrow().as_slice(),
        [FocusDecisionEvent::Prepare(10)]
    );
    assert!(runtime.repaint_requested());
}

#[test]
fn separator_pointer_double_click_claims_no_action_without_widget_fallthrough() {
    let events = Rc::new(RefCell::new(Vec::new()));
    let mut runtime = SurfaceRuntime::new(
        SplitFallbackBridge::new(Rc::clone(&events)),
        Vector2::new(200.0, 80.0),
    );
    let projection = runtime
        .split_pane_separator_projections()
        .first()
        .copied()
        .expect("current runtime-owned separator projection");
    let before_layout = runtime.layout().rects.clone();

    assert_eq!(
        runtime.dispatch_event(Event::primary_double_click(
            projection.divider_bounds.center()
        )),
        None
    );
    assert!(events.borrow().is_empty());
    assert!(matches!(
        runtime.interaction.focus.owner,
        Some(RuntimeFocusOwner::SplitPaneSeparator(_))
    ));
    assert_eq!(runtime.focused_widget(), None);
    assert_eq!(runtime.pointer_capture(), None);
    assert_eq!(runtime.layout_pointer_capture(), None);
    assert_eq!(runtime.layout().rects, before_layout);
}

#[test]
fn separator_pointer_admission_leaves_secondary_and_non_runtime_fallback_unchanged() {
    let mut runtime = SurfaceRuntime::new(
        SplitInteractionBridge::new(SplitInteractionMode::RuntimeOwned),
        Vector2::new(200.0, 80.0),
    );
    let position = Point::new(52.0, 40.0);
    let before_layout = runtime.layout().rects.clone();
    assert_eq!(
        runtime.dispatch_event(Event::secondary_press(position)),
        None
    );
    assert_eq!(runtime.interaction.focus.owner, None);
    assert_eq!(runtime.layout_pointer_capture(), None);
    assert_eq!(runtime.layout().rects, before_layout);

    for mode in [
        SplitInteractionMode::Static,
        SplitInteractionMode::Controlled,
    ] {
        let mut runtime =
            SurfaceRuntime::new(SplitInteractionBridge::new(mode), Vector2::new(200.0, 80.0));
        let before_layout = runtime.layout().rects.clone();
        assert_eq!(runtime.dispatch_event(Event::primary_press(position)), None);
        assert_eq!(runtime.interaction.focus.owner, None);
        assert_eq!(runtime.layout_pointer_capture(), None);
        assert_eq!(runtime.layout().rects, before_layout);
    }
}

#[test]
fn separator_pointer_focus_loss_reprojection_consumes_invalidated_candidate() {
    let mut runtime = SurfaceRuntime::new(
        FocusDecisionSplitBridge::new().with_split_removed_on_host_output(),
        Vector2::new(200.0, 160.0),
    );
    assert!(runtime.focus_widget(10));
    runtime.bridge().inner.events.borrow_mut().clear();
    runtime.take_repaint_requested();
    let position = runtime
        .split_pane_separator_projections()
        .first()
        .expect("current runtime-owned separator projection")
        .divider_bounds
        .center();

    assert_eq!(runtime.dispatch_event(Event::primary_press(position)), None);
    assert_eq!(runtime.interaction.focus.owner, None);
    assert_eq!(runtime.focused_widget(), None);
    assert_eq!(runtime.pointer_capture(), None);
    assert_eq!(runtime.layout_pointer_capture(), None);
    assert!(runtime.split_pane_separator_projections().is_empty());
    assert_eq!(
        runtime.bridge().inner.events.borrow().as_slice(),
        [
            FocusDecisionEvent::Prepare(10),
            FocusDecisionEvent::Changed(10, false),
            FocusDecisionEvent::HostOutput,
        ]
    );
}

#[test]
fn separator_pointer_focus_loss_geometry_reprojection_clears_candidate_without_fallback() {
    let mut runtime = SurfaceRuntime::new(
        FocusDecisionSplitBridge::new().with_split_geometry_changed_on_host_output(),
        Vector2::new(200.0, 160.0),
    );
    assert!(runtime.focus_widget(10));
    runtime.bridge().inner.events.borrow_mut().clear();
    runtime.take_repaint_requested();
    let before_projection = runtime
        .split_pane_separator_projections()
        .first()
        .copied()
        .expect("current runtime-owned separator projection");
    let before_state = runtime
        .interaction
        .layout_state
        .lookup_current_state_view(before_projection.mounted_state_id)
        .and_then(|read| read.downcast_ref::<SplitPaneRuntimeState>().copied())
        .expect("runtime-owned split state remains mounted");
    let position = before_projection.divider_bounds.center();

    assert_eq!(runtime.dispatch_event(Event::primary_press(position)), None);

    let after_projection = runtime
        .split_pane_separator_projections()
        .first()
        .copied()
        .expect("same runtime-owned separator remains mounted after reprojection");
    assert_eq!(after_projection.target, before_projection.target);
    assert_eq!(
        after_projection.mounted_state_id,
        before_projection.mounted_state_id
    );
    assert_eq!(after_projection.axis, before_projection.axis);
    assert_eq!(after_projection.behavior, before_projection.behavior);
    assert_ne!(
        after_projection.divider_bounds,
        before_projection.divider_bounds
    );
    assert_eq!(
        runtime
            .interaction
            .layout_state
            .lookup_current_state_view(after_projection.mounted_state_id)
            .and_then(|read| read.downcast_ref::<SplitPaneRuntimeState>().copied()),
        Some(before_state)
    );
    assert_eq!(runtime.interaction.focus.owner, None);
    assert_eq!(runtime.layout_pointer_capture(), None);
    assert_eq!(runtime.pointer_capture(), None);
    assert_eq!(runtime.focused_widget(), None);
    assert_eq!(
        runtime.bridge().inner.events.borrow().as_slice(),
        [
            FocusDecisionEvent::Prepare(10),
            FocusDecisionEvent::Changed(10, false),
            FocusDecisionEvent::HostOutput,
        ]
    );
}

#[test]
fn separator_pointer_focus_loss_initial_ratio_reprojection_clears_refreshed_candidate_without_fallback()
 {
    let mut runtime = SurfaceRuntime::new(
        FocusDecisionSplitBridge::new().with_split_initial_ratio_changed_on_host_output(),
        Vector2::new(200.0, 160.0),
    );
    assert!(runtime.focus_widget(10));
    runtime.bridge().inner.events.borrow_mut().clear();
    runtime.take_repaint_requested();
    let before_projection = runtime
        .split_pane_separator_projections()
        .first()
        .copied()
        .expect("current runtime-owned separator projection");
    let before_layout = runtime.layout().rects.clone();
    let position = before_projection.divider_bounds.center();

    assert_eq!(runtime.dispatch_event(Event::primary_press(position)), None);

    let after_projection = runtime
        .split_pane_separator_projections()
        .first()
        .copied()
        .expect("same runtime-owned separator remains mounted after reprojection");
    assert_eq!(after_projection.target, before_projection.target);
    assert_eq!(
        after_projection.mounted_state_id,
        before_projection.mounted_state_id
    );
    assert_eq!(after_projection.axis, before_projection.axis);
    assert_ne!(after_projection.behavior, before_projection.behavior);
    assert!(
        before_projection
            .behavior
            .compatible_with(after_projection.behavior)
    );
    assert_eq!(
        after_projection.divider_bounds,
        before_projection.divider_bounds
    );
    assert_eq!(runtime.layout().rects, before_layout);
    assert_eq!(runtime.interaction.focus.owner, None);
    assert_eq!(runtime.pointer_capture(), None);
    assert_eq!(runtime.layout_pointer_capture(), None);
    assert_eq!(runtime.focused_widget(), None);
    assert_eq!(
        runtime.bridge().inner.events.borrow().as_slice(),
        [
            FocusDecisionEvent::Prepare(10),
            FocusDecisionEvent::Changed(10, false),
            FocusDecisionEvent::HostOutput,
        ]
    );
}

#[test]
fn nested_separator_pointer_transfer_moves_private_owner_to_exact_inner_target() {
    let mut runtime = SurfaceRuntime::new(
        SplitInteractionBridge::new(SplitInteractionMode::RuntimeOwned).with_nested(),
        Vector2::new(200.0, 120.0),
    );
    let outer = runtime
        .split_pane_separator_projections()
        .iter()
        .find(|projection| projection.target.container_id == 1)
        .copied()
        .expect("outer separator projection");
    let inner = runtime
        .split_pane_separator_projections()
        .iter()
        .find(|projection| projection.target.container_id == 4)
        .copied()
        .expect("inner separator projection");

    assert_eq!(
        runtime.dispatch_event(Event::primary_double_click(outer.divider_bounds.center())),
        None
    );
    assert!(matches!(
        runtime.interaction.focus.owner,
        Some(RuntimeFocusOwner::SplitPaneSeparator(owner)) if owner.target == outer.target
    ));
    assert_eq!(runtime.layout_pointer_capture(), None);

    assert_eq!(
        runtime.dispatch_event(Event::primary_press(inner.divider_bounds.center())),
        None
    );
    assert!(matches!(
        runtime.interaction.focus.owner,
        Some(RuntimeFocusOwner::SplitPaneSeparator(owner)) if owner.target == inner.target
    ));
    assert_eq!(runtime.layout_pointer_capture(), Some(inner.target));
}

#[test]
fn separator_focus_allow_routes_widget_loss_once_before_private_ownership() {
    let mut runtime =
        SurfaceRuntime::new(FocusDecisionSplitBridge::new(), Vector2::new(200.0, 160.0));
    assert!(runtime.focus_widget(10));
    runtime.bridge().inner.events.borrow_mut().clear();
    let projection = runtime
        .split_pane_separator_projections()
        .first()
        .copied()
        .expect("current runtime-owned separator projection");

    assert_eq!(
        runtime.request_split_pane_separator_focus(projection),
        FocusTransition::Changed
    );
    let events = runtime.bridge().inner.events.borrow();
    let prepare = events
        .iter()
        .position(|event| matches!(*event, FocusDecisionEvent::Prepare(10)))
        .expect("widget loss was prepared");
    let changed = events
        .iter()
        .position(|event| matches!(*event, FocusDecisionEvent::Changed(10, false)))
        .expect("widget focus loss was routed");
    let output = events
        .iter()
        .position(|event| matches!(*event, FocusDecisionEvent::HostOutput))
        .expect("widget focus-loss output was reduced");
    assert!(prepare < changed && changed < output);
    assert_eq!(
        events
            .iter()
            .filter(|event| matches!(**event, FocusDecisionEvent::Changed(10, false)))
            .count(),
        1
    );
    assert!(
        !events
            .iter()
            .any(|event| { matches!(*event, FocusDecisionEvent::Changed(_, true)) })
    );
    assert_eq!(runtime.focused_widget(), None);
    assert!(matches!(
        runtime.interaction.focus.owner,
        Some(RuntimeFocusOwner::SplitPaneSeparator(_))
    ));
}

#[test]
fn separator_focus_veto_preserves_widget_and_split_runtime_state() {
    let mut runtime =
        SurfaceRuntime::new(FocusDecisionSplitBridge::new(), Vector2::new(200.0, 160.0));
    assert!(runtime.focus_widget(10));
    runtime.bridge().inner.events.borrow_mut().clear();
    runtime
        .bridge_mut()
        .inner
        .old_decision
        .set(FocusLossDecision::Veto);
    runtime.take_repaint_requested();
    let projection = runtime
        .split_pane_separator_projections()
        .first()
        .copied()
        .expect("current runtime-owned separator projection");
    let before_layout = runtime.layout().rects.clone();
    let before_projection = projection;
    let before_counters = runtime.refresh_counters();

    assert_eq!(
        runtime.request_split_pane_separator_focus(projection),
        FocusTransition::Vetoed
    );
    assert!(matches!(
        runtime.interaction.focus.owner,
        Some(RuntimeFocusOwner::Widget(10))
    ));
    assert_eq!(runtime.focused_widget(), Some(10));
    assert_eq!(runtime.layout().rects, before_layout);
    assert_eq!(
        runtime.split_pane_separator_projections().first().copied(),
        Some(before_projection)
    );
    assert_eq!(runtime.layout_pointer_capture(), None);
    assert_eq!(runtime.refresh_counters(), before_counters);
    assert_eq!(
        runtime.bridge().inner.events.borrow().as_slice(),
        [FocusDecisionEvent::Prepare(10)]
    );
    assert!(runtime.repaint_requested());
}

#[test]
fn separator_focus_rejects_malformed_stale_and_non_runtime_candidates_without_veto() {
    let mut runtime = SurfaceRuntime::new(
        SplitInteractionBridge::new(SplitInteractionMode::RuntimeOwned),
        Vector2::new(200.0, 80.0),
    );
    let projection = runtime
        .split_pane_separator_projections()
        .first()
        .copied()
        .expect("current runtime-owned separator projection");

    let mut malformed = projection;
    malformed.divider_bounds = Rect::default();
    assert_eq!(
        runtime.request_split_pane_separator_focus(malformed),
        FocusTransition::InvalidTarget
    );
    let mut wrong_region = projection;
    wrong_region.target.region_id = LayoutHitRegionId::new(7);
    assert_eq!(
        runtime.request_split_pane_separator_focus(wrong_region),
        FocusTransition::InvalidTarget
    );
    let mut wrong_schema = projection;
    wrong_schema.behavior.state_schema_version = 99;
    assert_eq!(
        runtime.request_split_pane_separator_focus(wrong_schema),
        FocusTransition::InvalidTarget
    );
    let mut stale_generation = projection;
    stale_generation.mounted_state_id = crate::gui::layout_core::MountedContainerStateId::new(
        crate::layout::ContainerStateId::new::<SplitPaneRuntimeState>(1, 3),
        std::num::NonZeroU64::new(2).expect("non-zero stale generation"),
    );
    assert_eq!(
        runtime.request_split_pane_separator_focus(stale_generation),
        FocusTransition::InvalidTarget
    );
    assert_eq!(runtime.interaction.focus.owner, None);

    let mut static_runtime = SurfaceRuntime::new(
        SplitInteractionBridge::new(SplitInteractionMode::Static),
        Vector2::new(200.0, 80.0),
    );
    assert_eq!(
        static_runtime.request_split_pane_separator_focus(projection),
        FocusTransition::InvalidTarget
    );
    let mut controlled_runtime = SurfaceRuntime::new(
        SplitInteractionBridge::new(SplitInteractionMode::Controlled),
        Vector2::new(200.0, 80.0),
    );
    assert_eq!(
        controlled_runtime.request_split_pane_separator_focus(projection),
        FocusTransition::InvalidTarget
    );
}

#[test]
fn separator_focus_retains_compatible_ratio_geometry_viewport_and_collapse_lifecycle() {
    let mut bridge = SplitInteractionBridge::new(SplitInteractionMode::RuntimeOwned)
        .with_collapse_policy(SplitPaneCollapsePolicy::FirstPane);
    bridge.policy.first_min_extent = 20.0;
    let mut runtime = SurfaceRuntime::new(bridge, Vector2::new(200.0, 80.0));
    let initial_projection = runtime
        .split_pane_separator_projections()
        .first()
        .copied()
        .expect("initial separator projection");
    assert_eq!(
        runtime.request_split_pane_separator_focus(initial_projection),
        FocusTransition::Changed
    );

    runtime.dispatch_event(Event::primary_press(Point::new(52.0, 40.0)));
    runtime.dispatch_event(Event::pointer_move(Point::new(130.0, 100.0)));
    runtime.dispatch_event(Event::pointer_release(
        Point::new(130.0, 100.0),
        PointerButton::Primary,
        PointerModifiers::default(),
    ));
    runtime.dispatch_event(Event::primary_double_click(Point::new(134.0, 40.0)));
    runtime.dispatch_event(Event::primary_double_click(Point::new(24.0, 40.0)));
    runtime.set_viewport(Vector2::new(240.0, 80.0));

    let retained = match runtime.interaction.focus.owner {
        Some(RuntimeFocusOwner::SplitPaneSeparator(owner)) => owner,
        _ => panic!("compatible split lifecycle should retain separator focus"),
    };
    let current_projection = runtime
        .split_pane_separator_projections()
        .first()
        .copied()
        .expect("current separator projection after compatible changes");
    assert_eq!(retained.target, initial_projection.target);
    assert_eq!(
        retained.mounted_state_id,
        initial_projection.mounted_state_id
    );
    assert_eq!(retained.axis, current_projection.axis);
    assert!(
        retained
            .behavior
            .compatible_with(current_projection.behavior)
    );
}

#[test]
fn separator_focus_retires_on_mount_mode_policy_projection_and_lifecycle_boundaries() {
    let mut runtime = SurfaceRuntime::new(
        SplitInteractionBridge::new(SplitInteractionMode::RuntimeOwned),
        Vector2::new(200.0, 80.0),
    );
    let initial = runtime
        .split_pane_separator_projections()
        .first()
        .copied()
        .expect("initial separator projection");
    assert_eq!(
        runtime.request_split_pane_separator_focus(initial),
        FocusTransition::Changed
    );
    runtime.bridge_mut().mounted = false;
    runtime.refresh();
    assert_eq!(runtime.interaction.focus.owner, None);

    runtime.bridge_mut().mounted = true;
    runtime.refresh();
    let remounted = runtime
        .split_pane_separator_projections()
        .first()
        .copied()
        .expect("remounted separator projection");
    assert_ne!(initial.mounted_state_id, remounted.mounted_state_id);
    assert_eq!(
        runtime.request_split_pane_separator_focus(remounted),
        FocusTransition::Changed
    );

    runtime.bridge_mut().policy.axis = SplitPaneAxis::Vertical;
    runtime.refresh();
    assert_eq!(runtime.interaction.focus.owner, None);
    let changed_axis = runtime
        .split_pane_separator_projections()
        .first()
        .copied()
        .expect("changed-axis separator projection");
    assert_eq!(
        runtime.request_split_pane_separator_focus(changed_axis),
        FocusTransition::Changed
    );

    runtime.bridge_mut().mode = SplitInteractionMode::Controlled;
    runtime.bridge_mut().generation = 2;
    runtime.refresh();
    assert_eq!(runtime.interaction.focus.owner, None);

    runtime.bridge_mut().mode = SplitInteractionMode::RuntimeOwned;
    runtime.refresh();
    let explicit_clear_projection = runtime
        .split_pane_separator_projections()
        .first()
        .copied()
        .expect("runtime-owned separator after mode restore");
    assert_eq!(
        runtime.request_split_pane_separator_focus(explicit_clear_projection),
        FocusTransition::Changed
    );
    runtime.clear_focus();
    assert_eq!(runtime.interaction.focus.owner, None);

    assert_eq!(
        runtime.request_split_pane_separator_focus(explicit_clear_projection),
        FocusTransition::Changed
    );
    assert!(runtime.begin_native_recovery());
    assert_eq!(runtime.interaction.focus.owner, None);

    let mut closing_runtime = SurfaceRuntime::new(
        SplitInteractionBridge::new(SplitInteractionMode::RuntimeOwned),
        Vector2::new(200.0, 80.0),
    );
    let closing_projection = closing_runtime
        .split_pane_separator_projections()
        .first()
        .copied()
        .expect("closing separator projection");
    assert_eq!(
        closing_runtime.request_split_pane_separator_focus(closing_projection),
        FocusTransition::Changed
    );
    assert!(closing_runtime.begin_closing());
    assert_eq!(closing_runtime.interaction.focus.owner, None);
}

#[test]
fn prepared_discarded_and_stale_refresh_candidates_cannot_mutate_separator_focus() {
    let mut runtime = SurfaceRuntime::new(
        SplitInteractionBridge::new(SplitInteractionMode::RuntimeOwned),
        Vector2::new(200.0, 80.0),
    );
    let projection = runtime
        .split_pane_separator_projections()
        .first()
        .copied()
        .expect("separator projection");
    assert_eq!(
        runtime.request_split_pane_separator_focus(projection),
        FocusTransition::Changed
    );
    let before = runtime.interaction.focus.owner;

    let request = runtime
        .issue_fresh_surface_refresh_request(RepaintScope::Projection)
        .expect("fresh projection request");
    let candidate_surface = runtime.bridge().surface();
    let candidate = runtime
        .prepare_fresh_surface(candidate_surface, request)
        .expect("unchanged split candidate");
    assert_eq!(runtime.interaction.focus.owner, before);
    candidate.discard();
    assert_eq!(runtime.interaction.focus.owner, before);

    let stale_request = runtime
        .issue_fresh_surface_refresh_request(RepaintScope::Projection)
        .expect("stale projection request");
    runtime.advance_fresh_surface_active_generation();
    let candidate_surface = runtime.bridge().surface();
    assert!(
        runtime
            .prepare_fresh_surface(candidate_surface, stale_request)
            .is_none()
    );
    assert_eq!(runtime.interaction.focus.owner, before);
}

#[test]
fn default_focus_loss_decision_allows_existing_widget_contracts() {
    let mut button =
        ButtonWidget::new(10, "Default", WidgetSizing::fixed(Vector2::new(80.0, 28.0)));

    assert_eq!(FocusLossDecision::default(), FocusLossDecision::Allow);
    assert_eq!(button.prepare_focus_loss(), FocusLossDecision::Allow);
}

#[test]
fn removed_focused_widget_cannot_veto_forced_cleanup() {
    let mut runtime = SurfaceRuntime::new(FocusDecisionBridge::new(), Vector2::new(200.0, 80.0));

    assert!(runtime.focus_widget(10));
    runtime.bridge().events.borrow_mut().clear();
    runtime
        .bridge_mut()
        .old_decision
        .set(FocusLossDecision::Veto);
    runtime.bridge_mut().remove_old = true;
    runtime.refresh();

    assert_eq!(runtime.focused_widget(), None);
    assert!(runtime.surface().find_widget(10).is_none());
    assert!(runtime.bridge().events.borrow().is_empty());
}

#[test]
fn invalid_focus_targets_do_not_prepare_a_vetoing_owner() {
    let mut runtime = SurfaceRuntime::new(
        FocusDecisionBridge::new().with_target_focusable(false),
        Vector2::new(200.0, 80.0),
    );

    assert!(runtime.focus_widget(10));
    runtime.take_repaint_requested();
    runtime.bridge().events.borrow_mut().clear();
    runtime
        .bridge_mut()
        .old_decision
        .set(FocusLossDecision::Veto);

    assert!(!runtime.focus_widget(20));
    assert!(!runtime.focus_widget(999));

    assert_eq!(runtime.focused_widget(), Some(10));
    assert!(runtime.bridge().events.borrow().is_empty());
    assert!(
        runtime
            .surface()
            .find_widget(10)
            .expect("vetoing widget")
            .widget()
            .common()
            .state
            .focused
    );
}

#[test]
fn pointer_focus_veto_unwinds_press_and_double_click_capture_without_target_input() {
    let mut runtime = SurfaceRuntime::new(FocusDecisionBridge::new(), Vector2::new(200.0, 80.0));

    assert!(runtime.focus_widget(10));
    runtime.take_repaint_requested();
    runtime.bridge().events.borrow_mut().clear();
    runtime
        .bridge_mut()
        .old_decision
        .set(FocusLossDecision::Veto);

    let target_point = Point::new(4.0, 32.0);
    assert_eq!(
        runtime.dispatch_event(Event::primary_press(target_point)),
        None
    );
    assert_eq!(runtime.focused_widget(), Some(10));
    assert_eq!(runtime.pointer_capture(), None);
    assert_eq!(
        runtime.bridge().events.borrow().as_slice(),
        [FocusDecisionEvent::Prepare(10)]
    );

    runtime.bridge().events.borrow_mut().clear();
    assert_eq!(
        runtime.dispatch_event(Event::primary_double_click(target_point)),
        None
    );
    assert_eq!(runtime.focused_widget(), Some(10));
    assert_eq!(runtime.pointer_capture(), None);
    assert_eq!(
        runtime.bridge().events.borrow().as_slice(),
        [FocusDecisionEvent::Prepare(10)]
    );
}

#[test]
fn scrollbar_press_focus_veto_prevents_capture_and_scroll() {
    let mut runtime = SurfaceRuntime::new(
        FocusDecisionBridge::new().with_scroll(),
        Vector2::new(100.0, 50.0),
    );

    assert!(runtime.focus_widget(10));
    runtime.take_repaint_requested();
    runtime.bridge().events.borrow_mut().clear();
    runtime
        .bridge_mut()
        .old_decision
        .set(FocusLossDecision::Veto);

    let scrollbar_point = (0..100)
        .flat_map(|x| (0..50).map(move |y| Point::new(x as f32 + 0.5, y as f32 + 0.5)))
        .find(|point| runtime.scroll_affordance_at(*point).is_some())
        .expect("overflow surface should expose a scrollbar thumb");
    let initial_offset = runtime.layout_state.scroll_offset(30);

    assert_eq!(
        runtime.dispatch_event(Event::primary_press(scrollbar_point)),
        None
    );
    assert_eq!(runtime.focused_widget(), Some(10));
    assert_eq!(runtime.pointer_capture(), None);
    assert!(!runtime.scrollbar_drag_active());
    assert_eq!(runtime.hovered_scroll_affordance(), None);
    assert_eq!(runtime.layout_state.scroll_offset(30), initial_offset);
    assert_eq!(
        runtime.bridge().events.borrow().as_slice(),
        [FocusDecisionEvent::Prepare(10)]
    );

    let _ = runtime.dispatch_event(Event::pointer_move(Point::new(scrollbar_point.x, 49.5)));
    assert_eq!(runtime.layout_state.scroll_offset(30), initial_offset);
    assert_eq!(runtime.pointer_capture(), None);
    assert!(!runtime.scrollbar_drag_active());
    assert_eq!(
        runtime.bridge().events.borrow().as_slice(),
        [FocusDecisionEvent::Prepare(10)]
    );
}

#[test]
fn scrollbar_double_click_focus_veto_prevents_capture_and_scroll() {
    let mut runtime = SurfaceRuntime::new(
        FocusDecisionBridge::new().with_scroll(),
        Vector2::new(100.0, 50.0),
    );

    assert!(runtime.focus_widget(10));
    runtime.take_repaint_requested();
    runtime.bridge().events.borrow_mut().clear();
    runtime
        .bridge_mut()
        .old_decision
        .set(FocusLossDecision::Veto);

    let scrollbar_point = (0..100)
        .flat_map(|x| (0..50).map(move |y| Point::new(x as f32 + 0.5, y as f32 + 0.5)))
        .find(|point| runtime.scroll_affordance_at(*point).is_some())
        .expect("overflow surface should expose a scrollbar thumb");
    let initial_offset = runtime.layout_state.scroll_offset(30);

    assert_eq!(
        runtime.dispatch_event(Event::primary_double_click(scrollbar_point)),
        None
    );
    assert_eq!(runtime.focused_widget(), Some(10));
    assert_eq!(runtime.pointer_capture(), None);
    assert!(!runtime.scrollbar_drag_active());
    assert_eq!(runtime.hovered_scroll_affordance(), None);
    assert_eq!(runtime.layout_state.scroll_offset(30), initial_offset);
    assert_eq!(
        runtime.bridge().events.borrow().as_slice(),
        [FocusDecisionEvent::Prepare(10)]
    );

    let _ = runtime.dispatch_event(Event::pointer_move(Point::new(scrollbar_point.x, 49.5)));
    assert_eq!(runtime.layout_state.scroll_offset(30), initial_offset);
    assert_eq!(runtime.pointer_capture(), None);
    assert!(!runtime.scrollbar_drag_active());
    assert_eq!(
        runtime.bridge().events.borrow().as_slice(),
        [FocusDecisionEvent::Prepare(10)]
    );
}

#[test]
fn managed_capture_precedes_scrollbar_press_and_double_click() {
    let mut runtime = SurfaceRuntime::new(
        FocusDecisionBridge::new()
            .with_scroll()
            .with_managed_capture(),
        Vector2::new(100.0, 50.0),
    );
    let owner_point = Point::new(8.0, 8.0);

    assert_eq!(
        runtime.dispatch_event(Event::primary_press(owner_point)),
        Some(10)
    );
    assert_eq!(runtime.focused_widget(), Some(10));
    assert_eq!(runtime.pointer_capture(), Some(10));

    let scrollbar_point = (0..100)
        .flat_map(|x| (0..50).map(move |y| Point::new(x as f32 + 0.5, y as f32 + 0.5)))
        .find(|point| runtime.scroll_affordance_at(*point).is_some())
        .expect("overflow surface should expose a scrollbar thumb");
    let initial_offset = runtime.layout_state.scroll_offset(30);
    let initial_scrollbar_drag = runtime.scrollbar_drag_active();
    let initial_layout_capture = runtime.layout_pointer_capture();
    let initial_hovered_scrollbar = runtime.hovered_scroll_affordance();
    runtime.bridge().events.borrow_mut().clear();

    assert_eq!(
        runtime.dispatch_event(Event::primary_press(scrollbar_point)),
        None
    );
    assert_eq!(runtime.focused_widget(), Some(10));
    assert_eq!(runtime.pointer_capture(), Some(10));
    assert_eq!(runtime.scrollbar_drag_active(), initial_scrollbar_drag);
    assert_eq!(runtime.layout_pointer_capture(), initial_layout_capture);
    assert_eq!(
        runtime.hovered_scroll_affordance(),
        initial_hovered_scrollbar
    );
    assert_eq!(runtime.layout_state.scroll_offset(30), initial_offset);
    assert!(runtime.bridge().events.borrow().is_empty());

    assert_eq!(
        runtime.dispatch_event(Event::primary_double_click(scrollbar_point)),
        None
    );
    assert_eq!(runtime.focused_widget(), Some(10));
    assert_eq!(runtime.pointer_capture(), Some(10));
    assert_eq!(runtime.scrollbar_drag_active(), initial_scrollbar_drag);
    assert_eq!(runtime.layout_pointer_capture(), initial_layout_capture);
    assert_eq!(
        runtime.hovered_scroll_affordance(),
        initial_hovered_scrollbar
    );
    assert_eq!(runtime.layout_state.scroll_offset(30), initial_offset);
    assert!(runtime.bridge().events.borrow().is_empty());
}

#[test]
fn pointer_non_focusable_hit_with_veto_retains_focus_and_unwinds_capture() {
    let mut runtime = SurfaceRuntime::new(
        FocusDecisionBridge::new().with_target_focusable(false),
        Vector2::new(200.0, 80.0),
    );

    assert!(runtime.focus_widget(10));
    runtime.take_repaint_requested();
    runtime.bridge().events.borrow_mut().clear();
    runtime
        .bridge_mut()
        .old_decision
        .set(FocusLossDecision::Veto);

    assert_eq!(
        runtime.dispatch_event(Event::primary_press(Point::new(4.0, 32.0))),
        None
    );

    assert_eq!(runtime.focused_widget(), Some(10));
    assert_eq!(runtime.pointer_capture(), None);
    assert_eq!(
        runtime.bridge().events.borrow().as_slice(),
        [FocusDecisionEvent::Prepare(10)]
    );
}

#[test]
fn pointer_press_on_non_focusable_hit_target_clears_existing_focus() {
    let mut runtime = SurfaceRuntime::new(FocusTestBridge, Vector2::new(200.0, 80.0));
    assert_eq!(
        runtime
            .surface()
            .find_widget(20)
            .map(|widget| widget.is_focusable()),
        Some(false)
    );

    runtime.dispatch_event(Event::PointerPress {
        position: Point::new(4.0, 4.0),
        button: PointerButton::Primary,
        modifiers: PointerModifiers::default(),
        timestamp: None,
    });
    assert_eq!(runtime.focused_widget(), Some(10));

    assert_eq!(
        runtime.dispatch_event(Event::PointerPress {
            position: Point::new(4.0, 32.0),
            button: PointerButton::Primary,
            modifiers: PointerModifiers::default(),
            timestamp: None,
        }),
        Some(20)
    );

    assert_eq!(runtime.focused_widget(), None);
}

#[test]
fn clear_pointer_hover_clears_runtime_owner_and_retained_widget_state() {
    let mut runtime = SurfaceRuntime::new(FocusTestBridge, Vector2::new(200.0, 80.0));

    runtime.dispatch_pointer_move_with_outcome(Point::new(4.0, 32.0));
    assert_eq!(runtime.hovered_widget(), Some(20));
    assert!(
        runtime
            .surface()
            .find_widget(20)
            .expect("hovered widget")
            .widget()
            .common()
            .state
            .hovered
    );

    assert!(runtime.clear_pointer_hover());

    assert_eq!(runtime.hovered_widget(), None);
    assert!(runtime.repaint_requested());
    assert!(
        !runtime
            .surface()
            .find_widget(20)
            .expect("previous hovered widget")
            .widget()
            .common()
            .state
            .hovered
    );
}

#[test]
fn cancel_pointer_capture_clears_captured_pressed_widget_state() {
    let mut runtime = SurfaceRuntime::new(FocusTestBridge, Vector2::new(200.0, 80.0));

    runtime.dispatch_event(Event::PointerPress {
        position: Point::new(4.0, 32.0),
        button: PointerButton::Primary,
        modifiers: PointerModifiers::default(),
        timestamp: None,
    });
    assert_eq!(runtime.pointer_capture(), Some(20));
    assert!(
        runtime
            .surface()
            .find_widget(20)
            .expect("captured widget")
            .widget()
            .common()
            .state
            .pressed
    );

    runtime.cancel_pointer_capture();

    assert_eq!(runtime.pointer_capture(), None);
    assert!(runtime.repaint_requested());
    assert!(
        !runtime
            .surface()
            .find_widget(20)
            .expect("previously captured widget")
            .widget()
            .common()
            .state
            .pressed
    );
}

#[test]
fn cancel_pointer_capture_does_not_dispatch_focus_loss_output() {
    let mut runtime =
        SurfaceRuntime::new(FocusLossOutputBridge::default(), Vector2::new(200.0, 80.0));

    runtime.dispatch_event(Event::PointerPress {
        position: Point::new(4.0, 4.0),
        button: PointerButton::Primary,
        modifiers: PointerModifiers::default(),
        timestamp: None,
    });
    assert_eq!(runtime.pointer_capture(), Some(30));
    assert!(
        runtime
            .surface()
            .find_widget(30)
            .expect("captured widget")
            .widget()
            .common()
            .state
            .pressed
    );

    runtime.cancel_pointer_capture();

    assert_eq!(runtime.pointer_capture(), None);
    assert_eq!(runtime.bridge().dispatched, Vec::<usize>::new());
    assert!(
        !runtime
            .surface()
            .find_widget(30)
            .expect("previously captured widget")
            .widget()
            .common()
            .state
            .pressed
    );

    assert!(runtime.dispatch_input(30, WidgetInput::FocusChanged(false)));
    assert_eq!(runtime.bridge().dispatched, vec![99]);
}

#[test]
fn cancel_pointer_capture_delivers_slider_cancel_before_clearing_capture() {
    let mut runtime =
        SurfaceRuntime::new(SliderCaptureBridge::default(), Vector2::new(120.0, 28.0));

    runtime.dispatch_event(Event::PointerPress {
        position: Point::new(60.0, 14.0),
        button: PointerButton::Primary,
        modifiers: PointerModifiers::default(),
        timestamp: None,
    });
    assert_eq!(runtime.pointer_capture(), Some(31));
    assert_eq!(runtime.bridge().batches.len(), 1);
    assert_eq!(
        runtime.bridge().batches[0]
            .events()
            .iter()
            .map(|event| event.phase)
            .collect::<Vec<_>>(),
        [EditPhase::Begin, EditPhase::Update]
    );
    runtime.cancel_pointer_capture();

    assert_eq!(runtime.pointer_capture(), None);
    assert_eq!(runtime.bridge().batches.len(), 2);
    assert_eq!(runtime.bridge().batches[1].events().len(), 1);
    assert_eq!(
        runtime.bridge().batches[1].events()[0].phase,
        EditPhase::Cancel
    );
    assert_eq!(runtime.bridge().batches[1].value_change(), Some(0.25));
    let slider = runtime
        .surface()
        .find_widget(31)
        .expect("slider exists")
        .widget();
    assert!(!slider.common().state.pressed);
}

#[test]
fn failed_domain_slider_release_keeps_capture_cleanup_and_does_not_reopen_on_move() {
    let mut runtime =
        SurfaceRuntime::new(FailingDomainSliderBridge::new(), Vector2::new(120.0, 28.0));

    assert_eq!(
        runtime.dispatch_event(Event::primary_press(Point::new(60.0, 14.0))),
        Some(31)
    );
    assert_eq!(runtime.pointer_capture(), Some(31));
    assert_eq!(runtime.bridge().forward_calls.get(), 1);
    assert!(matches!(
        runtime.bridge().messages.as_slice(),
        [SliderDomainMessage::ValueChanged { value }] if *value == 60.0
    ));
    assert!(
        runtime
            .surface()
            .find_widget(31)
            .expect("domain slider exists")
            .widget()
            .common()
            .state
            .pressed
    );

    assert_eq!(
        runtime.dispatch_event(Event::pointer_release(
            Point::new(72.0, 14.0),
            PointerButton::Primary,
            PointerModifiers::default(),
        )),
        Some(31)
    );
    assert_eq!(runtime.pointer_capture(), None);
    assert_eq!(runtime.bridge().forward_calls.get(), 2);
    assert!(matches!(
        runtime.bridge().messages.as_slice(),
        [
            SliderDomainMessage::ValueChanged { value: 60.0 },
            SliderDomainMessage::MappingFailed {
                normalized: 0.6,
                error: SliderDomainError::NormalizedToValue {
                    error: DomainAdjustmentError::Forward,
                },
            },
        ]
    ));
    let slider = runtime
        .surface()
        .find_widget(31)
        .expect("domain slider exists after failed release")
        .widget();
    assert!(!slider.common().state.pressed);
    assert_eq!(
        slider.automation_semantics().value_text.as_deref(),
        Some("60.000")
    );

    let move_outcome = runtime.dispatch_pointer_move_with_outcome(Point::new(96.0, 14.0));
    assert_eq!(move_outcome.target, Some(31));
    assert!(!move_outcome.pointer_captured);
    assert_eq!(runtime.bridge().messages.len(), 2);
    assert_eq!(runtime.bridge().forward_calls.get(), 2);
}

#[test]
fn captured_slider_ignores_keyboard_edits_until_pointer_release() {
    let mut runtime =
        SurfaceRuntime::new(SliderCaptureBridge::default(), Vector2::new(120.0, 28.0));

    runtime.dispatch_event(Event::PointerPress {
        position: Point::new(60.0, 14.0),
        button: PointerButton::Primary,
        modifiers: PointerModifiers::default(),
        timestamp: None,
    });
    assert_eq!(runtime.pointer_capture(), Some(31));
    assert_eq!(runtime.focused_widget(), Some(31));
    assert_eq!(runtime.bridge().batches.len(), 1);
    let pointer_transaction = runtime.bridge().batches[0].events()[0].transaction;
    assert_eq!(
        runtime.bridge().batches[0]
            .events()
            .iter()
            .map(|event| event.phase)
            .collect::<Vec<_>>(),
        [EditPhase::Begin, EditPhase::Update]
    );
    assert!(
        runtime.bridge().batches[0]
            .events()
            .iter()
            .all(|event| event.transaction.source() == InteractionSource::Pointer)
    );
    let value_after_press = runtime
        .surface()
        .find_widget(31)
        .expect("slider exists")
        .widget()
        .automation_semantics()
        .value_text;

    for key in [WidgetKey::ArrowRight, WidgetKey::Home, WidgetKey::End] {
        assert_eq!(
            runtime.dispatch_event(Event::KeyPress {
                key,
                modifiers: KeyboardModifiers::default(),
                repeat: false,
                timestamp: None,
            }),
            Some(31)
        );
        assert_eq!(runtime.bridge().batches.len(), 1);
        assert_eq!(
            runtime
                .surface()
                .find_widget(31)
                .expect("slider exists")
                .widget()
                .automation_semantics()
                .value_text,
            value_after_press
        );
        assert!(
            runtime
                .surface()
                .find_widget(31)
                .expect("slider exists")
                .widget()
                .common()
                .state
                .pressed
        );
    }

    assert_eq!(
        runtime.dispatch_event(Event::PointerRelease {
            position: Point::new(96.0, 14.0),
            button: PointerButton::Primary,
            modifiers: PointerModifiers::default(),
            timestamp: None,
        }),
        Some(31)
    );
    assert_eq!(runtime.pointer_capture(), None);
    assert_eq!(runtime.bridge().batches.len(), 2);
    let release = runtime.bridge().batches[1];
    assert_eq!(
        release
            .events()
            .iter()
            .map(|event| event.phase)
            .collect::<Vec<_>>(),
        [EditPhase::Update, EditPhase::Commit]
    );
    assert!(
        release
            .events()
            .iter()
            .all(|event| event.transaction == pointer_transaction)
    );
    assert!(
        runtime
            .bridge()
            .batches
            .iter()
            .flat_map(|batch| batch.events())
            .all(|event| event.transaction.source() == InteractionSource::Pointer)
    );
    assert!(
        !runtime
            .surface()
            .find_widget(31)
            .expect("slider exists")
            .widget()
            .common()
            .state
            .pressed
    );
}

#[test]
fn clear_focus_delivers_slider_cancel_without_committing_the_pointer_edit() {
    let mut runtime =
        SurfaceRuntime::new(SliderCaptureBridge::default(), Vector2::new(120.0, 28.0));

    runtime.dispatch_event(Event::PointerPress {
        position: Point::new(60.0, 14.0),
        button: PointerButton::Primary,
        modifiers: PointerModifiers::default(),
        timestamp: None,
    });
    runtime.clear_focus();

    assert_eq!(runtime.bridge().batches.len(), 2);
    assert_eq!(runtime.bridge().batches[1].events().len(), 1);
    assert_eq!(
        runtime.bridge().batches[1].events()[0].phase,
        EditPhase::Cancel
    );
    assert_eq!(runtime.bridge().batches[1].value_change(), Some(0.25));
    assert_eq!(runtime.focused_widget(), None);
    assert_eq!(runtime.pointer_capture(), Some(31));
    let slider = runtime
        .surface()
        .find_widget(31)
        .expect("slider exists after focus loss")
        .widget();
    assert!(!slider.common().state.pressed);
}

#[test]
fn refresh_clears_retained_hover_from_non_owner_widgets() {
    let mut runtime = SurfaceRuntime::new(FocusTestBridge, Vector2::new(200.0, 80.0));

    runtime.dispatch_pointer_move_with_outcome(Point::new(4.0, 32.0));
    assert_eq!(runtime.hovered_widget(), Some(20));
    runtime.dispatch_input(10, WidgetInput::pointer_move(Point::new(4.0, 4.0)));
    assert!(
        runtime
            .surface()
            .find_widget(10)
            .expect("stale hover widget")
            .widget()
            .common()
            .state
            .hovered
    );

    runtime.refresh();

    assert_eq!(runtime.hovered_widget(), Some(20));
    assert!(
        !runtime
            .surface()
            .find_widget(10)
            .expect("stale hover widget")
            .widget()
            .common()
            .state
            .hovered
    );
    assert!(
        runtime
            .surface()
            .find_widget(20)
            .expect("current hover widget")
            .widget()
            .common()
            .state
            .hovered
    );
}

#[test]
fn pointer_hover_transition_clears_retained_hover_from_non_owner_widgets() {
    let mut runtime = SurfaceRuntime::new(FocusTestBridge, Vector2::new(200.0, 80.0));

    runtime.dispatch_input(10, WidgetInput::pointer_move(Point::new(4.0, 4.0)));
    assert!(
        runtime
            .surface()
            .find_widget(10)
            .expect("stale hover widget")
            .widget()
            .common()
            .state
            .hovered
    );

    let outcome = runtime.dispatch_pointer_move_with_outcome(Point::new(4.0, 32.0));

    assert!(outcome.hover_changed);
    assert_eq!(runtime.hovered_widget(), Some(20));
    assert!(
        !runtime
            .surface()
            .find_widget(10)
            .expect("stale hover widget")
            .widget()
            .common()
            .state
            .hovered
    );
    assert!(
        runtime
            .surface()
            .find_widget(20)
            .expect("current hover widget")
            .widget()
            .common()
            .state
            .hovered
    );
    assert!(outcome.needs_redraw());
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum ManagedPointerMessage {
    Press,
    DoubleClick,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum ManagedPointerEvent {
    Preflight(PointerPressAdmission),
    PrepareFocusLoss,
    FocusChanged(bool),
    Press(Option<InputTimestamp>),
    DoubleClick(Option<InputTimestamp>),
    Release(PointerButton),
    Move,
    Modifiers(Option<InputTimestamp>),
    Cancel,
}

#[derive(Clone)]
struct ManagedPointerFixture {
    id: u64,
    admission: Rc<Cell<PointerPressAdmission>>,
    retains: Rc<Cell<bool>>,
    retains_on_press: Rc<Cell<bool>>,
    focus_decision: Rc<Cell<FocusLossDecision>>,
    events: Rc<RefCell<Vec<ManagedPointerEvent>>>,
    emit_press: Rc<Cell<bool>>,
    handles_double_click: Rc<Cell<bool>>,
    present: Rc<Cell<bool>>,
    disabled: Rc<Cell<bool>>,
    read_only: Rc<Cell<bool>>,
}

impl ManagedPointerFixture {
    fn new(id: u64, admission: PointerPressAdmission) -> Self {
        Self {
            id,
            admission: Rc::new(Cell::new(admission)),
            retains: Rc::new(Cell::new(
                admission == PointerPressAdmission::ManagedCapture,
            )),
            retains_on_press: Rc::new(Cell::new(false)),
            focus_decision: Rc::new(Cell::new(FocusLossDecision::Allow)),
            events: Rc::new(RefCell::new(Vec::new())),
            emit_press: Rc::new(Cell::new(false)),
            handles_double_click: Rc::new(Cell::new(false)),
            present: Rc::new(Cell::new(true)),
            disabled: Rc::new(Cell::new(false)),
            read_only: Rc::new(Cell::new(false)),
        }
    }

    fn managed(id: u64) -> Self {
        Self::new(id, PointerPressAdmission::ManagedCapture)
    }

    fn legacy(id: u64) -> Self {
        Self::new(id, PointerPressAdmission::Legacy)
    }

    fn blocked(id: u64) -> Self {
        Self::new(id, PointerPressAdmission::Blocked)
    }

    fn with_press_output(self, emits_output: bool) -> Self {
        self.emit_press.set(emits_output);
        self
    }

    fn with_retention_on_press(self) -> Self {
        self.retains.set(false);
        self.retains_on_press.set(true);
        self
    }

    fn with_double_click_handler(self, handles: bool) -> Self {
        self.handles_double_click.set(handles);
        self
    }
}

#[derive(Clone)]
struct ManagedPointerWidget {
    common: WidgetCommon,
    fixture: ManagedPointerFixture,
    compatibility_kind: &'static str,
}

impl ManagedPointerWidget {
    fn new(fixture: ManagedPointerFixture, compatibility_kind: &'static str) -> Self {
        let mut common = WidgetCommon::fixed(fixture.id, 180.0, 28.0)
            .with_keyboard_focus()
            .without_default_chrome();
        common.state.disabled = fixture.disabled.get();
        common.state.read_only = fixture.read_only.get();
        Self {
            common,
            fixture,
            compatibility_kind,
        }
    }
}

impl Widget for ManagedPointerWidget {
    fn compatibility_kind(&self) -> &'static str {
        self.compatibility_kind
    }

    fn common(&self) -> &WidgetCommon {
        &self.common
    }

    fn common_mut(&mut self) -> &mut WidgetCommon {
        &mut self.common
    }

    fn preflight_pointer_press(
        &self,
        _bounds: Rect,
        _input: &WidgetInput,
    ) -> PointerPressAdmission {
        self.fixture
            .events
            .borrow_mut()
            .push(ManagedPointerEvent::Preflight(self.fixture.admission.get()));
        self.fixture.admission.get()
    }

    fn retains_managed_pointer_capture(&self) -> bool {
        self.fixture.retains.get()
    }

    fn prepare_focus_loss(&mut self) -> FocusLossDecision {
        self.fixture
            .events
            .borrow_mut()
            .push(ManagedPointerEvent::PrepareFocusLoss);
        self.fixture.focus_decision.get()
    }

    fn handle_input(&mut self, bounds: Rect, input: WidgetInput) -> Option<WidgetOutput> {
        match input {
            WidgetInput::FocusChanged(focused) => {
                self.common.state.focused = focused;
                self.fixture
                    .events
                    .borrow_mut()
                    .push(ManagedPointerEvent::FocusChanged(focused));
                None
            }
            WidgetInput::PointerPress {
                position,
                timestamp,
                ..
            } if bounds.contains(position) => {
                self.common.state.pressed = true;
                self.fixture
                    .events
                    .borrow_mut()
                    .push(ManagedPointerEvent::Press(timestamp));
                if self.fixture.retains_on_press.get() {
                    self.fixture.retains.set(true);
                }
                self.fixture
                    .emit_press
                    .get()
                    .then_some(WidgetOutput::typed(ManagedPointerMessage::Press))
            }
            WidgetInput::PointerDoubleClick {
                position,
                timestamp,
                ..
            } if bounds.contains(position) => {
                self.fixture
                    .events
                    .borrow_mut()
                    .push(ManagedPointerEvent::DoubleClick(timestamp));
                self.fixture
                    .handles_double_click
                    .get()
                    .then_some(WidgetOutput::typed(ManagedPointerMessage::DoubleClick))
            }
            WidgetInput::PointerRelease { button, .. } => {
                self.common.state.pressed = false;
                self.fixture
                    .events
                    .borrow_mut()
                    .push(ManagedPointerEvent::Release(button));
                None
            }
            WidgetInput::PointerMove { .. } => {
                self.fixture
                    .events
                    .borrow_mut()
                    .push(ManagedPointerEvent::Move);
                None
            }
            WidgetInput::PointerModifiersChanged { timestamp, .. } => {
                self.fixture
                    .events
                    .borrow_mut()
                    .push(ManagedPointerEvent::Modifiers(timestamp));
                None
            }
            _ => None,
        }
    }

    fn handle_pointer_capture_cancelled(&mut self, _bounds: Rect) -> Option<WidgetOutput> {
        self.common.state.pressed = false;
        self.fixture
            .events
            .borrow_mut()
            .push(ManagedPointerEvent::Cancel);
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

struct ManagedPointerBridge {
    fixtures: Vec<ManagedPointerFixture>,
    compatibility_kind: &'static str,
    messages: Rc<RefCell<Vec<ManagedPointerMessage>>>,
    projections: Rc<Cell<usize>>,
}

impl ManagedPointerBridge {
    fn single(fixture: ManagedPointerFixture) -> Self {
        Self {
            fixtures: vec![fixture],
            compatibility_kind: "managed-pointer-widget-v1",
            messages: Rc::new(RefCell::new(Vec::new())),
            projections: Rc::new(Cell::new(0)),
        }
    }

    fn pair(first: ManagedPointerFixture, second: ManagedPointerFixture) -> Self {
        Self {
            fixtures: vec![first, second],
            compatibility_kind: "managed-pointer-widget-v1",
            messages: Rc::new(RefCell::new(Vec::new())),
            projections: Rc::new(Cell::new(0)),
        }
    }
}

impl RuntimeBridge<ManagedPointerMessage> for ManagedPointerBridge {
    fn project_surface(&mut self) -> Arc<UiSurface<ManagedPointerMessage>> {
        self.projections.set(self.projections.get() + 1);
        let children = self
            .fixtures
            .iter()
            .filter(|fixture| fixture.present.get())
            .map(|fixture| {
                fixed_child(
                    28.0,
                    SurfaceNode::widget(
                        ManagedPointerWidget::new(fixture.clone(), self.compatibility_kind),
                        WidgetMessageMapper::typed(|message: ManagedPointerMessage| message),
                    ),
                )
            })
            .collect();
        crate::runtime::test_arc_surface(UiSurface::new(SurfaceNode::column(1, 0.0, children)))
    }

    fn update(&mut self, message: ManagedPointerMessage) -> Command<ManagedPointerMessage> {
        self.messages.borrow_mut().push(message);
        Command::repaint(RepaintScope::Surface)
    }
}

#[test]
fn managed_pointer_press_admission_captures_before_no_output_and_output_refresh() {
    let no_output = ManagedPointerFixture::managed(41);
    let no_output_events = Rc::clone(&no_output.events);
    let mut runtime = SurfaceRuntime::new(
        ManagedPointerBridge::single(no_output),
        Vector2::new(180.0, 40.0),
    );
    let timestamp = InputTimestamp::capture();

    assert_eq!(
        runtime.dispatch_event(Event::PointerPress {
            position: Point::new(12.0, 12.0),
            button: PointerButton::Primary,
            modifiers: PointerModifiers::default(),
            timestamp: Some(timestamp),
        }),
        Some(41)
    );
    assert_eq!(runtime.pointer_capture(), Some(41));
    assert_eq!(runtime.focused_widget(), Some(41));
    assert!(runtime.bridge().messages.borrow().is_empty());
    assert!(
        no_output_events
            .borrow()
            .contains(&ManagedPointerEvent::Press(Some(timestamp)))
    );

    assert_eq!(
        runtime.dispatch_event(Event::PointerRelease {
            position: Point::new(120.0, 12.0),
            button: PointerButton::Primary,
            modifiers: PointerModifiers::default(),
            timestamp: Some(timestamp),
        }),
        Some(41)
    );
    assert_eq!(runtime.pointer_capture(), None);

    let with_output = ManagedPointerFixture::managed(42).with_press_output(true);
    let projections = Rc::new(Cell::new(0));
    let mut bridge = ManagedPointerBridge::single(with_output);
    bridge.projections = Rc::clone(&projections);
    let mut runtime = SurfaceRuntime::new(bridge, Vector2::new(180.0, 40.0));

    assert_eq!(
        runtime.dispatch_event(Event::PointerPress {
            position: Point::new(12.0, 12.0),
            button: PointerButton::Primary,
            modifiers: PointerModifiers::default(),
            timestamp: None,
        }),
        Some(42)
    );
    assert_eq!(
        runtime.bridge().messages.borrow().as_slice(),
        &[ManagedPointerMessage::Press]
    );
    assert!(projections.get() >= 2);
    assert_eq!(runtime.pointer_capture(), Some(42));
}

#[test]
fn managed_pointer_capture_reserves_until_press_establishes_retention() {
    let fixture = ManagedPointerFixture::managed(43).with_retention_on_press();
    let events = Rc::clone(&fixture.events);
    assert!(!fixture.retains.get());
    let mut runtime = SurfaceRuntime::new(
        ManagedPointerBridge::single(fixture.clone()),
        Vector2::new(180.0, 40.0),
    );

    assert_eq!(
        runtime.dispatch_event(Event::PointerPress {
            position: Point::new(12.0, 12.0),
            button: PointerButton::Primary,
            modifiers: PointerModifiers::default(),
            timestamp: None,
        }),
        Some(43)
    );
    assert!(fixture.retains.get());
    assert_eq!(runtime.pointer_capture(), Some(43));
    assert!(events.borrow().contains(&ManagedPointerEvent::Press(None)));

    assert_eq!(
        runtime.dispatch_event(Event::PointerMove {
            position: Point::new(120.0, 12.0),
            modifiers: PointerModifiers::default(),
            timestamp: None,
            sequence_range: None,
        }),
        Some(43)
    );
    assert!(events.borrow().contains(&ManagedPointerEvent::Move));
}

#[test]
fn blocked_pointer_press_does_not_mutate_incumbent_focus_capture_or_target() {
    let incumbent = ManagedPointerFixture::legacy(51);
    let blocked = ManagedPointerFixture::blocked(52);
    let incumbent_events = Rc::clone(&incumbent.events);
    let blocked_events = Rc::clone(&blocked.events);
    let mut runtime = SurfaceRuntime::new(
        ManagedPointerBridge::pair(incumbent, blocked),
        Vector2::new(180.0, 70.0),
    );

    assert_eq!(
        runtime.dispatch_event(Event::PointerPress {
            position: Point::new(12.0, 12.0),
            button: PointerButton::Primary,
            modifiers: PointerModifiers::default(),
            timestamp: None,
        }),
        Some(51)
    );
    let incumbent_events_before = incumbent_events.borrow().clone();
    assert_eq!(runtime.focused_widget(), Some(51));
    assert_eq!(runtime.pointer_capture(), Some(51));

    assert_eq!(
        runtime.dispatch_event(Event::PointerPress {
            position: Point::new(12.0, 40.0),
            button: PointerButton::Primary,
            modifiers: PointerModifiers::default(),
            timestamp: None,
        }),
        None
    );
    assert!(!runtime.dispatch_input(
        52,
        WidgetInput::pointer_press(
            Point::new(12.0, 40.0),
            PointerButton::Primary,
            PointerModifiers::default(),
        ),
    ));
    assert_eq!(runtime.focused_widget(), Some(51));
    assert_eq!(runtime.pointer_capture(), Some(51));
    assert_eq!(
        incumbent_events.borrow().as_slice(),
        incumbent_events_before.as_slice()
    );
    assert_eq!(
        blocked_events.borrow().as_slice(),
        &[
            ManagedPointerEvent::Preflight(PointerPressAdmission::Blocked),
            ManagedPointerEvent::Preflight(PointerPressAdmission::Blocked),
        ]
    );
}

#[test]
fn managed_pointer_capture_routes_exact_button_owner_for_move_modifiers_and_release() {
    let owner = ManagedPointerFixture::managed(61);
    let other = ManagedPointerFixture::legacy(62);
    let owner_events = Rc::clone(&owner.events);
    let other_events = Rc::clone(&other.events);
    let mut runtime = SurfaceRuntime::new(
        ManagedPointerBridge::pair(owner, other),
        Vector2::new(180.0, 70.0),
    );

    runtime.dispatch_event(Event::PointerPress {
        position: Point::new(12.0, 12.0),
        button: PointerButton::Primary,
        modifiers: PointerModifiers::default(),
        timestamp: None,
    });
    let move_timestamp = InputTimestamp::capture();
    assert_eq!(
        runtime.dispatch_event(Event::PointerMove {
            position: Point::new(12.0, 40.0),
            modifiers: PointerModifiers::default(),
            timestamp: Some(move_timestamp),
            sequence_range: None,
        }),
        Some(61)
    );
    assert_eq!(
        runtime.dispatch_event(Event::PointerModifiersChanged {
            modifiers: PointerModifiers {
                shift: true,
                ..PointerModifiers::default()
            },
            timestamp: Some(move_timestamp),
        }),
        Some(61)
    );
    assert!(!other_events.borrow().iter().any(|event| matches!(
        event,
        ManagedPointerEvent::Move | ManagedPointerEvent::Modifiers(_)
    )));

    assert_eq!(
        runtime.dispatch_event(Event::PointerRelease {
            position: Point::new(12.0, 40.0),
            button: PointerButton::Secondary,
            modifiers: PointerModifiers::default(),
            timestamp: None,
        }),
        None
    );
    assert_eq!(runtime.pointer_capture(), Some(61));
    assert!(
        !owner_events
            .borrow()
            .iter()
            .any(|event| matches!(event, ManagedPointerEvent::Release(_)))
    );

    assert_eq!(
        runtime.dispatch_event(Event::PointerRelease {
            position: Point::new(12.0, 40.0),
            button: PointerButton::Primary,
            modifiers: PointerModifiers::default(),
            timestamp: None,
        }),
        Some(61)
    );
    assert_eq!(runtime.pointer_capture(), None);
    assert_eq!(
        owner_events
            .borrow()
            .iter()
            .filter(|event| matches!(event, ManagedPointerEvent::Release(PointerButton::Primary)))
            .count(),
        1
    );
}

#[test]
fn managed_pointer_authority_loss_orphans_release_without_freezing_future_motion_or_press() {
    let owner = ManagedPointerFixture::managed(71);
    let other = ManagedPointerFixture::legacy(72);
    let owner_events = Rc::clone(&owner.events);
    let other_events = Rc::clone(&other.events);
    let mut runtime = SurfaceRuntime::new(
        ManagedPointerBridge::pair(owner.clone(), other),
        Vector2::new(180.0, 70.0),
    );

    runtime.dispatch_event(Event::PointerPress {
        position: Point::new(12.0, 12.0),
        button: PointerButton::Primary,
        modifiers: PointerModifiers::default(),
        timestamp: None,
    });
    owner.retains.set(false);

    assert_eq!(
        runtime.dispatch_event(Event::PointerMove {
            position: Point::new(12.0, 40.0),
            modifiers: PointerModifiers::default(),
            timestamp: None,
            sequence_range: None,
        }),
        Some(72)
    );
    assert_eq!(runtime.pointer_capture(), None);
    assert!(other_events.borrow().contains(&ManagedPointerEvent::Move));
    assert_eq!(
        runtime.dispatch_event(Event::PointerRelease {
            position: Point::new(12.0, 40.0),
            button: PointerButton::Primary,
            modifiers: PointerModifiers::default(),
            timestamp: None,
        }),
        None
    );
    assert!(
        !other_events
            .borrow()
            .iter()
            .any(|event| matches!(event, ManagedPointerEvent::Release(_)))
    );
    assert!(
        !owner_events
            .borrow()
            .iter()
            .any(|event| matches!(event, ManagedPointerEvent::Release(_)))
    );

    owner.retains.set(true);
    assert_eq!(
        runtime.dispatch_event(Event::PointerPress {
            position: Point::new(12.0, 12.0),
            button: PointerButton::Primary,
            modifiers: PointerModifiers::default(),
            timestamp: None,
        }),
        Some(71)
    );
    assert_eq!(runtime.pointer_capture(), Some(71));
}

#[test]
fn orphaned_primary_does_not_suppress_secondary_legacy_release() {
    let owner = ManagedPointerFixture::managed(111);
    let other = ManagedPointerFixture::legacy(112);
    let owner_events = Rc::clone(&owner.events);
    let other_events = Rc::clone(&other.events);
    let mut runtime = SurfaceRuntime::new(
        ManagedPointerBridge::pair(owner.clone(), other),
        Vector2::new(180.0, 70.0),
    );

    runtime.dispatch_event(Event::PointerPress {
        position: Point::new(12.0, 12.0),
        button: PointerButton::Primary,
        modifiers: PointerModifiers::default(),
        timestamp: None,
    });
    owner.retains.set(false);
    assert_eq!(
        runtime.dispatch_event(Event::PointerMove {
            position: Point::new(12.0, 40.0),
            modifiers: PointerModifiers::default(),
            timestamp: None,
            sequence_range: None,
        }),
        Some(112)
    );
    assert_eq!(runtime.pointer_capture(), None);

    assert_eq!(
        runtime.dispatch_event(Event::PointerPress {
            position: Point::new(12.0, 40.0),
            button: PointerButton::Secondary,
            modifiers: PointerModifiers::default(),
            timestamp: None,
        }),
        Some(112)
    );
    assert_eq!(runtime.pointer_capture(), Some(112));
    assert_eq!(
        runtime.dispatch_event(Event::PointerRelease {
            position: Point::new(12.0, 40.0),
            button: PointerButton::Secondary,
            modifiers: PointerModifiers::default(),
            timestamp: None,
        }),
        Some(112)
    );
    assert_eq!(runtime.pointer_capture(), None);
    assert_eq!(
        other_events
            .borrow()
            .iter()
            .filter(|event| matches!(
                event,
                ManagedPointerEvent::Release(PointerButton::Secondary)
            ))
            .count(),
        1
    );

    assert_eq!(
        runtime.dispatch_event(Event::PointerModifiersChanged {
            modifiers: PointerModifiers {
                shift: true,
                ..PointerModifiers::default()
            },
            timestamp: None,
        }),
        Some(112)
    );
    assert!(
        other_events
            .borrow()
            .contains(&ManagedPointerEvent::Modifiers(None))
    );

    assert_eq!(
        runtime.dispatch_event(Event::PointerRelease {
            position: Point::new(12.0, 40.0),
            button: PointerButton::Primary,
            modifiers: PointerModifiers::default(),
            timestamp: None,
        }),
        None
    );
    assert_eq!(
        other_events
            .borrow()
            .iter()
            .filter(|event| matches!(event, ManagedPointerEvent::Release(PointerButton::Primary)))
            .count(),
        0
    );
    assert!(
        !owner_events
            .borrow()
            .iter()
            .any(|event| matches!(event, ManagedPointerEvent::Release(_)))
    );

    assert_eq!(
        runtime.dispatch_event(Event::PointerMove {
            position: Point::new(12.0, 40.0),
            modifiers: PointerModifiers::default(),
            timestamp: None,
            sequence_range: None,
        }),
        Some(112)
    );
    assert!(other_events.borrow().contains(&ManagedPointerEvent::Move));

    assert_eq!(
        runtime.dispatch_event(Event::PointerPress {
            position: Point::new(12.0, 40.0),
            button: PointerButton::Primary,
            modifiers: PointerModifiers::default(),
            timestamp: None,
        }),
        Some(112)
    );
    assert_eq!(
        runtime.dispatch_event(Event::PointerRelease {
            position: Point::new(12.0, 40.0),
            button: PointerButton::Primary,
            modifiers: PointerModifiers::default(),
            timestamp: None,
        }),
        Some(112)
    );
    assert_eq!(
        other_events
            .borrow()
            .iter()
            .filter(|event| matches!(event, ManagedPointerEvent::Release(PointerButton::Primary)))
            .count(),
        1
    );
}

#[test]
fn managed_pointer_cancellation_is_once_and_focus_veto_preserves_authority() {
    let owner = ManagedPointerFixture::managed(81);
    let other = ManagedPointerFixture::legacy(82);
    let owner_events = Rc::clone(&owner.events);
    let mut runtime = SurfaceRuntime::new(
        ManagedPointerBridge::pair(owner.clone(), other),
        Vector2::new(180.0, 70.0),
    );

    runtime.dispatch_event(Event::PointerPress {
        position: Point::new(12.0, 12.0),
        button: PointerButton::Primary,
        modifiers: PointerModifiers::default(),
        timestamp: None,
    });
    owner.focus_decision.set(FocusLossDecision::Veto);
    assert_eq!(
        runtime.dispatch_event(Event::PointerPress {
            position: Point::new(12.0, 40.0),
            button: PointerButton::Primary,
            modifiers: PointerModifiers::default(),
            timestamp: None,
        }),
        None
    );
    assert_eq!(runtime.focused_widget(), Some(81));
    assert_eq!(runtime.pointer_capture(), Some(81));
    assert!(
        !owner_events
            .borrow()
            .contains(&ManagedPointerEvent::FocusChanged(false))
    );

    runtime.cancel_pointer_capture();
    runtime.cancel_pointer_capture();
    assert_eq!(runtime.pointer_capture(), None);
    assert_eq!(
        owner_events
            .borrow()
            .iter()
            .filter(|event| **event == ManagedPointerEvent::Cancel)
            .count(),
        1
    );
    assert_eq!(
        runtime.dispatch_event(Event::PointerRelease {
            position: Point::new(12.0, 12.0),
            button: PointerButton::Primary,
            modifiers: PointerModifiers::default(),
            timestamp: None,
        }),
        None
    );
}

#[test]
fn managed_pointer_refresh_preserves_only_live_compatible_same_id() {
    let fixture = ManagedPointerFixture::managed(91);
    let mut runtime = SurfaceRuntime::new(
        ManagedPointerBridge::single(fixture.clone()),
        Vector2::new(180.0, 40.0),
    );
    runtime.dispatch_event(Event::PointerPress {
        position: Point::new(12.0, 12.0),
        button: PointerButton::Primary,
        modifiers: PointerModifiers::default(),
        timestamp: None,
    });
    assert_eq!(runtime.pointer_capture(), Some(91));

    runtime.refresh();
    assert_eq!(runtime.pointer_capture(), Some(91));

    runtime.bridge_mut().compatibility_kind = "managed-pointer-widget-v2";
    runtime.refresh();
    assert_eq!(runtime.pointer_capture(), None);

    fixture.present.set(false);
    runtime.refresh();
    assert_eq!(runtime.pointer_capture(), None);
}

#[test]
fn managed_double_click_fallback_preflights_exact_press_metadata_and_blocked_fallback_is_inert() {
    let managed = ManagedPointerFixture::managed(101);
    let events = Rc::clone(&managed.events);
    let mut runtime = SurfaceRuntime::new(
        ManagedPointerBridge::single(managed.with_double_click_handler(false)),
        Vector2::new(180.0, 40.0),
    );
    let timestamp = InputTimestamp::capture();
    assert_eq!(
        runtime.dispatch_event(Event::PointerDoubleClick {
            position: Point::new(12.0, 12.0),
            button: PointerButton::Primary,
            modifiers: PointerModifiers::default(),
            timestamp: Some(timestamp),
        }),
        Some(101)
    );
    let events = events.borrow();
    let preflight_index = events
        .iter()
        .position(|event| matches!(event, ManagedPointerEvent::Preflight(_)))
        .expect("fallback press was preflighted");
    let double_click_index = events
        .iter()
        .position(|event| matches!(event, ManagedPointerEvent::DoubleClick(_)))
        .expect("double click was dispatched");
    let press_index = events
        .iter()
        .position(|event| matches!(event, ManagedPointerEvent::Press(_)))
        .expect("fallback press was dispatched");
    assert!(preflight_index < double_click_index);
    assert!(double_click_index < press_index);
    assert!(events.contains(&ManagedPointerEvent::Press(Some(timestamp))));
    drop(events);
    assert_eq!(runtime.pointer_capture(), Some(101));

    let blocked = ManagedPointerFixture::blocked(102);
    let blocked_events = Rc::clone(&blocked.events);
    let mut blocked_runtime = SurfaceRuntime::new(
        ManagedPointerBridge::single(blocked),
        Vector2::new(180.0, 40.0),
    );
    assert_eq!(
        blocked_runtime.dispatch_event(Event::PointerDoubleClick {
            position: Point::new(12.0, 12.0),
            button: PointerButton::Primary,
            modifiers: PointerModifiers::default(),
            timestamp: Some(timestamp),
        }),
        None
    );
    assert_eq!(blocked_runtime.pointer_capture(), None);
    assert_eq!(blocked_runtime.focused_widget(), None);
    assert_eq!(
        blocked_events.borrow().as_slice(),
        &[ManagedPointerEvent::Preflight(
            PointerPressAdmission::Blocked
        )]
    );
}

#[test]
fn runtime_owned_split_projects_only_one_positive_clipped_divider_target() {
    let static_runtime = SurfaceRuntime::new(
        SplitInteractionBridge::new(SplitInteractionMode::Static),
        Vector2::new(200.0, 80.0),
    );
    assert_eq!(
        static_runtime.layout_target_at(Point::new(52.0, 40.0)),
        None
    );
    assert!(static_runtime.split_pane_separator_projections().is_empty());
    assert!(
        static_runtime
            .automation_snapshot()
            .root
            .automation_targets()
            .iter()
            .all(|target| target.role != AutomationRole::Separator)
    );

    let controlled_runtime = SurfaceRuntime::new(
        SplitInteractionBridge::new(SplitInteractionMode::Controlled),
        Vector2::new(200.0, 80.0),
    );
    assert_eq!(
        controlled_runtime.layout_target_at(Point::new(52.0, 40.0)),
        None
    );
    assert!(
        controlled_runtime
            .split_pane_separator_projections()
            .is_empty()
    );
    assert!(
        controlled_runtime
            .automation_snapshot()
            .root
            .automation_targets()
            .iter()
            .all(|target| target.role != AutomationRole::Separator)
    );

    let mut zero_divider = SplitInteractionBridge::new(SplitInteractionMode::RuntimeOwned);
    zero_divider.policy.divider_extent = 0.0;
    let zero_runtime = SurfaceRuntime::new(zero_divider, Vector2::new(200.0, 80.0));
    assert_eq!(zero_runtime.layout_target_at(Point::new(50.0, 40.0)), None);
    assert!(zero_runtime.split_pane_separator_projections().is_empty());
    assert!(
        zero_runtime
            .automation_snapshot()
            .root
            .automation_targets()
            .iter()
            .all(|target| target.role != AutomationRole::Separator)
    );

    let mut full_divider = SplitInteractionBridge::new(SplitInteractionMode::RuntimeOwned);
    full_divider.policy.divider_extent = 400.0;
    let mut full_runtime = SurfaceRuntime::new(full_divider, Vector2::new(200.0, 80.0));
    assert_eq!(
        full_runtime
            .layout_target_at(Point::new(100.0, 40.0))
            .expect("a positive resolved divider still projects its geometry")
            .bounds,
        Rect::from_xy_size(0.0, 0.0, 200.0, 80.0)
    );
    assert!(full_runtime.split_pane_separator_projections().is_empty());
    let before_full_layout = full_runtime.layout().rects.clone();
    assert_eq!(
        full_runtime.dispatch_event(Event::primary_press(Point::new(100.0, 40.0))),
        None
    );
    assert_eq!(full_runtime.layout_pointer_capture(), None);
    assert_eq!(full_runtime.layout().rects, before_full_layout);

    let mut malformed = SplitInteractionBridge::new(SplitInteractionMode::RuntimeOwned);
    malformed.policy.divider_extent = f32::NAN;
    let malformed_runtime = SurfaceRuntime::new(malformed, Vector2::new(200.0, 80.0));
    assert_eq!(
        malformed_runtime.layout_target_at(Point::new(50.0, 40.0)),
        None
    );
    assert!(
        malformed_runtime
            .split_pane_separator_projections()
            .is_empty()
    );
    assert!(
        malformed_runtime
            .automation_snapshot()
            .root
            .automation_targets()
            .iter()
            .all(|target| target.role != AutomationRole::Separator)
    );
    assert_eq!(
        malformed_runtime.layout_target_at(Point::new(f32::NAN, 40.0)),
        None
    );

    let runtime = SurfaceRuntime::new(
        SplitInteractionBridge::new(SplitInteractionMode::RuntimeOwned),
        Vector2::new(200.0, 80.0),
    );
    let target = runtime
        .layout_target_at(Point::new(52.0, 40.0))
        .expect("runtime-owned split divider target");
    assert_eq!(target.container_id, 1);
    assert_eq!(
        target.region_id,
        crate::gui::layout_core::SPLIT_PANE_DIVIDER_REGION_ID
    );
    assert_eq!(target.bounds, Rect::from_xy_size(48.0, 0.0, 8.0, 80.0));
    let separator = runtime
        .split_pane_separator_projections()
        .first()
        .copied()
        .expect("runtime-owned split separator projection");
    assert_eq!(separator.target, target.identity());
    assert_eq!(separator.axis, SplitPaneAxis::Horizontal);
    assert_eq!(separator.divider_bounds, target.bounds);
    assert_eq!(separator.live_ratio, 0.25);
    assert_eq!(separator.mounted_state_id.generation().get(), 1);

    let mut clipped_bridge = SplitInteractionBridge::new(SplitInteractionMode::RuntimeOwned);
    clipped_bridge.clipped = true;
    clipped_bridge.policy.initial_ratio = 0.8;
    let clipped_runtime = SurfaceRuntime::new(clipped_bridge, Vector2::new(100.0, 40.0));
    assert_eq!(
        clipped_runtime.layout_target_at(Point::new(98.0, 20.0)),
        None,
        "a divider wholly outside its scroll clip is not projected"
    );
    assert!(
        clipped_runtime
            .split_pane_separator_projections()
            .is_empty()
    );
    assert!(
        clipped_runtime
            .automation_snapshot()
            .root
            .automation_targets()
            .iter()
            .all(|target| target.role != AutomationRole::Separator)
    );
}

#[test]
fn runtime_owned_nested_splits_project_independent_separators() {
    let runtime = SurfaceRuntime::new(
        SplitInteractionBridge::new(SplitInteractionMode::RuntimeOwned).with_nested(),
        Vector2::new(200.0, 120.0),
    );
    let projections = runtime.split_pane_separator_projections();
    assert_eq!(projections.len(), 2);

    let outer = projections
        .iter()
        .find(|projection| projection.target.container_id == 1)
        .expect("outer split separator projection");
    assert_eq!(outer.axis, SplitPaneAxis::Horizontal);
    assert_eq!(outer.mounted_state_id.generation().get(), 1);
    assert!(outer.divider_bounds.has_finite_positive_area());

    let inner = projections
        .iter()
        .find(|projection| projection.target.container_id == 4)
        .expect("inner split separator projection");
    assert_eq!(inner.axis, SplitPaneAxis::Vertical);
    assert_eq!(inner.mounted_state_id.generation().get(), 2);
    assert!(inner.divider_bounds.has_finite_positive_area());
    assert_ne!(outer.mounted_state_id, inner.mounted_state_id);
}

#[test]
fn runtime_owned_nested_split_collapse_keeps_inner_authority_independent() {
    let mut bridge = SplitInteractionBridge::new(SplitInteractionMode::RuntimeOwned)
        .with_nested()
        .with_collapse_policy(SplitPaneCollapsePolicy::FirstPane);
    bridge.policy.first_min_extent = 20.0;
    let mut runtime = SurfaceRuntime::new(bridge, Vector2::new(200.0, 120.0));
    let inner_first_height = runtime.layout().rects[&5].height();

    runtime.dispatch_event(Event::primary_double_click(Point::new(52.0, 60.0)));

    assert_eq!(runtime.layout().rects[&4].width(), 20.0);
    assert_eq!(runtime.layout().rects[&5].height(), inner_first_height);
    let projections = runtime.split_pane_separator_projections();
    assert_eq!(
        projections
            .iter()
            .find(|projection| projection.target.container_id == 1)
            .expect("outer separator remains projected")
            .live_ratio,
        20.0_f32 / 192.0_f32
    );
    assert_eq!(
        projections
            .iter()
            .find(|projection| projection.target.container_id == 4)
            .expect("inner separator remains independently projected")
            .live_ratio,
        0.25
    );
}

#[test]
fn runtime_owned_split_drag_reprojects_without_application_projection_and_commits() {
    let mut runtime = SurfaceRuntime::new(
        SplitInteractionBridge::new(SplitInteractionMode::RuntimeOwned),
        Vector2::new(200.0, 80.0),
    );
    let target = runtime
        .layout_target_at(Point::new(52.0, 40.0))
        .expect("divider target at initial ratio");
    let initial_separator = runtime
        .split_pane_separator_projections()
        .first()
        .copied()
        .expect("initial separator projection");
    let counters = runtime.refresh_counters();
    let initial_first = runtime.layout().rects[&2];

    assert_eq!(
        runtime.dispatch_event(Event::secondary_press(Point::new(52.0, 40.0))),
        None
    );
    assert_eq!(runtime.layout_pointer_capture(), None);
    assert_eq!(runtime.layout().rects[&2], initial_first);
    let no_op_counters = runtime.refresh_counters();
    runtime.dispatch_event(Event::primary_press(Point::new(52.0, 40.0)));
    runtime.dispatch_event(Event::pointer_move(Point::new(48.0, 100.0)));
    assert_eq!(runtime.layout().rects[&2], initial_first);
    assert_eq!(runtime.refresh_counters(), no_op_counters);
    runtime.cancel_pointer_capture();
    assert_eq!(runtime.layout_pointer_capture(), None);

    assert_eq!(
        runtime.dispatch_event(Event::primary_press(Point::new(52.0, 40.0))),
        None
    );
    assert_eq!(runtime.layout_pointer_capture(), Some(target.identity()));
    assert_eq!(runtime.pointer_capture(), None);

    let moved = Point::new(130.0, 120.0);
    assert_eq!(runtime.dispatch_event(Event::pointer_move(moved)), None);
    assert_eq!(runtime.layout_pointer_capture(), Some(target.identity()));
    assert_eq!(runtime.layout().rects[&2].width(), 130.0);
    assert_eq!(
        runtime
            .layout_target_at(Point::new(134.0, 40.0))
            .expect("divider target follows the live ratio")
            .bounds,
        Rect::from_xy_size(130.0, 0.0, 8.0, 80.0)
    );
    assert_eq!(
        runtime.refresh_counters().application_projection,
        counters.application_projection,
        "live runtime-owned movement does not reproject application state"
    );
    let moved_separator = runtime
        .split_pane_separator_projections()
        .first()
        .copied()
        .expect("moved separator projection");
    assert_eq!(moved_separator.target, initial_separator.target);
    assert_eq!(
        moved_separator.mounted_state_id,
        initial_separator.mounted_state_id
    );
    assert_eq!(
        moved_separator.divider_bounds,
        Rect::from_xy_size(130.0, 0.0, 8.0, 80.0)
    );
    assert_eq!(moved_separator.live_ratio, 130.0_f32 / 192.0_f32);

    assert_eq!(
        runtime.dispatch_event(Event::pointer_release(
            moved,
            PointerButton::Primary,
            PointerModifiers::default(),
        )),
        None
    );
    assert_eq!(runtime.layout_pointer_capture(), None);
    assert_eq!(runtime.layout().rects[&2].width(), 130.0);
    assert_eq!(
        runtime
            .split_pane_separator_projections()
            .first()
            .copied()
            .expect("committed separator projection"),
        moved_separator
    );
    assert_eq!(
        runtime.refresh_counters().application_projection,
        counters.application_projection
    );
}

#[test]
fn runtime_owned_vertical_split_drag_reprojects_and_commits_outside_bounds() {
    let mut runtime = SurfaceRuntime::new(
        SplitInteractionBridge::new(SplitInteractionMode::RuntimeOwned)
            .with_axis(SplitPaneAxis::Vertical),
        Vector2::new(80.0, 200.0),
    );
    let target = runtime
        .layout_target_at(Point::new(40.0, 52.0))
        .expect("vertical divider target at initial ratio");
    assert_eq!(target.bounds, Rect::from_xy_size(0.0, 48.0, 80.0, 8.0));

    runtime.dispatch_event(Event::primary_press(Point::new(40.0, 52.0)));
    assert_eq!(runtime.layout_pointer_capture(), Some(target.identity()));
    runtime.dispatch_event(Event::pointer_move(Point::new(120.0, 130.0)));

    assert_eq!(runtime.layout().rects[&2].height(), 130.0);
    assert_eq!(
        runtime
            .layout_target_at(Point::new(40.0, 134.0))
            .expect("vertical divider follows the live ratio")
            .bounds,
        Rect::from_xy_size(0.0, 130.0, 80.0, 8.0)
    );
    let moved_separator = runtime
        .split_pane_separator_projections()
        .first()
        .copied()
        .expect("moved vertical separator projection");
    assert_eq!(moved_separator.axis, SplitPaneAxis::Vertical);
    assert_eq!(
        moved_separator.divider_bounds,
        Rect::from_xy_size(0.0, 130.0, 80.0, 8.0)
    );
    assert_eq!(moved_separator.live_ratio, 130.0_f32 / 192.0_f32);

    runtime.dispatch_event(Event::pointer_release(
        Point::new(120.0, 130.0),
        PointerButton::Primary,
        PointerModifiers::default(),
    ));
    assert_eq!(runtime.layout_pointer_capture(), None);
    assert_eq!(runtime.layout().rects[&2].height(), 130.0);
    assert_eq!(
        runtime
            .split_pane_separator_projections()
            .first()
            .expect("committed vertical separator projection")
            .live_ratio,
        130.0_f32 / 192.0_f32
    );
}

#[test]
fn runtime_owned_split_capture_cancellation_rolls_back_once_and_delayed_release_is_inert() {
    let mut runtime = SurfaceRuntime::new(
        SplitInteractionBridge::new(SplitInteractionMode::RuntimeOwned),
        Vector2::new(200.0, 80.0),
    );
    let initial_separator = runtime
        .automation_target_snapshot()
        .targets
        .into_iter()
        .find(|target| target.role == AutomationRole::Separator)
        .expect("initial semantic separator target");
    let divider = Point::new(52.0, 40.0);
    runtime.dispatch_event(Event::primary_press(divider));
    runtime.dispatch_event(Event::pointer_move(Point::new(130.0, 100.0)));
    assert_eq!(runtime.layout().rects[&2].width(), 130.0);
    assert!(runtime.layout_pointer_capture().is_some());

    runtime.cancel_pointer_capture();
    assert_eq!(runtime.layout_pointer_capture(), None);
    assert_eq!(runtime.layout().rects[&2].width(), 48.0);
    assert_eq!(
        runtime
            .split_pane_separator_projections()
            .first()
            .expect("cancelled separator projection")
            .live_ratio,
        0.25
    );
    let cancelled_separator = runtime
        .automation_target_snapshot()
        .targets
        .into_iter()
        .find(|target| target.role == AutomationRole::Separator)
        .expect("cancelled semantic separator target");
    assert_eq!(cancelled_separator.id, initial_separator.id);
    assert_eq!(cancelled_separator.value, initial_separator.value);
    assert_eq!(cancelled_separator.bounds, initial_separator.bounds);
    runtime.dispatch_event(Event::pointer_release(
        Point::new(160.0, 100.0),
        PointerButton::Primary,
        PointerModifiers::default(),
    ));
    assert_eq!(runtime.layout().rects[&2].width(), 48.0);

    runtime.dispatch_event(Event::primary_press(divider));
    runtime.dispatch_event(Event::pointer_move(Point::new(140.0, 100.0)));
    runtime.bridge_mut().policy.divider_extent = 12.0;
    runtime.refresh();
    assert_eq!(runtime.layout_pointer_capture(), None);
    assert_eq!(runtime.layout().rects[&2].width(), 47.0);
    assert_eq!(
        runtime
            .split_pane_separator_projections()
            .first()
            .expect("runtime-owned replacement projection")
            .divider_bounds,
        Rect::from_xy_size(47.0, 0.0, 12.0, 80.0)
    );
    runtime.dispatch_event(Event::pointer_release(
        Point::new(160.0, 100.0),
        PointerButton::Primary,
        PointerModifiers::default(),
    ));
    assert_eq!(runtime.layout().rects[&2].width(), 47.0);

    runtime.bridge_mut().mounted = false;
    runtime.refresh();
    assert_eq!(runtime.layout_pointer_capture(), None);
    assert!(runtime.split_pane_separator_projections().is_empty());
}

#[test]
fn runtime_owned_split_capture_cancels_on_mode_and_container_geometry_changes() {
    let mut mode_runtime = SurfaceRuntime::new(
        SplitInteractionBridge::new(SplitInteractionMode::RuntimeOwned),
        Vector2::new(200.0, 80.0),
    );
    mode_runtime.dispatch_event(Event::primary_press(Point::new(52.0, 40.0)));
    mode_runtime.dispatch_event(Event::pointer_move(Point::new(130.0, 100.0)));
    mode_runtime.bridge_mut().ratio = 0.75;
    mode_runtime.bridge_mut().generation = 2;
    mode_runtime.bridge_mut().mode = SplitInteractionMode::Controlled;
    mode_runtime.refresh();
    assert_eq!(mode_runtime.layout_pointer_capture(), None);
    assert_eq!(mode_runtime.layout().rects[&2].width(), 144.0);
    assert!(mode_runtime.split_pane_separator_projections().is_empty());
    mode_runtime.dispatch_event(Event::pointer_release(
        Point::new(160.0, 100.0),
        PointerButton::Primary,
        PointerModifiers::default(),
    ));
    assert_eq!(mode_runtime.layout().rects[&2].width(), 144.0);

    let mut geometry_runtime = SurfaceRuntime::new(
        SplitInteractionBridge::new(SplitInteractionMode::RuntimeOwned),
        Vector2::new(200.0, 80.0),
    );
    geometry_runtime.dispatch_event(Event::primary_press(Point::new(52.0, 40.0)));
    geometry_runtime.dispatch_event(Event::pointer_move(Point::new(130.0, 100.0)));
    geometry_runtime.set_viewport(Vector2::new(240.0, 80.0));
    assert_eq!(geometry_runtime.layout_pointer_capture(), None);
    assert_eq!(geometry_runtime.layout().rects[&2].width(), 58.0);
    assert_eq!(
        geometry_runtime
            .split_pane_separator_projections()
            .first()
            .expect("runtime-owned geometry replacement projection")
            .divider_bounds
            .width(),
        8.0
    );
    geometry_runtime.dispatch_event(Event::pointer_release(
        Point::new(180.0, 100.0),
        PointerButton::Primary,
        PointerModifiers::default(),
    ));
    assert_eq!(geometry_runtime.layout().rects[&2].width(), 58.0);
}

#[test]
fn runtime_owned_split_repeated_effective_moves_keep_one_bounded_capture() {
    let mut runtime = SurfaceRuntime::new(
        SplitInteractionBridge::new(SplitInteractionMode::RuntimeOwned),
        Vector2::new(200.0, 80.0),
    );
    runtime.dispatch_event(Event::primary_press(Point::new(52.0, 40.0)));
    for index in 0..100 {
        let x = 60.0 + index as f32;
        runtime.dispatch_event(Event::pointer_move(Point::new(x, 100.0)));
        assert_eq!(
            runtime.layout_pointer_capture(),
            Some(LayoutTargetIdentity::new(
                1,
                crate::gui::layout_core::SPLIT_PANE_DIVIDER_REGION_ID,
            ))
        );
    }
    assert_eq!(runtime.layout().rects[&2].width(), 159.0);
    runtime.dispatch_event(Event::pointer_release(
        Point::new(159.0, 100.0),
        PointerButton::Primary,
        PointerModifiers::default(),
    ));
    assert_eq!(runtime.layout_pointer_capture(), None);
}

#[test]
fn runtime_owned_split_settled_output_emits_once_for_meaningful_commit_and_silences_noop() {
    let messages = Rc::new(RefCell::new(Vec::new()));
    let mut runtime = SurfaceRuntime::new(
        SettledSplitInteractionBridge::new(
            SplitInteractionMode::RuntimeOwned,
            Rc::clone(&messages),
        ),
        Vector2::new(200.0, 80.0),
    );
    let divider = Point::new(52.0, 40.0);

    runtime.dispatch_event(Event::primary_press(divider));
    runtime.dispatch_event(Event::pointer_move(Point::new(130.0, 100.0)));
    assert!(
        messages.borrow().is_empty(),
        "intermediate moves are silent"
    );
    runtime.dispatch_event(Event::pointer_move(Point::new(48.0, 100.0)));
    runtime.dispatch_event(Event::pointer_release(
        Point::new(48.0, 100.0),
        PointerButton::Primary,
        PointerModifiers::default(),
    ));
    assert!(
        messages.borrow().is_empty(),
        "a move back to the start is a no-op"
    );
    assert_eq!(runtime.layout_pointer_capture(), None);

    let moved = Point::new(130.0, 100.0);
    runtime.dispatch_event(Event::primary_press(divider));
    runtime.dispatch_event(Event::pointer_move(moved));
    runtime.dispatch_event(Event::pointer_release(
        moved,
        PointerButton::Primary,
        PointerModifiers::default(),
    ));

    assert_eq!(runtime.layout_pointer_capture(), None);
    assert_eq!(
        messages.borrow().as_slice(),
        &[SplitSettledMessage::Settled {
            mapper: 1,
            ratio: 130.0_f32 / 192.0_f32,
        }]
    );
    runtime.dispatch_event(Event::pointer_release(
        moved,
        PointerButton::Primary,
        PointerModifiers::default(),
    ));
    assert_eq!(
        messages.borrow().len(),
        1,
        "one commit produces one message"
    );
}

#[test]
fn runtime_owned_split_collapse_restores_latest_horizontal_drag_ratio() {
    let messages = Rc::new(RefCell::new(Vec::new()));
    let mut bridge = SettledSplitInteractionBridge::new(
        SplitInteractionMode::RuntimeOwned,
        Rc::clone(&messages),
    )
    .with_collapse_policy(SplitPaneCollapsePolicy::FirstPane);
    bridge.policy.initial_ratio = 0.5;
    bridge.policy.first_min_extent = 40.0;
    bridge.policy.second_min_extent = 60.0;
    let mut runtime = SurfaceRuntime::new(bridge, Vector2::new(200.0, 80.0));

    let initial_divider = Point::new(100.0, 40.0);
    let moved = Point::new(130.0, 100.0);
    runtime.dispatch_event(Event::primary_press(initial_divider));
    runtime.dispatch_event(Event::pointer_move(moved));
    runtime.dispatch_event(Event::pointer_release(
        moved,
        PointerButton::Primary,
        PointerModifiers::default(),
    ));
    runtime.refresh();
    assert_eq!(runtime.layout().rects[&2].width(), 130.0);

    runtime.dispatch_event(Event::primary_double_click(Point::new(134.0, 40.0)));
    assert_eq!(runtime.layout_pointer_capture(), None);
    assert_eq!(runtime.layout().rects[&2].width(), 40.0);
    assert_eq!(runtime.layout().rects[&3].width(), 152.0);

    runtime.set_viewport(Vector2::new(240.0, 80.0));
    assert_eq!(runtime.layout().rects[&2].width(), 48.0);
    runtime.dispatch_event(Event::primary_double_click(Point::new(52.0, 40.0)));
    assert_eq!(runtime.layout_pointer_capture(), None);
    assert_eq!(runtime.layout().rects[&2].width(), 157.0);
    assert_eq!(runtime.layout().rects[&3].width(), 75.0);
    assert_eq!(
        messages.borrow().as_slice(),
        &[
            SplitSettledMessage::Settled {
                mapper: 1,
                ratio: 130.0_f32 / 192.0_f32,
            },
            SplitSettledMessage::Settled {
                mapper: 1,
                ratio: 40.0_f32 / 192.0_f32,
            },
            SplitSettledMessage::Settled {
                mapper: 1,
                ratio: 130.0_f32 / 192.0_f32,
            },
        ]
    );
}

#[test]
fn runtime_owned_vertical_split_second_pane_collapse_restores_seed_ratio() {
    let messages = Rc::new(RefCell::new(Vec::new()));
    let mut bridge = SettledSplitInteractionBridge::new(
        SplitInteractionMode::RuntimeOwned,
        Rc::clone(&messages),
    )
    .with_axis(SplitPaneAxis::Vertical)
    .with_collapse_policy(SplitPaneCollapsePolicy::SecondPane);
    bridge.policy.initial_ratio = 0.4;
    bridge.policy.first_min_extent = 30.0;
    bridge.policy.second_min_extent = 50.0;
    let mut runtime = SurfaceRuntime::new(bridge, Vector2::new(80.0, 200.0));

    runtime.dispatch_event(Event::primary_double_click(Point::new(40.0, 81.0)));
    assert_eq!(runtime.layout().rects[&2].height(), 142.0);
    assert_eq!(runtime.layout().rects[&3].height(), 50.0);

    runtime.dispatch_event(Event::primary_double_click(Point::new(40.0, 146.0)));
    assert_eq!(runtime.layout().rects[&2].height(), 77.0);
    assert_eq!(runtime.layout().rects[&3].height(), 115.0);
    assert_eq!(
        messages.borrow().as_slice(),
        &[
            SplitSettledMessage::Settled {
                mapper: 1,
                ratio: 142.0_f32 / 192.0_f32,
            },
            SplitSettledMessage::Settled {
                mapper: 1,
                ratio: 0.4,
            },
        ]
    );
}

#[test]
fn runtime_owned_split_double_activation_is_inert_during_active_drag() {
    let messages = Rc::new(RefCell::new(Vec::new()));
    let mut runtime = SurfaceRuntime::new(
        SettledSplitInteractionBridge::new(
            SplitInteractionMode::RuntimeOwned,
            Rc::clone(&messages),
        )
        .with_collapse_policy(SplitPaneCollapsePolicy::FirstPane),
        Vector2::new(200.0, 80.0),
    );

    runtime.dispatch_event(Event::primary_press(Point::new(52.0, 40.0)));
    runtime.dispatch_event(Event::pointer_move(Point::new(120.0, 100.0)));
    runtime.dispatch_event(Event::primary_double_click(Point::new(124.0, 40.0)));
    assert!(runtime.layout_pointer_capture().is_some());
    assert_eq!(runtime.layout().rects[&2].width(), 120.0);
    assert!(messages.borrow().is_empty());

    runtime.dispatch_event(Event::pointer_release(
        Point::new(120.0, 100.0),
        PointerButton::Primary,
        PointerModifiers::default(),
    ));
    assert_eq!(messages.borrow().len(), 1);
}

#[test]
fn runtime_owned_undersized_split_collapse_is_fail_closed_for_each_axis_and_policy() {
    for (axis, collapse_policy, viewport, divider_position) in [
        (
            SplitPaneAxis::Horizontal,
            SplitPaneCollapsePolicy::FirstPane,
            Vector2::new(100.0, 40.0),
            Point::new(50.0, 20.0),
        ),
        (
            SplitPaneAxis::Horizontal,
            SplitPaneCollapsePolicy::SecondPane,
            Vector2::new(100.0, 40.0),
            Point::new(50.0, 20.0),
        ),
        (
            SplitPaneAxis::Vertical,
            SplitPaneCollapsePolicy::FirstPane,
            Vector2::new(40.0, 100.0),
            Point::new(20.0, 50.0),
        ),
        (
            SplitPaneAxis::Vertical,
            SplitPaneCollapsePolicy::SecondPane,
            Vector2::new(40.0, 100.0),
            Point::new(20.0, 50.0),
        ),
    ] {
        let messages = Rc::new(RefCell::new(Vec::new()));
        let mut bridge = SettledSplitInteractionBridge::new(
            SplitInteractionMode::RuntimeOwned,
            Rc::clone(&messages),
        )
        .with_axis(axis)
        .with_collapse_policy(collapse_policy);
        bridge.policy.initial_ratio = 0.5;
        bridge.policy.divider_extent = 20.0;
        bridge.policy.first_min_extent = 60.0;
        bridge.policy.second_min_extent = 60.0;
        let mut runtime = SurfaceRuntime::new(bridge, viewport);

        let mounted_state_id = runtime
            .split_pane_separator_projections()
            .first()
            .expect("undersized split still has a valid divider projection")
            .mounted_state_id;
        let before_state = runtime
            .interaction
            .layout_state
            .lookup_current_state_view(mounted_state_id)
            .and_then(|read| read.downcast_ref::<SplitPaneRuntimeState>().copied())
            .expect("runtime-owned split state is mounted");
        let before_layout = runtime.layout().rects.clone();
        let before_layout_count = runtime.refresh_counters().layout;

        runtime.dispatch_event(Event::primary_double_click(divider_position));

        let after_state = runtime
            .interaction
            .layout_state
            .lookup_current_state_view(mounted_state_id)
            .and_then(|read| read.downcast_ref::<SplitPaneRuntimeState>().copied())
            .expect("runtime-owned split state remains mounted");
        assert_eq!(after_state, before_state);
        assert_eq!(runtime.layout().rects, before_layout);
        assert_eq!(runtime.refresh_counters().layout, before_layout_count);
        assert!(messages.borrow().is_empty());
    }
}

#[test]
fn runtime_owned_split_settled_output_is_silent_for_cancellation_loss_and_unmount() {
    let messages = Rc::new(RefCell::new(Vec::new()));
    let mut runtime = SurfaceRuntime::new(
        SettledSplitInteractionBridge::new(
            SplitInteractionMode::RuntimeOwned,
            Rc::clone(&messages),
        ),
        Vector2::new(200.0, 80.0),
    );
    runtime.dispatch_event(Event::primary_press(Point::new(52.0, 40.0)));
    runtime.dispatch_event(Event::pointer_move(Point::new(130.0, 100.0)));
    runtime.cancel_pointer_capture();
    assert!(messages.borrow().is_empty());
    runtime.dispatch_event(Event::pointer_release(
        Point::new(160.0, 100.0),
        PointerButton::Primary,
        PointerModifiers::default(),
    ));
    assert!(messages.borrow().is_empty(), "delayed release stays inert");

    let incompatible_messages = Rc::new(RefCell::new(Vec::new()));
    let mut incompatible = SurfaceRuntime::new(
        SettledSplitInteractionBridge::new(
            SplitInteractionMode::RuntimeOwned,
            Rc::clone(&incompatible_messages),
        ),
        Vector2::new(200.0, 80.0),
    );
    incompatible.dispatch_event(Event::primary_press(Point::new(52.0, 40.0)));
    incompatible.dispatch_event(Event::pointer_move(Point::new(130.0, 100.0)));
    incompatible.bridge_mut().policy.divider_extent = 12.0;
    incompatible.refresh();
    assert_eq!(incompatible.layout_pointer_capture(), None);
    assert!(incompatible_messages.borrow().is_empty());

    let unmounted_messages = Rc::new(RefCell::new(Vec::new()));
    let mut unmounted = SurfaceRuntime::new(
        SettledSplitInteractionBridge::new(
            SplitInteractionMode::RuntimeOwned,
            Rc::clone(&unmounted_messages),
        ),
        Vector2::new(200.0, 80.0),
    );
    unmounted.dispatch_event(Event::primary_press(Point::new(52.0, 40.0)));
    unmounted.dispatch_event(Event::pointer_move(Point::new(130.0, 100.0)));
    unmounted.bridge_mut().mounted = false;
    unmounted.refresh();
    assert_eq!(unmounted.layout_pointer_capture(), None);
    assert!(unmounted_messages.borrow().is_empty());
}

#[test]
fn runtime_owned_split_settled_output_is_silent_for_static_and_controlled_modes() {
    for mode in [
        SplitInteractionMode::Static,
        SplitInteractionMode::Controlled,
    ] {
        let messages = Rc::new(RefCell::new(Vec::new()));
        let mut runtime = SurfaceRuntime::new(
            SettledSplitInteractionBridge::new(mode, Rc::clone(&messages)),
            Vector2::new(200.0, 80.0),
        );
        assert_eq!(runtime.layout_target_at(Point::new(52.0, 40.0)), None);
        runtime.dispatch_event(Event::primary_press(Point::new(52.0, 40.0)));
        runtime.dispatch_event(Event::pointer_move(Point::new(130.0, 100.0)));
        runtime.dispatch_event(Event::pointer_release(
            Point::new(130.0, 100.0),
            PointerButton::Primary,
            PointerModifiers::default(),
        ));
        assert!(messages.borrow().is_empty());
        assert_eq!(runtime.layout_pointer_capture(), None);
    }
}

#[test]
fn runtime_owned_split_capture_keeps_original_mapper_and_geometry_across_refresh() {
    let messages = Rc::new(RefCell::new(Vec::new()));
    let mut runtime = SurfaceRuntime::new(
        SettledSplitInteractionBridge::new(
            SplitInteractionMode::RuntimeOwned,
            Rc::clone(&messages),
        ),
        Vector2::new(200.0, 80.0),
    );
    let moved = Point::new(130.0, 100.0);
    runtime.dispatch_event(Event::primary_press(Point::new(52.0, 40.0)));
    runtime.dispatch_event(Event::pointer_move(moved));
    let initial_generation = runtime
        .split_pane_separator_projections()
        .first()
        .expect("separator before compatible refresh")
        .mounted_state_id;
    let before_refresh_separator = runtime
        .automation_target_snapshot()
        .targets
        .into_iter()
        .find(|target| target.role == AutomationRole::Separator)
        .expect("semantic separator before compatible refresh");
    runtime.bridge_mut().mapper = 2;
    runtime.refresh();

    assert!(runtime.layout_pointer_capture().is_some());
    assert_eq!(runtime.layout().rects[&2].width(), 130.0);
    assert_eq!(
        runtime
            .split_pane_separator_projections()
            .first()
            .expect("separator after compatible refresh")
            .mounted_state_id,
        initial_generation
    );
    let after_refresh_separator = runtime
        .automation_target_snapshot()
        .targets
        .into_iter()
        .find(|target| target.role == AutomationRole::Separator)
        .expect("semantic separator after compatible refresh");
    assert_eq!(after_refresh_separator.id, before_refresh_separator.id);
    assert_eq!(
        after_refresh_separator.value,
        before_refresh_separator.value
    );
    assert_eq!(
        after_refresh_separator.bounds,
        before_refresh_separator.bounds
    );
    runtime.dispatch_event(Event::pointer_release(
        moved,
        PointerButton::Primary,
        PointerModifiers::default(),
    ));
    assert_eq!(
        messages.borrow().as_slice(),
        &[SplitSettledMessage::Settled {
            mapper: 1,
            ratio: 130.0_f32 / 192.0_f32,
        }]
    );
}

#[test]
fn runtime_owned_split_same_identity_refresh_preserves_mounted_ratio() {
    let messages = Rc::new(RefCell::new(Vec::new()));
    let mut runtime = SurfaceRuntime::new(
        SettledSplitInteractionBridge::new(
            SplitInteractionMode::RuntimeOwned,
            Rc::clone(&messages),
        ),
        Vector2::new(200.0, 80.0),
    );
    let moved = Point::new(130.0, 100.0);
    runtime.dispatch_event(Event::primary_press(Point::new(52.0, 40.0)));
    runtime.dispatch_event(Event::pointer_move(moved));
    runtime.refresh();

    assert_eq!(runtime.layout().rects[&2].width(), 130.0);
    assert!(runtime.layout_pointer_capture().is_some());
    runtime.dispatch_event(Event::pointer_release(
        moved,
        PointerButton::Primary,
        PointerModifiers::default(),
    ));
    assert_eq!(messages.borrow().len(), 1);
}
