//! Stack a normal widget overlay above a retained GPU surface.

use radiant::prelude::*;
use radiant::widgets::PaintBounds;
use std::sync::Arc;

#[derive(Default)]
struct DemoState {
    selected: bool,
}

#[derive(Clone, Copy)]
enum DemoMessage {
    ToggleSelection,
}

#[derive(Clone)]
struct SelectionOverlay {
    common: WidgetCommon,
    selected: bool,
}

impl SelectionOverlay {
    fn new(selected: bool) -> Self {
        let mut common = WidgetCommon::new(0, WidgetSizing::fixed(Vector2::new(560.0, 220.0)));
        common.focus = FocusBehavior::Pointer;
        common.paint.bounds = PaintBounds::ClipToRect;
        common.paint.paints_focus = false;
        common.paint.paints_state_layers = false;
        Self { common, selected }
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
            WidgetInput::PointerRelease {
                position,
                button: PointerButton::Primary,
            } if bounds.contains(position) => {
                Some(WidgetOutput::custom(DemoMessage::ToggleSelection))
            }
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
        let selection = Rect::from_min_max(
            Point::new(bounds.min.x + bounds.width() * 0.22, bounds.min.y),
            Point::new(bounds.min.x + bounds.width() * 0.68, bounds.max.y),
        );
        let accent = if self.selected {
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
        };
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
        for x in [selection.min.x, selection.max.x] {
            let handle = Rect::from_min_size(
                Point::new(x - 4.0, bounds.min.y + 16.0),
                Vector2::new(8.0, bounds.height() - 32.0),
            );
            primitives.push(PaintPrimitive::FillRect(PaintFillRect {
                widget_id: self.common.id,
                rect: handle,
                color: accent,
            }));
        }
    }
}

fn main() -> radiant::Result {
    radiant::app(DemoState::default())
        .title("Radiant GPU Surface Stack Overlay")
        .size(640, 320)
        .view(|state| {
            column([
                text("Click the overlay to toggle the ordinary widget handles")
                    .id(1)
                    .fill_width(),
                stack([
                    gpu_surface(42, 1, demo_gpu_content())
                        .id(10)
                        .size(560.0, 220.0),
                    custom_widget_mapped(SelectionOverlay::new(state.selected), |message| message)
                        .id(11)
                        .size(560.0, 220.0),
                ])
                .id(12)
                .size(560.0, 220.0),
            ])
            .id(100)
            .padding(24.0)
            .spacing(16.0)
        })
        .update(|state, message| match message {
            DemoMessage::ToggleSelection => state.selected = !state.selected,
        })
        .run()
}

fn demo_gpu_content() -> GpuSurfaceContent {
    let width = 560;
    let height = 220;
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
