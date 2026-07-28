use crate::runtime::{LayerKind, SurfaceLayer, surface::WidgetStateSyncPolicy};
use crate::runtime::{SurfaceChild, SurfaceNode, WidgetMessageMapper};
use crate::{
    gui::types::{Point, Rect, Vector2},
    runtime::surface::{WidgetDispatchResult, WidgetPath},
    widgets::{
        ButtonWidget, KnobMessage, KnobWidget, PointerButton, ScrollbarAxis, ScrollbarWidget,
        WidgetInput, WidgetSizing,
    },
};
use std::collections::HashMap;

#[test]
fn scene_without_layers_routes_base_widget_at_transparent_path() {
    let mut root: SurfaceNode<()> = SurfaceNode::scene(
        1,
        SurfaceNode::widget(
            ButtonWidget::new(10, "Base", WidgetSizing::fixed(Vector2::new(80.0, 28.0))),
            WidgetMessageMapper::none(),
        ),
        Vec::new(),
    );

    let result = root.dispatch_input_at_path(
        10,
        &[],
        Rect::from_min_size(Point::new(0.0, 0.0), Vector2::new(80.0, 28.0)),
        WidgetInput::PointerMove {
            position: Point::new(8.0, 8.0),
        },
    );

    assert!(matches!(result, Some(WidgetDispatchResult::NoOutput)));
    assert!(
        root.find_widget_at_path(&[])
            .expect("base widget exists at transparent path")
            .widget()
            .common()
            .state
            .hovered
    );
}

fn mapped_knob(value: f32, disabled: bool) -> SurfaceNode<KnobMessage> {
    let mut knob = KnobWidget::new(30, value);
    knob.common.state.disabled = disabled;
    SurfaceNode::widget(
        knob,
        WidgetMessageMapper::typed(|message: KnobMessage| message),
    )
}

#[test]
fn mapped_knob_reprojection_preserves_pointer_gesture_and_authoritative_value() {
    let bounds = Rect::from_min_size(Point::default(), Vector2::new(40.0, 40.0));
    let paths = HashMap::from([(30, WidgetPath::from_slice(&[]))]);
    let mut previous = mapped_knob(0.5, false);
    let mut current = mapped_knob(0.5, false);

    assert!(matches!(
        previous.dispatch_input_at_path(
            30,
            &[],
            bounds,
            WidgetInput::primary_press(Point::new(20.0, 20.0)),
        ),
        Some(WidgetDispatchResult::Message(KnobMessage::GestureStarted {
            value: 0.5
        }))
    ));
    current.synchronize_widget_state_from_paths(
        &[30],
        &paths,
        &previous,
        &paths,
        WidgetStateSyncPolicy::default(),
    );
    assert!(matches!(
        current.dispatch_input_at_path(
            30,
            &[],
            bounds,
            WidgetInput::pointer_move(Point::new(20.0, 10.0)),
        ),
        Some(WidgetDispatchResult::Message(KnobMessage::ValueChanged { value }))
            if value > 0.5
    ));

    // The reducer's fresh projection owns the value while the active pointer
    // gesture remains runtime-owned across this refresh.
    previous = current;
    current = mapped_knob(0.62, false);
    current.synchronize_widget_state_from_paths(
        &[30],
        &paths,
        &previous,
        &paths,
        WidgetStateSyncPolicy::default(),
    );
    let final_value = current
        .find_widget_at_path(&[])
        .expect("knob exists")
        .widget()
        .as_any()
        .downcast_ref::<KnobWidget>()
        .expect("knob type is retained")
        .state
        .value;
    assert_eq!(final_value, 0.62);
    assert!(matches!(
        current.dispatch_input_at_path(
            30,
            &[],
            bounds,
            WidgetInput::primary_release(Point::new(20.0, 10.0))
        ),
        Some(WidgetDispatchResult::Message(KnobMessage::GestureEnded {
            value: 0.62
        }))
    ));
}

