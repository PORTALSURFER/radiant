use crate::{
    gui::types::Point,
    runtime::{GpuSurfaceOverlay, PaintGpuSurface, PaintPrimitive},
};

pub(super) fn topmost_native_hover_surface_index(
    primitives: &[PaintPrimitive],
    position: Point,
) -> Option<usize> {
    primitives.iter().rposition(|primitive| match primitive {
        PaintPrimitive::GpuSurface(surface) => {
            surface
                .capabilities
                .runtime_overlays
                .pointer_vertical_line
                .is_some()
                && surface.rect.width() > 0.0
                && surface.rect.height() > 0.0
                && surface.content.is_renderable()
                && surface.rect.contains(position)
        }
        _ => false,
    })
}

pub(super) fn update_surface_cursor_overlay(
    surface: &mut PaintGpuSurface,
    position: Point,
) -> bool {
    let Some(cursor) = surface.capabilities.runtime_overlays.pointer_vertical_line else {
        return false;
    };
    let ratio = ((position.x - surface.rect.min.x) / surface.rect.width().max(1.0)).clamp(0.0, 1.0);
    let mut cursor_count = 0;
    let mut cursor_is_current = false;
    for overlay in &surface.overlays {
        let GpuSurfaceOverlay::RuntimeVerticalLine {
            ratio: current_ratio,
            color,
            width,
        } = overlay
        else {
            continue;
        };
        cursor_count += 1;
        cursor_is_current |=
            *current_ratio == ratio && *color == cursor.color && *width == cursor.width;
    }
    if cursor_count == 1 && cursor_is_current {
        return false;
    }
    clear_surface_cursor_overlay(surface);
    surface
        .overlays
        .push(GpuSurfaceOverlay::RuntimeVerticalLine {
            ratio,
            color: cursor.color,
            width: cursor.width,
        });
    true
}

pub(super) fn clear_surface_cursor_overlay(surface: &mut PaintGpuSurface) -> bool {
    let previous_len = surface.overlays.len();
    surface
        .overlays
        .retain(|overlay| !matches!(overlay, GpuSurfaceOverlay::RuntimeVerticalLine { .. }));
    previous_len != surface.overlays.len()
}
