use super::{fixtures::*, shared::*};
use crate::gui::input::InputTimestamp;
use crate::widgets::PointerModifiers;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum TimestampDropMessage {
    Release(Option<InputTimestamp>),
    Drop(Option<InputTimestamp>),
}

#[derive(Clone, Debug)]
struct TimestampSourceWidget {
    common: WidgetCommon,
}

impl TimestampSourceWidget {
    fn new(id: WidgetId) -> Self {
        Self {
            common: WidgetCommon::new(id, WidgetSizing::fixed(Vector2::new(88.0, 24.0))),
        }
    }
}

impl Widget for TimestampSourceWidget {
    fn common(&self) -> &WidgetCommon {
        &self.common
    }

    fn common_mut(&mut self) -> &mut WidgetCommon {
        &mut self.common
    }

    fn handle_input(&mut self, _bounds: Rect, input: WidgetInput) -> Option<WidgetOutput> {
        match input {
            WidgetInput::PointerRelease { timestamp, .. } => Some(WidgetOutput::typed(
                TimestampDropMessage::Release(timestamp),
            )),
            _ => None,
        }
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

#[derive(Clone, Debug)]
struct TimestampDropTargetWidget {
    common: WidgetCommon,
}

impl TimestampDropTargetWidget {
    fn new(id: WidgetId) -> Self {
        Self {
            common: WidgetCommon::new(id, WidgetSizing::fixed(Vector2::new(88.0, 24.0))),
        }
    }
}

impl Widget for TimestampDropTargetWidget {
    fn common(&self) -> &WidgetCommon {
        &self.common
    }

    fn common_mut(&mut self) -> &mut WidgetCommon {
        &mut self.common
    }

    fn handle_input(&mut self, _bounds: Rect, input: WidgetInput) -> Option<WidgetOutput> {
        match input {
            WidgetInput::PointerDrop { timestamp, .. } => {
                Some(WidgetOutput::typed(TimestampDropMessage::Drop(timestamp)))
            }
            _ => None,
        }
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
struct TimestampDropBridge {
    deliveries: Vec<TimestampDropMessage>,
}

impl RuntimeBridge<TimestampDropMessage> for TimestampDropBridge {
    fn project_surface(&mut self) -> Arc<UiSurface<TimestampDropMessage>> {
        crate::runtime::test_arc_surface(UiSurface::new(SurfaceNode::container(
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
                        TimestampSourceWidget::new(71),
                        WidgetMessageMapper::typed(|message: TimestampDropMessage| message),
                    ),
                ),
                SurfaceChild::new(
                    SlotParams::fill(),
                    SurfaceNode::widget(
                        TimestampDropTargetWidget::new(72),
                        WidgetMessageMapper::typed(|message: TimestampDropMessage| message),
                    ),
                ),
            ],
        )))
    }

    fn reduce_message(&mut self, message: TimestampDropMessage) {
        self.deliveries.push(message);
    }
}

#[test]
fn captured_release_routes_pointer_drop_to_widget_under_release_point() {
    let mut core = GenericNativeRuntimeCore::new(DropBridge::default(), Vector2::new(220.0, 32.0));
    let source_point = widget_point(&core, 71, "source");
    let target_point = widget_point(&core, 72, "target");

    assert!(
        core.route_pointer_press(source_point, PointerButton::Primary)
            .routed
    );
    let _ = core.route_pointer_release(target_point, PointerButton::Primary);

    assert_eq!(core.runtime.bridge().drops, 1);
}

#[test]
fn captured_release_preserves_one_timestamp_for_drop_and_release() {
    let mut core =
        GenericNativeRuntimeCore::new(TimestampDropBridge::default(), Vector2::new(220.0, 32.0));
    let source_point = widget_point(&core, 71, "source");
    let target_point = widget_point(&core, 72, "target");
    let timestamp = Some(InputTimestamp::capture());

    assert!(
        core.route_pointer_press(source_point, PointerButton::Primary)
            .routed
    );
    assert!(
        core.route_pointer_release_with_timestamp(
            target_point,
            PointerButton::Primary,
            PointerModifiers::default(),
            timestamp,
        )
        .routed
    );

    assert_eq!(
        core.runtime.bridge().deliveries,
        vec![
            TimestampDropMessage::Drop(timestamp),
            TimestampDropMessage::Release(timestamp),
        ]
    );
}
