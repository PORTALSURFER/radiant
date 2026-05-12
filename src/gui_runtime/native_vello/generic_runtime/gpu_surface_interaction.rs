//! Interaction fast paths for retained GPU surface primitives.

use super::{
    GenericNativeVelloRunner, GenericRouteOutcome, RenderFrameProfile, maybe_log_route_profile,
};
use crate::{
    gui::types::{Point, Vector2},
    runtime::{GpuSurfaceOverlay, PaintGpuSurface, PaintPrimitive},
};
use std::time::Instant;

#[derive(Clone, Copy, Debug)]
pub(super) struct PendingGpuSurfaceWheel {
    pub(super) position: Point,
    pub(super) delta: Vector2,
}

impl<Bridge, Message> GenericNativeVelloRunner<Bridge, Message>
where
    Bridge: crate::runtime::RuntimeBridge<Message>,
{
    pub(super) fn handle_gpu_surface_route_outcome(
        &mut self,
        outcome: GenericRouteOutcome,
        position: Point,
        delta: Vector2,
    ) {
        if !outcome.needs_redraw() {
            return;
        }
        if self.can_fast_path_gpu_surface_route(position, delta) {
            self.deferred_surface_refresh = true;
            self.request_redraw_if_needed();
            return;
        }
        self.rebuild_scene();
        self.request_redraw_if_needed();
    }

    pub(super) fn queue_gpu_surface_wheel(&mut self, position: Point, delta: Vector2) {
        match &mut self.pending_gpu_surface_wheel {
            Some(pending) => {
                pending.position = position;
                pending.delta = Vector2::new(pending.delta.x + delta.x, pending.delta.y + delta.y);
            }
            None => {
                self.pending_gpu_surface_wheel = Some(PendingGpuSurfaceWheel { position, delta });
            }
        }
        self.update_gpu_surface_cursor_overlay(position);
        self.request_redraw_if_needed();
    }

    pub(super) fn flush_pending_gpu_surface_wheel(&mut self, profile: &mut RenderFrameProfile) {
        let Some(pending) = self.pending_gpu_surface_wheel.take() else {
            return;
        };
        let started = Instant::now();
        let outcome = self
            .core
            .route_scroll_deferred_refresh(pending.position, pending.delta);
        profile.coalesced_wheel_route = started.elapsed();
        maybe_log_route_profile("coalesced_wheel", profile.coalesced_wheel_route, outcome);
        if outcome.needs_redraw() {
            self.deferred_surface_refresh = true;
        }
    }

    pub(super) fn handle_gpu_surface_pointer_move_outcome(
        &mut self,
        outcome: GenericRouteOutcome,
        previous: Option<Point>,
        position: Point,
    ) {
        if !outcome.needs_redraw() {
            return;
        }
        if self.can_fast_path_gpu_surface_pointer_move(previous, position) {
            if self.update_gpu_surface_cursor_overlay(position) {
                self.request_redraw_if_needed();
            }
            return;
        }
        self.rebuild_scene();
        self.request_redraw_if_needed();
    }

    pub(super) fn can_fast_path_gpu_surface_route(&self, position: Point, delta: Vector2) -> bool {
        let is_horizontal_pan = delta.x.abs() > delta.y.abs() && delta.x.abs() > f32::EPSILON;
        !is_horizontal_pan && self.paint_plan_has_coalescing_gpu_surface_at(position)
    }

    pub(super) fn can_fast_path_gpu_surface_pointer_move(
        &self,
        previous: Option<Point>,
        position: Point,
    ) -> bool {
        let Some(previous) = previous else {
            return false;
        };
        self.gpu_surface_interaction_regions.iter().any(|region| {
            region.fast_pointer_move && region.contains(previous) && region.contains(position)
        })
    }

    pub(super) fn paint_plan_has_coalescing_gpu_surface_at(&self, position: Point) -> bool {
        self.gpu_surface_interaction_regions
            .iter()
            .any(|region| region.coalesce_vertical_wheel && region.contains(position))
    }

    pub(super) fn native_hover_surface_contains(&self, position: Point) -> bool {
        self.gpu_surface_interaction_regions
            .iter()
            .any(|region| region.native_hover_cursor.is_some() && region.contains(position))
    }

    pub(super) fn can_coalesce_gpu_surface_wheel(&self, position: Point, delta: Vector2) -> bool {
        let is_vertical = delta.y.abs() >= delta.x.abs() && delta.y.abs() > f32::EPSILON;
        is_vertical && self.paint_plan_has_coalescing_gpu_surface_at(position)
    }

    pub(super) fn update_gpu_surface_cursor_overlay(&mut self, position: Point) -> bool {
        let target_index =
            topmost_native_hover_surface_index(&self.last_paint_plan.primitives, position);
        let mut changed = false;
        for (index, primitive) in self.last_paint_plan.primitives.iter_mut().enumerate() {
            let PaintPrimitive::GpuSurface(surface) = primitive else {
                continue;
            };
            if surface.capabilities.native_hover_cursor.is_none() {
                continue;
            }
            if Some(index) == target_index {
                changed |= update_surface_cursor_overlay(surface, position);
            } else {
                changed |= clear_surface_cursor_overlay(surface);
            }
        }
        changed
    }

    pub(super) fn clear_gpu_surface_cursor_overlay(&mut self, position: Point) -> bool {
        self.last_paint_plan
            .primitives
            .iter_mut()
            .filter_map(|primitive| match primitive {
                PaintPrimitive::GpuSurface(surface)
                    if surface.capabilities.native_hover_cursor.is_some()
                        && surface.rect.contains(position) =>
                {
                    Some(surface)
                }
                _ => None,
            })
            .fold(false, |changed, surface| {
                clear_surface_cursor_overlay(surface) || changed
            })
    }
}

