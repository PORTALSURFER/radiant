use crate::runtime::{LayerKind, SurfaceLayer};
use crate::runtime::{SurfaceChild, SurfaceNode, WidgetMessageMapper};
use crate::{
    gui::types::{Point, Rect, Vector2},
    runtime::surface::{WidgetDispatchResult, WidgetPath},
    widgets::{
        ButtonWidget, PointerButton, ScrollbarAxis, ScrollbarWidget, WidgetInput, WidgetSizing,
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
    current.synchronize_widget_state_from_paths(&[20], &current_paths, &previous, &previous_paths);

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
    current.synchronize_widget_state_from_paths(&[20], &current_paths, &previous, &previous_paths);

    let synced = current
        .find_widget_at_path(&[1])
        .expect("layer widget exists")
        .widget()
        .as_any()
        .downcast_ref::<ScrollbarWidget>()
        .expect("layer widget stays a scrollbar");
    assert_eq!(synced.state.drag_grip_fraction, Some(0.08));
}
