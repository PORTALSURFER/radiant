use crate::{
    gui::types::{Point, Rect},
    runtime::{GpuSurfaceOverlay, PaintGpuSurface, PaintPrimitive},
};

pub(super) fn topmost_native_hover_surface_index(
    primitives: &[PaintPrimitive],
    position: Point,
) -> Option<usize> {
    primitives.iter().rposition(|primitive| match primitive {
        PaintPrimitive::GpuSurface(surface) => surface_supports_native_hover(surface, position),
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
    let Some(ratio) = pointer_ratio_for_surface(surface.rect, position) else {
        return clear_surface_cursor_overlay(surface);
    };
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

fn surface_supports_native_hover(surface: &PaintGpuSurface, position: Point) -> bool {
    surface
        .capabilities
        .runtime_overlays
        .pointer_vertical_line
        .is_some()
        && rect_has_finite_positive_size(surface.rect)
        && point_is_finite(position)
        && surface.content.is_renderable()
        && surface.rect.contains(position)
}

fn pointer_ratio_for_surface(rect: Rect, position: Point) -> Option<f32> {
    if !rect_has_finite_positive_size(rect) || !point_is_finite(position) {
        return None;
    }
    let ratio = (position.x - rect.min.x) / rect.width();
    ratio.is_finite().then_some(ratio.clamp(0.0, 1.0))
}

fn rect_has_finite_positive_size(rect: Rect) -> bool {
    rect.min.x.is_finite()
        && rect.min.y.is_finite()
        && rect.max.x.is_finite()
        && rect.max.y.is_finite()
        && rect.width() > 0.0
        && rect.height() > 0.0
}

fn point_is_finite(point: Point) -> bool {
    point.x.is_finite() && point.y.is_finite()
}
