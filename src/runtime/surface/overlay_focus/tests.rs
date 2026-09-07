use super::*;
use crate::{
    layout::{ContainerPolicy, LayoutOutput},
    runtime::{OverlayFocusOwner, SurfaceLayer},
};

fn node(id: u64) -> SurfaceNode<()> {
    SurfaceNode::container(id, ContainerPolicy::default(), Vec::new())
}

#[test]
fn raw_overlay_order_and_membership_follow_rendered_layers() {
    let lower = OverlayFocusOwner::new();
    let upper = OverlayFocusOwner::new();
    let surface = UiSurface::new(SurfaceNode::scene(
        1,
        node(2),
        vec![
            SurfaceLayer::new(LayerKind::Tooltip, node(4))
                .focus_owner(upper, OverlayFocusPolicy::Modal),
            SurfaceLayer::new(LayerKind::Modal, node(3))
                .focus_owner(lower, OverlayFocusPolicy::Modal),
        ],
    ));
    let mut projection = OverlayFocusProjection::collect(&surface);
    let mut layout = LayoutOutput::default();
    layout.rects.insert(3, Default::default());
    layout.rects.insert(4, Default::default());
    projection.qualify(&layout);
    assert!(projection.has_authority());
    assert_eq!(projection.records().len(), 2);
    assert_eq!(projection.top_modal(), Some(1));
    assert!(projection.contains_top_modal(4));
}

#[test]
fn omitted_overlay_root_is_inactive() {
    let surface = UiSurface::new(SurfaceNode::scene(
        1,
        node(2),
        vec![
            SurfaceLayer::new(LayerKind::Modal, node(3))
                .focus_owner(OverlayFocusOwner::new(), OverlayFocusPolicy::Modal),
        ],
    ));
    let mut projection = OverlayFocusProjection::collect(&surface);
    projection.qualify(&LayoutOutput::default());
    assert_eq!(projection.top_modal(), None);
}

#[test]
fn duplicate_ids_without_overlay_authority_stay_non_authoritative() {
    let surface = UiSurface::new(SurfaceNode::scene(
        1,
        node(2),
        vec![SurfaceLayer::new(LayerKind::Floating, node(2))],
    ));
    let projection = OverlayFocusProjection::collect(&surface);
    assert!(!projection.is_invalid());
    assert!(!projection.has_authority());
}

#[test]
fn duplicate_raw_owner_fails_closed() {
    let owner = OverlayFocusOwner::new();
    let surface = UiSurface::new(SurfaceNode::scene(
        1,
        node(2),
        vec![
            SurfaceLayer::new(LayerKind::Modal, node(3))
                .focus_owner(owner.clone(), OverlayFocusPolicy::Modal),
            SurfaceLayer::new(LayerKind::Modal, node(4))
                .focus_owner(owner, OverlayFocusPolicy::Modal),
        ],
    ));
    assert!(OverlayFocusProjection::collect(&surface).is_invalid());
}
