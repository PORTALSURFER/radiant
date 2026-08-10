use super::*;
use crate::{
    gui::input::{InputSequence, InputSequenceRange, InputTimestamp},
    gui::types::{Point, Rect, Vector2},
    layout::{
        Constraints, ContainerKind, ContainerPolicy, ContainerStateDeclaration,
        LAYOUT_CAPABILITIES_CONTRACT_VERSION, LAYOUT_CAPABILITIES_PROJECTION_CONTRACT_VERSION,
        LayoutCapabilities, LayoutContainerStateContext, LayoutHitRegion, LayoutHitRegionId,
        LayoutInput, LayoutInteraction, LayoutInteractionRevision, LayoutOutput,
        LayoutTargetIdentity, OverflowPolicy, SizeModeCross, SizeModeMain, SlotParams,
    },
    runtime::{
        Command, CommandOutcome, Event, PaintPrimitive, RepaintScope, SurfaceChild, SurfaceNode,
        UiSurface, WidgetMessageMapper,
    },
    theme::ThemeTokens,
    widgets::{
        ButtonWidget, DragHandleWidget, EditPhase, FocusBehavior, FocusLossDecision,
        InteractionSource, InteractiveRowWidget, KeyboardModifiers, PointerButton,
        PointerModifiers, PointerPressAdmission, PointerShieldMessage, PointerShieldWidget,
        SliderEditBatch, TextInputWidget, TextWidget, Widget, WidgetCommon, WidgetInput, WidgetKey,
        WidgetOutput, WidgetSizing,
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
