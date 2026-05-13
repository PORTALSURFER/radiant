use super::*;
use crate::{
    gui::types::{Point, Rect, Vector2},
    layout::{ContainerKind, ContainerPolicy, LayoutOutput, NodeId},
    runtime::SurfaceContainer,
    theme::ThemeTokens,
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
