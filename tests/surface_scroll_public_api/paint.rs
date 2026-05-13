use super::{DemoMessage, intrinsic_slot};
use radiant::{
    layout::{ContainerKind, ContainerPolicy, Point, Rect, Vector2, layout_tree},
    runtime::{PaintPrimitive, SurfaceChild, SurfaceNode, UiSurface},
    widgets::{WidgetProminence, WidgetSizing, WidgetStyle, WidgetTone},
};

#[test]
fn surface_paint_plan_clips_scroll_content_and_draws_scrollbar_affordance() {
    let surface: UiSurface<DemoMessage> = UiSurface::new(SurfaceNode::scroll_area(
        31,
        SurfaceNode::column(
            32,
            2.0,
            (0..8)
                .map(|index| {
                    SurfaceChild::new(
                        intrinsic_slot(),
                        SurfaceNode::text(
                            100 + index,
                            format!("Row {index}"),
                            WidgetSizing::fixed(Vector2::new(180.0, 24.0)),
                        ),
                    )
                })
                .collect(),
        ),
    ));
    let layout = layout_tree(
        &surface.layout_node(),
        Rect::from_min_size(Point::new(0.0, 0.0), Vector2::new(220.0, 80.0)),
    );

    let plan = surface.paint_plan(&layout, &Default::default());
    let clip_start = plan
        .primitives
        .iter()
        .position(
            |primitive| matches!(primitive, PaintPrimitive::ClipStart(clip) if clip.node_id == 31),
        )
        .expect("scroll paint should start a clip");
    let PaintPrimitive::ClipStart(clip) = &plan.primitives[clip_start] else {
        unreachable!("clip_start index was matched above");
    };
    let clip_end = plan
        .primitives
        .iter()
        .position(
            |primitive| matches!(primitive, PaintPrimitive::ClipEnd(clip) if clip.node_id == 31),
        )
        .expect("scroll paint should end the clip");
    let scrollbar_fills: Vec<_> = plan
        .primitives
        .iter()
        .skip(clip_end + 1)
        .filter_map(|primitive| match primitive {
            PaintPrimitive::FillRect(fill) if fill.widget_id == 31 => Some(fill),
            _ => None,
        })
        .collect();

    assert!(clip_start < clip_end);
    assert_eq!(clip.rect, layout.rects[&31]);
    assert_eq!(scrollbar_fills.len(), 1);
    assert!(scrollbar_fills[0].rect.width() <= 3.0);
    assert_eq!(scrollbar_fills[0].rect.max.x, layout.rects[&31].max.x);
}

#[test]
fn surface_paint_plan_culls_scroll_content_outside_visible_clip() {
    let surface: UiSurface<DemoMessage> = UiSurface::new(SurfaceNode::scroll_area(
        31,
        SurfaceNode::column(
            32,
            2.0,
            (0..64)
                .map(|index| {
                    SurfaceChild::new(
                        intrinsic_slot(),
                        SurfaceNode::text(
                            100 + index,
                            format!("Row {index}"),
                            WidgetSizing::fixed(Vector2::new(180.0, 24.0)),
                        ),
                    )
                })
                .collect(),
        ),
    ));
    let layout = layout_tree(
        &surface.layout_node(),
        Rect::from_min_size(Point::new(0.0, 0.0), Vector2::new(220.0, 80.0)),
    );

    let plan = surface.paint_plan(&layout, &Default::default());
    let stats = plan.stats();

    assert!(stats.text > 0);
    assert!(
        stats.text < 8,
        "normal scroll paint should emit only text rows intersecting the visible clip"
    );
}

#[test]
fn styled_scroll_container_paints_own_chrome_then_clips_content() {
    let surface: UiSurface<DemoMessage> = UiSurface::new(SurfaceNode::styled_container(
        31,
        ContainerPolicy {
            kind: ContainerKind::ScrollView,
            overflow: radiant::layout::OverflowPolicy::Scroll,
            ..ContainerPolicy::default()
        },
        WidgetStyle {
            tone: WidgetTone::Accent,
            prominence: WidgetProminence::Subtle,
        },
        vec![SurfaceChild::fill(SurfaceNode::text(
            32,
            "Long content",
            WidgetSizing::fixed(Vector2::new(220.0, 160.0)),
        ))],
    ));
    let layout = layout_tree(
        &surface.layout_node(),
        Rect::from_min_size(Point::new(0.0, 0.0), Vector2::new(100.0, 80.0)),
    );

    let plan = surface.paint_plan(&layout, &Default::default());
    let chrome = plan
        .primitives
        .iter()
        .position(
            |primitive| matches!(primitive, PaintPrimitive::FillRect(fill) if fill.widget_id == 31),
        )
        .expect("styled scroll container should paint its own chrome");
    let clip_start = plan
        .primitives
        .iter()
        .position(
            |primitive| matches!(primitive, PaintPrimitive::ClipStart(clip) if clip.node_id == 31),
        )
        .expect("styled scroll container should clip its content after chrome");

    assert!(chrome < clip_start);
}
