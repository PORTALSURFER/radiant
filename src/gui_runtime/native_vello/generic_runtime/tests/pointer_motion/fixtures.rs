use crate::{
    layout::{Point, Rect, Vector2},
    runtime::{
        PaintFillRect, PaintPrimitive, RuntimeBridge, SurfaceNode, UiSurface, WidgetMessageMapper,
    },
    theme::ThemeTokens,
    widgets::{FocusBehavior, Widget, WidgetCommon, WidgetInput, WidgetOutput, WidgetSizing},
};
use std::sync::Arc;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) struct PointerMoveMessage;

#[derive(Clone, Debug)]
pub(super) struct PointerMoveWidget {
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
        _theme: &ThemeTokens,
    ) {
    }
}

#[derive(Default)]
pub(super) struct PointerMoveBridge {
    pub(super) moves: usize,
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
pub(super) struct LocalPointerMoveWidget {
    common: WidgetCommon,
    last_position: Option<Point>,
}

impl LocalPointerMoveWidget {
    fn new() -> Self {
        let mut common = WidgetCommon::new(72, WidgetSizing::fixed(Vector2::new(80.0, 24.0)));
        common.focus = FocusBehavior::Pointer;
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
        _theme: &ThemeTokens,
    ) {
    }
}

pub(super) struct LocalPointerMoveBridge;

impl RuntimeBridge<()> for LocalPointerMoveBridge {
    fn project_surface(&mut self) -> Arc<UiSurface<()>> {
        Arc::new(UiSurface::new(SurfaceNode::custom_widget(
            LocalPointerMoveWidget::new(),
            WidgetMessageMapper::none(),
        )))
    }

    fn reduce_message(&mut self, _message: ()) {}
}

#[derive(Clone, Debug)]
pub(super) struct PaintOnlyPointerMoveWidget {
    common: WidgetCommon,
    last_position: Option<Point>,
}

impl PaintOnlyPointerMoveWidget {
    fn new() -> Self {
        let mut common = WidgetCommon::new(73, WidgetSizing::fixed(Vector2::new(80.0, 24.0)));
        common.focus = FocusBehavior::Pointer;
        Self {
            common,
            last_position: None,
        }
    }
}

impl Widget for PaintOnlyPointerMoveWidget {
    fn common(&self) -> &WidgetCommon {
        &self.common
    }

    fn common_mut(&mut self) -> &mut WidgetCommon {
        &mut self.common
    }

    fn prefers_pointer_move_paint_only(&self) -> bool {
        true
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
        _theme: &ThemeTokens,
    ) {
    }

    fn append_runtime_overlay_paint(
        &self,
        primitives: &mut Vec<PaintPrimitive>,
        _bounds: Rect,
        _layout: &crate::layout::LayoutOutput,
        theme: &ThemeTokens,
    ) {
        let Some(position) = self.last_position else {
            return;
        };
        primitives.push(PaintPrimitive::FillRect(PaintFillRect {
            widget_id: self.common.id,
            rect: Rect::from_min_size(Point::new(position.x - 1.0, 0.0), Vector2::new(2.0, 24.0)),
            color: theme.highlight_orange,
        }));
    }
}

pub(super) struct PaintOnlyPointerMoveBridge;

impl RuntimeBridge<()> for PaintOnlyPointerMoveBridge {
    fn project_surface(&mut self) -> Arc<UiSurface<()>> {
        Arc::new(UiSurface::new(SurfaceNode::custom_widget(
            PaintOnlyPointerMoveWidget::new(),
            WidgetMessageMapper::none(),
        )))
    }

    fn reduce_message(&mut self, _message: ()) {}
}
