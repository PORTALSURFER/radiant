use super::shared::*;

pub(super) struct ExclusiveCaptureHoverBridge;

impl RuntimeBridge<()> for ExclusiveCaptureHoverBridge {
    fn project_surface(&mut self) -> Arc<UiSurface<()>> {
        let source = DragHandleWidget::new(91, WidgetSizing::fixed(Vector2::new(16.0, 24.0)));
        Arc::new(UiSurface::new(SurfaceNode::container(
            90,
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
                        HoverPaintWidget::new(92),
                        WidgetMessageMapper::none(),
                    ),
                ),
            ],
        )))
    }

    fn reduce_message(&mut self, _message: ()) {}
}

#[derive(Clone, Debug)]
struct HoverPaintWidget {
    common: WidgetCommon,
}

impl HoverPaintWidget {
    fn new(id: WidgetId) -> Self {
        let mut common = WidgetCommon::new(id, WidgetSizing::fixed(Vector2::new(88.0, 24.0)));
        common.focus = crate::widgets::FocusBehavior::Pointer;
        Self { common }
    }
}

impl Widget for HoverPaintWidget {
    fn common(&self) -> &WidgetCommon {
        &self.common
    }

    fn common_mut(&mut self) -> &mut WidgetCommon {
        &mut self.common
    }

    fn handle_input(&mut self, bounds: Rect, input: WidgetInput) -> Option<WidgetOutput> {
        if let WidgetInput::PointerMove { position } = input {
            self.common.state.hovered = bounds.contains(position);
        }
        None
    }

    fn synchronize_from_previous(&mut self, previous: &dyn Widget) {
        if let Some(previous) = previous.as_any().downcast_ref::<Self>() {
            self.common.state = previous.common.state;
        }
    }

    fn append_paint(
        &self,
        primitives: &mut Vec<PaintPrimitive>,
        bounds: Rect,
        _layout: &crate::layout::LayoutOutput,
        _theme: &crate::theme::ThemeTokens,
    ) {
        if self.common.state.hovered {
            primitives.push(PaintPrimitive::FillRect(crate::runtime::PaintFillRect {
                widget_id: self.common.id,
                rect: bounds,
                color: crate::gui::types::Rgba8::new(255, 0, 0, 255),
            }));
        }
    }
}

pub(super) fn hover_fill_visible(
    core: &GenericNativeRuntimeCore<ExclusiveCaptureHoverBridge, ()>,
    widget_id: WidgetId,
) -> bool {
    core.runtime
        .frame_with_default_theme()
        .paint_plan
        .fill_rects_for_widget(widget_id)
        .next()
        .is_some()
}

#[derive(Default)]
pub(super) struct DragHandlePassThroughBridge {
    pub(super) hovers: usize,
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
pub(super) enum DropMessage {
    Source,
    TargetHover,
    TargetDrop,
}

#[derive(Default)]
pub(super) struct DropBridge {
    pub(super) hovers: usize,
    pub(super) drops: usize,
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

pub(super) fn widget_point<Bridge, Message>(
    core: &GenericNativeRuntimeCore<Bridge, Message>,
    widget_id: WidgetId,
    label: &str,
) -> Point
where
    Bridge: RuntimeBridge<Message>,
{
    core.runtime
        .layout()
        .rects
        .get(&widget_id)
        .map(|rect| Point::new(rect.min.x + 4.0, rect.min.y + 4.0))
        .unwrap_or_else(|| panic!("{label} should be laid out"))
}
