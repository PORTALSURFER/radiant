use super::*;
use crate::{
    gui::types::{Point, Rect, Vector2},
    layout::{
        ContainerKind, ContainerPolicy, DebugPrimitiveKind, LayoutDebugPrimitive, LayoutOutput,
        NodeId,
    },
    runtime::{PaintPrimitive, SurfaceChild, SurfaceContainer, SurfaceNode, UiSurface},
    theme::ThemeTokens,
    widgets::{TextWidget, WidgetSizing},
};

fn child_is_past_ordered_clip_for(
    kind: ContainerKind,
    clip_rect: Rect,
    child_id: NodeId,
    child_rect: Rect,
) -> bool {
    let mut layout = LayoutOutput::default();
    layout.rects.insert(child_id, child_rect);
    let theme = ThemeTokens::default();
    let context = SurfacePaintContext {
        layout: &layout,
        theme: &theme,
        hovered_container: None,
        active_scroll_affordance: None,
        clip_rect: Some(clip_rect),
    };
    let container = SurfaceContainer::<()>::new(
        1,
        ContainerPolicy {
            kind,
            ..ContainerPolicy::default()
        },
        Vec::new(),
    );
    context.child_is_past_ordered_clip(&container, child_id)
}

#[test]
fn ordered_clip_detects_row_children_past_right_edge() {
    let clip_rect = Rect::from_min_size(Point::new(0.0, 0.0), Vector2::new(100.0, 40.0));
    let child_rect = Rect::from_min_size(Point::new(100.0, 0.0), Vector2::new(24.0, 20.0));

    assert!(child_is_past_ordered_clip_for(
        ContainerKind::Row,
        clip_rect,
        20,
        child_rect
    ));
    assert!(!child_is_past_ordered_clip_for(
        ContainerKind::Column,
        Rect::from_min_size(Point::new(0.0, 0.0), Vector2::new(100.0, 40.0)),
        20,
        Rect::from_min_size(Point::new(100.0, 0.0), Vector2::new(24.0, 20.0))
    ));
}

#[test]
fn ordered_clip_detects_column_children_past_bottom_edge() {
    let clip_rect = Rect::from_min_size(Point::new(0.0, 0.0), Vector2::new(100.0, 40.0));
    let child_rect = Rect::from_min_size(Point::new(0.0, 40.0), Vector2::new(24.0, 20.0));

    assert!(child_is_past_ordered_clip_for(
        ContainerKind::Column,
        clip_rect,
        20,
        child_rect
    ));
    assert!(!child_is_past_ordered_clip_for(
        ContainerKind::Row,
        Rect::from_min_size(Point::new(0.0, 0.0), Vector2::new(100.0, 40.0)),
        20,
        Rect::from_min_size(Point::new(0.0, 40.0), Vector2::new(24.0, 20.0))
    ));
}

#[test]
fn clipped_container_wraps_child_paint_in_clip_primitives() {
    let surface: UiSurface<()> = UiSurface::new(SurfaceNode::row(
        1,
        0.0,
        vec![SurfaceChild::fill(SurfaceNode::static_widget(
            TextWidget::new(
                2,
                "Overflow",
                WidgetSizing::fixed(Vector2::new(160.0, 20.0)),
            ),
        ))],
    ));
    let mut layout = LayoutOutput::default();
    layout.rects.insert(
        1,
        Rect::from_min_size(Point::new(0.0, 0.0), Vector2::new(80.0, 20.0)),
    );
    layout.rects.insert(
        2,
        Rect::from_min_size(Point::new(0.0, 0.0), Vector2::new(160.0, 20.0)),
    );

    let plan = surface.paint_plan(&layout, &ThemeTokens::default());

    assert!(matches!(
        plan.primitives.first(),
        Some(PaintPrimitive::ClipStart(clip))
            if clip.node_id == 1
                && clip.rect
                    == Rect::from_min_size(Point::new(0.0, 0.0), Vector2::new(80.0, 20.0))
    ));
    assert!(
        plan.primitives.iter().any(
            |primitive| matches!(primitive, PaintPrimitive::Text(text) if text.widget_id == 2)
        )
    );
    assert!(matches!(
        plan.primitives.last(),
        Some(PaintPrimitive::ClipEnd(end)) if end.node_id == 1
    ));
}

#[test]
fn layout_debug_strokes_for_children_stay_inside_parent_clip_scope() {
    let surface: UiSurface<()> = UiSurface::new(SurfaceNode::row(
        1,
        0.0,
        vec![SurfaceChild::fill(SurfaceNode::static_widget(
            TextWidget::new(2, "Debug", WidgetSizing::fixed(Vector2::new(160.0, 20.0))),
        ))],
    ));
    let mut layout = LayoutOutput::default();
    layout.rects.insert(
        1,
        Rect::from_min_size(Point::new(0.0, 0.0), Vector2::new(80.0, 20.0)),
    );
    layout.rects.insert(
        2,
        Rect::from_min_size(Point::new(0.0, 0.0), Vector2::new(160.0, 20.0)),
    );
    layout.debug_primitives.push(LayoutDebugPrimitive {
        node_id: 2,
        kind: DebugPrimitiveKind::NodeBounds,
        rect: Rect::from_min_size(Point::new(0.0, 0.0), Vector2::new(160.0, 20.0)),
    });

    let plan = surface.paint_plan(&layout, &ThemeTokens::default());

    let child_debug_stroke = plan
        .primitives
        .iter()
        .position(|primitive| {
            matches!(
                primitive,
                PaintPrimitive::StrokeRect(stroke)
                    if stroke.widget_id == 2
                        && stroke.color == crate::gui::types::Rgba8::new(255, 0, 0, 255)
            )
        })
        .expect("child debug stroke should be painted");
    let parent_clip_start = plan
        .primitives
        .iter()
        .position(
            |primitive| matches!(primitive, PaintPrimitive::ClipStart(clip) if clip.node_id == 1),
        )
        .expect("parent clip should start before child paint");
    let parent_clip_end = plan
        .primitives
        .iter()
        .position(|primitive| matches!(primitive, PaintPrimitive::ClipEnd(end) if end.node_id == 1))
        .expect("parent clip should end after child paint");

    assert!(parent_clip_start < child_debug_stroke);
    assert!(child_debug_stroke < parent_clip_end);
}