fn topmost_native_hover_surface_index(
    primitives: &[PaintPrimitive],
    position: Point,
) -> Option<usize> {
    primitives.iter().rposition(|primitive| match primitive {
        PaintPrimitive::GpuSurface(surface) => {
            surface.capabilities.native_hover_cursor.is_some()
                && surface.rect.width() > 0.0
                && surface.rect.height() > 0.0
                && surface.content.is_renderable()
                && surface.rect.contains(position)
        }
        _ => false,
    })
}

fn update_surface_cursor_overlay(surface: &mut PaintGpuSurface, position: Point) -> bool {
    let Some(cursor) = surface.capabilities.native_hover_cursor else {
        return false;
    };
    let ratio = ((position.x - surface.rect.min.x) / surface.rect.width().max(1.0)).clamp(0.0, 1.0);
    let mut cursor_count = 0;
    let mut cursor_is_current = false;
    for overlay in &surface.overlays {
        let GpuSurfaceOverlay::VerticalCursor {
            ratio: current_ratio,
            color,
            width,
        } = overlay;
        cursor_count += 1;
        cursor_is_current |=
            *current_ratio == ratio && *color == cursor.color && *width == cursor.width;
    }
    if cursor_count == 1 && cursor_is_current {
        return false;
    }
    clear_surface_cursor_overlay(surface);
    surface.overlays.push(GpuSurfaceOverlay::VerticalCursor {
        ratio,
        color: cursor.color,
        width: cursor.width,
    });
    true
}

