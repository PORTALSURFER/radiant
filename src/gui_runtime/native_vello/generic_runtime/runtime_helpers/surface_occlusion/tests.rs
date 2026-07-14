use super::*;
use crate::gui::types::{Point, Rgba8, Vector2};

#[test]
fn surface_occlusion_regions_ignore_translucent_fills() {
    let surface = UiRect::from_min_size(Point::new(0.0, 0.0), Vector2::new(100.0, 80.0));
    let suffix = [
        PaintPrimitive::FillRect(crate::runtime::PaintFillRect {
            widget_id: 7,
            rect: UiRect::from_min_size(Point::new(10.0, 10.0), Vector2::new(20.0, 20.0)),
            color: Rgba8 {
                r: 255,
                g: 255,
                b: 255,
                a: 160,
            },
        }),
        PaintPrimitive::FillRect(crate::runtime::PaintFillRect {
            widget_id: 8,
            rect: UiRect::from_min_size(Point::new(30.0, 10.0), Vector2::new(20.0, 20.0)),
            color: Rgba8 {
                r: 255,
                g: 255,
                b: 255,
                a: 255,
            },
        }),
    ];

    let mut regions = Vec::new();
    let mut clip_stack = Vec::new();
    surface_occlusion_regions_into(
        surface,
        &[],
        &suffix,
        SurfaceOcclusionPolicy::Exact,
        &mut regions,
        &mut clip_stack,
    );

    assert_eq!(regions.len(), 1);
    assert_eq!(
        regions[0],
        UiRect::from_min_size(Point::new(30.0, 10.0), Vector2::new(20.0, 20.0))
    );
}

#[test]
fn surface_occlusion_policy_requires_exact_opacity_when_requested() {
    let surface = UiRect::from_min_size(Point::new(0.0, 0.0), Vector2::new(100.0, 80.0));
    let suffix = [PaintPrimitive::FillRect(crate::runtime::PaintFillRect {
        widget_id: 7,
        rect: surface,
        color: Rgba8 {
            r: 255,
            g: 255,
            b: 255,
            a: OPAQUE_SUFFIX_OCCLUSION_ALPHA,
        },
    })];
    let mut regions = Vec::new();
    let mut clip_stack = Vec::new();

    surface_occlusion_regions_into(
        surface,
        &[],
        &suffix,
        SurfaceOcclusionPolicy::Exact,
        &mut regions,
        &mut clip_stack,
    );
    assert!(regions.is_empty());

    surface_occlusion_regions_into(
        surface,
        &[],
        &suffix,
        SurfaceOcclusionPolicy::GpuCompositor,
        &mut regions,
        &mut clip_stack,
    );
    assert_eq!(regions, [surface]);
}

#[test]
fn surface_occlusion_regions_into_reuses_existing_storage() {
    let surface = UiRect::from_min_size(Point::new(0.0, 0.0), Vector2::new(100.0, 80.0));
    let suffix = [PaintPrimitive::FillRect(crate::runtime::PaintFillRect {
        widget_id: 7,
        rect: UiRect::from_min_size(Point::new(10.0, 10.0), Vector2::new(20.0, 20.0)),
        color: Rgba8 {
            r: 255,
            g: 255,
            b: 255,
            a: 255,
        },
    })];
    let mut regions = Vec::with_capacity(8);
    let mut clip_stack = Vec::with_capacity(4);

    surface_occlusion_regions_into(
        surface,
        &[],
        &suffix,
        SurfaceOcclusionPolicy::Exact,
        &mut regions,
        &mut clip_stack,
    );
    let capacity = regions.capacity();
    let clip_capacity = clip_stack.capacity();
    surface_occlusion_regions_into(
        surface,
        &[],
        &[],
        SurfaceOcclusionPolicy::Exact,
        &mut regions,
        &mut clip_stack,
    );

    assert_eq!(capacity, 8);
    assert_eq!(regions.capacity(), capacity);
    assert_eq!(clip_stack.capacity(), clip_capacity);
    assert!(regions.is_empty());
}

#[test]
fn surface_occlusion_regions_respect_nested_clip_intersections() {
    let surface = UiRect::from_min_size(Point::new(0.0, 0.0), Vector2::new(100.0, 80.0));
    let suffix = [
        PaintPrimitive::ClipStart(crate::runtime::PaintClipStart {
            node_id: 1,
            rect: UiRect::from_min_size(Point::new(0.0, 0.0), Vector2::new(40.0, 80.0)),
        }),
        PaintPrimitive::ClipStart(crate::runtime::PaintClipStart {
            node_id: 2,
            rect: UiRect::from_min_size(Point::new(10.0, 0.0), Vector2::new(90.0, 80.0)),
        }),
        PaintPrimitive::FillRect(crate::runtime::PaintFillRect {
            widget_id: 7,
            rect: surface,
            color: Rgba8 {
                r: 255,
                g: 255,
                b: 255,
                a: 255,
            },
        }),
        PaintPrimitive::ClipEnd(crate::runtime::PaintClipEnd { node_id: 2 }),
        PaintPrimitive::ClipEnd(crate::runtime::PaintClipEnd { node_id: 1 }),
    ];
    let mut regions = Vec::new();
    let mut clip_stack = Vec::new();

    surface_occlusion_regions_into(
        surface,
        &[],
        &suffix,
        SurfaceOcclusionPolicy::Exact,
        &mut regions,
        &mut clip_stack,
    );

    assert_eq!(
        regions,
        [UiRect::from_min_size(
            Point::new(10.0, 0.0),
            Vector2::new(30.0, 80.0)
        )]
    );
}

