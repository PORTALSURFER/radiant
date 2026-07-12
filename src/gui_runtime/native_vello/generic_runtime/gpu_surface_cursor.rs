use crate::{
    gui::types::{Point, Rect},
    runtime::{GpuSurfaceOverlay, PaintGpuSurface},
};

use super::GpuSurfaceInteractionRegion;

pub(super) fn topmost_native_hover_surface_index(
    regions: &[GpuSurfaceInteractionRegion],
    position: Point,
) -> Option<usize> {
    regions
        .iter()
        .rev()
        .find(|region| {
            region.runtime_overlays.pointer_vertical_line.is_some() && region.contains(position)
        })
        .map(|region| region.primitive_index)
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
    let mut cursor_index = None;
    let mut cursor_is_current = false;
    for (index, overlay) in surface.overlays.iter().enumerate() {
        let GpuSurfaceOverlay::RuntimeVerticalLine {
            ratio: current_ratio,
            color,
            width,
        } = overlay
        else {
            continue;
        };
        cursor_count += 1;
        cursor_index = Some(index);
        cursor_is_current |=
            *current_ratio == ratio && *color == cursor.color && *width == cursor.width;
    }
    if cursor_count == 1 && cursor_is_current {
        return false;
    }
    if cursor_count == 1
        && let Some(overlay) = cursor_index.and_then(|index| surface.overlays.get_mut(index))
    {
        *overlay = GpuSurfaceOverlay::RuntimeVerticalLine {
            ratio,
            color: cursor.color,
            width: cursor.width,
        };
        return true;
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

fn pointer_ratio_for_surface(rect: Rect, position: Point) -> Option<f32> {
    if !rect.has_finite_positive_area() || !position.is_finite() {
        return None;
    }
    let ratio = (position.x - rect.min.x) / rect.width();
    ratio.is_finite().then_some(ratio.clamp(0.0, 1.0))
}