fn clear_surface_cursor_overlay(surface: &mut PaintGpuSurface) -> bool {
    let previous_len = surface.overlays.len();
    surface
        .overlays
        .retain(|overlay| !matches!(overlay, GpuSurfaceOverlay::VerticalCursor { .. }));
    previous_len != surface.overlays.len()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        gui::types::{ImageRgba, Rect, Rgba8},
        runtime::{GpuHoverCursor, GpuSurfaceCapabilities, GpuSurfaceContent},
    };
    use std::sync::Arc;

    #[test]
    fn gpu_surface_lookup_skips_unrenderable_surface_content() {
        let rect = Rect::from_min_size(Point::new(0.0, 0.0), Vector2::new(40.0, 20.0));
        let capabilities = GpuSurfaceCapabilities {
            fast_pointer_move: true,
            coalesce_vertical_wheel: true,
            native_hover_cursor: Some(GpuHoverCursor {
                color: Rgba8 {
                    r: 255,
                    g: 255,
                    b: 255,
                    a: 255,
                },
                width: 1.0,
            }),
        };
        let primitives = vec![
            PaintPrimitive::GpuSurface(PaintGpuSurface {
                widget_id: 1,
                key: 1,
                revision: 1,
                rect,
                content: GpuSurfaceContent::SignalBands {
                    frames: 1,
                    band_count: 0,
                    frame_range: [0.0, 1.0],
                    samples: Arc::<[f32]>::from([0.0]),
                },
                capabilities,
                overlays: Vec::new(),
            }),
            PaintPrimitive::GpuSurface(PaintGpuSurface {
                widget_id: 2,
                key: 2,
                revision: 1,
                rect,
                content: GpuSurfaceContent::RgbaAtlas {
                    source_rect: Rect::from_min_size(
                        Point::new(0.0, 0.0),
                        Vector2::new(20.0, 20.0),
                    ),
                    atlas: Arc::new(
                        ImageRgba::new(20, 20, vec![255; 20 * 20 * 4]).expect("valid image"),
                    ),
                },
                capabilities,
                overlays: Vec::new(),
            }),
        ];

        let surface_index = topmost_native_hover_surface_index(&primitives, Point::new(10.0, 10.0))
            .expect("valid surface");

        assert_eq!(surface_index, 1);
    }

    #[test]
    fn gpu_surface_lookup_skips_empty_surface_rects() {
        let rect = Rect::from_min_size(Point::new(0.0, 0.0), Vector2::new(0.0, 20.0));
        let primitives = vec![PaintPrimitive::GpuSurface(PaintGpuSurface {
            widget_id: 1,
            key: 1,
            revision: 1,
            rect,
            content: GpuSurfaceContent::RgbaAtlas {
                source_rect: Rect::from_min_size(Point::new(0.0, 0.0), Vector2::new(1.0, 1.0)),
                atlas: Arc::new(ImageRgba::new(1, 1, vec![255; 4]).expect("valid image")),
            },
            capabilities: GpuSurfaceCapabilities {
                fast_pointer_move: true,
                coalesce_vertical_wheel: true,
                native_hover_cursor: None,
            },
            overlays: Vec::new(),
        })];

        assert!(topmost_native_hover_surface_index(&primitives, Point::new(0.0, 10.0)).is_none());
    }

    #[test]
    fn native_hover_cursor_updates_topmost_surface_and_clears_stale_cursors() {
        let rect = Rect::from_min_size(Point::new(0.0, 0.0), Vector2::new(40.0, 20.0));
        let capabilities = GpuSurfaceCapabilities {
            fast_pointer_move: true,
            coalesce_vertical_wheel: false,
            native_hover_cursor: Some(GpuHoverCursor {
                color: Rgba8 {
                    r: 255,
                    g: 160,
                    b: 0,
                    a: 255,
                },
                width: 2.0,
            }),
        };
        let content = GpuSurfaceContent::RgbaAtlas {
            source_rect: Rect::from_min_size(Point::new(0.0, 0.0), Vector2::new(1.0, 1.0)),
            atlas: Arc::new(ImageRgba::new(1, 1, vec![255; 4]).expect("valid image")),
        };
        let mut primitives = vec![
            PaintPrimitive::GpuSurface(PaintGpuSurface {
                widget_id: 1,
                key: 1,
                revision: 1,
                rect,
                content: content.clone(),
                capabilities,
                overlays: vec![GpuSurfaceOverlay::VerticalCursor {
                    ratio: 0.1,
                    color: capabilities.native_hover_cursor.unwrap().color,
                    width: 2.0,
                }],
            }),
            PaintPrimitive::GpuSurface(PaintGpuSurface {
                widget_id: 2,
                key: 2,
                revision: 1,
                rect,
                content,
                capabilities,
                overlays: Vec::new(),
            }),
        ];

        let target = topmost_native_hover_surface_index(&primitives, Point::new(30.0, 10.0));

        assert_eq!(target, Some(1));
        for (index, primitive) in primitives.iter_mut().enumerate() {
            let PaintPrimitive::GpuSurface(surface) = primitive else {
                continue;
            };
            if Some(index) == target {
                assert!(update_surface_cursor_overlay(
                    surface,
                    Point::new(30.0, 10.0)
                ));
            } else {
                assert!(clear_surface_cursor_overlay(surface));
            }
        }
        let [
            PaintPrimitive::GpuSurface(bottom),
            PaintPrimitive::GpuSurface(top),
        ] = primitives.as_slice()
        else {
            panic!("expected GPU surfaces");
        };
        assert!(bottom.overlays.is_empty());
        assert!(matches!(
            top.overlays.as_slice(),
            [GpuSurfaceOverlay::VerticalCursor { ratio, .. }] if *ratio == 0.75
        ));
    }
}
