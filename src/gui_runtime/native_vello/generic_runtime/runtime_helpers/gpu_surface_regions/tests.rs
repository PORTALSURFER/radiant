use super::*;
use crate::gui::types::{ImageRgba, Point, Rgba8, Vector2};
use crate::runtime::{
    GpuSurfaceCapabilities, GpuSurfaceContent, GpuSurfaceLineStyle, GpuSurfaceRuntimeOverlays,
    PaintGpuSurface,
};
use std::sync::Arc;

#[test]
fn gpu_surface_interaction_region_collection_reuses_existing_buffer() {
    let mut regions = Vec::with_capacity(8);
    regions.push(GpuSurfaceInteractionRegion {
        widget_id: 99,
        rect: Rect::from_min_size(Point::new(99.0, 99.0), Vector2::new(1.0, 1.0)),
        fast_pointer_move: true,
        coalesce_vertical_wheel: false,
        runtime_overlays: GpuSurfaceRuntimeOverlays::default(),
    });
    let initial_capacity = regions.capacity();
    let rect = Rect::from_min_size(Point::new(1.0, 2.0), Vector2::new(3.0, 4.0));
    let ignored_rect = Rect::from_min_size(Point::new(5.0, 6.0), Vector2::new(7.0, 8.0));
    let native_hover_rect = Rect::from_min_size(Point::new(9.0, 10.0), Vector2::new(11.0, 12.0));
    let surface = PaintGpuSurface {
        widget_id: 7,
        key: 7,
        revision: 1,
        rect,
        content: GpuSurfaceContent::RgbaAtlas {
            source_rect: Rect::from_min_size(Point::new(0.0, 0.0), Vector2::new(3.0, 4.0)),
            atlas: Arc::new(ImageRgba::new(3, 4, vec![255; 3 * 4 * 4]).expect("valid image")),
        },
        capabilities: GpuSurfaceCapabilities {
            fast_pointer_move: true,
            coalesce_vertical_wheel: true,
            runtime_overlays: GpuSurfaceRuntimeOverlays::default(),
        },
        overlays: Vec::new(),
    };
    let mut ignored_surface = surface.clone();
    ignored_surface.rect = ignored_rect;
    ignored_surface.capabilities.fast_pointer_move = false;
    ignored_surface.capabilities.coalesce_vertical_wheel = false;
    let mut invalid_surface = surface.clone();
    invalid_surface.content = GpuSurfaceContent::SignalBands {
        frames: 1,
        band_count: 0,
        frame_range: [0.0, 1.0],
        samples: Arc::<[f32]>::from([0.0]),
    };
    let mut native_hover_surface = surface.clone();
    native_hover_surface.rect = native_hover_rect;
    native_hover_surface.capabilities.fast_pointer_move = false;
    native_hover_surface.capabilities.coalesce_vertical_wheel = false;
    native_hover_surface.capabilities.runtime_overlays =
        GpuSurfaceRuntimeOverlays::pointer_vertical_line(GpuSurfaceLineStyle {
            color: Rgba8 {
                r: 255,
                g: 255,
                b: 255,
                a: 255,
            },
            width: 1.0,
        });
    let primitives = [
        PaintPrimitive::GpuSurface(ignored_surface),
        PaintPrimitive::GpuSurface(invalid_surface),
        PaintPrimitive::GpuSurface(surface),
        PaintPrimitive::GpuSurface(native_hover_surface),
    ];

    collect_gpu_surface_interaction_regions(&primitives, &mut regions);

    assert_eq!(
        regions,
        [
            GpuSurfaceInteractionRegion {
                widget_id: 7,
                rect,
                fast_pointer_move: true,
                coalesce_vertical_wheel: true,
                runtime_overlays: GpuSurfaceRuntimeOverlays::default(),
            },
            GpuSurfaceInteractionRegion {
                widget_id: 7,
                rect: native_hover_rect,
                fast_pointer_move: false,
                coalesce_vertical_wheel: false,
                runtime_overlays: GpuSurfaceRuntimeOverlays::pointer_vertical_line(
                    GpuSurfaceLineStyle {
                        color: Rgba8 {
                            r: 255,
                            g: 255,
                            b: 255,
                            a: 255,
                        },
                        width: 1.0,
                    },
                ),
            }
        ]
    );
    assert_eq!(regions.capacity(), initial_capacity);
}

