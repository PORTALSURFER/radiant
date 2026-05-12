use super::{GenericNativeRuntimeCore, demo_bridge};
use crate::{
    layout::{Point, Rect, Vector2},
    runtime::{PaintPrimitive, RuntimeBridge, SurfaceNode, UiSurface, WidgetMessageMapper},
    widgets::{Widget, WidgetCommon, WidgetInput, WidgetOutput, WidgetSizing},
};
use std::sync::Arc;

#[test]
fn pointer_move_inside_same_widget_does_not_request_redundant_redraw() {
    let bridge = demo_bridge();
    let mut core = GenericNativeRuntimeCore::new(bridge, Vector2::new(320.0, 40.0));
    let button_rect = core
        .runtime
        .layout()
        .rects
        .get(&11)
        .copied()
        .expect("button should be laid out");
    let first_point = Point::new(button_rect.min.x + 2.0, button_rect.min.y + 2.0);
    let second_point = Point::new(button_rect.min.x + 4.0, button_rect.min.y + 2.0);

    let first = core.route_pointer_move(first_point);
    assert!(first.routed);
    assert!(first.needs_redraw());

    let second = core.route_pointer_move(second_point);
    assert!(second.routed);
    assert!(!second.needs_redraw());
}

#[test]
fn pointer_move_message_inside_same_widget_still_requests_redraw() {
    let mut core =
        GenericNativeRuntimeCore::new(PointerMoveBridge::default(), Vector2::new(120.0, 40.0));
    let point = core
        .runtime
        .layout()
        .rects
        .get(&71)
        .map(|rect| Point::new(rect.min.x + 2.0, rect.min.y + 2.0))
        .expect("pointer widget should be laid out");

    let first = core.route_pointer_move(point);
    assert!(first.routed);
    assert!(first.needs_redraw());
    let second = core.route_pointer_move(Point::new(point.x + 1.0, point.y));

    assert!(second.routed);
    assert!(second.needs_redraw());
    assert_eq!(core.runtime.bridge().moves, 2);
}

#[test]
fn local_pointer_move_state_inside_same_widget_requests_redraw() {
    let mut core = GenericNativeRuntimeCore::new(LocalPointerMoveBridge, Vector2::new(120.0, 40.0));
    let point = core
        .runtime
        .layout()
        .rects
        .get(&72)
        .map(|rect| Point::new(rect.min.x + 2.0, rect.min.y + 2.0))
        .expect("local pointer widget should be laid out");

    let first = core.route_pointer_move(point);
    assert!(first.routed);
    assert!(first.needs_redraw());
    let second = core.route_pointer_move(Point::new(point.x + 1.0, point.y));

    assert!(second.routed);
    assert!(second.needs_redraw());
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct PointerMoveMessage;

#[derive(Clone, Debug)]
struct PointerMoveWidget {
    common: WidgetCommon,
}

impl PointerMoveWidget {
    fn new() -> Self {
        Self {
            common: WidgetCommon::new(71, WidgetSizing::fixed(Vector2::new(80.0, 24.0))),
        }
    }
}

impl Widget for PointerMoveWidget {
    fn common(&self) -> &WidgetCommon {
        &self.common
    }

    fn common_mut(&mut self) -> &mut WidgetCommon {
        &mut self.common
    }

    fn handle_input(&mut self, _bounds: Rect, input: WidgetInput) -> Option<WidgetOutput> {
        matches!(input, WidgetInput::PointerMove { .. })
            .then(|| WidgetOutput::typed(PointerMoveMessage))
    }

    fn append_paint(
        &self,
        _primitives: &mut Vec<PaintPrimitive>,
        _bounds: Rect,
        _layout: &crate::layout::LayoutOutput,
        _theme: &crate::theme::ThemeTokens,
    ) {
    }
}

#[derive(Default)]
struct PointerMoveBridge {
    moves: usize,
}

impl RuntimeBridge<PointerMoveMessage> for PointerMoveBridge {
    fn project_surface(&mut self) -> Arc<UiSurface<PointerMoveMessage>> {
        Arc::new(UiSurface::new(SurfaceNode::custom_widget(
            PointerMoveWidget::new(),
            WidgetMessageMapper::typed(|message: PointerMoveMessage| message),
        )))
    }

    fn reduce_message(&mut self, _message: PointerMoveMessage) {
        self.moves += 1;
    }
}

#[derive(Clone, Debug)]
struct LocalPointerMoveWidget {
    common: WidgetCommon,
    last_position: Option<Point>,
}

impl LocalPointerMoveWidget {
    fn new() -> Self {
        let mut common = WidgetCommon::new(72, WidgetSizing::fixed(Vector2::new(80.0, 24.0)));
        common.focus = crate::widgets::FocusBehavior::Pointer;
        Self {
            common,
            last_position: None,
        }
    }
}

impl Widget for LocalPointerMoveWidget {
    fn common(&self) -> &WidgetCommon {
        &self.common
    }

    fn common_mut(&mut self) -> &mut WidgetCommon {
        &mut self.common
    }

    fn handle_input(&mut self, _bounds: Rect, input: WidgetInput) -> Option<WidgetOutput> {
        if let WidgetInput::PointerMove { position } = input {
            self.last_position = Some(position);
        }
        None
    }

    fn append_paint(
        &self,
        _primitives: &mut Vec<PaintPrimitive>,
        _bounds: Rect,
        _layout: &crate::layout::LayoutOutput,
        _theme: &crate::theme::ThemeTokens,
    ) {
    }
}

struct LocalPointerMoveBridge;

impl RuntimeBridge<()> for LocalPointerMoveBridge {
    fn project_surface(&mut self) -> Arc<UiSurface<()>> {
        Arc::new(UiSurface::new(SurfaceNode::custom_widget(
            LocalPointerMoveWidget::new(),
            WidgetMessageMapper::none(),
        )))
    }

    fn reduce_message(&mut self, _message: ()) {}
}
