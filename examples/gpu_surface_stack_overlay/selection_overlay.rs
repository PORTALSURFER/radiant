use super::*;
use crate::model::{DemoMessage, DemoState};
use crate::transient_overlay::overlay_accent;
use crate::view::{SURFACE_HEIGHT, SURFACE_WIDTH};
use radiant::gui::visualization::{
    DragHandleRole, HorizontalValueAxis, canvas_selection_edge_handles,
    canvas_selection_edge_visual_rect, canvas_selection_rect, drag_handle_at_point,
};

const HANDLE_HIT_WIDTH: f32 = 18.0;
const HANDLE_VISUAL_WIDTH: f32 = 8.0;
const HANDLE_INSET: f32 = 16.0;
const ACTIVE_HANDLE_INSET: f32 = 8.0;

#[derive(Clone)]
pub(super) struct SelectionOverlay {
    common: WidgetCommon,
    selected: bool,
    pub(super) selection_start: f32,
    pub(super) selection_end: f32,
    pub(super) drag_handle: Option<DragHandleRole>,
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

    fn selection_rect(&self, bounds: Rect) -> Option<Rect> {
        canvas_selection_rect(bounds, self.selection_start, self.selection_end)
    }

    fn handle_at(&self, bounds: Rect, position: Point) -> Option<DragHandleRole> {
        let handles = canvas_selection_edge_handles(
            bounds,
            self.selection_start,
            self.selection_end,
            HANDLE_HIT_WIDTH,
            self.common.id,
        )?;
        drag_handle_at_point(&handles, position).map(|handle| handle.role)
    }

    fn accent(&self) -> Rgba8 {
        overlay_accent(self.selected)
    }

    pub(super) fn resize_selection(&mut self, ratio: f32) {
        let ratio = ratio.clamp(0.02, 0.98);
        match self.drag_handle {
            Some(DragHandleRole::Start) => {
                self.selection_start = ratio.min(self.selection_end - 0.04);
            }
            Some(DragHandleRole::End) => {
                self.selection_end = ratio.max(self.selection_start + 0.04);
            }
            None => {}
            Some(_) => {}
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
                self.resize_selection(
                    HorizontalValueAxis::normalized(bounds).value_for_x(position.x),
                );
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
        let Some(selection) = self.selection_rect(bounds) else {
            return;
        };
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
        for (handle, fraction) in [
            (DragHandleRole::Start, self.selection_start),
            (DragHandleRole::End, self.selection_end),
        ] {
            let active = self.drag_handle == Some(handle);
            if let Some(handle_rect) = canvas_selection_edge_visual_rect(
                bounds,
                fraction,
                HANDLE_VISUAL_WIDTH,
                if active {
                    ACTIVE_HANDLE_INSET
                } else {
                    HANDLE_INSET
                },
            ) {
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
}