#[test]
fn gpu_surface_interaction_region_collection_reuses_scratch_buffers() {
    let surface_rect = Rect::from_min_size(Point::new(0.0, 0.0), Vector2::new(100.0, 80.0));
    let panel_rect = Rect::from_min_size(Point::new(30.0, 20.0), Vector2::new(40.0, 30.0));
    let primitives = [
        PaintPrimitive::GpuSurface(test_surface(surface_rect)),
        PaintPrimitive::FillRect(crate::runtime::PaintFillRect {
            widget_id: 9,
            rect: panel_rect,
            color: Rgba8 {
                r: 47,
                g: 47,
                b: 47,
                a: 255,
            },
        }),
    ];
    let mut regions = Vec::new();
    let mut scratch = GpuSurfaceInteractionScratch {
        opaque_rects: Vec::with_capacity(8),
        visible_rects: Vec::with_capacity(8),
        occlusion_scratch: Vec::with_capacity(8),
        clip_stack: Vec::with_capacity(8),
    };
    let opaque_capacity = scratch.opaque_rects.capacity();
    let visible_capacity = scratch.visible_rects.capacity();
    let occlusion_capacity = scratch.occlusion_scratch.capacity();
    let clip_stack_capacity = scratch.clip_stack.capacity();

    collect_gpu_surface_interaction_regions_with_scratch(&primitives, &mut regions, &mut scratch);
    collect_gpu_surface_interaction_regions_with_scratch(
        &[PaintPrimitive::GpuSurface(test_surface(surface_rect))],
        &mut regions,
        &mut scratch,
    );

    assert_eq!(scratch.opaque_rects.capacity(), opaque_capacity);
    assert_eq!(scratch.visible_rects.capacity(), visible_capacity);
    assert_eq!(scratch.occlusion_scratch.capacity(), occlusion_capacity);
    assert_eq!(scratch.clip_stack.capacity(), clip_stack_capacity);
}

#[test]
fn gpu_surface_interaction_regions_respect_active_prefix_clips() {
    let surface_rect = Rect::from_min_size(Point::new(0.0, 0.0), Vector2::new(100.0, 80.0));
    let clip_rect = Rect::from_min_size(Point::new(20.0, 10.0), Vector2::new(50.0, 30.0));
    let primitives = [
        PaintPrimitive::ClipStart(crate::runtime::PaintClipStart {
            node_id: 1,
            rect: clip_rect,
        }),
        PaintPrimitive::GpuSurface(test_surface(surface_rect)),
        PaintPrimitive::ClipEnd(crate::runtime::PaintClipEnd { node_id: 1 }),
    ];
    let mut regions = Vec::new();

    collect_gpu_surface_interaction_regions(&primitives, &mut regions);

    assert_eq!(regions.len(), 1);
    assert_eq!(regions[0].rect, clip_rect);
    assert!(regions[0].contains(Point::new(30.0, 20.0)));
    assert!(!regions[0].contains(Point::new(10.0, 20.0)));
}

#[test]
fn gpu_surface_interaction_regions_skip_surfaces_outside_active_prefix_clips() {
    let surface_rect = Rect::from_min_size(Point::new(0.0, 0.0), Vector2::new(100.0, 80.0));
    let primitives = [
        PaintPrimitive::ClipStart(crate::runtime::PaintClipStart {
            node_id: 1,
            rect: Rect::from_min_size(Point::new(200.0, 0.0), Vector2::new(20.0, 20.0)),
        }),
        PaintPrimitive::GpuSurface(test_surface(surface_rect)),
        PaintPrimitive::ClipEnd(crate::runtime::PaintClipEnd { node_id: 1 }),
    ];
    let mut regions = Vec::new();

    collect_gpu_surface_interaction_regions(&primitives, &mut regions);

    assert!(regions.is_empty());
}

