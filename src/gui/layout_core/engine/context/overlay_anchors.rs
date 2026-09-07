//! Bounded structural evidence for current-pass overlay anchors.

use crate::gui::layout_core::tree::LayoutNode;
use crate::gui::layout_core::{ContainerKind, NodeId, OverflowPolicy, OverlayAnchor};
use crate::gui::types::Rect;
use std::collections::{HashMap, HashSet};

const MAX_ANCHORS: usize = 64;
const MAX_NODES: usize = 65_536;
const MAX_CLIP_ANCESTORS: usize = 64;

#[derive(Clone, Copy)]
pub(super) enum ClipAncestor {
    Container(NodeId),
    ScrollViewport(NodeId),
}

pub(super) struct OverlayAnchorEvidence {
    targets: HashMap<NodeId, TargetEvidence>,
}

struct TargetEvidence {
    seen: bool,
    clips: Option<Vec<ClipAncestor>>,
}

impl OverlayAnchorEvidence {
    pub(super) fn collect(root: &LayoutNode) -> Option<Self> {
        if !root.contains_overlay_anchor() {
            return None;
        }
        let mut requested = HashSet::new();
        let mut requested_nodes = 0;
        if !collect_requested(root, &mut requested, &mut requested_nodes) || requested.is_empty() {
            return Some(Self {
                targets: HashMap::new(),
            });
        }
        let mut targets = requested
            .into_iter()
            .map(|id| {
                (
                    id,
                    TargetEvidence {
                        seen: false,
                        clips: None,
                    },
                )
            })
            .collect();
        let mut state = TraverseState {
            nodes: 0,
            exhausted: false,
        };
        collect_targets(root, &mut Vec::new(), &mut targets, &mut state);
        if state.exhausted {
            for value in targets.values_mut() {
                value.clips = None;
            }
        }
        Some(Self { targets })
    }

    pub(super) fn clips_for(&self, anchor: OverlayAnchor) -> Option<&[ClipAncestor]> {
        self.targets.get(&anchor.target)?.clips.as_deref()
    }
}

struct TraverseState {
    nodes: usize,
    exhausted: bool,
}

fn collect_requested(
    node: &LayoutNode,
    requested: &mut HashSet<NodeId>,
    visited: &mut usize,
) -> bool {
    *visited += 1;
    if *visited > MAX_NODES {
        return false;
    }
    if let LayoutNode::Container(container) = node {
        if let Some(anchor) = &container.overlay_anchor {
            requested.insert(anchor.anchor.target);
            if requested.len() > MAX_ANCHORS {
                return false;
            }
        }
        for child in &container.children {
            if !collect_requested(&child.child, requested, visited) {
                return false;
            }
        }
    }
    true
}

fn collect_targets(
    node: &LayoutNode,
    clips: &mut Vec<ClipAncestor>,
    targets: &mut HashMap<NodeId, TargetEvidence>,
    state: &mut TraverseState,
) {
    if state.exhausted {
        return;
    }
    state.nodes += 1;
    if state.nodes > MAX_NODES {
        state.exhausted = true;
        return;
    }
    if let Some(entry) = targets.get_mut(&node.id()) {
        if entry.seen {
            entry.clips = None;
        } else {
            entry.seen = true;
            entry.clips = Some(clips.clone());
        }
    }
    let LayoutNode::Container(container) = node else {
        return;
    };
    let clip = match container.policy.kind {
        ContainerKind::ScrollView => Some(ClipAncestor::ScrollViewport(container.id)),
        _ if container.policy.overflow == OverflowPolicy::Clip => {
            Some(ClipAncestor::Container(container.id))
        }
        _ => None,
    };
    if let Some(clip) = clip {
        if clips.len() >= MAX_CLIP_ANCESTORS {
            // Current-pass placement requires complete clipping evidence. A
            // partial subtree cannot be repaired by a later duplicate target.
            state.exhausted = true;
            return;
        }
        clips.push(clip);
    }
    for child in &container.children {
        collect_targets(&child.child, clips, targets, state);
        if state.exhausted {
            break;
        }
    }
    if clip.is_some() {
        clips.pop();
    }
}

pub(super) fn current_trigger(
    anchor: OverlayAnchor,
    evidence: &OverlayAnchorEvidence,
    rects: &std::collections::BTreeMap<NodeId, Rect>,
    viewports: &std::collections::BTreeMap<NodeId, Rect>,
    viewport: Rect,
) -> Option<Rect> {
    let trigger = *rects.get(&anchor.target)?;
    let mut visible = intersect(trigger, viewport)?;
    for clip in evidence.clips_for(anchor)? {
        let clip_rect = match clip {
            ClipAncestor::Container(id) => *rects.get(id)?,
            ClipAncestor::ScrollViewport(id) => *viewports.get(id)?,
        };
        visible = intersect(visible, clip_rect)?;
    }
    visible.has_finite_positive_area().then_some(trigger)
}

fn intersect(left: Rect, right: Rect) -> Option<Rect> {
    let min =
        crate::gui::types::Point::new(left.min.x.max(right.min.x), left.min.y.max(right.min.y));
    let max =
        crate::gui::types::Point::new(left.max.x.min(right.max.x), left.max.y.min(right.max.y));
    let rect = Rect::from_min_max(min, max);
    rect.has_finite_positive_area().then_some(rect)
}
