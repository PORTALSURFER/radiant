use super::gpu_surface_types::{GPU_SURFACE_OVERLAY_VEC4_SLOTS, MAX_GPU_SURFACE_OVERLAYS};
use crate::{gui::types::Rgba8, runtime::GpuSurfaceOverlay};

pub(super) type OverlayVec4Slots = [[f32; 4]; GPU_SURFACE_OVERLAY_VEC4_SLOTS];
pub(super) type OverlayColorSlots = [[f32; 4]; MAX_GPU_SURFACE_OVERLAYS];
pub(super) type VerticalOverlayUniforms = (OverlayVec4Slots, OverlayVec4Slots, OverlayColorSlots);

#[derive(Clone, Copy)]
struct VerticalOverlayParts {
    ratio: f32,
    color: Rgba8,
    width: f32,
}

pub(super) fn vertical_overlays(overlays: &[GpuSurfaceOverlay]) -> VerticalOverlayUniforms {
    let mut ratios = [[-1.0; 4]; GPU_SURFACE_OVERLAY_VEC4_SLOTS];
    let mut widths = [[1.0; 4]; GPU_SURFACE_OVERLAY_VEC4_SLOTS];
    let mut colors = [[1.0, 1.0, 1.0, 0.0]; MAX_GPU_SURFACE_OVERLAYS];
    for (index, overlay) in overlays
        .iter()
        .filter(|overlay| {
            matches!(
                overlay,
                GpuSurfaceOverlay::HorizontalRange { .. }
                    | GpuSurfaceOverlay::VerticalCursor { .. }
            )
        })
        .chain(
            overlays
                .iter()
                .filter(|overlay| matches!(overlay, GpuSurfaceOverlay::RuntimeVerticalLine { .. })),
        )
        .take(MAX_GPU_SURFACE_OVERLAYS)
        .enumerate()
    {
        let parts = vertical_overlay_parts(*overlay);
        ratios[index / 4][index % 4] = parts.ratio;
        widths[index / 4][index % 4] = parts.width;
        colors[index] = rgba_to_float(parts.color);
    }
    (ratios, widths, colors)
}

fn vertical_overlay_parts(overlay: GpuSurfaceOverlay) -> VerticalOverlayParts {
    match overlay {
        GpuSurfaceOverlay::HorizontalRange { start, end, color } => {
            let Some((start, end)) = normalized_range(start, end) else {
                return hidden_overlay(color);
            };
            VerticalOverlayParts {
                ratio: start,
                color,
                width: -end,
            }
        }
        GpuSurfaceOverlay::VerticalCursor {
            ratio,
            color,
            width,
        }
        | GpuSurfaceOverlay::RuntimeVerticalLine {
            ratio,
            color,
            width,
        } => VerticalOverlayParts {
            ratio: normalized_ratio(ratio).unwrap_or(-1.0),
            color,
            width: normalized_line_width(width),
        },
    }
}

fn normalized_range(start: f32, end: f32) -> Option<(f32, f32)> {
    let start = normalized_ratio(start)?;
    let end = normalized_ratio(end)?;
    Some((start.min(end), start.max(end)))
}

fn normalized_ratio(ratio: f32) -> Option<f32> {
    ratio.is_finite().then_some(ratio.clamp(0.0, 1.0))
}

fn normalized_line_width(width: f32) -> f32 {
    if width.is_finite() && width > 0.0 {
        width
    } else {
        1.0
    }
}

fn hidden_overlay(color: Rgba8) -> VerticalOverlayParts {
    VerticalOverlayParts {
        ratio: -1.0,
        color,
        width: 1.0,
    }
}

fn rgba_to_float(color: Rgba8) -> [f32; 4] {
    [
        color.r as f32 / 255.0,
        color.g as f32 / 255.0,
        color.b as f32 / 255.0,
        color.a as f32 / 255.0,
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    const WHITE: Rgba8 = Rgba8 {
        r: 255,
        g: 255,
        b: 255,
        a: 255,
    };

    #[test]
    fn vertical_overlay_uniforms_sanitize_invalid_cursor_inputs() {
        let (ratios, widths, colors) = vertical_overlays(&[
            GpuSurfaceOverlay::VerticalCursor {
                ratio: f32::NAN,
                color: WHITE,
                width: f32::INFINITY,
            },
            GpuSurfaceOverlay::RuntimeVerticalLine {
                ratio: 1.5,
                color: WHITE,
                width: -2.0,
            },
        ]);

        assert_eq!(ratios[0][0], -1.0);
        assert_eq!(widths[0][0], 1.0);
        assert_eq!(ratios[0][1], 1.0);
        assert_eq!(widths[0][1], 1.0);
        assert_eq!(colors[0], rgba_to_float(WHITE));
    }

    #[test]
    fn vertical_overlay_uniforms_order_and_sanitize_ranges() {
        let (ratios, widths, _) = vertical_overlays(&[
            GpuSurfaceOverlay::HorizontalRange {
                start: 0.8,
                end: 0.2,
                color: WHITE,
            },
            GpuSurfaceOverlay::HorizontalRange {
                start: f32::NAN,
                end: 0.5,
                color: WHITE,
            },
        ]);

        assert_eq!(ratios[0][0], 0.2);
        assert_eq!(widths[0][0], -0.8);
        assert_eq!(ratios[0][1], -1.0);
        assert_eq!(widths[0][1], 1.0);
    }
}