#[test]
fn disabled_knob_reprojection_clears_pointer_gesture_state() {
    let bounds = Rect::from_min_size(Point::default(), Vector2::new(40.0, 40.0));
    let paths = HashMap::from([(30, WidgetPath::from_slice(&[]))]);
    let mut previous = mapped_knob(0.5, false);
    assert!(matches!(
        previous.dispatch_input_at_path(
            30,
            &[],
            bounds,
            WidgetInput::primary_press(Point::new(20.0, 20.0)),
        ),
        Some(WidgetDispatchResult::Message(
            KnobMessage::GestureStarted { .. }
        ))
    ));
    let mut current = mapped_knob(0.7, true);
    current.synchronize_widget_state_from_paths(
        &[30],
        &paths,
        &previous,
        &paths,
        WidgetStateSyncPolicy::default(),
    );
    assert!(matches!(
        current.dispatch_input_at_path(
            30,
            &[],
            bounds,
            WidgetInput::pointer_move(Point::new(20.0, 10.0))
        ),
        Some(WidgetDispatchResult::NoOutput)
    ));
    assert!(matches!(
        current.dispatch_input_at_path(
            30,
            &[],
            bounds,
            WidgetInput::primary_double_click(Point::new(20.0, 20.0))
        ),
        Some(WidgetDispatchResult::NoOutput)
    ));
    assert!(matches!(
        current.dispatch_input_at_path(
            30,
            &[],
            bounds,
            WidgetInput::KeyPress(crate::widgets::WidgetKey::ArrowRight)
        ),
        Some(WidgetDispatchResult::NoOutput)
    ));
    assert!(matches!(
        current.dispatch_input_at_path(
            30,
            &[],
            bounds,
            WidgetInput::primary_release(Point::new(20.0, 10.0))
        ),
        Some(WidgetDispatchResult::Message(KnobMessage::GestureEnded {
            value: 0.7
        }))
    ));
    assert!(matches!(
        current.dispatch_input_at_path(
            30,
            &[],
            bounds,
            WidgetInput::primary_release(Point::new(20.0, 10.0))
        ),
        Some(WidgetDispatchResult::NoOutput)
    ));
    assert!(matches!(
        current.dispatch_input_at_path(30, &[], bounds, WidgetInput::FocusChanged(false)),
        Some(WidgetDispatchResult::NoOutput)
    ));
    let knob = current
        .find_widget_at_path(&[])
        .expect("disabled knob exists")
        .widget()
        .as_any()
        .downcast_ref::<KnobWidget>()
        .expect("knob type is retained");
    assert!(!knob.common.state.pressed);
    assert_eq!(knob.state.gesture_origin, None);
    assert_eq!(knob.state.value, 0.7);
}

#[test]
fn dispatch_input_at_child_path_routes_without_tree_search() {
    let mut root: SurfaceNode<()> = SurfaceNode::column(
        1,
        0.0,
        vec![
            SurfaceChild::fill(SurfaceNode::widget(
                ButtonWidget::new(10, "First", WidgetSizing::fixed(Vector2::new(80.0, 28.0))),
                WidgetMessageMapper::none(),
            )),
            SurfaceChild::fill(SurfaceNode::widget(
                ButtonWidget::new(20, "Second", WidgetSizing::fixed(Vector2::new(80.0, 28.0))),
                WidgetMessageMapper::none(),
            )),
        ],
    );

    let result = root.dispatch_input_at_path(
        20,
        &[1],
        Rect::from_min_size(Point::new(0.0, 0.0), Vector2::new(80.0, 28.0)),
        WidgetInput::PointerMove {
            position: Point::new(8.0, 8.0),
        },
    );

    assert!(matches!(result, Some(WidgetDispatchResult::NoOutput)));
    assert!(
        root.find_widget(20)
            .expect("target widget exists")
            .widget()
            .common()
            .state
            .hovered
    );
    assert!(
        !root
            .find_widget(10)
            .expect("sibling widget exists")
            .widget()
            .common()
            .state
            .hovered
    );
}

