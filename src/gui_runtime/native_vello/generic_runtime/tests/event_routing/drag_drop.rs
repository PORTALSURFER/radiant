use super::super::*;

#[test]
fn captured_release_routes_pointer_drop_to_widget_under_release_point() {
    let mut core = GenericNativeRuntimeCore::new(DropBridge::default(), Vector2::new(220.0, 32.0));
    let source_point = core
        .runtime
        .layout()
        .rects
        .get(&71)
        .map(|rect| Point::new(rect.min.x + 4.0, rect.min.y + 4.0))
        .expect("source should be laid out");
    let target_point = core
        .runtime
        .layout()
        .rects
        .get(&72)
        .map(|rect| Point::new(rect.min.x + 4.0, rect.min.y + 4.0))
        .expect("target should be laid out");

    assert!(
        core.route_pointer_press(source_point, PointerButton::Primary)
            .routed
    );
    let _ = core.route_pointer_release(target_point, PointerButton::Primary);

    assert_eq!(core.runtime.bridge().drops, 1);
}

#[test]
fn captured_drag_routes_pointer_move_to_hovered_drop_target() {
    let mut core = GenericNativeRuntimeCore::new(DropBridge::default(), Vector2::new(220.0, 32.0));
    let source_point = core
        .runtime
        .layout()
        .rects
        .get(&71)
        .map(|rect| Point::new(rect.min.x + 4.0, rect.min.y + 4.0))
        .expect("source should be laid out");
    let target_point = core
        .runtime
        .layout()
        .rects
        .get(&72)
        .map(|rect| Point::new(rect.min.x + 4.0, rect.min.y + 4.0))
        .expect("target should be laid out");

    assert!(
        core.route_pointer_press(source_point, PointerButton::Primary)
            .routed
    );
    let outcome = core.route_pointer_move(target_point);

    assert!(outcome.routed);
    assert_eq!(core.runtime.bridge().hovers, 1);
    assert_eq!(core.runtime.pointer_capture(), Some(71));
}

#[test]
fn captured_drag_routes_pointer_move_to_drop_target_after_surface_refresh() {
    let mut core = GenericNativeRuntimeCore::new(DropBridge::default(), Vector2::new(220.0, 32.0));
    let source_point = core
        .runtime
        .layout()
        .rects
        .get(&71)
        .map(|rect| Point::new(rect.min.x + 4.0, rect.min.y + 4.0))
        .expect("source should be laid out");
    let target_point = core
        .runtime
        .layout()
        .rects
        .get(&72)
        .map(|rect| Point::new(rect.min.x + 4.0, rect.min.y + 4.0))
        .expect("target should be laid out");

    assert!(
        core.route_pointer_press(source_point, PointerButton::Primary)
            .routed
    );
    core.runtime.refresh();
    let outcome = core.route_pointer_move(target_point);

    assert!(outcome.routed);
    assert_eq!(core.runtime.bridge().hovers, 1);
    assert_eq!(core.runtime.pointer_capture(), Some(71));
}

#[test]
fn captured_drag_handle_does_not_route_pointer_move_to_hovered_widget() {
    let mut core = GenericNativeRuntimeCore::new(
        DragHandlePassThroughBridge::default(),
        Vector2::new(220.0, 32.0),
    );
    let source_point = core
        .runtime
        .layout()
        .rects
        .get(&81)
        .map(|rect| Point::new(rect.min.x + 4.0, rect.min.y + 4.0))
        .expect("drag handle should be laid out");
    let target_point = core
        .runtime
        .layout()
        .rects
        .get(&82)
        .map(|rect| Point::new(rect.min.x + 4.0, rect.min.y + 4.0))
        .expect("hover target should be laid out");

    assert!(
        core.route_pointer_press(source_point, PointerButton::Primary)
            .routed
    );
    let outcome = core.route_pointer_move(target_point);

    assert!(outcome.routed);
    assert_eq!(core.runtime.bridge().hovers, 0);
    assert_eq!(core.runtime.pointer_capture(), Some(81));
}

#[test]
fn captured_drag_hover_message_requests_scene_rebuild_without_hover_change() {
    let mut core = GenericNativeRuntimeCore::new(DropBridge::default(), Vector2::new(220.0, 32.0));
    let source_point = core
        .runtime
        .layout()
        .rects
        .get(&71)
        .map(|rect| Point::new(rect.min.x + 4.0, rect.min.y + 4.0))
        .expect("source should be laid out");
    let target_point = core
        .runtime
        .layout()
        .rects
        .get(&72)
        .map(|rect| Point::new(rect.min.x + 4.0, rect.min.y + 4.0))
        .expect("target should be laid out");

    assert!(
        core.route_pointer_press(source_point, PointerButton::Primary)
            .routed
    );
    let _ = core.route_pointer_move(target_point);
    let outcome = core.route_pointer_move(Point::new(target_point.x + 2.0, target_point.y));

    assert!(outcome.routed);
    assert!(
        outcome.needs_scene_rebuild(),
        "captured drag hover messages mutate app state and must rebuild the scene, not only repaint the drag preview"
    );
    assert!(!outcome.paint_only_requested);
    assert_eq!(core.runtime.bridge().hovers, 2);
}

