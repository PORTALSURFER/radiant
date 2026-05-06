//! Scene encoding for generic runtime paint plans.

use super::GenericSharedPixelBytes;
use crate::gui_runtime::native_vello::*;

pub(in crate::gui_runtime::native_vello) fn encode_surface_paint_plan_to_scene<Bridge, Message>(
    plan: &crate::runtime::SurfacePaintPlan,
    scene: &mut Scene,
    text_renderer: &mut NativeTextRenderer,
    bridge: &mut Bridge,
    viewport: Vector2,
) where
    Bridge: RuntimeBridge<Message>,
{
    scene.reset();
    let mut text_runs = Vec::new();
    for primitive in &plan.primitives {
        match primitive {
            PaintPrimitive::FillRect(fill) => encode_rect(scene, fill.color, fill.rect),
            PaintPrimitive::StrokeRect(stroke) => {
                scene.stroke(
                    &vello::kurbo::Stroke::new(stroke.width as f64),
                    Affine::IDENTITY,
                    color_from_rgba(stroke.color),
                    None,
                    &to_kurbo_rect(stroke.rect),
                );
            }
            PaintPrimitive::Text(text) => {
                let align = match text.align {
                    PaintTextAlign::Left => TextAlign::Left,
                    PaintTextAlign::Center => TextAlign::Center,
                    PaintTextAlign::Right => TextAlign::Right,
                };
                let baseline_offset = text.baseline.unwrap_or(text.font_size);
                text_runs.push(TextRun {
                    text: text.text.clone(),
                    position: Point::new(
                        text.rect.min.x,
                        text.rect.min.y + baseline_offset - text.font_size,
                    ),
                    font_size: text.font_size,
                    color: text.color,
                    max_width: Some(text.rect.width().max(0.0)),
                    align,
                });
            }
            PaintPrimitive::Image(draw) => {
                encode_image(
                    scene,
                    Arc::clone(&draw.image.pixels),
                    draw.image.width,
                    draw.image.height,
                    draw.rect,
                );
            }
            PaintPrimitive::CustomSurface(custom) => {
                if let Some(retained) = custom.retained
                    && let Some(frame) =
                        bridge.render_retained_surface(retained, custom.rect, viewport)
                {
                    encode_paint_frame_to_scene(&frame, scene, text_renderer);
                    continue;
                }
                scene.stroke(
                    &vello::kurbo::Stroke::new(1.0),
                    Affine::IDENTITY,
                    color_from_rgba(Rgba8 {
                        r: 96,
                        g: 96,
                        b: 96,
                        a: 255,
                    }),
                    None,
                    &to_kurbo_rect(custom.rect),
                );
            }
        }
    }
    text_renderer.draw_text_runs(scene, &text_runs);
}

fn encode_paint_frame_to_scene(
    frame: &PaintFrame,
    scene: &mut Scene,
    text_renderer: &mut NativeTextRenderer,
) {
    for primitive in frame.primitives.iter() {
        match primitive {
            Primitive::Rect(fill) => encode_rect(scene, fill.color, fill.rect),
            Primitive::Circle(fill) => {
                scene.fill(
                    Fill::NonZero,
                    Affine::IDENTITY,
                    color_from_rgba(fill.color),
                    None,
                    &Circle::new(
                        (fill.center.x as f64, fill.center.y as f64),
                        fill.radius as f64,
                    ),
                );
            }
            Primitive::LinearGradient(fill) => {
                let mut gradient = Gradient::new_linear(
                    KurboPoint::new(fill.start.x as f64, fill.start.y as f64),
                    KurboPoint::new(fill.end.x as f64, fill.end.y as f64),
                );
                gradient
                    .stops
                    .push((0.0, color_from_rgba(fill.start_color)).into());
                gradient
                    .stops
                    .push((1.0, color_from_rgba(fill.end_color)).into());
                scene.fill(
                    Fill::NonZero,
                    Affine::IDENTITY,
                    &gradient,
                    None,
                    &to_kurbo_rect(fill.rect),
                );
            }
            Primitive::Image(draw) => {
                encode_image(
                    scene,
                    Arc::clone(&draw.image.pixels),
                    draw.image.width,
                    draw.image.height,
                    draw.rect,
                );
            }
        }
    }
    text_renderer.draw_text_runs(scene, &frame.text_runs);
}

fn encode_rect(scene: &mut Scene, color: Rgba8, rect: UiRect) {
    scene.fill(
        Fill::NonZero,
        Affine::IDENTITY,
        color_from_rgba(color),
        None,
        &to_kurbo_rect(rect),
    );
}

fn encode_image(
    scene: &mut Scene,
    pixels: Arc<[u8]>,
    image_width: usize,
    image_height: usize,
    rect: UiRect,
) {
    let (Ok(width), Ok(height)) = (u32::try_from(image_width), u32::try_from(image_height)) else {
        return;
    };
    if width == 0 || height == 0 || rect.width() <= 0.0 || rect.height() <= 0.0 {
        return;
    }
    let image_data = ImageData {
        data: Blob::new(Arc::new(GenericSharedPixelBytes(pixels))),
        format: ImageFormat::Rgba8,
        alpha_type: ImageAlphaType::Alpha,
        width,
        height,
    };
    let transform = Affine::translate((rect.min.x as f64, rect.min.y as f64))
        * Affine::scale_non_uniform(
            rect.width() as f64 / f64::from(width),
            rect.height() as f64 / f64::from(height),
        );
    scene.draw_image(&image_data, transform);
}
