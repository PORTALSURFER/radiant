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
    pub(super) split_pane_runtime: Vec<crate::gui::layout_core::SplitPaneRuntimeStateInput>,
    pub(super) split_pane_dividers: Vec<crate::gui::layout_core::SplitPaneDividerDescriptor>,
    pub(super) virtual_layout_registrations:
        Vec<crate::runtime::surface::VirtualLayoutRegistration<Message>>,
    pub(super) layout_targets: Vec<RuntimeLayoutHitTarget<Message>>,
    pub(super) split_pane_separator_projections:
        Vec<super::split_pane_separator::SplitPaneSeparatorProjection>,
    pub(super) layout_hit_region_diagnostics: LayoutHitRegionDiagnostics,
    layout_region_declarations: Vec<LayoutHitRegion>,
}

pub(super) struct RuntimeLayoutHitTarget<Message> {
    pub(super) target: LayoutHitTarget,
    pub(super) contract_version: u16,
    pub(super) state_id: Option<crate::layout::ContainerStateId>,
    pub(super) interaction: std::rc::Rc<dyn LayoutInteraction<Message>>,
    pub(super) revision: LayoutInteractionRevision,
    pub(super) container_bounds: Option<Rect>,
    pub(super) target_bounds: Option<Rect>,
    pub(super) divider_bounds: Option<Rect>,
    pub(super) mounted_state_id: Option<crate::gui::layout_core::MountedContainerStateId>,
    pub(super) split_capture_witness: Option<crate::gui::layout_core::SplitPaneCaptureWitness>,
}

