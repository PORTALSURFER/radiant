//! Bounded accepted notification metadata projected from the application tree.

use super::{SourceCompatibility, SourceIdentity, SurfaceNode, UiSurface};
use crate::{application::notifications::NoticeDemand, layout::NodeId, widgets::WidgetId};
use std::{collections::HashSet, rc::Rc};

#[derive(Clone, Debug, Eq, PartialEq, Hash)]
pub(crate) struct NoticeOwnerIdentity {
    pub(crate) capability: std::any::TypeId,
    pub(crate) ancestry: Vec<(NodeId, Option<SourceIdentity>, SourceCompatibility)>,
}

pub(crate) struct ProjectedNoticeDescriptor<Message> {
    pub(crate) identity: NoticeOwnerIdentity,
    pub(crate) node_id: NodeId,
    pub(crate) demand: Rc<NoticeDemand<Message>>,
    pub(crate) widgets: Vec<WidgetId>,
}

impl<Message> UiSurface<Message> {
    /// Collect notice cards from the accepted surface with bounded ancestry and
    /// source identity evidence. Invalid or ambiguous projections fail closed.
    pub(crate) fn notice_descriptors(&self) -> Option<Vec<ProjectedNoticeDescriptor<Message>>> {
        let mut out = Vec::new();
        let mut ancestry = Vec::new();
        let mut seen = HashSet::new();
        collect(&self.root, &mut ancestry, &mut seen, &mut out)?;
        Some(out)
    }

    pub(crate) fn has_notice_modal(&self) -> bool {
        self.root.has_notice_modal()
    }
}

fn collect<Message>(
    node: &SurfaceNode<Message>,
    ancestry: &mut Vec<(NodeId, Option<SourceIdentity>, SourceCompatibility)>,
    seen: &mut HashSet<NodeId>,
    out: &mut Vec<ProjectedNoticeDescriptor<Message>>,
) -> Option<()> {
    if !node.has_notice_demand() {
        return Some(());
    }
    if ancestry.len() >= 128 || seen.len() >= 65_536 || !seen.insert(node.id()) {
        return None;
    }
    ancestry.push((
        node.id(),
        node.source_metadata_handle().map(|source| source.identity),
        SourceCompatibility::from_surface_node(node),
    ));
    if let Some(demand) = node.notice_demand() {
        if out.len() >= 64 {
            return None;
        }
        let mut widgets = Vec::new();
        let mut widget_visited = 0;
        collect_widgets(node, &mut widgets, 0, &mut widget_visited)?;
        out.push(ProjectedNoticeDescriptor {
            identity: NoticeOwnerIdentity {
                capability: std::any::TypeId::of::<NoticeDemand<()>>(),
                ancestry: ancestry.clone(),
            },
            node_id: node.id(),
            demand,
            widgets,
        });
    }
    match node {
        SurfaceNode::Container(container) => {
            for child in &container.children {
                collect(&child.child, ancestry, seen, out)?;
            }
        }
        SurfaceNode::FloatingLayer(layer) => {
            for child in &layer.container.children {
                collect(&child.child, ancestry, seen, out)?;
            }
        }
        SurfaceNode::Scene(scene) => {
            collect(&scene.base, ancestry, seen, out)?;
            for layer in scene.ordered_layers() {
                if let Some(input) = &layer.input {
                    collect(input, ancestry, seen, out)?;
                }
                collect(&layer.node, ancestry, seen, out)?;
            }
        }
        _ => {}
    }
    ancestry.pop();
    Some(())
}

impl<Message> SurfaceNode<Message> {
    pub(crate) fn has_notice_modal(&self) -> bool {
        self.has_notice_modal_bounded(0, &mut 0)
    }

    fn has_notice_modal_bounded(&self, depth: usize, visited: &mut usize) -> bool {
        if depth >= 128 || *visited >= 65_536 {
            // An incomplete accepted-surface walk must pause expiry safely.
            return true;
        }
        *visited += 1;
        match self {
            Self::Scene(scene) => {
                scene.layers.iter().any(|layer| {
                    layer.kind == super::LayerKind::Modal
                        || layer
                            .input
                            .as_ref()
                            .is_some_and(|n| n.has_notice_modal_bounded(depth + 1, visited))
                        || layer.node.has_notice_modal_bounded(depth + 1, visited)
                }) || scene.base.has_notice_modal_bounded(depth + 1, visited)
            }
            Self::Container(container) => container
                .children
                .iter()
                .any(|child| child.child.has_notice_modal_bounded(depth + 1, visited)),
            Self::FloatingLayer(layer) => layer
                .container
                .children
                .iter()
                .any(|child| child.child.has_notice_modal_bounded(depth + 1, visited)),
            Self::Widget(_) | Self::Overlay(_) => false,
        }
    }
}