#[test]
fn surface_occlusion_preprocessing_visits_each_paint_primitive_once() {
    let surface = UiRect::from_min_size(Point::new(0.0, 0.0), Vector2::new(100.0, 80.0));
    let primitives = (0..128)
        .flat_map(|index| {
            [
                PaintPrimitive::FillRect(crate::runtime::PaintFillRect {
                    widget_id: index,
                    rect: surface,
                    color: Rgba8 {
                        r: 47,
                        g: 47,
                        b: 47,
                        a: 255,
                    },
                }),
                PaintPrimitive::ClipEnd(crate::runtime::PaintClipEnd { node_id: index }),
            ]
        })
        .collect::<Vec<_>>();
    let mut plan = SurfaceOcclusionPlan::default();

    plan.preprocess(&primitives);

    assert_eq!(plan.stats().paint_primitives_visited, primitives.len());
    assert_eq!(plan.stats().occluder_rects_indexed, 128);
}

#[test]
fn surface_occlusion_plan_reuses_preprocessing_storage() {
    let surface = UiRect::from_min_size(Point::new(0.0, 0.0), Vector2::new(100.0, 80.0));
    let primitives = (0..128)
        .map(|index| {
            PaintPrimitive::FillRect(crate::runtime::PaintFillRect {
                widget_id: index,
                rect: surface,
                color: Rgba8 {
                    r: 47,
                    g: 47,
                    b: 47,
                    a: 255,
                },
            })
        })
        .collect::<Vec<_>>();
    let mut plan = SurfaceOcclusionPlan::default();
    plan.preprocess(&primitives);
    let clip_capacity = plan.clip_states.capacity();
    let occluder_capacity = plan.occluders.capacity();
    let index_capacity = plan.index_nodes.capacity();

    plan.preprocess(&primitives[..1]);

    assert_eq!(plan.clip_states.capacity(), clip_capacity);
    assert_eq!(plan.occluders.capacity(), occluder_capacity);
    assert_eq!(plan.index_nodes.capacity(), index_capacity);
}

#[test]
fn surface_occlusion_regions_treat_empty_active_clip_as_fully_hidden() {
    let surface = UiRect::from_min_size(Point::new(0.0, 0.0), Vector2::new(100.0, 80.0));
    let prefix = [PaintPrimitive::ClipStart(crate::runtime::PaintClipStart {
        node_id: 1,
        rect: UiRect::from_min_size(Point::new(0.0, 0.0), Vector2::new(0.0, 80.0)),
    })];
    let mut regions = Vec::new();
    let mut clip_stack = Vec::new();

    surface_occlusion_regions_into(
        surface,
        &prefix,
        &[],
        SurfaceOcclusionPolicy::Exact,
        &mut regions,
        &mut clip_stack,
    );

    assert_eq!(regions, [surface]);
}

#[test]
fn surface_occlusion_regions_index_clipped_opaque_fill_batches() {
    let surface = UiRect::from_min_size(Point::new(0.0, 0.0), Vector2::new(100.0, 80.0));
    let clip = UiRect::from_min_size(Point::new(10.0, 0.0), Vector2::new(60.0, 80.0));
    let suffix = [
        PaintPrimitive::ClipStart(crate::runtime::PaintClipStart {
            node_id: 1,
            rect: clip,
        }),
        PaintPrimitive::FillRectBatch(crate::runtime::PaintFillRectBatch {
            widget_id: 7,
            rects: [
                UiRect::from_min_size(Point::new(0.0, 0.0), Vector2::new(30.0, 20.0)),
                UiRect::from_min_size(Point::new(60.0, 40.0), Vector2::new(30.0, 20.0)),
            ]
            .into(),
            color: Rgba8 {
                r: 255,
                g: 255,
                b: 255,
                a: 255,
            },
        }),
        PaintPrimitive::ClipEnd(crate::runtime::PaintClipEnd { node_id: 1 }),
    ];
    let mut regions = Vec::new();
    let mut clip_stack = Vec::new();

    surface_occlusion_regions_into(
        surface,
        &[],
        &suffix,
        SurfaceOcclusionPolicy::Exact,
        &mut regions,
        &mut clip_stack,
    );

    assert_eq!(
        regions,
        [
            UiRect::from_min_size(Point::new(10.0, 0.0), Vector2::new(20.0, 20.0)),
            UiRect::from_min_size(Point::new(60.0, 40.0), Vector2::new(10.0, 20.0)),
        ]
    );
}

#[test]
fn surface_occlusion_regions_clip_overlay_panels() {
    let surface = UiRect::from_min_size(Point::new(0.0, 0.0), Vector2::new(100.0, 80.0));
    let suffix = [
        PaintPrimitive::ClipStart(crate::runtime::PaintClipStart {
            node_id: 1,
            rect: UiRect::from_min_size(Point::new(0.0, 0.0), Vector2::new(40.0, 80.0)),
        }),
        PaintPrimitive::OverlayPanel(crate::runtime::PaintOverlayPanel {
            widget_id: 7,
            rect: surface,
            label: None,
            style: crate::widgets::WidgetStyle::default(),
        }),
        PaintPrimitive::ClipEnd(crate::runtime::PaintClipEnd { node_id: 1 }),
    ];
    let mut regions = Vec::new();
    let mut clip_stack = Vec::new();

    surface_occlusion_regions_into(
        surface,
        &[],
        &suffix,
        SurfaceOcclusionPolicy::Exact,
        &mut regions,
        &mut clip_stack,
    );

    assert_eq!(
        regions,
        [UiRect::from_min_size(
            Point::new(0.0, 0.0),
            Vector2::new(40.0, 80.0)
        )]
    );
}
