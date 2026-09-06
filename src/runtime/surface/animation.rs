use super::{SourceCompatibility, SurfaceNode, UiSurface};
use crate::{
    animation::{AnimationImpact, AnimationTarget, FeedbackAnimation},
    layout::NodeId,
};
use std::collections::{HashMap, HashSet};
type SampleIndex<'a> = HashMap<NodeId, &'a [(NodeId, u64, f64)]>;
#[derive(Clone, Debug, Eq, PartialEq, Hash)]
pub(crate) struct ProjectedAnimationIdentity {
    pub capability: std::any::TypeId,
    pub ancestry: Vec<(NodeId, Option<super::SourceIdentity>, SourceCompatibility)>,
}
pub(crate) struct ProjectedAnimationDescriptor {
    pub identity: ProjectedAnimationIdentity,
    pub node_id: NodeId,
    pub targets: Vec<AnimationTarget>,
    pub feedback: Vec<FeedbackAnimation>,
}
#[derive(Clone, Copy, Default)]
pub(crate) struct AnimationSampleImpact {
    pub changed: bool,
    pub geometry: bool,
}
impl<Message> UiSurface<Message> {
    pub(crate) fn animation_descriptors(&self) -> Option<Vec<ProjectedAnimationDescriptor>> {
        let mut out = Vec::new();
        let mut path = Vec::new();
        let mut seen = HashSet::new();
        collect(&self.root, &mut path, &mut seen, &mut out)?;
        Some(out)
    }
    pub(crate) fn apply_animation_samples(
        &mut self,
        samples: &[(NodeId, u64, f64)],
    ) -> AnimationSampleImpact {
        let mut impact = AnimationSampleImpact::default();
        let index: SampleIndex<'_> = samples
            .chunk_by(|a, b| a.0 == b.0)
            .filter_map(|group| group.first().map(|first| (first.0, group)))
            .collect();
        apply(&mut self.root, &index, &mut impact);
        impact
    }
}
fn collect<M>(
    node: &SurfaceNode<M>,
    path: &mut Vec<(NodeId, Option<super::SourceIdentity>, SourceCompatibility)>,
    seen: &mut HashSet<NodeId>,
    out: &mut Vec<ProjectedAnimationDescriptor>,
) -> Option<()> {
    if !node.has_animation() {
        return Some(());
    }
    if path.len() >= 128 || seen.len() >= 65_536 || !seen.insert(node.id()) {
        return None;
    }
    let source = node.source_metadata_handle();
    path.push((
        node.id(),
        source.as_ref().map(|s| s.identity),
        SourceCompatibility::from_surface_node(node),
    ));
    let container = match node {
        SurfaceNode::Container(c) => Some(c),
        SurfaceNode::FloatingLayer(l) => Some(&l.container),
        _ => None,
    };
    if let Some(c) = container {
        if let Some(a) = &c.animation {
            if !c.animation_valid || out.len() >= 1024 {
                return None;
            }
            let mut properties = HashSet::new();
            for property in c
                .animation_targets
                .iter()
                .map(|t| t.property())
                .chain(c.animation_feedback.iter().map(|t| t.property()))
            {
                if !properties.insert(property) {
                    return None;
                }
            }
            let existing: usize = out.iter().map(|d| d.targets.len() + d.feedback.len()).sum();
            if existing.saturating_add(properties.len()) > 1024 {
                return None;
            }
            out.push(ProjectedAnimationDescriptor {
                identity: ProjectedAnimationIdentity {
                    capability: a.as_ref().type_id(),
                    ancestry: path.clone(),
                },
                node_id: c.id,
                targets: c.animation_targets.clone(),
                feedback: c.animation_feedback.clone(),
            });
        }
        for child in &c.children {
            collect(&child.child, path, seen, out)?;
        }
    } else if let SurfaceNode::Scene(scene) = node {
        collect(&scene.base, path, seen, out)?;
        for layer in scene.ordered_layers() {
            if let Some(input) = &layer.input {
                collect(input, path, seen, out)?;
            }
            collect(&layer.node, path, seen, out)?;
        }
    }
    path.pop();
    Some(())
}
fn apply<M>(
    node: &mut SurfaceNode<M>,
    samples: &SampleIndex<'_>,
    impact: &mut AnimationSampleImpact,
) {
    apply_bounded(node, samples, impact, 0, &mut 0);
}
fn apply_bounded<M>(
    node: &mut SurfaceNode<M>,
    samples: &SampleIndex<'_>,
    impact: &mut AnimationSampleImpact,
    depth: usize,
    visited: &mut usize,
) {
    if !node.has_animation() || depth >= 128 || *visited >= 65_536 {
        return;
    }
    *visited += 1;
    match node {
        SurfaceNode::Container(c) => apply_container(c, samples, impact, depth, visited),
        SurfaceNode::FloatingLayer(l) => {
            apply_container(&mut l.container, samples, impact, depth, visited)
        }
        SurfaceNode::Scene(scene) => {
            apply_bounded(&mut scene.base, samples, impact, depth + 1, visited);
            for layer in &mut scene.layers {
                if let Some(input) = &mut layer.input {
                    apply_bounded(input, samples, impact, depth + 1, visited);
                }
                apply_bounded(&mut layer.node, samples, impact, depth + 1, visited);
            }
        }
        _ => {}
    }
}
fn apply_container<M>(
    c: &mut super::SurfaceContainer<M>,
    samples: &SampleIndex<'_>,
    impact: &mut AnimationSampleImpact,
    depth: usize,
    visited: &mut usize,
) {
    if c.animation.is_some() {
        let values = samples.get(&c.id).copied().unwrap_or(&[]);
        if !c
            .animation_values
            .iter()
            .copied()
            .eq(values.iter().map(|(_, p, v)| (*p, *v)))
        {
            impact.geometry |= c.animation_targets.iter().any(|t| {
                t.impact() == AnimationImpact::Geometry
                    && c.animation_values
                        .iter()
                        .find(|(p, _)| *p == t.property())
                        .map(|(_, v)| *v)
                        != values
                            .iter()
                            .find(|(_, p, _)| *p == t.property())
                            .map(|(_, _, v)| *v)
            });
            c.animation_values.clear();
            c.animation_values
                .extend(values.iter().map(|(_, p, v)| (*p, *v)));
            impact.changed = true;
        }
    }
    for child in &mut c.children {
        apply_bounded(&mut child.child, samples, impact, depth + 1, visited);
    }
}

