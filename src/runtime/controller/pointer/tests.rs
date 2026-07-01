use super::*;
use crate::{
    gui::types::{Rect, Vector2},
    layout::{Constraints, LayoutOutput, SizeModeCross, SizeModeMain, SlotParams},
    runtime::{
        Command, Event, PaintPrimitive, SurfaceChild, SurfaceNode, UiSurface, WidgetMessageMapper,
    },
    theme::ThemeTokens,
    widgets::{
        FocusBehavior, InteractiveRowWidget, PointerButton, PointerModifiers, TextInputWidget,
        Widget, WidgetCommon, WidgetInput, WidgetOutput, WidgetSizing,
    },
};
use std::sync::Arc;

struct FocusTestBridge;

impl RuntimeBridge<usize> for FocusTestBridge {
    fn project_surface(&mut self) -> Arc<UiSurface<usize>> {
        Arc::new(UiSurface::new(SurfaceNode::column(
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
        Arc::new(UiSurface::new(SurfaceNode::widget(
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

impl RuntimeBridge<()> for PointerSnapshotBridge {
    fn project_surface(&mut self) -> Arc<UiSurface<()>> {
        Arc::new(UiSurface::new(SurfaceNode::container(
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
fn pointer_press_on_non_focusable_hit_target_clears_existing_focus() {
    let mut runtime = SurfaceRuntime::new(FocusTestBridge, Vector2::new(200.0, 80.0));

    runtime.dispatch_event(Event::PointerPress {
        position: Point::new(4.0, 4.0),
        button: PointerButton::Primary,
        modifiers: PointerModifiers::default(),
    });
    assert_eq!(runtime.focused_widget(), Some(10));

    runtime.dispatch_event(Event::PointerPress {
        position: Point::new(4.0, 32.0),
        button: PointerButton::Primary,
        modifiers: PointerModifiers::default(),
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
    runtime.dispatch_input(
        10,
        WidgetInput::PointerMove {
            position: Point::new(4.0, 4.0),
        },
    );
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

    runtime.dispatch_input(
        10,
        WidgetInput::PointerMove {
            position: Point::new(4.0, 4.0),
        },
    );
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
