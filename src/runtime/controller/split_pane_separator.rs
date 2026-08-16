//! Crate-private observational projection for mounted split-pane separators.

use super::{
    layout_state::RuntimeLayoutContainerStateStore, traversal_state::RuntimeLayoutHitTarget,
};
use crate::gui::{
    layout_core::{
        LayoutTargetIdentity, MountedContainerStateId, SPLIT_PANE_DIVIDER_REGION_ID,
        SplitPaneDividerDescriptor, SplitPaneRuntimeMode, SplitPaneRuntimeOwnership,
        SplitPaneRuntimeState, SplitPaneRuntimeStateInput,
    },
    panel::SplitPaneAxis,
    types::Rect,
};

/// Read-only evidence for one currently mounted runtime-owned split divider.
///
/// This projection is deliberately not an interaction, focus, or
/// application-output authority. It is built only from a committed mounted
/// state and the already-projected clipped divider target for the same split.
/// The controller-owned pure automation compositor may consume it to publish a
/// passive backend-neutral separator semantic node; that publication does not
/// grant the projection focus, key, action, paint, relayout, provider/native,
/// or application-message authority.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(super) struct SplitPaneSeparatorProjection {
    pub(super) target: LayoutTargetIdentity,
    pub(super) mounted_state_id: MountedContainerStateId,
    pub(super) axis: SplitPaneAxis,
    pub(super) divider_bounds: Rect,
    pub(super) live_ratio: f32,
}

