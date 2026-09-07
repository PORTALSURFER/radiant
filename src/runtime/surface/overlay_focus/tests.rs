use super::*;
use crate::{
    application::{IntoView, Layer, scene, text},
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

#[test]
fn declarative_nested_modal_with_input_shield_preserves_overlay_ancestry() {
    let surface = scene(text::<()>("base").id(2))
        .layer(
            Layer::modal(
                scene(text("outer").id(3))
                    .layer(Layer::modal(text("inner").id(4)))
                    .into_view(),
            )
            .block_input(),
        )
        .into_view()
        .into_surface();
    let projection = OverlayFocusProjection::collect(&surface);
    assert!(projection.is_valid());
    assert_eq!(projection.records().len(), 2);
    assert_eq!(projection.records()[1].parent(), Some(0));
}

#[test]
fn overlay_record_capacity_fails_closed_at_sixty_five() {
    let mut layers = Vec::new();
    for index in 0..65 {
        layers.push(
            SurfaceLayer::new(LayerKind::Modal, node(100 + index))
                .focus_owner(OverlayFocusOwner::new(), OverlayFocusPolicy::Modal),
        );
    }
    let surface = UiSurface::new(SurfaceNode::scene(1, node(2), layers));
    let projection = OverlayFocusProjection::collect(&surface);
    assert!(projection.is_invalid());
    assert_eq!(projection.records().len(), 64);
}