#[test]
fn find_widget_at_child_path_returns_only_the_target_leaf() {
    let root: SurfaceNode<()> = SurfaceNode::column(
        1,
        0.0,
        vec![
            SurfaceChild::fill(SurfaceNode::widget(
                ButtonWidget::new(10, "First", WidgetSizing::fixed(Vector2::new(80.0, 28.0))),
                WidgetMessageMapper::none(),
            )),
            SurfaceChild::fill(SurfaceNode::widget(
                ButtonWidget::new(20, "Second", WidgetSizing::fixed(Vector2::new(80.0, 28.0))),
                WidgetMessageMapper::none(),
            )),
        ],
    );

    assert_eq!(
        root.find_widget_at_path(&[1])
            .expect("target widget exists")
            .id(),
        20
    );
    assert!(root.find_widget_at_path(&[2]).is_none());
}

#[test]
fn synchronize_widget_state_from_paths_preserves_state_after_reorder() {
    let mut previous: SurfaceNode<()> = SurfaceNode::column(
        1,
        0.0,
        vec![
            SurfaceChild::fill(SurfaceNode::widget(
                ButtonWidget::new(10, "First", WidgetSizing::fixed(Vector2::new(80.0, 28.0))),
                WidgetMessageMapper::none(),
            )),
            SurfaceChild::fill(SurfaceNode::widget(
                ScrollbarWidget::new(
                    20,
                    ScrollbarAxis::Vertical,
                    WidgetSizing::fixed(Vector2::new(16.0, 100.0)),
                ),
                WidgetMessageMapper::none(),
            )),
        ],
    );
    let mut current: SurfaceNode<()> = SurfaceNode::column(
        1,
        0.0,
        vec![
            SurfaceChild::fill(SurfaceNode::widget(
                ScrollbarWidget::new(
                    20,
                    ScrollbarAxis::Vertical,
                    WidgetSizing::fixed(Vector2::new(16.0, 100.0)),
                ),
                WidgetMessageMapper::none(),
            )),
            SurfaceChild::fill(SurfaceNode::widget(
                ButtonWidget::new(10, "First", WidgetSizing::fixed(Vector2::new(80.0, 28.0))),
                WidgetMessageMapper::none(),
            )),
        ],
    );

    let _ = previous.dispatch_input_at_path(
        20,
        &[1],
        Rect::from_min_size(Point::new(0.0, 0.0), Vector2::new(16.0, 100.0)),
        WidgetInput::PointerPress {
            position: Point::new(8.0, 8.0),
            button: PointerButton::Primary,
            modifiers: Default::default(),
        },
    );

    let previous_paths = HashMap::from([
        (10, WidgetPath::from_slice(&[0])),
        (20, WidgetPath::from_slice(&[1])),
    ]);
    let current_paths = HashMap::from([
        (20, WidgetPath::from_slice(&[0])),
        (10, WidgetPath::from_slice(&[1])),
    ]);
    current.synchronize_widget_state_from_paths(
        &[20],
        &current_paths,
        &previous,
        &previous_paths,
        WidgetStateSyncPolicy::default(),
    );

    let moved = current
        .find_widget_at_path(&[0])
        .expect("moved widget exists")
        .widget()
        .as_any()
        .downcast_ref::<ScrollbarWidget>()
        .expect("moved widget stays a scrollbar");
    assert_eq!(moved.state.drag_grip_fraction, Some(0.08));
}

#[test]
fn synchronize_widget_state_from_paths_skips_incompatible_replacement() {
    let mut previous: SurfaceNode<()> = SurfaceNode::widget(
        ButtonWidget::new(
            20,
            "Previous",
            WidgetSizing::fixed(Vector2::new(80.0, 28.0)),
        ),
        WidgetMessageMapper::none(),
    );
    let mut current: SurfaceNode<()> = SurfaceNode::widget(
        ScrollbarWidget::new(
            20,
            ScrollbarAxis::Vertical,
            WidgetSizing::fixed(Vector2::new(16.0, 100.0)),
        ),
        WidgetMessageMapper::none(),
    );
    let _ = previous.dispatch_input_at_path(
        20,
        &[],
        Rect::from_min_size(Point::default(), Vector2::new(80.0, 28.0)),
        WidgetInput::PointerPress {
            position: Point::new(8.0, 8.0),
            button: PointerButton::Primary,
            modifiers: Default::default(),
        },
    );
    let paths = HashMap::from([(20, WidgetPath::from_slice(&[]))]);

    current.synchronize_widget_state_from_paths(
        &[20],
        &paths,
        &previous,
        &paths,
        WidgetStateSyncPolicy::default(),
    );

    assert!(
        !current
            .find_widget_at_path(&[])
            .expect("replacement exists")
            .widget()
            .common()
            .state
            .pressed
    );
}