fn collect_widgets<Message>(
    node: &SurfaceNode<Message>,
    out: &mut Vec<WidgetId>,
    depth: usize,
    visited: &mut usize,
) -> Option<()> {
    if depth >= 128 || *visited >= 65_536 {
        return None;
    }
    *visited += 1;
    match node {
        SurfaceNode::Widget(widget) => out.push(widget.id()),
        SurfaceNode::Container(container) => {
            for child in &container.children {
                collect_widgets(&child.child, out, depth + 1, visited)?;
            }
        }
        SurfaceNode::FloatingLayer(layer) => {
            for child in &layer.container.children {
                collect_widgets(&child.child, out, depth + 1, visited)?;
            }
        }
        SurfaceNode::Scene(scene) => {
            collect_widgets(&scene.base, out, depth + 1, visited)?;
            for layer in scene.ordered_layers() {
                if let Some(input) = &layer.input {
                    collect_widgets(input, out, depth + 1, visited)?;
                }
                collect_widgets(&layer.node, out, depth + 1, visited)?;
            }
        }
        SurfaceNode::Overlay(_) => {}
    }
    Some(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        application::{IntoView, NoticeQueue, NoticeSeverity, notifications, scene, text},
        layout::ContainerPolicy,
        runtime::{SurfaceChild, SurfaceNode, UiSurface},
    };

    fn queue_with(count: u64) -> NoticeQueue {
        let mut queue = NoticeQueue::new();
        for id in 0..count {
            queue
                .push(
                    crate::application::Notice::new(
                        id,
                        NoticeSeverity::Info,
                        format!("notice-{id}"),
                    )
                    .unwrap(),
                )
                .unwrap();
        }
        queue
    }

    #[test]
    fn accepted_notice_projection_is_bounded_and_clone_preserves_demand() {
        let queue = queue_with(64);
        let surface = scene(text::<()>("base"))
            .layer(notifications(queue.snapshot()).on_dismiss(|_| ()).layer())
            .into_view()
            .into_surface();
        let descriptors = surface
            .notice_descriptors()
            .expect("valid notice projection");
        assert_eq!(descriptors.len(), 4);
        assert!(descriptors.iter().all(|item| !item.widgets.is_empty()));
        assert_eq!(surface.clone().notice_descriptors().unwrap().len(), 4);
    }

    #[test]
    fn notice_descriptor_collection_fails_closed_above_sixty_four() {
        let queue = queue_with(1);
        let demand = scene(text::<()>("base"))
            .layer(notifications(queue.snapshot()).on_dismiss(|_| ()).layer())
            .into_view()
            .into_surface()
            .notice_descriptors()
            .unwrap()
            .remove(0)
            .demand;
        let children = (0..65)
            .map(|id| {
                SurfaceChild::fill(
                    SurfaceNode::container(id + 1000, ContainerPolicy::default(), Vec::new())
                        .with_notice_demand(Some(Rc::clone(&demand))),
                )
            })
            .collect();
        let surface = UiSurface::new(SurfaceNode::container(
            999,
            ContainerPolicy::default(),
            children,
        ));
        assert!(surface.notice_descriptors().is_none());
    }

    #[test]
    fn modal_detection_follows_accepted_scene_layers() {
        let base = crate::application::text::<()>("base");
        let modal = scene(base.overlays(crate::application::overlays().modal(text("modal"))))
            .into_view()
            .into_surface();
        assert!(modal.has_notice_modal());
        assert!(
            !crate::application::text::<()>("plain")
                .into_surface()
                .has_notice_modal()
        );
    }

    #[test]
    fn modal_detection_pauses_on_bounded_walk_exhaustion() {
        let mut node = SurfaceNode::<()>::container(1, ContainerPolicy::default(), Vec::new());
        for id in 2..140 {
            node = SurfaceNode::container(
                id,
                ContainerPolicy::default(),
                vec![SurfaceChild::fill(node)],
            );
        }
        assert!(node.has_notice_modal());
    }
}
