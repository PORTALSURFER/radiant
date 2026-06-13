use super::*;
use crate::{
    gui::types::Vector2,
    layout::{Constraints, SizeModeCross, SizeModeMain, SlotParams},
    runtime::{Event, SurfaceChild, SurfaceNode, UiSurface, WidgetMessageMapper},
    widgets::{
        FocusBehavior, InteractiveRowWidget, PointerButton, PointerModifiers, TextInputWidget,
        WidgetInput, WidgetSizing,
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
