use super::*;
use crate::model::{DemoMessage, DemoState, ResizeHandle};
use crate::transient_overlay::overlay_accent;
use crate::view::{SURFACE_HEIGHT, SURFACE_WIDTH};

const HANDLE_HIT_WIDTH: f32 = 18.0;

#[derive(Clone)]
pub(super) struct SelectionOverlay {
    common: WidgetCommon,
    selected: bool,
    pub(super) selection_start: f32,
    pub(super) selection_end: f32,
    pub(super) drag_handle: Option<ResizeHandle>,
}

impl SelectionOverlay {
    pub(super) fn new(state: &DemoState) -> Self {
        let mut common = WidgetCommon::new(
            0,
            WidgetSizing::fixed(Vector2::new(SURFACE_WIDTH, SURFACE_HEIGHT)),
        );
        common.focus = FocusBehavior::Pointer;
        common.paint.bounds = PaintBounds::ClipToRect;
        common.paint.paints_focus = false;
        common.paint.paints_state_layers = false;
        Self {
            common,
            selected: state.selected,
            selection_start: state.selection_start,
            selection_end: state.selection_end,
            drag_handle: None,
        }
    }

    fn ratio_from_position(bounds: Rect, position: Point) -> f32 {
        bounds.ratio_for_x(position.x)
    }

    fn selection_rect(&self, bounds: Rect) -> Rect {
        Rect::from_min_max(
            Point::new(bounds.x_for_ratio(self.selection_start), bounds.min.y),
            Point::new(bounds.x_for_ratio(self.selection_end), bounds.max.y),
        )
    }

    fn handle_at(&self, bounds: Rect, position: Point) -> Option<ResizeHandle> {
        let selection = self.selection_rect(bounds);
        [
            (ResizeHandle::Start, selection.min.x),
            (ResizeHandle::End, selection.max.x),
        ]
        .into_iter()
        .find_map(|(handle, x)| {
            let rect = Rect::from_min_size(
                Point::new(x - HANDLE_HIT_WIDTH * 0.5, bounds.min.y),
                Vector2::new(HANDLE_HIT_WIDTH, bounds.height()),
            );
            rect.contains(position).then_some(handle)
        })
    }

    fn accent(&self) -> Rgba8 {
        overlay_accent(self.selected)
    }

    pub(super) fn resize_selection(&mut self, ratio: f32) {
        let ratio = ratio.clamp(0.02, 0.98);
        match self.drag_handle {
            Some(ResizeHandle::Start) => {
                self.selection_start = ratio.min(self.selection_end - 0.04);
            }
            Some(ResizeHandle::End) => {
                self.selection_end = ratio.max(self.selection_start + 0.04);
            }
            None => {}
        }
    }
}

impl Widget for SelectionOverlay {
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
                if let Some(handle) = self.handle_at(bounds, position) {
                    self.drag_handle = Some(handle);
                    return None;
                }
                Some(WidgetOutput::custom(DemoMessage::ToggleSelection))
            }
            WidgetInput::PointerMove { position } if self.drag_handle.is_some() => {
                self.resize_selection(Self::ratio_from_position(bounds, position));
                None
            }
            WidgetInput::PointerRelease {
                button: PointerButton::Primary,
                ..
            } if self.drag_handle.take().is_some() => {
                Some(WidgetOutput::custom(DemoMessage::CommitResize {
                    start: self.selection_start,
                    end: self.selection_end,
                }))
            }
            _ => None,
        }
    }

    fn synchronize_from_previous(&mut self, previous: &dyn Widget) {
        let Some(previous) = previous.as_any().downcast_ref::<SelectionOverlay>() else {
            return;
        };
        if previous.drag_handle.is_some() {
            self.selection_start = previous.selection_start;
            self.selection_end = previous.selection_end;
            self.drag_handle = previous.drag_handle;
        }
    }

    fn append_paint(
        &self,
        primitives: &mut Vec<PaintPrimitive>,
        bounds: Rect,
        _layout: &LayoutOutput,
        _theme: &ThemeTokens,
    ) {
        let selection = self.selection_rect(bounds);
        let accent = self.accent();
        primitives.push(PaintPrimitive::FillRect(PaintFillRect {
            widget_id: self.common.id,
            rect: selection,
            color: Rgba8 { a: 52, ..accent },
        }));
        primitives.push(PaintPrimitive::StrokeRect(PaintStrokeRect {
            widget_id: self.common.id,
            rect: selection,
            color: accent,
            width: 2.0,
        }));
        for (handle, x) in [
            (ResizeHandle::Start, selection.min.x),
            (ResizeHandle::End, selection.max.x),
        ] {
            let active = self.drag_handle == Some(handle);
            let handle_rect = Rect::from_min_size(
                Point::new(x - 4.0, bounds.min.y + if active { 8.0 } else { 16.0 }),
                Vector2::new(8.0, bounds.height() - if active { 16.0 } else { 32.0 }),
            );
            primitives.push(PaintPrimitive::FillRect(PaintFillRect {
                widget_id: self.common.id,
                rect: handle_rect,
                color: Rgba8 {
                    a: if active { 255 } else { accent.a },
                    ..accent
                },
            }));
        }
    }
}
