//! Private runtime-owned split-pane divider interaction.

use super::{
    ContainerStateDeclaration, LayoutCapabilities, LayoutContainerStateContext, LayoutEventContext,
    LayoutHitRegionId, LayoutInput, LayoutInteraction, LayoutInteractionRevision, NodeId,
    SplitPanePolicy, SplitPaneRuntimeMode, SplitPaneRuntimeOwnership,
    SplitPaneRuntimePolicyRevision, SplitPaneRuntimeState, SplitPaneRuntimeStateInput,
};
use crate::{
    gui::{
        panel::{
            PanelResizeConstraints, PanelResizeEdge, SplitPaneAxis, SplitPaneCollapsePolicy,
            SplitPaneCollapseTarget, SplitPaneLayout, SplitPaneLayoutParts,
            quantized_split_pane_rects, split_pane_collapse_target,
        },
        types::{Point, Rect},
    },
    widgets::{DragHandleMessage, DragHandleMetadata, EditPhase, PointerButton, PointerModifiers},
};
use std::rc::Rc;

/// Stable private identity for the built-in split divider region.
pub(crate) const SPLIT_PANE_DIVIDER_REGION_ID: LayoutHitRegionId =
    LayoutHitRegionId::new(0x5350_4c49_545f_4449);

#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct SplitPaneDividerDescriptor {
    pub(crate) container_id: NodeId,
    pub(crate) first_child: NodeId,
    pub(crate) second_child: NodeId,
    pub(crate) axis: SplitPaneAxis,
    pub(crate) first_min_extent: f32,
    pub(crate) second_min_extent: f32,
    pub(crate) divider_extent: f32,
}

impl SplitPaneDividerDescriptor {
    pub(crate) fn from_policy(
        container_id: NodeId,
        policy: SplitPanePolicy,
        children: &[NodeId],
    ) -> Option<Self> {
        let [first_child, second_child] = children else {
            return None;
        };
        Some(Self {
            container_id,
            first_child: *first_child,
            second_child: *second_child,
            axis: policy.axis,
            first_min_extent: finite_nonnegative(policy.first_min_extent),
            second_min_extent: finite_nonnegative(policy.second_min_extent),
            divider_extent: finite_nonnegative(policy.divider_extent),
        })
    }

    pub(crate) fn witness(self, container_bounds: Rect) -> SplitPaneCaptureWitness {
        SplitPaneCaptureWitness {
            container_bounds,
            axis: self.axis,
            first_min_extent: self.first_min_extent,
            second_min_extent: self.second_min_extent,
            divider_extent: self.divider_extent,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct SplitPaneCaptureWitness {
    pub(crate) container_bounds: Rect,
    pub(crate) axis: SplitPaneAxis,
    pub(crate) first_min_extent: f32,
    pub(crate) second_min_extent: f32,
    pub(crate) divider_extent: f32,
}

pub(crate) struct SplitPaneDividerInteraction<Message> {
    policy: SplitPanePolicy,
    initial_ratio: f32,
    collapse_policy: Option<SplitPaneCollapsePolicy>,
    on_ratio_settled: Option<Rc<dyn Fn(f32) -> Message>>,
}

impl<Message> SplitPaneDividerInteraction<Message> {
    pub(crate) fn new(
        policy: SplitPanePolicy,
        collapse_policy: Option<SplitPaneCollapsePolicy>,
        on_ratio_settled: Option<Rc<dyn Fn(f32) -> Message>>,
    ) -> Self {
        Self {
            policy,
            initial_ratio: policy.initial_ratio,
            collapse_policy,
            on_ratio_settled,
        }
    }
}

pub(crate) fn runtime_owned_split_pane_capabilities<Message: 'static>(
    policy: SplitPanePolicy,
    collapse_policy: Option<SplitPaneCollapsePolicy>,
) -> LayoutCapabilities<Message> {
    runtime_owned_split_pane_capabilities_with_ratio_settled(policy, collapse_policy, None)
}

pub(crate) fn runtime_owned_split_pane_capabilities_with_ratio_settled<Message: 'static>(
    policy: SplitPanePolicy,
    collapse_policy: Option<SplitPaneCollapsePolicy>,
    on_ratio_settled: Option<Rc<dyn Fn(f32) -> Message>>,
) -> LayoutCapabilities<Message> {
    LayoutCapabilities::new().interaction_local(SplitPaneDividerInteraction::new(
        policy,
        collapse_policy,
        on_ratio_settled,
    ))
}

impl<Message> LayoutInteraction<Message> for SplitPaneDividerInteraction<Message> {
    fn revision(&self) -> LayoutInteractionRevision {
        LayoutInteractionRevision::exact(SplitPaneRuntimePolicyRevision::new(
            self.policy,
            self.collapse_policy,
        ))
    }