#[test]
fn scene_widget_state_sync_finds_widgets_inside_layers() {
    let mut previous: SurfaceNode<()> = SurfaceNode::scene(
        1,
        SurfaceNode::widget(
            ButtonWidget::new(10, "Base", WidgetSizing::fixed(Vector2::new(80.0, 28.0))),
            WidgetMessageMapper::none(),
        ),
        vec![SurfaceLayer::new(
            LayerKind::Modal,
            SurfaceNode::widget(
                ScrollbarWidget::new(
                    20,
                    ScrollbarAxis::Vertical,
                    WidgetSizing::fixed(Vector2::new(16.0, 100.0)),
                ),
                WidgetMessageMapper::none(),
            ),
        )],
    );
    let mut current: SurfaceNode<()> = SurfaceNode::scene(
        1,
        SurfaceNode::widget(
            ButtonWidget::new(10, "Base", WidgetSizing::fixed(Vector2::new(80.0, 28.0))),
            WidgetMessageMapper::none(),
        ),
        vec![SurfaceLayer::new(
            LayerKind::Modal,
            SurfaceNode::widget(
                ScrollbarWidget::new(
                    20,
                    ScrollbarAxis::Vertical,
                    WidgetSizing::fixed(Vector2::new(16.0, 100.0)),
                ),
                WidgetMessageMapper::none(),
            ),
        )],
    );

    let _ = previous.dispatch_input_at_path(
        20,
        &[1],
        Rect::from_min_size(Point::new(0.0, 0.0), Vector2::new(16.0, 100.0)),
        WidgetInput::PointerPress {
            position: Point::new(8.0, 8.0),
            button: PointerButton::Primary,
            modifiers: Default::default(),
        },
    );

    let previous_paths = HashMap::from([(20, WidgetPath::from_slice(&[1]))]);
    let current_paths = HashMap::from([(20, WidgetPath::from_slice(&[1]))]);
    current.synchronize_widget_state_from_paths(
        &[20],
        &current_paths,
        &previous,
        &previous_paths,
        WidgetStateSyncPolicy::default(),
    );

    let synced = current
        .find_widget_at_path(&[1])
        .expect("layer widget exists")
        .widget()
        .as_any()
        .downcast_ref::<ScrollbarWidget>()
        .expect("layer widget stays a scrollbar");
    assert_eq!(synced.state.drag_grip_fraction, Some(0.08));
}

#[test]
fn exclusive_pointer_capture_sync_clears_non_captured_hover_state() {
    let mut previous: SurfaceNode<()> = SurfaceNode::column(
        1,
        0.0,
        vec![
            SurfaceChild::fill(SurfaceNode::widget(
                ButtonWidget::new(10, "Hover", WidgetSizing::fixed(Vector2::new(80.0, 28.0))),
                WidgetMessageMapper::none(),
            )),
            SurfaceChild::fill(SurfaceNode::widget(
                ButtonWidget::new(
                    20,
                    "Captured",
                    WidgetSizing::fixed(Vector2::new(80.0, 28.0)),
                ),
                WidgetMessageMapper::none(),
            )),
        ],
    );
    let mut current = previous.clone();

    let _ = previous.dispatch_input_at_path(
        10,
        &[0],
        Rect::from_min_size(Point::new(0.0, 0.0), Vector2::new(80.0, 28.0)),
        WidgetInput::PointerMove {
            position: Point::new(8.0, 8.0),
        },
    );
    let _ = previous.dispatch_input_at_path(
        20,
        &[1],
        Rect::from_min_size(Point::new(0.0, 28.0), Vector2::new(80.0, 28.0)),
        WidgetInput::PointerPress {
            position: Point::new(8.0, 36.0),
            button: PointerButton::Primary,
            modifiers: Default::default(),
        },
    );

    let previous_paths = HashMap::from([
        (10, WidgetPath::from_slice(&[0])),
        (20, WidgetPath::from_slice(&[1])),
    ]);
    let current_paths = previous_paths.clone();
    current.synchronize_widget_state_from_paths(
        &[10, 20],
        &current_paths,
        &previous,
        &previous_paths,
        WidgetStateSyncPolicy::exclusive_pointer_capture(20),
    );

    assert!(
        !current
            .find_widget_at_path(&[0])
            .expect("hover widget exists")
            .widget()
            .common()
            .state
            .hovered
    );
    assert!(
        current
            .find_widget_at_path(&[1])
            .expect("captured widget exists")
            .widget()
            .common()
            .state
            .pressed
    );
}

