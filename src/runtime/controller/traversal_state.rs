//! Traversal indexes and lookup caches derived from the projected surface tree.

use super::{ClipAncestors, WidgetPath, hit_order::HitOrderIndex};
use crate::{
    layout::{LayoutOutput, NodeId},
    runtime::WheelHitTarget,
    widgets::WidgetId,
};
use std::collections::{HashMap, HashSet};

#[derive(Default)]
pub(super) struct RuntimeTraversalState {
    pub(super) widgets: RuntimeWidgetTraversal,
    pub(super) containers: RuntimeContainerTraversal,
}

#[derive(Default)]
pub(super) struct RuntimeWidgetTraversal {
    pub(super) hit_order: Vec<WidgetId>,
    pub(super) focusable: HitOrderIndex,
    pub(super) pointer: HitOrderIndex,
    pub(super) native_file_drop: HitOrderIndex,
    pub(super) keyboard_focus: HitOrderIndex,
    pub(super) wheel: HitOrderIndex,
    pub(super) wheel_targets: RuntimeWheelTargetTraversal,
    pub(super) stateful_order: Vec<WidgetId>,
    pub(super) paths: RuntimeWidgetPathState,
}

#[derive(Default)]
pub(super) struct RuntimeWheelTargetTraversal {
    order: Vec<WheelHitTarget>,
    visible: Vec<WheelHitTarget>,
}

impl RuntimeWheelTargetTraversal {
    pub(super) fn set_order(&mut self, order: Vec<WheelHitTarget>) {
        self.order = order;
        self.visible.clear();
    }

    pub(super) fn refresh_visible(&mut self, layout: &LayoutOutput) {
        self.visible.clear();
        self.visible.extend(
            self.order
                .iter()
                .copied()
                .filter(|target: &WheelHitTarget| layout.rects.contains_key(&target.node_id())),
        );
    }

    pub(super) fn visible(&self) -> &[WheelHitTarget] {
        &self.visible
    }

    pub(super) fn take_order(&mut self) -> Vec<WheelHitTarget> {
        self.visible.clear();
        std::mem::take(&mut self.order)
    }
}

#[derive(Default)]
pub(super) struct RuntimeWidgetPathState {
    pub(super) current: HashMap<WidgetId, WidgetPath>,
    pub(super) previous: HashMap<WidgetId, WidgetPath>,
    pub(super) clip_ancestors: HashMap<WidgetId, ClipAncestors>,
    pub(super) container_hover_suppression: HashSet<WidgetId>,
}

#[derive(Default)]
pub(super) struct RuntimeContainerTraversal {
    pub(super) styled: HitOrderIndex,
    pub(super) scroll: HitOrderIndex,
    pub(super) clip_ancestors: HashMap<NodeId, ClipAncestors>,
    pub(super) scroll_content_by_container: HashMap<NodeId, NodeId>,
}
