//! Bounded command ownership derived from the accepted surface projection.
use super::{LayerKind, SourceTraversalIndex, SurfaceNode, WidgetPath};
use crate::{
    application::{
        CommandScopeAttachment, CommandScopeKind, CommandSuppression, ResolvedCommandScope,
    },
    layout::{LayoutOutput, NodeId},
};
use std::collections::HashSet;

const MAX_SCOPES: usize = 1024;
const MAX_SOURCE_NODES: usize = 65_536;

#[derive(Clone, Copy, Default)]
pub(in crate::runtime) struct CommandLayerContext {
    modal: Option<u32>,
    overlay: Option<u32>,
    passive: bool,
    input: bool,
}
struct Record {
    node_id: NodeId,
    layout_id: NodeId,
    path: WidgetPath,
    depth: u32,
    layer: CommandLayerContext,
    attachment: CommandScopeAttachment,
}
#[derive(Default)]
pub(in crate::runtime) struct SurfaceCommandScopes {
    records: Vec<Record>,
    error: Option<CommandSuppression>,
    depth: u32,
    layer_order: u32,
    layer: CommandLayerContext,
    seen: HashSet<NodeId>,
}
impl SurfaceCommandScopes {
    pub(in crate::runtime) fn clear(&mut self) {
        self.records.clear();
        self.seen.clear();
        self.error = None;
        self.depth = 0;
        self.layer_order = 0;
        self.layer = CommandLayerContext::default();
    }
    pub(in crate::runtime) fn enter<Message>(
        &mut self,
        node: &SurfaceNode<Message>,
        path: &[usize],
    ) -> u32 {
        let depth = self.depth;
        self.depth = self.depth.saturating_add(1);
        let Some(attachment) = node.command_scope() else {
            return depth;
        };
        if self.layer.passive
            || self.layer.input
            || matches!(node, SurfaceNode::Overlay(_))
            || matches!(node, SurfaceNode::FloatingLayer(layer) if !layer.interactive)
        {
            return depth;
        }
        if self.records.len() == MAX_SCOPES {
            self.error = Some(CommandSuppression::Capacity);
            return depth;
        }
        self.records.push(Record {
            node_id: node.id(),
            layout_id: layout_identity(node),
            path: WidgetPath::from_slice(path),
            depth,
            layer: self.layer,
            attachment: attachment.clone(),
        });
        depth
    }
    pub(in crate::runtime) fn leave(&mut self, depth: u32) {
        self.depth = depth;
    }
    pub(in crate::runtime) fn enter_layer(
        &mut self,
        kind: LayerKind,
        input: bool,
    ) -> CommandLayerContext {
        let previous = self.layer;
        self.layer_order = self.layer_order.saturating_add(1);
        if self.layer_order == u32::MAX {
            self.error = Some(CommandSuppression::Capacity);
        }
        self.layer.input |= input;
        self.layer.passive |= matches!(kind, LayerKind::Tooltip | LayerKind::DragPreview);
        if kind == LayerKind::Modal {
            self.layer.modal = Some(self.layer_order);
        } else {
            self.layer.overlay = Some(self.layer_order);
        }
        previous
    }
    pub(in crate::runtime) fn leave_layer(&mut self, previous: CommandLayerContext) {
        self.layer = previous;
    }
    pub(in crate::runtime) fn qualify(&mut self, source: &SourceTraversalIndex) {
        if self.records.is_empty() {
            return;
        }
        if source.records.len() > MAX_SOURCE_NODES {
            self.error = Some(CommandSuppression::Capacity);
            return;
        }
        for record in &source.records {
            if !self.seen.insert(record.node_id) {
                self.error = Some(CommandSuppression::InvalidScopes);
            }
        }
    }
    pub(in crate::runtime) fn application(
        &self,
        layout: &LayoutOutput,
    ) -> (Vec<ResolvedCommandScope>, Option<CommandSuppression>) {
        if self.error.is_some() {
            return (Vec::new(), self.error);
        }
        let mut scopes = Vec::new();
        for record in &self.records {
            if record.attachment.kind != CommandScopeKind::Application
                || !layout.rects.contains_key(&record.layout_id)
            {
                continue;
            }
            if scopes.len() == 64 {
                return (Vec::new(), Some(CommandSuppression::Capacity));
            }
            scopes.push(ResolvedCommandScope {
                node_id: record.node_id,
                kind: CommandScopeKind::Application,
                attachment: record.attachment.clone(),
            });
        }
        (scopes, None)
    }
    pub(in crate::runtime) fn active(
        &self,
        layout: &LayoutOutput,
        focused: Option<&[usize]>,
    ) -> (Vec<ResolvedCommandScope>, Option<CommandSuppression>) {
        if self.error.is_some() {
            return (Vec::new(), self.error);
        }
        let mut scopes = Vec::new();
        for record in &self.records {
            if !layout.rects.contains_key(&record.layout_id) {
                continue;
            }
            let kind = match record.attachment.kind {
                CommandScopeKind::Editor { .. } => {
                    if !focused.is_some_and(|path| path.starts_with(record.path.as_slice())) {
                        continue;
                    }
                    CommandScopeKind::Editor {
                        depth: record.depth,
                    }
                }
                CommandScopeKind::Modal { .. } => {
                    let Some(order) = record.layer.modal else {
                        return (Vec::new(), Some(CommandSuppression::InvalidScopes));
                    };
                    CommandScopeKind::Modal { order }
                }
                CommandScopeKind::Overlay { .. } => {
                    let Some(order) = record.layer.overlay else {
                        return (Vec::new(), Some(CommandSuppression::InvalidScopes));
                    };
                    CommandScopeKind::Overlay { order }
                }
                kind => kind,
            };
            if scopes.len() == 64 {
                return (Vec::new(), Some(CommandSuppression::Capacity));
            }
            scopes.push(ResolvedCommandScope {
                node_id: record.node_id,
                kind,
                attachment: record.attachment.clone(),
            });
        }
        (scopes, None)
    }
}
fn layout_identity<Message>(node: &SurfaceNode<Message>) -> NodeId {
    match node {
        SurfaceNode::Scene(scene) if !scene.has_layers() => layout_identity(&scene.base),
        _ => node.id(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::application::{CommandBinding, CommandId, CommandScope, IntoView, text};

    fn node(id: NodeId) -> SurfaceNode<()> {
        text("scope")
            .id(id)
            .command_scope(
                CommandScope::new(
                    format!("scope-{id}"),
                    CommandScopeKind::Window,
                    [CommandBinding::new(CommandId::new("save").unwrap(), ())],
                )
                .unwrap(),
            )
            .into_node()
    }

    #[test]
    fn application_exports_require_layout_and_preserve_collection_errors() {
        let mut index = SurfaceCommandScopes::default();
        let local = node(1);
        let global = text::<()>("global")
            .id(2)
            .command_scope(
                CommandScope::new(
                    "global",
                    CommandScopeKind::Application,
                    [CommandBinding::new(CommandId::new("save").unwrap(), ())],
                )
                .unwrap(),
            )
            .into_node();
        for node in [&local, &global] {
            let depth = index.enter(node, &[]);
            index.leave(depth);
        }
        let mut layout = LayoutOutput::default();
        layout.rects.insert(1, Default::default());
        assert!(index.application(&layout).0.is_empty());
        layout.rects.insert(2, Default::default());
        let (records, error) = index.application(&layout);
        assert_eq!(error, None);
        assert_eq!(records.len(), 1);
        assert_eq!(records[0].node_id, 2);
        index.error = Some(CommandSuppression::Capacity);
        let (records, error) = index.application(&layout);
        assert!(records.is_empty());
        assert_eq!(error, Some(CommandSuppression::Capacity));
    }

    #[test]
    fn omitted_layout_and_duplicate_raw_identity_cannot_activate() {
        let mut index = SurfaceCommandScopes::default();
        let node = node(42);
        let depth = index.enter(&node, &[]);
        index.leave(depth);
        let (active, error) = index.active(&LayoutOutput::default(), None);
        assert!(active.is_empty());
        assert_eq!(error, None);
        let mut source = SourceTraversalIndex::default();
        source.record_node(&node);
        source.record_node(&node);
        index.qualify(&source);
        assert_eq!(index.error, Some(CommandSuppression::InvalidScopes));
        index.clear();
        assert!(index.records.is_empty());
        assert_eq!(index.error, None);
    }

    #[test]
    fn collection_limits_fail_closed_without_retaining_excess_attachments() {
        let mut index = SurfaceCommandScopes::default();
        for id in 1..=MAX_SCOPES + 1 {
            let depth = index.enter(&node(id as NodeId), &[]);
            index.leave(depth);
        }
        assert_eq!(index.records.len(), MAX_SCOPES);
        assert_eq!(index.error, Some(CommandSuppression::Capacity));
        index.clear();
        let node = node(42);
        let depth = index.enter(&node, &[]);
        index.leave(depth);
        let mut source = SourceTraversalIndex::default();
        for _ in 0..=MAX_SOURCE_NODES {
            source.record_node(&node);
        }
        index.qualify(&source);
        assert_eq!(index.error, Some(CommandSuppression::Capacity));
        assert!(index.seen.is_empty());
    }
}
