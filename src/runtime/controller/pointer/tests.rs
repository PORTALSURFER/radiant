use super::*;
use crate::{
    gui::input::{InputSequence, InputSequenceRange, InputTimestamp},
    gui::types::{Rect, Vector2},
    layout::{Constraints, LayoutOutput, SizeModeCross, SizeModeMain, SlotParams},
    runtime::{
        Command, Event, PaintPrimitive, SurfaceChild, SurfaceNode, UiSurface, WidgetMessageMapper,
    },
    theme::ThemeTokens,
    widgets::{
        FocusBehavior, InteractiveRowWidget, PointerButton, PointerModifiers, PointerShieldMessage,
        PointerShieldWidget, TextInputWidget, Widget, WidgetCommon, WidgetInput, WidgetOutput,
        WidgetSizing,
    },
};
use std::sync::Arc;

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
fn pointer_press_on_non_focusable_hit_target_clears_existing_focus() {
    let mut runtime = SurfaceRuntime::new(FocusTestBridge, Vector2::new(200.0, 80.0));

    runtime.dispatch_event(Event::PointerPress {
        position: Point::new(4.0, 4.0),
        button: PointerButton::Primary,
        modifiers: PointerModifiers::default(),
        timestamp: None,
    });
    assert_eq!(runtime.focused_widget(), Some(10));

    runtime.dispatch_event(Event::PointerPress {
        position: Point::new(4.0, 32.0),
        button: PointerButton::Primary,
        modifiers: PointerModifiers::default(),
        timestamp: None,
    });

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