#[test]
fn gpu_surface_interaction_regions_skip_opaque_later_panels() {
    let surface_rect = Rect::from_min_size(Point::new(0.0, 0.0), Vector2::new(100.0, 80.0));
    let panel_rect = Rect::from_min_size(Point::new(30.0, 20.0), Vector2::new(40.0, 30.0));
    let primitives = [
        PaintPrimitive::GpuSurface(test_surface(surface_rect)),
        PaintPrimitive::FillRect(crate::runtime::PaintFillRect {
            widget_id: 9,
            rect: panel_rect,
            color: Rgba8 {
                r: 47,
                g: 47,
                b: 47,
                a: 255,
            },
        }),
    ];
    let mut regions = Vec::new();

    collect_gpu_surface_interaction_regions(&primitives, &mut regions);

    assert_eq!(regions.len(), 4);
    assert!(
        !regions
            .iter()
            .any(|region| region.contains(Point::new(50.0, 35.0)))
    );
    assert!(
        regions
            .iter()
            .any(|region| region.contains(Point::new(10.0, 35.0)))
    );
}

#[test]
fn gpu_surface_interaction_regions_keep_full_region_without_intersecting_occluders() {
    let surface_rect = Rect::from_min_size(Point::new(0.0, 0.0), Vector2::new(100.0, 80.0));
    let outside_panel_rect = Rect::from_min_size(Point::new(150.0, 20.0), Vector2::new(40.0, 30.0));
    let primitives = [
        PaintPrimitive::GpuSurface(test_surface(surface_rect)),
        PaintPrimitive::FillRect(crate::runtime::PaintFillRect {
            widget_id: 9,
            rect: outside_panel_rect,
            color: Rgba8 {
                r: 47,
                g: 47,
                b: 47,
                a: 255,
            },
        }),
    ];
    let mut regions = Vec::new();

    collect_gpu_surface_interaction_regions(&primitives, &mut regions);

    assert_eq!(
        regions,
        [GpuSurfaceInteractionRegion {
            widget_id: 7,
            rect: surface_rect,
            fast_pointer_move: true,
            coalesce_vertical_wheel: true,
            runtime_overlays: GpuSurfaceRuntimeOverlays::pointer_vertical_line(
                GpuSurfaceLineStyle {
                    color: Rgba8 {
                        r: 255,
                        g: 255,
                        b: 255,
                        a: 255,
                    },
                    width: 1.0,
                },
            ),
        }]
    );
}

#[test]
fn gpu_surface_interaction_regions_reject_nonfinite_geometry() {
    let invalid_surface_rect =
        Rect::from_min_max(Point::new(f32::NEG_INFINITY, 0.0), Point::new(100.0, 80.0));
    let invalid_panel_rect = Rect::from_min_max(Point::new(30.0, f32::NAN), Point::new(70.0, 50.0));
    let primitives = [
        PaintPrimitive::GpuSurface(test_surface(invalid_surface_rect)),
        PaintPrimitive::GpuSurface(test_surface(Rect::from_min_size(
            Point::new(0.0, 0.0),
            Vector2::new(100.0, 80.0),
        ))),
        PaintPrimitive::FillRect(crate::runtime::PaintFillRect {
            widget_id: 9,
            rect: invalid_panel_rect,
            color: Rgba8 {
                r: 47,
                g: 47,
                b: 47,
                a: 255,
            },
        }),
    ];
    let mut regions = Vec::new();

    collect_gpu_surface_interaction_regions(&primitives, &mut regions);

    assert_eq!(regions.len(), 1);
    assert!(regions[0].contains(Point::new(10.0, 10.0)));
    assert!(!regions[0].contains(Point::new(f32::NAN, 10.0)));
}

fn test_surface(rect: Rect) -> PaintGpuSurface {
    PaintGpuSurface {
        widget_id: 7,
        key: 7,
        revision: 1,
        rect,
        content: GpuSurfaceContent::RgbaAtlas {
            source_rect: Rect::from_min_size(Point::new(0.0, 0.0), Vector2::new(3.0, 4.0)),
            atlas: Arc::new(ImageRgba::new(3, 4, vec![255; 3 * 4 * 4]).expect("valid image")),
        },
        capabilities: GpuSurfaceCapabilities {
            fast_pointer_move: true,
            coalesce_vertical_wheel: true,
            runtime_overlays: GpuSurfaceRuntimeOverlays::pointer_vertical_line(
                GpuSurfaceLineStyle {
                    color: Rgba8 {
                        r: 255,
                        g: 255,
                        b: 255,
                        a: 255,
                    },
                    width: 1.0,
                },
            ),
        },
        overlays: Vec::new(),
    }
}