#[derive(Default)]
struct DragHandlePassThroughBridge {
    hovers: usize,
}

impl RuntimeBridge<DropMessage> for DragHandlePassThroughBridge {
    fn project_surface(&mut self) -> Arc<UiSurface<DropMessage>> {
        let source = DragHandleWidget::new(81, WidgetSizing::fixed(Vector2::new(16.0, 24.0)));
        Arc::new(UiSurface::new(SurfaceNode::container(
            80,
            ContainerPolicy {
                kind: ContainerKind::Row,
                spacing: 8.0,
                ..ContainerPolicy::default()
            },
            vec![
                SurfaceChild::new(
                    SlotParams::fill(),
                    SurfaceNode::widget(source, WidgetMessageMapper::none()),
                ),
                SurfaceChild::new(
                    SlotParams::fill(),
                    SurfaceNode::custom_widget(
                        HoverTargetWidget::new(82),
                        WidgetMessageMapper::typed(|message: DropMessage| message),
                    ),
                ),
            ],
        )))
    }

    fn reduce_message(&mut self, message: DropMessage) {
        if matches!(message, DropMessage::TargetHover) {
            self.hovers += 1;
        }
    }
}

#[derive(Clone, Debug)]
struct HoverTargetWidget {
    common: WidgetCommon,
}

impl HoverTargetWidget {
    fn new(id: WidgetId) -> Self {
        Self {
            common: WidgetCommon::new(id, WidgetSizing::fixed(Vector2::new(88.0, 24.0))),
        }
    }
}

impl Widget for HoverTargetWidget {
    fn common(&self) -> &WidgetCommon {
        &self.common
    }

    fn common_mut(&mut self) -> &mut WidgetCommon {
        &mut self.common
    }

    fn handle_input(&mut self, _bounds: Rect, input: WidgetInput) -> Option<WidgetOutput> {
        matches!(input, WidgetInput::PointerMove { .. })
            .then_some(WidgetOutput::typed(DropMessage::TargetHover))
    }

    fn accepts_pointer_move(&self) -> bool {
        true
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

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum DropMessage {
    Source,
    TargetHover,
    TargetDrop,
}

#[derive(Default)]
struct DropBridge {
    hovers: usize,
    drops: usize,
}

impl RuntimeBridge<DropMessage> for DropBridge {
    fn project_surface(&mut self) -> Arc<UiSurface<DropMessage>> {
        let source = ButtonWidget::new(71, "Source", WidgetSizing::fixed(Vector2::new(88.0, 24.0)));
        Arc::new(UiSurface::new(SurfaceNode::container(
            70,
            ContainerPolicy {
                kind: ContainerKind::Row,
                spacing: 8.0,
                ..ContainerPolicy::default()
            },
            vec![
                SurfaceChild::new(
                    SlotParams::fill(),
                    SurfaceNode::widget(
                        source,
                        WidgetMessageMapper::button(|_| DropMessage::Source),
                    ),
                ),
                SurfaceChild::new(
                    SlotParams::fill(),
                    SurfaceNode::custom_widget(
                        DropTargetWidget::new(),
                        WidgetMessageMapper::typed(|message: DropMessage| message),
                    ),
                ),
            ],
        )))
    }

    fn reduce_message(&mut self, message: DropMessage) {
        match message {
            DropMessage::TargetHover => self.hovers += 1,
            DropMessage::TargetDrop => self.drops += 1,
            DropMessage::Source => {}
        }
    }
}

#[derive(Clone, Debug)]
struct DropTargetWidget {
    common: WidgetCommon,
}

impl DropTargetWidget {
    fn new() -> Self {
        Self {
            common: WidgetCommon::new(72, WidgetSizing::fixed(Vector2::new(88.0, 24.0))),
        }
    }
}

impl Widget for DropTargetWidget {
    fn common(&self) -> &WidgetCommon {
        &self.common
    }

    fn common_mut(&mut self) -> &mut WidgetCommon {
        &mut self.common
    }

    fn handle_input(&mut self, _bounds: Rect, input: WidgetInput) -> Option<WidgetOutput> {
        match input {
            WidgetInput::PointerMove { .. } => Some(WidgetOutput::typed(DropMessage::TargetHover)),
            WidgetInput::PointerDrop { .. } => Some(WidgetOutput::typed(DropMessage::TargetDrop)),
            _ => None,
        }
    }

    fn accepts_pointer_move(&self) -> bool {
        true
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
