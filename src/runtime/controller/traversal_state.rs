//! Traversal indexes and lookup caches derived from the projected surface tree.

use super::{ClipAncestors, WidgetPath, hit_order::HitOrderIndex};
use crate::{
    layout::{
        LayoutHitRegion, LayoutHitRegionDiagnostics, LayoutHitTarget, LayoutInteraction,
        LayoutInteractionRevision, LayoutOutput, NodeId, Rect,
    },
    runtime::{SurfaceLayoutInteractionRecord, WheelHitTarget},
    widgets::WidgetId,
};
use std::collections::{HashMap, HashSet};

pub(super) struct RuntimeTraversalState<Message = ()> {
    pub(super) widgets: RuntimeWidgetTraversal,
    pub(super) containers: RuntimeContainerTraversal<Message>,
}

impl<Message> Default for RuntimeTraversalState<Message> {
    fn default() -> Self {
        Self {
            widgets: RuntimeWidgetTraversal::default(),
            containers: RuntimeContainerTraversal::default(),
        }
    }
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

pub(super) struct RuntimeContainerTraversal<Message = ()> {
    pub(super) styled: HitOrderIndex,
    pub(super) scroll: HitOrderIndex,
    pub(super) clip_ancestors: HashMap<NodeId, ClipAncestors>,
    pub(super) scroll_content_by_container: HashMap<NodeId, NodeId>,
    pub(super) layout_interactions: Vec<SurfaceLayoutInteractionRecord<Message>>,
    pub(super) layout_targets: Vec<RuntimeLayoutHitTarget<Message>>,
    pub(super) layout_hit_region_diagnostics: LayoutHitRegionDiagnostics,
    layout_region_declarations: Vec<LayoutHitRegion>,
}

pub(super) struct RuntimeLayoutHitTarget<Message> {
    pub(super) target: LayoutHitTarget,
    pub(super) contract_version: u16,
    pub(super) interaction: std::rc::Rc<dyn LayoutInteraction<Message>>,
    pub(super) revision: LayoutInteractionRevision,
}

impl<Message> Default for RuntimeContainerTraversal<Message> {
    fn default() -> Self {
        Self {
            styled: HitOrderIndex::default(),
            scroll: HitOrderIndex::default(),
            clip_ancestors: HashMap::new(),
            scroll_content_by_container: HashMap::new(),
            layout_interactions: Vec::new(),
            layout_targets: Vec::new(),
            layout_hit_region_diagnostics: LayoutHitRegionDiagnostics::default(),
            layout_region_declarations: Vec::new(),
        }
    }
}

impl<Message> RuntimeContainerTraversal<Message> {
    pub(super) fn project_layout_targets(&mut self, layout: &LayoutOutput) {
        self.layout_targets.clear();
        self.layout_hit_region_diagnostics = LayoutHitRegionDiagnostics::default();
        for interaction in &self.layout_interactions {
            let Some(container_bounds) = layout.rects.get(&interaction.id).copied() else {
                continue;
            };
            if !container_bounds.has_finite_positive_area() {
                continue;
            }

            self.layout_region_declarations.clear();
            interaction
                .interaction
                .visit_hit_regions(container_bounds, &mut |region| {
                    if !self
                        .layout_region_declarations
                        .iter()
                        .any(|candidate| candidate.id() == region.id())
                    {
                        self.layout_region_declarations.push(region);
                    } else {
                        self.layout_hit_region_diagnostics.record_duplicate();
                    }
                });

            for region in self.layout_region_declarations.iter().copied() {
                let Some(bounds) = project_layout_region(container_bounds, region.bounds()) else {
                    continue;
                };
                let Some(bounds) = self
                    .layout_clip_for_container(interaction.id, layout)
                    .try_fold(bounds, |bounds, clip| bounds.intersection(clip))
                    .filter(|rect| rect.has_finite_positive_area())
                else {
                    continue;
                };
                self.layout_targets.push(RuntimeLayoutHitTarget {
                    target: LayoutHitTarget {
                        container_id: interaction.id,
                        region_id: region.id(),
                        bounds,
                    },
                    contract_version: interaction.contract_version,
                    interaction: std::rc::Rc::clone(&interaction.interaction),
                    revision: interaction.revision.clone(),
                });
            }
        }
    }

    fn layout_clip_for_container<'a>(
        &'a self,
        container_id: NodeId,
        layout: &'a LayoutOutput,
    ) -> impl Iterator<Item = Rect> + 'a {
        let own_viewport = layout.viewport_bounds.get(&container_id).copied();
        let ancestors = self
            .clip_ancestors
            .get(&container_id)
            .into_iter()
            .flat_map(|ancestors| ancestors.as_slice().iter().copied())
            .filter_map(|ancestor| {
                layout
                    .viewport_bounds
                    .get(&ancestor)
                    .copied()
                    .or_else(|| layout.rects.get(&ancestor).copied())
            });
        own_viewport.into_iter().chain(ancestors)
    }
}

fn project_layout_region(container_bounds: Rect, local_bounds: Rect) -> Option<Rect> {
    let projected = Rect::from_min_max(
        crate::gui::types::Point::new(
            container_bounds.min.x + container_bounds.width() * local_bounds.min.x,
            container_bounds.min.y + container_bounds.height() * local_bounds.min.y,
        ),
        crate::gui::types::Point::new(
            container_bounds.min.x + container_bounds.width() * local_bounds.max.x,
            container_bounds.min.y + container_bounds.height() * local_bounds.max.y,
        ),
    );
    projected.has_finite_positive_area().then_some(projected)
}