#[test]
fn retained_state_sync_keeps_only_current_hover_owner() {
    let mut previous: SurfaceNode<()> = SurfaceNode::column(
        1,
        0.0,
        vec![
            SurfaceChild::fill(SurfaceNode::widget(
                ButtonWidget::new(
                    10,
                    "Previous",
                    WidgetSizing::fixed(Vector2::new(80.0, 28.0)),
                ),
                WidgetMessageMapper::none(),
            )),
            SurfaceChild::fill(SurfaceNode::widget(
                ButtonWidget::new(20, "Current", WidgetSizing::fixed(Vector2::new(80.0, 28.0))),
                WidgetMessageMapper::none(),
            )),
        ],
    );
    let mut current = previous.clone();

    let _ = previous.dispatch_input_at_path(
        10,
        &[0],
        Rect::from_min_size(Point::new(0.0, 0.0), Vector2::new(80.0, 28.0)),
        WidgetInput::PointerMove {
            position: Point::new(8.0, 8.0),
        },
    );
    let _ = previous.dispatch_input_at_path(
        20,
        &[1],
        Rect::from_min_size(Point::new(0.0, 28.0), Vector2::new(80.0, 28.0)),
        WidgetInput::PointerMove {
            position: Point::new(8.0, 36.0),
        },
    );

    let previous_paths = HashMap::from([
        (10, WidgetPath::from_slice(&[0])),
        (20, WidgetPath::from_slice(&[1])),
    ]);
    let current_paths = previous_paths.clone();
    current.synchronize_widget_state_from_paths(
        &[10, 20],
        &current_paths,
        &previous,
        &previous_paths,
        WidgetStateSyncPolicy::retained_hover_owner(Some(20)),
    );

    assert!(
        !current
            .find_widget_at_path(&[0])
            .expect("previous hover widget exists")
            .widget()
            .common()
            .state
            .hovered
    );
    assert!(
        current
            .find_widget_at_path(&[1])
            .expect("current hover widget exists")
            .widget()
            .common()
            .state
            .hovered
    );
}

#[test]
fn retained_state_sync_clears_all_hover_when_pointer_has_no_owner() {
    let mut previous: SurfaceNode<()> = SurfaceNode::column(
        1,
        0.0,
        vec![SurfaceChild::fill(SurfaceNode::widget(
            ButtonWidget::new(
                10,
                "Previous",
                WidgetSizing::fixed(Vector2::new(80.0, 28.0)),
            ),
            WidgetMessageMapper::none(),
        ))],
    );
    let mut current = previous.clone();

    let _ = previous.dispatch_input_at_path(
        10,
        &[0],
        Rect::from_min_size(Point::new(0.0, 0.0), Vector2::new(80.0, 28.0)),
        WidgetInput::PointerMove {
            position: Point::new(8.0, 8.0),
        },
    );

    let paths = HashMap::from([(10, WidgetPath::from_slice(&[0]))]);
    current.synchronize_widget_state_from_paths(
        &[10],
        &paths,
        &previous,
        &paths,
        WidgetStateSyncPolicy::retained_hover_owner(None),
    );

    assert!(
        !current
            .find_widget_at_path(&[0])
            .expect("previous hover widget exists")
            .widget()
            .common()
            .state
            .hovered
    );
}
