//! Bounded accepted resource-view demand inventory.
use super::{SourceCompatibility, SourceIdentity, SurfaceNode, UiSurface};
use crate::{application::resource_view::demand::ResourceViewDemand, layout::NodeId};
use std::{collections::HashSet, rc::Rc};

#[derive(Clone, Debug, Eq, PartialEq, Hash)]
pub(in crate::runtime) struct ResourceViewIdentity {
    ancestry: Vec<(NodeId, Option<SourceIdentity>, SourceCompatibility)>,
    pub interest_id: u64,
}

pub(in crate::runtime) struct ProjectedResourceDemand {
    pub identity: ResourceViewIdentity,
    pub demand: Rc<ResourceViewDemand>,
}

impl<Message> UiSurface<Message> {
    pub(in crate::runtime) fn resource_view_demands(&self) -> Option<Vec<ProjectedResourceDemand>> {
        let mut demands = Vec::new();
        let mut ancestry = Vec::new();
        let mut seen = HashSet::new();
        collect(&self.root, &mut ancestry, &mut seen, &mut demands)?;
        Some(demands)
    }
}

fn collect<Message>(
    node: &SurfaceNode<Message>,
    ancestry: &mut Vec<(NodeId, Option<SourceIdentity>, SourceCompatibility)>,
    seen: &mut HashSet<NodeId>,
    demands: &mut Vec<ProjectedResourceDemand>,
) -> Option<()> {
    if !node.has_resource_view_demand() {
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
    if let Some(demand) = node.resource_view_demand() {
        if demands.len() >= 1024 {
            return None;
        }
        demands.push(ProjectedResourceDemand {
            identity: ResourceViewIdentity {
                ancestry: ancestry.clone(),
                interest_id: demand.interest_id,
            },
            demand,
        });
    }
    match node {
        SurfaceNode::Container(container) => {
            for child in &container.children {
                collect(&child.child, ancestry, seen, demands)?;
            }
        }
        SurfaceNode::FloatingLayer(layer) => {
            for child in &layer.container.children {
                collect(&child.child, ancestry, seen, demands)?;
            }
        }
        SurfaceNode::Scene(scene) => {
            collect(&scene.base, ancestry, seen, demands)?;
            for layer in scene.ordered_layers() {
                if let Some(input) = &layer.input {
                    collect(input, ancestry, seen, demands)?;
                }
                collect(&layer.node, ancestry, seen, demands)?;
            }
        }
        _ => {}
    }
    ancestry.pop();
    Some(())
}