impl<M> super::SurfaceContainer<M> {
    pub(super) fn animated_layout_policy(
        &self,
    ) -> Option<std::rc::Rc<dyn crate::layout::LayoutPolicy>> {
        let policy = self.layout_policy.clone()?;
        let Some(animation) = &self.animation else {
            return Some(policy);
        };
        Some(std::rc::Rc::new(AnimatedPolicy {
            policy,
            animation: animation.clone(),
            values: self.animation_values.clone(),
        }))
    }
}
struct AnimatedPolicy {
    policy: std::rc::Rc<dyn crate::layout::LayoutPolicy>,
    animation: std::rc::Rc<dyn crate::animation::Animatable>,
    values: Vec<(u64, f64)>,
}
impl crate::layout::LayoutPolicy for AnimatedPolicy {
    fn measure(
        &self,
        children: &mut crate::layout::MeasureChildren<'_>,
        constraints: crate::layout::Constraints,
    ) -> crate::layout::SizeHint {
        self.animation.measure(
            crate::animation::AnimationValues::new(&self.values),
            self.policy.as_ref(),
            children,
            constraints,
        )
    }
    fn place(
        &self,
        children: &mut crate::layout::PlaceChildren<'_>,
        bounds: crate::gui::types::Rect,
    ) {
        self.animation.place(
            crate::animation::AnimationValues::new(&self.values),
            self.policy.as_ref(),
            children,
            bounds,
        );
    }
}

#[cfg(test)]
#[path = "animation/tests.rs"]
mod tests;