impl<Message> Default for RuntimeContainerTraversal<Message> {
    fn default() -> Self {
        Self {
            styled: HitOrderIndex::default(),
            scroll: HitOrderIndex::default(),
            clip_ancestors: HashMap::new(),
            scroll_content_by_container: HashMap::new(),
            layout_interactions: Vec::new(),
            split_pane_runtime: Vec::new(),
            split_pane_dividers: Vec::new(),
            virtual_layout_registrations: Vec::new(),
            layout_targets: Vec::new(),
            split_pane_separator_projections: Vec::new(),
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
                    state_id: interaction.state.as_ref().map(|state| state.id()),
                    interaction: std::rc::Rc::clone(&interaction.interaction),
                    revision: interaction.revision.clone(),
                    container_bounds: Some(container_bounds),
                    target_bounds: Some(bounds),
                    divider_bounds: None,
                    mounted_state_id: None,
                    split_capture_witness: None,
                });
            }

            let Some(descriptor) = self
                .split_pane_dividers
                .iter()
                .find(|descriptor| descriptor.container_id == interaction.id)
                .copied()
            else {
                continue;
            };
            let Some(first) = layout.rects.get(&descriptor.first_child).copied() else {
                continue;
            };
            let Some(second) = layout.rects.get(&descriptor.second_child).copied() else {
                continue;
            };
            let Some((divider_bounds, _split_bounds)) =
                split_divider_geometry(first, second, descriptor.axis)
            else {
                continue;
            };
            let Some(target_bounds) = std::iter::once(container_bounds)
                .chain(self.layout_clip_for_container(interaction.id, layout))
                .try_fold(divider_bounds, |bounds, clip| bounds.intersection(clip))
                .filter(|rect| rect.has_finite_positive_area())
            else {
                continue;
            };
            self.layout_targets.push(RuntimeLayoutHitTarget {
                target: LayoutHitTarget {
                    container_id: interaction.id,
                    region_id: crate::gui::layout_core::SPLIT_PANE_DIVIDER_REGION_ID,
                    bounds: target_bounds,
                },
                contract_version: interaction.contract_version,
                state_id: interaction.state.as_ref().map(|state| state.id()),
                interaction: std::rc::Rc::clone(&interaction.interaction),
                revision: interaction.revision.clone(),
                container_bounds: Some(container_bounds),
                target_bounds: Some(target_bounds),
                divider_bounds: Some(divider_bounds),
                mounted_state_id: None,
                split_capture_witness: Some(descriptor.witness(container_bounds)),
            });
        }
    }

    pub(super) fn bind_committed_mounted_state_ids(
        &mut self,
        state_store: &super::layout_state::RuntimeLayoutContainerStateStore,
    ) {
        for target in &mut self.layout_targets {
            target.mounted_state_id = target
                .state_id
                .and_then(|state_id| state_store.current_mounted_state_id(state_id));
        }
    }

    pub(super) fn rebuild_split_pane_separator_projections(
        &mut self,
        state_store: &super::layout_state::RuntimeLayoutContainerStateStore,
    ) {
        let mut next = Vec::with_capacity(
            self.split_pane_runtime
                .len()
                .min(self.split_pane_dividers.len()),
        );
        for input in &self.split_pane_runtime {
            if self
                .split_pane_runtime
                .iter()
                .filter(|candidate| candidate.container_id == input.container_id)
                .count()
                != 1
            {
                continue;
            }

            let descriptor_count = self
                .split_pane_dividers
                .iter()
                .filter(|descriptor| descriptor.container_id == input.container_id)
                .count();
            if descriptor_count != 1 {
                continue;
            }
            let Some(descriptor) = self
                .split_pane_dividers
                .iter()
                .find(|descriptor| descriptor.container_id == input.container_id)
                .copied()
            else {
                continue;
            };

            let identity = crate::layout::LayoutTargetIdentity::new(
                input.container_id,
                crate::gui::layout_core::SPLIT_PANE_DIVIDER_REGION_ID,
            );
            let target_count = self
                .layout_targets
                .iter()
                .filter(|target| target.target.identity() == identity)
                .count();
            if target_count != 1 {
                continue;
            }
            let Some(target) = self
                .layout_targets
                .iter()
                .find(|target| target.target.identity() == identity)
            else {
                continue;
            };
            if let Some(projection) =
                super::split_pane_separator::build_split_pane_separator_projection(
                    target,
                    *input,
                    descriptor,
                    state_store,
                )
            {
                next.push(projection);
            }
        }
        self.split_pane_separator_projections = next;
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

fn split_divider_geometry(
    first: Rect,
    second: Rect,
    axis: crate::gui::panel::SplitPaneAxis,
) -> Option<(Rect, Rect)> {
    if !first.is_finite() || !second.is_finite() {
        return None;
    }
    let (divider, split) = match axis {
        crate::gui::panel::SplitPaneAxis::Horizontal => {
            let start = first.max.x;
            let end = second.min.x;
            if !start.is_finite() || !end.is_finite() || end <= start {
                return None;
            }
            (
                Rect::from_min_max(
                    crate::gui::types::Point::new(start, first.min.y.min(second.min.y)),
                    crate::gui::types::Point::new(end, first.max.y.max(second.max.y)),
                ),
                Rect::from_min_max(
                    crate::gui::types::Point::new(
                        first.min.x.min(second.min.x),
                        first.min.y.min(second.min.y),
                    ),
                    crate::gui::types::Point::new(
                        first.max.x.max(second.max.x),
                        first.max.y.max(second.max.y),
                    ),
                ),
            )
        }
        crate::gui::panel::SplitPaneAxis::Vertical => {
            let start = first.max.y;
            let end = second.min.y;
            if !start.is_finite() || !end.is_finite() || end <= start {
                return None;
            }
            (
                Rect::from_min_max(
                    crate::gui::types::Point::new(first.min.x.min(second.min.x), start),
                    crate::gui::types::Point::new(first.max.x.max(second.max.x), end),
                ),
                Rect::from_min_max(
                    crate::gui::types::Point::new(
                        first.min.x.min(second.min.x),
                        first.min.y.min(second.min.y),
                    ),
                    crate::gui::types::Point::new(
                        first.max.x.max(second.max.x),
                        first.max.y.max(second.max.y),
                    ),
                ),
            )
        }
    };
    (divider.has_finite_positive_area() && split.has_finite_positive_area())
        .then_some((divider, split))
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
