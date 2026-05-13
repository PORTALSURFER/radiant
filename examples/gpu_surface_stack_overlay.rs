//! Stack animated normal widget overlays above a retained GPU surface.

use radiant::prelude::*;
use radiant::widgets::{PaintBounds, WidgetId};
use std::sync::Arc;

const SURFACE_WIDTH: f32 = 560.0;
const SURFACE_HEIGHT: f32 = 220.0;
const HANDLE_HIT_WIDTH: f32 = 18.0;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum ResizeHandle {
    Start,
    End,
}

#[derive(Debug)]
struct DemoState {
    selected: bool,
    running: bool,
    phase: f32,
    selection_start: f32,
    selection_end: f32,
    drag_handle: Option<ResizeHandle>,
}

impl Default for DemoState {
    fn default() -> Self {
        Self {
            selected: false,
            running: true,
            phase: 0.0,
            selection_start: 0.22,
            selection_end: 0.68,
            drag_handle: None,
        }
    }
}

impl DemoState {
    fn tick(&mut self) {
        self.phase = (self.phase + 0.009) % 1.0;
    }

    fn resize_selection(&mut self, ratio: f32) {
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

#[derive(Clone, Copy, Debug, PartialEq)]
enum DemoMessage {
    AnimationFrame,
    ToggleSelection,
    ToggleAnimation,
    BeginResize(ResizeHandle),
    ResizeTo(f32),
    EndResize,
}

#[derive(Clone)]
struct SelectionOverlay {
    common: WidgetCommon,
    selected: bool,
    selection_start: f32,
    selection_end: f32,
    drag_handle: Option<ResizeHandle>,
}

impl SelectionOverlay {
    fn new(state: &DemoState) -> Self {
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
            drag_handle: state.drag_handle,
        }
    }

    fn ratio_from_position(bounds: Rect, position: Point) -> f32 {
        ((position.x - bounds.min.x) / bounds.width().max(1.0)).clamp(0.0, 1.0)
    }

    fn selection_rect(&self, bounds: Rect) -> Rect {
        Rect::from_min_max(
            Point::new(
                bounds.min.x + bounds.width() * self.selection_start,
                bounds.min.y,
            ),
            Point::new(
                bounds.min.x + bounds.width() * self.selection_end,
                bounds.max.y,
            ),
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
            } if bounds.contains(position) => {
                if let Some(handle) = self.handle_at(bounds, position) {
                    return Some(WidgetOutput::custom(DemoMessage::BeginResize(handle)));
                }
                Some(WidgetOutput::custom(DemoMessage::ToggleSelection))
            }
            WidgetInput::PointerMove { position } if self.drag_handle.is_some() => {
                Some(WidgetOutput::custom(DemoMessage::ResizeTo(
                    Self::ratio_from_position(bounds, position),
                )))
            }
            WidgetInput::PointerRelease {
                button: PointerButton::Primary,
                ..
            } if self.drag_handle.is_some() => Some(WidgetOutput::custom(DemoMessage::EndResize)),
            _ => None,
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

fn main() -> radiant::Result {
    radiant::app(DemoState::default())
        .title("Radiant GPU Surface Stack Overlay")
        .size(640, 344)
        .view(|state| {
            column([
                row([
                    button(if state.running { "Pause" } else { "Run" })
                        .message(DemoMessage::ToggleAnimation)
                        .id(1)
                        .width(88.0)
                        .height(32.0),
                    text(format!(
                        "selection {:.0}% - {:.0}%",
                        state.selection_start * 100.0,
                        state.selection_end * 100.0
                    ))
                    .id(2)
                    .fill_width()
                    .height(32.0),
                ])
                .spacing(12.0)
                .fill_width(),
                stack([
                    gpu_surface(42, 1, demo_gpu_content())
                        .id(10)
                        .size(SURFACE_WIDTH, SURFACE_HEIGHT),
                    custom_widget_mapped(SelectionOverlay::new(state), |message| message)
                        .id(11)
                        .size(SURFACE_WIDTH, SURFACE_HEIGHT),
                ])
                .id(12)
                .size(SURFACE_WIDTH, SURFACE_HEIGHT),
            ])
            .id(100)
            .padding(24.0)
            .spacing(16.0)
        })
        .animation(|state| state.running)
        .on_frame(|| DemoMessage::AnimationFrame)
        .transient_overlay(|state, plan, primitives, _viewport, _animation_time| {
            paint_transient_blob(state, plan, primitives);
        })
        .update_command(|state: &mut DemoState, message| match message {
            DemoMessage::AnimationFrame => {
                state.tick();
                Command::request_paint_only()
            }
            DemoMessage::ToggleSelection => {
                state.selected = !state.selected;
                Command::request_repaint()
            }
            DemoMessage::ToggleAnimation => {
                state.running = !state.running;
                Command::request_repaint()
            }
            DemoMessage::BeginResize(handle) => {
                state.drag_handle = Some(handle);
                Command::request_repaint()
            }
            DemoMessage::ResizeTo(ratio) => {
                state.resize_selection(ratio);
                Command::request_repaint()
            }
            DemoMessage::EndResize => {
                state.drag_handle = None;
                Command::request_repaint()
            }
        })
        .run()
}

fn paint_transient_blob(
    state: &DemoState,
    plan: &SurfacePaintPlan,
    primitives: &mut Vec<PaintPrimitive>,
) {
    let Some(bounds) = plan
        .primitives
        .iter()
        .find_map(|primitive| match primitive {
            PaintPrimitive::GpuSurface(surface) if surface.widget_id == 10 => Some(surface.rect),
            _ => None,
        })
    else {
        return;
    };
    paint_bouncing_ball(
        primitives,
        11,
        bounds,
        state.phase,
        overlay_accent(state.selected),
    );
}

fn overlay_accent(selected: bool) -> Rgba8 {
    if selected {
        Rgba8 {
            r: 82,
            g: 168,
            b: 255,
            a: 220,
        }
    } else {
        Rgba8 {
            r: 255,
            g: 142,
            b: 92,
            a: 220,
        }
    }
}

fn paint_bouncing_ball(
    primitives: &mut Vec<PaintPrimitive>,
    widget_id: WidgetId,
    bounds: Rect,
    phase: f32,
    accent: Rgba8,
) {
    let travel_x = bounds.width() - 32.0;
    let travel_y = bounds.height() - 42.0;
    let x = bounds.min.x + 16.0 + travel_x * triangle_wave(phase);
    let y = bounds.min.y + 20.0 + travel_y * triangle_wave((phase * 1.37 + 0.19) % 1.0);
    let rows = [
        (-10.0, -5.0, 20.0),
        (-14.0, -9.0, 28.0),
        (-16.0, -11.0, 32.0),
        (-14.0, -9.0, 28.0),
        (-10.0, -5.0, 20.0),
    ];
    for (offset_y, offset_x, width) in rows {
        primitives.push(PaintPrimitive::FillRect(PaintFillRect {
            widget_id,
            rect: Rect::from_min_size(
                Point::new(x + offset_x, y + offset_y),
                Vector2::new(width, 5.0),
            ),
            color: Rgba8 {
                r: 255,
                g: 255_u8.saturating_sub(accent.g / 3),
                b: accent.b.saturating_add(24),
                a: 235,
            },
        }));
    }
}

fn triangle_wave(phase: f32) -> f32 {
    let wrapped = phase.fract();
    if wrapped < 0.5 {
        wrapped * 2.0
    } else {
        2.0 - wrapped * 2.0
    }
}

fn demo_gpu_content() -> GpuSurfaceContent {
    let width = SURFACE_WIDTH as usize;
    let height = SURFACE_HEIGHT as usize;
    let mut pixels = Vec::with_capacity(width * height * 4);
    for y in 0..height {
        let center = height as f32 * 0.5;
        let wave = ((y as f32 - center).abs() / center).clamp(0.0, 1.0);
        for x in 0..width {
            let phase = x as f32 / width as f32;
            let trace = (phase * std::f32::consts::TAU * 12.0).sin().abs();
            let bright = wave < trace * 0.72 + 0.04;
            let shade = if bright {
                180
            } else {
                30 + (phase * 50.0) as u8
            };
            pixels.extend_from_slice(&[shade / 3, shade, shade.saturating_add(45), 255]);
        }
    }
    GpuSurfaceContent::RgbaAtlas {
        atlas: Arc::new(ImageRgba::new(width, height, pixels).expect("valid demo image")),
        source_rect: Rect::from_min_size(
            Point::new(0.0, 0.0),
            Vector2::new(width as f32, height as f32),
        ),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn resize_selection_keeps_minimum_width() {
        let mut state = DemoState {
            drag_handle: Some(ResizeHandle::Start),
            ..DemoState::default()
        };

        state.resize_selection(0.67);

        assert!(state.selection_end - state.selection_start >= 0.04);
    }

    #[test]
    fn triangle_wave_bounces_between_edges() {
        assert_eq!(triangle_wave(0.0), 0.0);
        assert_eq!(triangle_wave(0.25), 0.5);
        assert_eq!(triangle_wave(0.5), 1.0);
        assert_eq!(triangle_wave(0.75), 0.5);
    }
}