    fn state(&self, container_id: NodeId) -> Option<ContainerStateDeclaration> {
        Some(
            SplitPaneRuntimeStateInput {
                container_id,
                initial_ratio: self.initial_ratio,
                mode: SplitPaneRuntimeMode::RuntimeOwned {
                    collapse_policy: self.collapse_policy,
                },
                policy_revision: SplitPaneRuntimePolicyRevision::new(
                    self.policy,
                    self.collapse_policy,
                ),
            }
            .declaration(),
        )
    }

    fn handle_layout_input_with_state(
        &self,
        input: LayoutInput,
        context: &mut LayoutEventContext<Message>,
        state_context: &mut LayoutContainerStateContext<'_>,
    ) {
        let Some(state) = state_context.state_mut::<SplitPaneRuntimeState>() else {
            return;
        };
        if state.ownership != SplitPaneRuntimeOwnership::RuntimeOwned {
            return;
        }

        let constraints = PanelResizeConstraints::new(PanelResizeEdge::Right, 0.0, 1.0);
        match input {
            LayoutInput::PointerPress {
                position,
                button: PointerButton::Primary,
                modifiers,
                timestamp,
            } => {
                if state.resize.is_resizing() {
                    return;
                }
                if ratio_for_pointer(position, context, self.policy).is_none() {
                    return;
                }
                let metadata = metadata(modifiers, timestamp, None);
                let origin = Point::new(state.ratio, 0.0);
                let event = state.resize.resize_edit(
                    DragHandleMessage::started_with_metadata(origin, origin, metadata),
                    constraints,
                );
                if event.is_some() {
                    context.handle();
                    context.capture_pointer();
                }
            }
            LayoutInput::PointerMove {
                position,
                modifiers,
                timestamp,
                sequence_range,
            } => {
                if !state.resize.is_resizing() {
                    return;
                }
                let Some(ratio) = ratio_for_pointer(position, context, self.policy) else {
                    return;
                };
                if ratio.to_bits() == state.ratio.to_bits() {
                    return;
                }
                let event = state.resize.resize_edit(
                    DragHandleMessage::moved_with_metadata(
                        Point::new(ratio, 0.0),
                        metadata(modifiers, timestamp, sequence_range),
                    ),
                    constraints,
                );
                if let Some(event) = event.filter(|event| event.phase == EditPhase::Update) {
                    state.ratio = event.value;
                    context.handle();
                    context.request_work();
                }
            }
            LayoutInput::PointerRelease {
                position,
                button: PointerButton::Primary,
                modifiers,
                timestamp,
            } => {
                if !state.resize.is_resizing() {
                    return;
                }
                let Some(ratio) = ratio_for_pointer(position, context, self.policy) else {
                    return;
                };
                let changed = ratio.to_bits() != state.ratio.to_bits();
                let event = state.resize.resize_edit(
                    DragHandleMessage::ended_with_metadata(
                        Point::new(ratio, 0.0),
                        metadata(modifiers, timestamp, None),
                    ),
                    constraints,
                );
                if let Some(event) = event.filter(|event| event.phase == EditPhase::Commit) {
                    state.ratio = event.value;
                    remember_expanded_ratio(state, context, self.policy, self.collapse_policy);
                    context.handle();
                    context.release_pointer();
                    if changed {
                        context.request_work();
                    }
                    if event.value.is_finite()
                        && (0.0..=1.0).contains(&event.value)
                        && event.value.to_bits() != event.start_value.to_bits()
                        && let Some(map) = &self.on_ratio_settled
                    {
                        context.emit_message(map(state.ratio));
                    }
                }
            }
            LayoutInput::PointerDoubleClick {
                position,
                button: PointerButton::Primary,
                ..
            } => {
                let Some(collapse_policy) = self.collapse_policy else {
                    return;
                };
                if state.resize.is_resizing()
                    || ratio_for_pointer(position, context, self.policy).is_none()
                {
                    return;
                }
                let Some(ratio) = toggle_split_pane_collapse(
                    state,
                    context,
                    self.policy,
                    collapse_policy,
                    constraints,
                ) else {
                    return;
                };
                context.handle();
                context.request_work();
                if let Some(map) = &self.on_ratio_settled {
                    context.emit_message(map(ratio));
                }
            }
            LayoutInput::PointerDoubleClick { .. } => {}
            LayoutInput::PointerCaptureCancelled { position, .. } => {
                if !state.resize.is_resizing() {
                    return;
                }
                let event = state
                    .resize
                    .resize_edit(DragHandleMessage::cancelled(position), constraints);
                if let Some(event) = event.filter(|event| event.phase == EditPhase::Cancel) {
                    let changed = event.value.to_bits() != state.ratio.to_bits();
                    state.ratio = event.value;
                    context.handle();
                    context.release_pointer();
                    if changed {
                        context.request_work();
                    }
                }
            }
            LayoutInput::PointerPress { .. }
            | LayoutInput::PointerRelease { .. }
            | LayoutInput::PointerModifiersChanged { .. } => {}
        }
    }
}

fn toggle_split_pane_collapse<Message>(
    state: &mut SplitPaneRuntimeState,
    context: &LayoutEventContext<Message>,
    policy: SplitPanePolicy,
    collapse_policy: SplitPaneCollapsePolicy,
    constraints: PanelResizeConstraints,
) -> Option<f32> {
    let target = collapse_target(context, policy, collapse_policy)?;
    let current_collapsed = state.collapsed_policy == Some(collapse_policy);
    let next_ratio = if current_collapsed {
        let expanded = state
            .last_expanded_ratio
            .filter(|ratio| ratio.is_finite() && (0.0..=1.0).contains(ratio))?;
        ratio_is_expanded(context, policy, collapse_policy, expanded).then_some(expanded)?
    } else {
        let current_extent = current_selected_extent(context, policy.axis, collapse_policy)?;
        if current_extent <= target.selected_extent
            || !state.ratio.is_finite()
            || !(0.0..=1.0).contains(&state.ratio)
        {
            return None;
        }
        target.ratio
    };
    if next_ratio.to_bits() == state.ratio.to_bits() {
        return None;
    }

    if !current_collapsed {
        state.last_expanded_ratio = Some(state.ratio);
        state.collapsed_policy = Some(collapse_policy);
    } else {
        state.collapsed_policy = None;
    }
    state.resize.set_size(next_ratio, constraints);
    state.ratio = next_ratio;
    Some(next_ratio)
}

fn remember_expanded_ratio<Message>(
    state: &mut SplitPaneRuntimeState,
    context: &LayoutEventContext<Message>,
    policy: SplitPanePolicy,
    collapse_policy: Option<SplitPaneCollapsePolicy>,
) {
    let Some(collapse_policy) = collapse_policy else {
        return;
    };
    let Some(target) = collapse_target(context, policy, collapse_policy) else {
        return;
    };
    let Some(selected_extent) =
        ratio_selected_extent(context, policy, collapse_policy, state.ratio)
    else {
        return;
    };
    if selected_extent > target.selected_extent {
        state.last_expanded_ratio = Some(state.ratio);
        state.collapsed_policy = None;
    } else if selected_extent.to_bits() == target.selected_extent.to_bits() {
        state.collapsed_policy = Some(collapse_policy);
    }
}

fn collapse_target(
    context: &LayoutEventContext<impl Sized>,
    policy: SplitPanePolicy,
    collapse_policy: SplitPaneCollapsePolicy,
) -> Option<SplitPaneCollapseTarget> {
    let bounds = context.container_bounds()?;
    if !bounds.has_finite_positive_area() || !context.divider_bounds()?.has_finite_positive_area() {
        return None;
    }
    split_pane_collapse_target(
        SplitPaneLayoutParts {
            bounds,
            axis: policy.axis,
            ratio: policy.initial_ratio,
            divider_extent: policy.divider_extent,
            first_min_extent: policy.first_min_extent,
            second_min_extent: policy.second_min_extent,
        },
        collapse_policy,
    )
}

fn current_selected_extent(
    context: &LayoutEventContext<impl Sized>,
    axis: SplitPaneAxis,
    collapse_policy: SplitPaneCollapsePolicy,
) -> Option<f32> {
    let bounds = context.container_bounds()?;
    let divider = context.divider_bounds()?;
    if !bounds.is_finite() || !divider.is_finite() {
        return None;
    }
    let (first, second) = match axis {
        SplitPaneAxis::Horizontal => (divider.min.x - bounds.min.x, bounds.max.x - divider.max.x),
        SplitPaneAxis::Vertical => (divider.min.y - bounds.min.y, bounds.max.y - divider.max.y),
    };
    (first.is_finite() && second.is_finite()).then_some(match collapse_policy {
        SplitPaneCollapsePolicy::FirstPane => first,
        SplitPaneCollapsePolicy::SecondPane => second,
    })
}

fn ratio_is_expanded(
    context: &LayoutEventContext<impl Sized>,
    policy: SplitPanePolicy,
    collapse_policy: SplitPaneCollapsePolicy,
    ratio: f32,
) -> bool {
    if !ratio.is_finite() || !(0.0..=1.0).contains(&ratio) {
        return false;
    }
    let Some(target) = collapse_target(context, policy, collapse_policy) else {
        return false;
    };
    ratio_selected_extent(context, policy, collapse_policy, ratio)
        .is_some_and(|selected_extent| selected_extent > target.selected_extent)
}

fn ratio_selected_extent(
    context: &LayoutEventContext<impl Sized>,
    policy: SplitPanePolicy,
    collapse_policy: SplitPaneCollapsePolicy,
    ratio: f32,
) -> Option<f32> {
    let bounds = context.container_bounds()?;
    let resolved = SplitPaneLayout::from_parts(SplitPaneLayoutParts {
        bounds,
        axis: policy.axis,
        ratio,
        divider_extent: policy.divider_extent,
        first_min_extent: policy.first_min_extent,
        second_min_extent: policy.second_min_extent,
    });
    let (first, _divider, second) = quantized_split_pane_rects(resolved);
    let selected_extent = match collapse_policy {
        SplitPaneCollapsePolicy::FirstPane => axis_extent(first, policy.axis),
        SplitPaneCollapsePolicy::SecondPane => axis_extent(second, policy.axis),
    };
    selected_extent.is_finite().then_some(selected_extent)
}

fn ratio_for_pointer(
    position: Point,
    context: &LayoutEventContext<impl Sized>,
    policy: SplitPanePolicy,
) -> Option<f32> {
    if !position.is_finite() {
        return None;
    }
    let bounds = context.container_bounds()?;
    if !bounds.has_finite_positive_area() {
        return None;
    }
    if !context.divider_bounds()?.has_finite_positive_area() {
        return None;
    }
    let resolved = SplitPaneLayout::from_parts(SplitPaneLayoutParts {
        bounds,
        axis: policy.axis,
        ratio: 0.5,
        divider_extent: policy.divider_extent,
        first_min_extent: policy.first_min_extent,
        second_min_extent: policy.second_min_extent,
    });
    let total = axis_extent(bounds, policy.axis);
    let divider = resolved.divider_extent.min(total);
    let movable = total - divider;
    if !movable.is_finite() || movable <= 0.0 {
        return None;
    }
    let pointer = match policy.axis {
        SplitPaneAxis::Horizontal => position.x,
        SplitPaneAxis::Vertical => position.y,
    };
    let start = match policy.axis {
        SplitPaneAxis::Horizontal => bounds.min.x,
        SplitPaneAxis::Vertical => bounds.min.y,
    };
    let raw = ((pointer - start) / movable).clamp(0.0, 1.0);
    let ratio = if resolved.minima_satisfied {
        let minimum = resolved.first_min_extent / movable;
        let maximum = 1.0 - resolved.second_min_extent / movable;
        raw.clamp(minimum, maximum)
    } else {
        raw
    };
    ratio.is_finite().then_some(ratio)
}

fn axis_extent(bounds: Rect, axis: SplitPaneAxis) -> f32 {
    match axis {
        SplitPaneAxis::Horizontal => bounds.width(),
        SplitPaneAxis::Vertical => bounds.height(),
    }
}

fn metadata(
    modifiers: PointerModifiers,
    timestamp: Option<crate::gui::input::InputTimestamp>,
    sequence_range: Option<crate::gui::input::InputSequenceRange>,
) -> DragHandleMetadata {
    DragHandleMetadata {
        modifiers,
        timestamp,
        sequence_range,
    }
}

fn finite_nonnegative(value: f32) -> f32 {
    if value.is_finite() {
        value.max(0.0)
    } else {
        0.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::gui::layout_core::{LayoutTargetIdentity, NodeId};

    #[test]
    fn ratio_normalization_uses_split_minima_and_divider() {
        let mut context = LayoutEventContext::<()>::with_geometry(
            LayoutTargetIdentity::new(1, SPLIT_PANE_DIVIDER_REGION_ID),
            Some(Rect::from_size(100.0, 20.0)),
            Some(Rect::from_xy_size(40.0, 0.0, 10.0, 20.0)),
            Some(Rect::from_xy_size(45.0, 0.0, 10.0, 20.0)),
        );
        let policy = SplitPanePolicy {
            divider_extent: 10.0,
            first_min_extent: 30.0,
            second_min_extent: 20.0,
            ..SplitPanePolicy::default()
        };
        assert_eq!(
            ratio_for_pointer(Point::new(0.0, 10.0), &context, policy),
            Some(1.0 / 3.0)
        );
        assert_eq!(
            ratio_for_pointer(Point::new(100.0, 10.0), &context, policy),
            Some(7.0 / 9.0)
        );
        context = LayoutEventContext::new(LayoutTargetIdentity::new(
            NodeId::from(1_u64),
            SPLIT_PANE_DIVIDER_REGION_ID,
        ));
        assert_eq!(
            ratio_for_pointer(Point::new(1.0, 1.0), &context, policy),
            None
        );
    }

    #[test]
    fn ratio_normalization_matches_undersized_split_fallback() {
        let context = LayoutEventContext::<()>::with_geometry(
            LayoutTargetIdentity::new(1, SPLIT_PANE_DIVIDER_REGION_ID),
            Some(Rect::from_size(40.0, 20.0)),
            Some(Rect::from_xy_size(0.0, 0.0, 20.0, 20.0)),
            Some(Rect::from_xy_size(20.0, 0.0, 1.0, 20.0)),
        );
        let policy = SplitPanePolicy {
            divider_extent: 10.0,
            first_min_extent: 30.0,
            second_min_extent: 20.0,
            ..SplitPanePolicy::default()
        };
        let resolved = SplitPaneLayout::from_parts(SplitPaneLayoutParts {
            bounds: Rect::from_size(40.0, 20.0),
            axis: SplitPaneAxis::Horizontal,
            ratio: 1.0,
            divider_extent: policy.divider_extent,
            first_min_extent: policy.first_min_extent,
            second_min_extent: policy.second_min_extent,
        });
        assert!(!resolved.minima_satisfied);
        assert_eq!(resolved.first.width(), 30.0);
        assert_eq!(
            ratio_for_pointer(Point::new(40.0, 10.0), &context, policy),
            Some(1.0)
        );
        assert_eq!(
            ratio_for_pointer(Point::new(20.0, 10.0), &context, policy),
            Some(2.0 / 3.0)
        );
    }
}