pub(super) fn build_split_pane_separator_projection<Message>(
    target: &RuntimeLayoutHitTarget<Message>,
    input: SplitPaneRuntimeStateInput,
    descriptor: SplitPaneDividerDescriptor,
    state_store: &RuntimeLayoutContainerStateStore,
) -> Option<SplitPaneSeparatorProjection> {
    if !matches!(input.mode, SplitPaneRuntimeMode::RuntimeOwned { .. })
        || descriptor.container_id != input.container_id
        || target.target.identity()
            != LayoutTargetIdentity::new(input.container_id, SPLIT_PANE_DIVIDER_REGION_ID)
        || target.state_id != Some(input.state_id())
        || !crate::layout::supports_layout_state_input_contract(target.contract_version)
    {
        return None;
    }

    let divider_bounds = target.divider_bounds?;
    let clipped_bounds = target.target.bounds;
    if !divider_bounds.has_finite_positive_area() || !clipped_bounds.has_finite_positive_area() {
        return None;
    }

    let committed = state_store.lookup_current_state_view_for_state_id(input.state_id())?;
    if target.mounted_state_id != Some(committed.mounted_id()) {
        return None;
    }
    let state = committed.get::<SplitPaneRuntimeState>()?;
    if state.ownership != SplitPaneRuntimeOwnership::RuntimeOwned
        || !state.ratio.is_finite()
        || !(0.0..=1.0).contains(&state.ratio)
    {
        return None;
    }

    Some(SplitPaneSeparatorProjection {
        target: target.target.identity(),
        mounted_state_id: committed.mounted_id(),
        axis: descriptor.axis,
        divider_bounds: clipped_bounds,
        live_ratio: state.ratio,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        gui::layout_core::{
            ContainerStateDeclaration, LayoutHitTarget, LayoutInteraction,
            LayoutInteractionRevision, SplitPaneRuntimePolicyRevision, SplitPaneRuntimeStateInput,
        },
        layout::{LayoutHitRegionId, LayoutTargetIdentity, NodeId},
        runtime::controller::{
            layout_state::MAX_LAYOUT_CONTAINER_STATE_SLOTS,
            layout_state::RuntimeLayoutContainerStateStore,
            traversal_state::{RuntimeContainerTraversal, RuntimeLayoutHitTarget},
        },
    };
    use std::rc::Rc;

    struct NoopInteraction;

    impl LayoutInteraction<()> for NoopInteraction {}

    fn split_input(container_id: NodeId, mode: SplitPaneRuntimeMode) -> SplitPaneRuntimeStateInput {
        SplitPaneRuntimeStateInput {
            container_id,
            initial_ratio: 0.25,
            mode,
            policy_revision: SplitPaneRuntimePolicyRevision::default(),
        }
    }

    fn descriptor(container_id: NodeId, axis: SplitPaneAxis) -> SplitPaneDividerDescriptor {
        SplitPaneDividerDescriptor {
            container_id,
            first_child: container_id + 1,
            second_child: container_id + 2,
            axis,
            first_min_extent: 0.0,
            second_min_extent: 0.0,
            divider_extent: 8.0,
        }
    }

    fn target(
        input: SplitPaneRuntimeStateInput,
        mounted_state_id: Option<MountedContainerStateId>,
        clipped_bounds: Rect,
        divider_bounds: Option<Rect>,
    ) -> RuntimeLayoutHitTarget<()> {
        RuntimeLayoutHitTarget {
            target: LayoutHitTarget {
                container_id: input.container_id,
                region_id: SPLIT_PANE_DIVIDER_REGION_ID,
                bounds: clipped_bounds,
            },
            contract_version: crate::layout::LAYOUT_CAPABILITIES_STATE_CONTRACT_VERSION,
            state_id: Some(input.state_id()),
            interaction: Rc::new(NoopInteraction),
            revision: LayoutInteractionRevision::exact(1_u8),
            container_bounds: Some(Rect::from_size(100.0, 100.0)),
            target_bounds: Some(clipped_bounds),
            divider_bounds,
            mounted_state_id,
            split_capture_witness: None,
        }
    }

    fn committed_state(
        store: &mut RuntimeLayoutContainerStateStore,
        input: SplitPaneRuntimeStateInput,
    ) -> MountedContainerStateId {
        let declaration = input.declaration();
        store.reconcile(std::slice::from_ref(&declaration));
        store
            .current_mounted_state_id(input.state_id())
            .expect("split state should be mounted")
    }

    #[test]
    fn admission_preserves_horizontal_and_vertical_exact_evidence() {
        for (container_id, axis, bounds) in [
            (
                1,
                SplitPaneAxis::Horizontal,
                Rect::from_xy_size(48.0, 0.0, 8.0, 80.0),
            ),
            (
                10,
                SplitPaneAxis::Vertical,
                Rect::from_xy_size(0.0, 48.0, 80.0, 8.0),
            ),
        ] {
            let input = split_input(
                container_id,
                SplitPaneRuntimeMode::RuntimeOwned {
                    collapse_policy: None,
                },
            );
            let mut store = RuntimeLayoutContainerStateStore::default();
            let mounted_state_id = committed_state(&mut store, input);
            let mut traversal = RuntimeContainerTraversal::default();
            traversal.split_pane_runtime.push(input);
            traversal
                .split_pane_dividers
                .push(descriptor(container_id, axis));
            traversal.layout_targets.push(target(
                input,
                Some(mounted_state_id),
                bounds,
                Some(bounds),
            ));
            traversal.bind_committed_mounted_state_ids(&store);
            traversal.rebuild_split_pane_separator_projections(&store);

            let projection = traversal
                .split_pane_separator_projections
                .first()
                .copied()
                .expect("valid split evidence should admit");
            assert_eq!(
                projection.target,
                LayoutTargetIdentity::new(container_id, SPLIT_PANE_DIVIDER_REGION_ID)
            );
            assert_eq!(projection.mounted_state_id, mounted_state_id);
            assert_eq!(projection.mounted_state_id.generation().get(), 1);
            assert_eq!(projection.axis, axis);
            assert_eq!(projection.divider_bounds, bounds);
            assert_eq!(projection.live_ratio, 0.25);
        }
    }

    #[test]
    fn admission_fails_closed_for_all_state_and_geometry_vetoes() {
        let input = split_input(
            1,
            SplitPaneRuntimeMode::RuntimeOwned {
                collapse_policy: None,
            },
        );
        let descriptor = descriptor(input.container_id, SplitPaneAxis::Horizontal);
        let bounds = Rect::from_xy_size(48.0, 0.0, 8.0, 80.0);

        let missing_store = RuntimeLayoutContainerStateStore::default();
        assert!(
            build_split_pane_separator_projection(
                &target(input, None, bounds, Some(bounds)),
                input,
                descriptor,
                &missing_store,
            )
            .is_none()
        );

        let mut capacity_store = RuntimeLayoutContainerStateStore::default();
        let declarations = (0..MAX_LAYOUT_CONTAINER_STATE_SLOTS)
            .map(|offset| {
                ContainerStateDeclaration::new::<u32, _>(100 + offset as NodeId, 1, || 1_u32)
            })
            .collect::<Vec<_>>();
        capacity_store.reconcile(&declarations);
        assert_eq!(
            capacity_store.slot_count(),
            MAX_LAYOUT_CONTAINER_STATE_SLOTS
        );
        assert!(
            build_split_pane_separator_projection(
                &target(input, None, bounds, Some(bounds)),
                input,
                descriptor,
                &capacity_store,
            )
            .is_none()
        );

        let mut wrong_type_store = RuntimeLayoutContainerStateStore::default();
        let wrong_type = ContainerStateDeclaration::new::<u32, _>(
            input.container_id,
            input.state_id().schema_version(),
            || 1_u32,
        );
        wrong_type_store.reconcile(std::slice::from_ref(&wrong_type));
        assert!(
            build_split_pane_separator_projection(
                &target(
                    input,
                    wrong_type_store.current_mounted_state_id(input.state_id()),
                    bounds,
                    Some(bounds),
                ),
                input,
                descriptor,
                &wrong_type_store,
            )
            .is_none()
        );

        let mut stale_store = RuntimeLayoutContainerStateStore::default();
        let stale_generation = committed_state(&mut stale_store, input);
        stale_store.reconcile(&[]);
        let current_generation = committed_state(&mut stale_store, input);
        assert_ne!(stale_generation, current_generation);
        assert!(
            build_split_pane_separator_projection(
                &target(input, Some(stale_generation), bounds, Some(bounds)),
                input,
                descriptor,
                &stale_store,
            )
            .is_none()
        );

        let mut malformed_ratio_store = RuntimeLayoutContainerStateStore::default();
        let mounted_state_id = committed_state(&mut malformed_ratio_store, input);
        let mut state_context =
            malformed_ratio_store.context(input.container_id, Some(input.state_id()));
        state_context
            .state_mut::<SplitPaneRuntimeState>()
            .expect("split state")
            .ratio = f32::NAN;
        assert!(
            build_split_pane_separator_projection(
                &target(input, Some(mounted_state_id), bounds, Some(bounds)),
                input,
                descriptor,
                &malformed_ratio_store,
            )
            .is_none()
        );

        let mut controlled_store = RuntimeLayoutContainerStateStore::default();
        let controlled = split_input(
            input.container_id,
            SplitPaneRuntimeMode::Controlled(crate::layout::Controlled::new(0.5, 1)),
        );
        let controlled_state_id = committed_state(&mut controlled_store, controlled);
        assert!(
            build_split_pane_separator_projection(
                &target(controlled, Some(controlled_state_id), bounds, Some(bounds),),
                controlled,
                descriptor,
                &controlled_store,
            )
            .is_none()
        );

        let mut malformed_geometry_store = RuntimeLayoutContainerStateStore::default();
        let malformed_geometry_state = committed_state(&mut malformed_geometry_store, input);
        assert!(
            build_split_pane_separator_projection(
                &target(
                    input,
                    Some(malformed_geometry_state),
                    Rect::from_xy_size(0.0, 0.0, 0.0, 80.0),
                    Some(bounds),
                ),
                input,
                descriptor,
                &malformed_geometry_store,
            )
            .is_none()
        );
        assert!(
            build_split_pane_separator_projection(
                &target(input, Some(malformed_geometry_state), bounds, None),
                input,
                descriptor,
                &malformed_geometry_store,
            )
            .is_none()
        );

        let mut wrong_region_target =
            target(input, Some(malformed_geometry_state), bounds, Some(bounds));
        wrong_region_target.target.region_id = LayoutHitRegionId::new(7);
        assert!(
            build_split_pane_separator_projection(
                &wrong_region_target,
                input,
                descriptor,
                &malformed_geometry_store,
            )
            .is_none()
        );

        malformed_geometry_store.reconcile(&[]);
        assert!(
            build_split_pane_separator_projection(
                &target(input, Some(malformed_geometry_state), bounds, Some(bounds)),
                input,
                descriptor,
                &malformed_geometry_store,
            )
            .is_none()
        );
        assert!(
            missing_store
                .current_mounted_state_id(input.state_id())
                .is_none()
        );
    }

    #[test]
    fn collection_is_bounded_and_duplicate_evidence_fails_closed() {
        let outer = split_input(
            1,
            SplitPaneRuntimeMode::RuntimeOwned {
                collapse_policy: None,
            },
        );
        let inner = split_input(
            10,
            SplitPaneRuntimeMode::RuntimeOwned {
                collapse_policy: None,
            },
        );
        let mut store = RuntimeLayoutContainerStateStore::default();
        let outer_declaration = outer.declaration();
        let inner_declaration = inner.declaration();
        store.reconcile(&[outer_declaration, inner_declaration]);
        let outer_state = store
            .current_mounted_state_id(outer.state_id())
            .expect("outer split state");
        let inner_state = store
            .current_mounted_state_id(inner.state_id())
            .expect("inner split state");
        let outer_bounds = Rect::from_xy_size(48.0, 0.0, 8.0, 80.0);
        let inner_bounds = Rect::from_xy_size(0.0, 48.0, 80.0, 8.0);
        let mut traversal = RuntimeContainerTraversal::default();
        traversal.split_pane_runtime.extend([outer, inner]);
        traversal.split_pane_dividers.extend([
            descriptor(1, SplitPaneAxis::Horizontal),
            descriptor(10, SplitPaneAxis::Vertical),
        ]);
        traversal.layout_targets.extend([
            target(outer, Some(outer_state), outer_bounds, Some(outer_bounds)),
            target(inner, Some(inner_state), inner_bounds, Some(inner_bounds)),
        ]);
        traversal.bind_committed_mounted_state_ids(&store);
        traversal.rebuild_split_pane_separator_projections(&store);

        assert_eq!(traversal.split_pane_separator_projections.len(), 2);
        assert!(traversal.split_pane_separator_projections.capacity() <= 2);
        assert_eq!(
            traversal.split_pane_separator_projections[0]
                .target
                .container_id,
            outer.container_id
        );
        assert_eq!(
            traversal.split_pane_separator_projections[1]
                .target
                .container_id,
            inner.container_id
        );

        traversal.layout_targets.push(target(
            outer,
            Some(outer_state),
            outer_bounds,
            Some(outer_bounds),
        ));
        traversal.bind_committed_mounted_state_ids(&store);
        traversal.rebuild_split_pane_separator_projections(&store);
        assert_eq!(traversal.split_pane_separator_projections.len(), 1);
        assert_eq!(
            traversal.split_pane_separator_projections[0]
                .target
                .container_id,
            inner.container_id
        );

        traversal.layout_targets.clear();
        traversal.split_pane_runtime.clear();
        traversal.split_pane_dividers.clear();
        traversal.rebuild_split_pane_separator_projections(&store);
        assert!(traversal.split_pane_separator_projections.is_empty());
    }
}
