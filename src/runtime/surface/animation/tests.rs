use super::*;
use crate::{
    animation::{Animatable, Transition},
    layout::ContainerPolicy,
    runtime::{LayerKind, SurfaceChild, SurfaceLayer},
};
use std::{rc::Rc, time::Duration};
struct Capability([AnimationTarget; 1]);
impl Animatable for Capability {
    fn targets(&self) -> &[AnimationTarget] {
        &self.0
    }
}
fn animated(id: u64) -> SurfaceNode<()> {
    SurfaceNode::container(id, ContainerPolicy::default(), Vec::new()).with_animation(Rc::new(
        Capability([AnimationTarget::new(
            7,
            0.0,
            1.0,
            Transition::linear(Duration::from_secs(1)),
            AnimationImpact::Paint,
        )
        .unwrap()]),
    ))
}
#[test]
fn nested_scenes_and_layers_preserve_animation_identity_and_samples() {
    let mut surface = UiSurface::new(SurfaceNode::scene(
        1,
        animated(2),
        vec![SurfaceLayer::new(LayerKind::Tooltip, animated(3))],
    ));
    assert_eq!(surface.animation_descriptors().unwrap().len(), 2);
    assert!(
        surface
            .apply_animation_samples(&[(2, 7, 0.25), (3, 7, 0.5)])
            .changed
    );
    assert!(
        !surface
            .apply_animation_samples(&[(2, 7, 0.25), (3, 7, 0.5)])
            .changed
    );
    assert_eq!(surface.root.clone().has_animation(), true);
}
#[test]
fn cap_scale_sample_application_reuses_container_storage() {
    let children = (1..=1024)
        .map(|id| SurfaceChild::fill(animated(id)))
        .collect();
    let mut surface = UiSurface::new(SurfaceNode::container(
        0,
        ContainerPolicy::default(),
        children,
    ));
    assert_eq!(surface.animation_descriptors().unwrap().len(), 1024);
    let mut values: Vec<_> = (1..=1024).map(|id| (id, 7, 0.25)).collect();
    surface.apply_animation_samples(&values);
    let SurfaceNode::Container(root) = &surface.root else {
        panic!("root")
    };
    let addresses: Vec<_> = root
        .children
        .iter()
        .map(|child| {
            let SurfaceNode::Container(c) = &child.child else {
                panic!("child")
            };
            c.animation_values.as_ptr()
        })
        .collect();
    for item in &mut values {
        item.2 = 0.75;
    }
    assert!(surface.apply_animation_samples(&values).changed);
    let SurfaceNode::Container(root) = &surface.root else {
        panic!("root")
    };
    for (index, child) in root.children.iter().enumerate() {
        let SurfaceNode::Container(c) = &child.child else {
            panic!("child")
        };
        assert_eq!(c.animation_values, [(7, 0.75)]);
        assert_eq!(c.animation_values.as_ptr(), addresses[index]);
    }
}
#[test]
fn invalid_capacity_and_duplicate_node_identity_reject_inventory() {
    let children = (1..=1025)
        .map(|id| SurfaceChild::fill(animated(id)))
        .collect();
    let surface = UiSurface::new(SurfaceNode::container(
        0,
        ContainerPolicy::default(),
        children,
    ));
    assert!(surface.animation_descriptors().is_none());
    let surface = UiSurface::new(SurfaceNode::container(
        0,
        ContainerPolicy::default(),
        vec![
            SurfaceChild::fill(animated(1)),
            SurfaceChild::fill(animated(1)),
        ],
    ));
    assert!(surface.animation_descriptors().is_none());
}
