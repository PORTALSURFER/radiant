use super::super::super::{
    LayoutContainerStateReadSource, LayoutDebugOptions, LayoutDiagnosticCode, LayoutEngine,
    LayoutOutput, LayoutState, LayoutStats, layout_tree,
};
use crate::gui::layout_core::{
    Constraints, ConstraintsParts, ContainerKind, ContainerPolicy, ContainerStateId, Controlled,
    DebugPrimitiveKind, Insets, LayoutNode, MountedContainerStateId, MountedContainerStateRead,
    NodeId, SizeModeCross, SizeModeMain, SlotChild, SlotParams, SplitPaneAxis, SplitPanePolicy,
    SplitPaneRuntimeMode, SplitPaneRuntimePolicyRevision, SplitPaneRuntimeState,
    SplitPaneRuntimeStateInput,
};
use crate::gui::types::{Point, Rect, Vector2};
use std::cell::RefCell;
use std::num::NonZeroU64;

const SPLIT_ID: NodeId = 1;

struct RecordingSource {
    mounted_id: MountedContainerStateId,
    value: u32,
    reads: RefCell<Vec<(NodeId, u32, MountedContainerStateId)>>,
}

impl RecordingSource {
    fn new(mounted_id: MountedContainerStateId, value: u32) -> Self {
        Self {
            mounted_id,
            value,
            reads: RefCell::new(Vec::new()),
        }
    }
}

impl LayoutContainerStateReadSource for RecordingSource {
    fn read_container_state(&self, container_id: NodeId) -> Option<MountedContainerStateRead<'_>> {
        self.reads
            .borrow_mut()
            .push((container_id, self.value, self.mounted_id));
        (container_id == SPLIT_ID)
            .then(|| MountedContainerStateRead::new(self.mounted_id, &self.value))
    }
}

struct SplitRuntimeSource {
    mounted_id: MountedContainerStateId,
    state: SplitPaneRuntimeState,
    reads: RefCell<Vec<NodeId>>,
}

impl LayoutContainerStateReadSource for SplitRuntimeSource {
    fn read_container_state(&self, container_id: NodeId) -> Option<MountedContainerStateRead<'_>> {
        self.reads.borrow_mut().push(container_id);
        (container_id == SPLIT_ID)
            .then(|| MountedContainerStateRead::new(self.mounted_id, &self.state))
    }
}

fn split_policy(
    axis: SplitPaneAxis,
    initial_ratio: f32,
    divider_extent: f32,
    first_min_extent: f32,
    second_min_extent: f32,
) -> ContainerPolicy {
    ContainerPolicy {
        kind: ContainerKind::SplitPane,
        split_pane: SplitPanePolicy {
            axis,
            initial_ratio,
            divider_extent,
            first_min_extent,
            second_min_extent,
        },
        ..ContainerPolicy::default()
    }
}

fn split_node(policy: ContainerPolicy, children: Vec<SlotChild>) -> LayoutNode {
    LayoutNode::container(1, policy, children)
}

fn runtime_split_node(
    policy: ContainerPolicy,
    children: Vec<SlotChild>,
    mode: SplitPaneRuntimeMode,
) -> LayoutNode {
    LayoutNode::container_with_split_pane_runtime_mode(1, policy, children, Some(mode))
}

fn split_runtime_state(mode: SplitPaneRuntimeMode, initial_ratio: f32) -> SplitPaneRuntimeState {
    SplitPaneRuntimeState::from_input(SplitPaneRuntimeStateInput {
        container_id: SPLIT_ID,
        initial_ratio,
        mode,
        policy_revision: SplitPaneRuntimePolicyRevision::default(),
    })
}

fn runtime_owned_mode() -> SplitPaneRuntimeMode {
    SplitPaneRuntimeMode::RuntimeOwned {
        collapse_policy: None,
    }
}

fn measured_debug_primitives(output: &LayoutOutput) -> Vec<(NodeId, Rect)> {
    output
        .debug_primitives
        .iter()
        .filter(|primitive| primitive.kind == DebugPrimitiveKind::MeasuredBounds)
        .map(|primitive| (primitive.node_id, primitive.rect))
        .collect()
}

fn child(id: u64, intrinsic: Vector2) -> SlotChild {
    SlotChild::new(
        SlotParams {
            constraints: Constraints::from_parts(ConstraintsParts {
                min_w: 0.0,
                max_w: f32::MAX,
                min_h: 0.0,
                max_h: f32::MAX,
            }),
            ..SlotParams::fill()
        },
        LayoutNode::widget(id, intrinsic),
    )
}

fn root_rect(x: f32, y: f32, width: f32, height: f32) -> Rect {
    Rect::from_min_size(Point::new(x, y), Vector2::new(width, height))
}

fn assert_quantized_tiling(
    outer: Rect,
    first: Rect,
    second: Rect,
    axis: SplitPaneAxis,
    expected_first_extent: f32,
    expected_divider_extent: f32,
    expected_second_extent: f32,
) {
    let (first_end, second_start) = match axis {
        SplitPaneAxis::Horizontal => (first.max.x, second.min.x),
        SplitPaneAxis::Vertical => (first.max.y, second.min.y),
    };
    assert!(first_end <= second_start);
    let divider = match axis {
        SplitPaneAxis::Horizontal => Rect::from_min_max(
            Point::new(first_end, outer.min.y),
            Point::new(second_start, outer.max.y),
        ),
        SplitPaneAxis::Vertical => Rect::from_min_max(
            Point::new(outer.min.x, first_end),
            Point::new(outer.max.x, second_start),
        ),
    };

    assert_eq!(
        match axis {
            SplitPaneAxis::Horizontal => first.width(),
            SplitPaneAxis::Vertical => first.height(),
        },
        expected_first_extent
    );
    assert_eq!(
        match axis {
            SplitPaneAxis::Horizontal => divider.width(),
            SplitPaneAxis::Vertical => divider.height(),
        },
        expected_divider_extent
    );
    assert_eq!(
        match axis {
            SplitPaneAxis::Horizontal => second.width(),
            SplitPaneAxis::Vertical => second.height(),
        },
        expected_second_extent
    );

    let panes = [first, divider, second];
    for pane in panes {
        assert!(pane.min.x >= outer.min.x);
        assert!(pane.min.y >= outer.min.y);
        assert!(pane.max.x <= outer.max.x);
        assert!(pane.max.y <= outer.max.y);
        match axis {
            SplitPaneAxis::Horizontal => {
                assert_eq!(pane.min.y, outer.min.y);
                assert_eq!(pane.max.y, outer.max.y);
            }
            SplitPaneAxis::Vertical => {
                assert_eq!(pane.min.x, outer.min.x);
                assert_eq!(pane.max.x, outer.max.x);
            }
        }
    }
    assert!(!first.overlaps(divider));
    assert!(!divider.overlaps(second));
    assert!(!first.overlaps(second));
    assert_eq!(first.union(divider).union(second), outer);
}

#[test]
fn horizontal_split_uses_exact_ratio_divider_and_o_one_stats() {
    let root = split_node(
        split_policy(SplitPaneAxis::Horizontal, 0.25, 8.0, 40.0, 60.0),
        vec![
            child(2, Vector2::new(10.0, 20.0)),
            child(3, Vector2::new(20.0, 30.0)),
        ],
    );
    let output = layout_tree(&root, root_rect(10.0, 20.0, 200.0, 100.0));

    assert_eq!(
        output.rects[&2],
        Rect::from_min_max(Point::new(10.0, 20.0), Point::new(58.0, 120.0))
    );
    assert_eq!(
        output.rects[&3],
        Rect::from_min_max(Point::new(66.0, 20.0), Point::new(210.0, 120.0))
    );
    assert!(output.diagnostics.is_empty());
    assert_eq!(
        output.stats,
        LayoutStats {
            measured_nodes: 3,
            laid_out_nodes: 3,
            materialized_nodes: 3,
        }
    );
}

#[test]
fn vertical_split_places_first_pane_above_second_pane() {
    let root = split_node(
        split_policy(SplitPaneAxis::Vertical, 0.4, 20.0, 30.0, 40.0),
        vec![
            child(2, Vector2::new(10.0, 20.0)),
            child(3, Vector2::new(20.0, 30.0)),
        ],
    );
    let output = layout_tree(&root, root_rect(20.0, 30.0, 200.0, 120.0));

    assert_eq!(
        output.rects[&2],
        Rect::from_min_max(Point::new(20.0, 30.0), Point::new(220.0, 70.0))
    );
    assert_eq!(
        output.rects[&3],
        Rect::from_min_max(Point::new(20.0, 90.0), Point::new(220.0, 150.0))
    );
    assert!(output.diagnostics.is_empty());
}

#[test]
fn horizontal_fractional_split_uses_cumulative_rounded_boundaries() {
    let root = split_node(
        split_policy(SplitPaneAxis::Horizontal, 0.525, 0.2, 0.0, 0.0),
        vec![
            child(2, Vector2::new(10.0, 20.0)),
            child(3, Vector2::new(20.0, 30.0)),
        ],
    );
    let output = layout_tree(&root, root_rect(0.0, 0.0, 9.0, 20.0));

    assert_eq!(output.rects[&1], root_rect(0.0, 0.0, 9.0, 20.0));
    assert_eq!(output.rects[&2], root_rect(0.0, 0.0, 5.0, 20.0));
    assert_eq!(output.rects[&3], root_rect(6.0, 0.0, 3.0, 20.0));
}

#[test]
fn vertical_fractional_split_uses_cumulative_rounded_boundaries() {
    let root = split_node(
        split_policy(SplitPaneAxis::Vertical, 0.53, 0.2, 0.0, 0.0),
        vec![
            child(2, Vector2::new(10.0, 20.0)),
            child(3, Vector2::new(20.0, 30.0)),
        ],
    );
    let output = layout_tree(&root, root_rect(2.0, 10.0, 20.0, 7.0));

    assert_eq!(output.rects[&1], root_rect(2.0, 10.0, 20.0, 7.0));
    assert_eq!(output.rects[&2], root_rect(2.0, 10.0, 20.0, 4.0));
    assert_eq!(output.rects[&3], root_rect(2.0, 15.0, 20.0, 2.0));
}

#[test]
fn split_rounds_fractional_content_before_preserving_saturated_divider() {
    let mut policy = split_policy(SplitPaneAxis::Horizontal, 0.5, 4.5, 0.0, 0.0);
    policy.padding = Insets::all(0.25);
    let root = split_node(
        policy,
        vec![
            child(2, Vector2::new(10.0, 20.0)),
            child(3, Vector2::new(20.0, 30.0)),
        ],
    );
    let output = layout_tree(&root, root_rect(0.0, 0.0, 5.0, 2.0));
    let outer = output.rects[&1];
    let first = output.rects[&2];
    let second = output.rects[&3];

    for rect in [first, second] {
        assert_eq!(rect.min.x, rect.min.x.round());
        assert_eq!(rect.min.y, rect.min.y.round());
        assert_eq!(rect.max.x, rect.max.x.round());
        assert_eq!(rect.max.y, rect.max.y.round());
    }
    assert_quantized_tiling(
        outer,
        first,
        second,
        SplitPaneAxis::Horizontal,
        0.0,
        5.0,
        0.0,
    );
}

#[test]
fn split_normalizes_invalid_policy_values_through_shared_geometry() {
    let root = split_node(
        split_policy(
            SplitPaneAxis::Horizontal,
            f32::NAN,
            -4.0,
            f32::INFINITY,
            f32::NEG_INFINITY,
        ),
        vec![
            child(2, Vector2::new(10.0, 20.0)),
            child(3, Vector2::new(20.0, 30.0)),
        ],
    );
    let output = layout_tree(&root, root_rect(0.0, 0.0, 100.0, 40.0));

    assert_eq!(output.rects[&2].width(), 50.0);
    assert_eq!(output.rects[&3].min.x, 50.0);
    assert_eq!(output.rects[&3].width(), 50.0);
    assert!(output.rects.values().all(|rect| {
        rect.min.x.is_finite()
            && rect.min.y.is_finite()
            && rect.max.x.is_finite()
            && rect.max.y.is_finite()
    }));
    assert!(output.diagnostics.is_empty());
}

#[test]
fn split_clamps_finite_ratio_extremes() {
    for (ratio, expected_first_width, expected_second_start) in
        [(-1.0, 0.0, 0.0), (2.0, 100.0, 100.0)]
    {
        let root = split_node(
            split_policy(SplitPaneAxis::Horizontal, ratio, 0.0, 0.0, 0.0),
            vec![
                child(2, Vector2::new(10.0, 20.0)),
                child(3, Vector2::new(20.0, 30.0)),
            ],
        );
        let output = layout_tree(&root, root_rect(0.0, 0.0, 100.0, 40.0));

        assert_eq!(output.rects[&2].width(), expected_first_width);
        assert_eq!(output.rects[&3].min.x, expected_second_start);
    }
}

#[test]
fn split_satisfies_minima_when_they_fit_and_reports_ratio_fallback_when_they_do_not() {
    let fitting = split_node(
        split_policy(SplitPaneAxis::Horizontal, 0.1, 10.0, 30.0, 40.0),
        vec![
            child(2, Vector2::new(10.0, 20.0)),
            child(3, Vector2::new(20.0, 30.0)),
        ],
    );
    let fitting_output = layout_tree(&fitting, root_rect(0.0, 0.0, 100.0, 40.0));
    assert_eq!(fitting_output.rects[&2].width(), 30.0);
    assert_eq!(fitting_output.rects[&3].width(), 60.0);
    assert!(!fitting_output
        .diagnostics
        .iter()
        .any(|diagnostic| diagnostic.code == LayoutDiagnosticCode::SplitPaneMinimumsUnsatisfied));

    let undersized = split_node(
        split_policy(SplitPaneAxis::Horizontal, 0.25, 20.0, 60.0, 60.0),
        vec![
            child(2, Vector2::new(10.0, 20.0)),
            child(3, Vector2::new(20.0, 30.0)),
        ],
    );
    let undersized_output = layout_tree(&undersized, root_rect(0.0, 0.0, 100.0, 40.0));
    assert_eq!(undersized_output.rects[&2].width(), 20.0);
    assert_eq!(undersized_output.rects[&3].width(), 60.0);
    assert!(undersized_output
        .diagnostics
        .iter()
        .any(|diagnostic| diagnostic.code == LayoutDiagnosticCode::SplitPaneMinimumsUnsatisfied));
}

#[test]
fn split_saturates_divider_and_handles_zero_size_without_invalid_rectangles() {
    let divider_saturated = split_node(
        split_policy(SplitPaneAxis::Horizontal, 0.5, 200.0, 0.0, 0.0),
        vec![
            child(2, Vector2::new(10.0, 20.0)),
            child(3, Vector2::new(20.0, 30.0)),
        ],
    );
    let saturated_output = layout_tree(&divider_saturated, root_rect(0.0, 0.0, 60.0, 40.0));
    assert_eq!(saturated_output.rects[&2].width(), 0.0);
    assert_eq!(saturated_output.rects[&3].width(), 0.0);

    let zero_size = split_node(
        split_policy(SplitPaneAxis::Vertical, 0.5, 12.0, 20.0, 20.0),
        vec![
            child(2, Vector2::new(10.0, 20.0)),
            child(3, Vector2::new(20.0, 30.0)),
        ],
    );
    let zero_output = layout_tree(&zero_size, root_rect(4.0, 6.0, 0.0, 0.0));
    assert_eq!(zero_output.rects[&2], root_rect(4.0, 6.0, 0.0, 0.0));
    assert_eq!(zero_output.rects[&3], root_rect(4.0, 6.0, 0.0, 0.0));
    assert!(zero_output.rects.values().all(|rect| {
        rect.min.x.is_finite()
            && rect.min.y.is_finite()
            && rect.max.x.is_finite()
            && rect.max.y.is_finite()
    }));
}

#[test]
fn split_quantization_handles_divider_boundaries_and_is_deterministic() {
    for (
        axis,
        bounds,
        ratio,
        divider_extent,
        expected_first_extent,
        expected_divider_extent,
        expected_second_extent,
    ) in [
        (
            SplitPaneAxis::Horizontal,
            root_rect(10.0, 20.0, 9.0, 8.0),
            0.525,
            0.0,
            5.0,
            0.0,
            4.0,
        ),
        (
            SplitPaneAxis::Horizontal,
            root_rect(10.0, 20.0, 9.0, 8.0),
            0.525,
            0.2,
            5.0,
            1.0,
            3.0,
        ),
        (
            SplitPaneAxis::Horizontal,
            root_rect(10.0, 20.0, 9.0, 8.0),
            0.525,
            100.0,
            0.0,
            9.0,
            0.0,
        ),
        (
            SplitPaneAxis::Vertical,
            root_rect(10.0, 20.0, 8.0, 7.0),
            0.53,
            0.0,
            4.0,
            0.0,
            3.0,
        ),
        (
            SplitPaneAxis::Vertical,
            root_rect(10.0, 20.0, 8.0, 7.0),
            0.53,
            0.2,
            4.0,
            1.0,
            2.0,
        ),
        (
            SplitPaneAxis::Vertical,
            root_rect(10.0, 20.0, 8.0, 7.0),
            0.53,
            100.0,
            0.0,
            7.0,
            0.0,
        ),
        (
            SplitPaneAxis::Vertical,
            root_rect(10.0, 20.0, 8.0, 0.0),
            0.53,
            0.2,
            0.0,
            0.0,
            0.0,
        ),
    ] {
        let root = split_node(
            split_policy(axis, ratio, divider_extent, 0.0, 0.0),
            vec![
                child(2, Vector2::new(10.0, 20.0)),
                child(3, Vector2::new(20.0, 30.0)),
            ],
        );
        let output = layout_tree(&root, bounds);
        let repeated = layout_tree(&root, bounds);
        assert_eq!(output, repeated);
        assert!(output.diagnostics.is_empty());

        assert_quantized_tiling(
            output.rects[&1],
            output.rects[&2],
            output.rects[&3],
            axis,
            expected_first_extent,
            expected_divider_extent,
            expected_second_extent,
        );
    }
}

#[test]
fn split_measurement_uses_child_requirements_minima_divider_and_cross_max() {
    let split = split_node(
        split_policy(SplitPaneAxis::Horizontal, 0.5, 8.0, 40.0, 50.0),
        vec![
            child(2, Vector2::new(20.0, 10.0)),
            child(3, Vector2::new(30.0, 25.0)),
        ],
    );
    let root = LayoutNode::container(
        10,
        ContainerPolicy {
            kind: ContainerKind::Column,
            ..ContainerPolicy::default()
        },
        vec![SlotChild::new(
            SlotParams {
                size_main: SizeModeMain::Intrinsic,
                size_cross: SizeModeCross::Intrinsic,
                constraints: Constraints::unconstrained(),
                ..SlotParams::fill()
            },
            split,
        )],
    );
    let output = layout_tree(&root, root_rect(0.0, 0.0, 80.0, 200.0));

    assert_eq!(output.rects[&1].width(), 80.0);
    assert_eq!(output.rects[&1].height(), 25.0);
}

#[test]
fn malformed_split_arity_is_diagnosed_and_lays_out_all_children_without_stale_rects() {
    let valid = split_node(
        split_policy(SplitPaneAxis::Horizontal, 0.5, 0.0, 0.0, 0.0),
        vec![
            child(2, Vector2::new(10.0, 10.0)),
            child(3, Vector2::new(10.0, 10.0)),
        ],
    );
    let mut engine = LayoutEngine::default();
    let mut output = engine.layout_with_state(
        &valid,
        root_rect(0.0, 0.0, 80.0, 40.0),
        &LayoutState::default(),
        Default::default(),
    );
    assert!(output.rects.contains_key(&2));

    for (index, children) in [
        Vec::new(),
        vec![child(4, Vector2::new(10.0, 10.0))],
        vec![
            child(5, Vector2::new(10.0, 10.0)),
            child(6, Vector2::new(10.0, 10.0)),
            child(7, Vector2::new(10.0, 10.0)),
        ],
    ]
    .into_iter()
    .enumerate()
    {
        let malformed = split_node(
            split_policy(SplitPaneAxis::Horizontal, 0.5, 0.0, 0.0, 0.0),
            children,
        );
        engine.layout_with_state_into(
            &malformed,
            root_rect(4.0, 6.0, 80.0, 40.0),
            &LayoutState::default(),
            Default::default(),
            &mut output,
        );
        assert_eq!(
            output
                .diagnostics
                .iter()
                .filter(|diagnostic| {
                    diagnostic.code == LayoutDiagnosticCode::SplitPaneChildCountMismatch
                })
                .count(),
            1,
            "malformed case {index} should emit one arity diagnostic"
        );
        assert!(!output.rects.contains_key(&2));
        assert!(output.rects.values().all(|rect| {
            *rect == root_rect(4.0, 6.0, 80.0, 40.0) || rect.min == Point::new(4.0, 6.0)
        }));
        match index {
            0 => assert_eq!(output.rects.len(), 1),
            1 => assert_eq!(output.rects.len(), 2),
            2 => assert_eq!(output.rects.len(), 4),
            _ => unreachable!(),
        }
    }
}

#[test]
fn split_pane_source_is_observed_cold_and_placement_only_when_warm() {
    let root = split_node(
        split_policy(SplitPaneAxis::Horizontal, 0.4, 6.0, 0.0, 0.0),
        vec![
            child(2, Vector2::new(10.0, 20.0)),
            child(3, Vector2::new(20.0, 30.0)),
        ],
    );
    let viewport = root_rect(3.0, 5.0, 120.0, 60.0);
    let expected = layout_tree(&root, viewport);
    let mounted_id = MountedContainerStateId::new(
        ContainerStateId::new::<u32>(SPLIT_ID, 1),
        NonZeroU64::new(1).expect("non-zero generation"),
    );
    let mut engine = LayoutEngine::default();
    let mut output = LayoutOutput::default();
    let cold_source = RecordingSource::new(mounted_id, 17);

    engine.layout_with_state_and_source_into(
        &root,
        viewport,
        &LayoutState::default(),
        LayoutDebugOptions::default(),
        Some(&cold_source),
        &mut output,
    );

    assert_eq!(output, expected);
    assert_eq!(
        cold_source.reads.borrow().as_slice(),
        &[(SPLIT_ID, 17, mounted_id)]
    );
    assert_eq!(
        output.stats,
        LayoutStats {
            measured_nodes: 3,
            laid_out_nodes: 3,
            materialized_nodes: 3,
        }
    );

    let warm_source = RecordingSource::new(mounted_id, 91);
    engine.layout_with_state_and_source_into(
        &root,
        viewport,
        &LayoutState::default(),
        LayoutDebugOptions::default(),
        Some(&warm_source),
        &mut output,
    );

    let mut expected_warm = expected.clone();
    expected_warm.stats = output.stats;
    assert_eq!(output, expected_warm);
    assert_eq!(
        warm_source.reads.borrow().as_slice(),
        &[(SPLIT_ID, 91, mounted_id)]
    );
    assert_eq!(
        output.stats,
        LayoutStats {
            measured_nodes: 0,
            laid_out_nodes: 3,
            materialized_nodes: 3,
        }
    );
}

#[test]
fn stateful_split_ratio_changes_placement_without_changing_measurement() {
    let policy = split_policy(SplitPaneAxis::Horizontal, 0.25, 0.0, 0.0, 0.0);
    let root = runtime_split_node(
        policy.clone(),
        vec![
            child(2, Vector2::new(10.0, 20.0)),
            child(3, Vector2::new(20.0, 30.0)),
        ],
        runtime_owned_mode(),
    );
    let viewport = root_rect(0.0, 0.0, 100.0, 40.0);
    let mounted_id = MountedContainerStateId::new(
        ContainerStateId::new::<SplitPaneRuntimeState>(SPLIT_ID, 2),
        NonZeroU64::new(1).expect("non-zero generation"),
    );
    let mut engine = LayoutEngine::default();
    let mut output = LayoutOutput::default();
    let first_source = SplitRuntimeSource {
        mounted_id,
        state: split_runtime_state(runtime_owned_mode(), 0.75),
        reads: RefCell::new(Vec::new()),
    };

    engine.layout_with_state_and_source_into(
        &root,
        viewport,
        &LayoutState::default(),
        LayoutDebugOptions::all_enabled(),
        Some(&first_source),
        &mut output,
    );
    assert_eq!(output.rects[&2].width(), 75.0);
    let first_measured_sizes = measured_debug_primitives(&output)
        .into_iter()
        .map(|(id, rect)| (id, rect.width(), rect.height()))
        .collect::<Vec<_>>();
    assert_eq!(output.stats.measured_nodes, 3);
    assert_eq!(first_source.reads.borrow().as_slice(), &[SPLIT_ID]);

    let fresh_second_source = SplitRuntimeSource {
        mounted_id,
        state: split_runtime_state(runtime_owned_mode(), 0.2),
        reads: RefCell::new(Vec::new()),
    };
    let mut fresh_engine = LayoutEngine::default();
    let mut fresh_output = LayoutOutput::default();
    fresh_engine.layout_with_state_and_source_into(
        &root,
        viewport,
        &LayoutState::default(),
        LayoutDebugOptions::all_enabled(),
        Some(&fresh_second_source),
        &mut fresh_output,
    );
    let fresh_second_measured_sizes = measured_debug_primitives(&fresh_output)
        .into_iter()
        .map(|(id, rect)| (id, rect.width(), rect.height()))
        .collect::<Vec<_>>();
    assert_eq!(fresh_second_measured_sizes, first_measured_sizes);

    let second_source = SplitRuntimeSource {
        mounted_id,
        state: split_runtime_state(runtime_owned_mode(), 0.2),
        reads: RefCell::new(Vec::new()),
    };
    engine.layout_with_state_and_source_into(
        &root,
        viewport,
        &LayoutState::default(),
        LayoutDebugOptions::all_enabled(),
        Some(&second_source),
        &mut output,
    );
    assert_eq!(output.rects[&2].width(), 20.0);
    let second_measured_sizes = measured_debug_primitives(&output)
        .into_iter()
        .map(|(id, rect)| (id, rect.width(), rect.height()))
        .collect::<Vec<_>>();
    assert_eq!(second_measured_sizes, vec![first_measured_sizes[0]]);
    assert_eq!(output.stats.measured_nodes, 0);
    assert_eq!(second_source.reads.borrow().as_slice(), &[SPLIT_ID]);
}

#[test]
fn controlled_split_uses_controlled_slot_and_falls_back_on_ownership_mismatch() {
    let policy = split_policy(SplitPaneAxis::Horizontal, 0.25, 0.0, 0.0, 0.0);
    let mode = SplitPaneRuntimeMode::Controlled(Controlled::new(0.6, 7));
    let root = runtime_split_node(
        policy.clone(),
        vec![
            child(2, Vector2::new(10.0, 20.0)),
            child(3, Vector2::new(20.0, 30.0)),
        ],
        mode,
    );
    let viewport = root_rect(0.0, 0.0, 100.0, 40.0);
    let mounted_id = MountedContainerStateId::new(
        ContainerStateId::new::<SplitPaneRuntimeState>(SPLIT_ID, 2),
        NonZeroU64::new(1).expect("non-zero generation"),
    );
    let matching = SplitRuntimeSource {
        mounted_id,
        state: split_runtime_state(
            SplitPaneRuntimeMode::Controlled(Controlled::new(0.6, 7)),
            policy.split_pane.initial_ratio,
        ),
        reads: RefCell::new(Vec::new()),
    };
    let matching_output = layout_with_optional_source(&root, viewport, Some(&matching));
    assert_eq!(matching_output.rects[&2].width(), 60.0);

    let mismatched = SplitRuntimeSource {
        mounted_id,
        state: split_runtime_state(runtime_owned_mode(), 0.9),
        reads: RefCell::new(Vec::new()),
    };
    let fallback = layout_with_optional_source(&root, viewport, Some(&mismatched));
    assert_eq!(fallback.rects[&2].width(), 25.0);
}

#[test]
fn stateful_split_missing_wrong_and_invalid_state_fail_closed_to_policy_ratio() {
    let policy = split_policy(SplitPaneAxis::Horizontal, 0.25, 0.0, 0.0, 0.0);
    let root = runtime_split_node(
        policy.clone(),
        vec![
            child(2, Vector2::new(10.0, 20.0)),
            child(3, Vector2::new(20.0, 30.0)),
        ],
        runtime_owned_mode(),
    );
    let viewport = root_rect(0.0, 0.0, 100.0, 40.0);
    let fallback = layout_tree(&split_node(policy, root_children(&root)), viewport);
    let mounted_id = MountedContainerStateId::new(
        ContainerStateId::new::<SplitPaneRuntimeState>(SPLIT_ID, 2),
        NonZeroU64::new(1).expect("non-zero generation"),
    );

    let missing = layout_with_optional_source(&root, viewport, None);
    assert_eq!(missing.rects[&2], fallback.rects[&2]);
    assert_eq!(missing.rects[&3], fallback.rects[&3]);

    let mut invalid_state = split_runtime_state(runtime_owned_mode(), 0.25);
    invalid_state.ratio = f32::NAN;
    let invalid_source = SplitRuntimeSource {
        mounted_id,
        state: invalid_state,
        reads: RefCell::new(Vec::new()),
    };
    let invalid = layout_with_optional_source(&root, viewport, Some(&invalid_source));
    assert_eq!(invalid.rects[&2], fallback.rects[&2]);
    assert_eq!(invalid.rects[&3], fallback.rects[&3]);

    let wrong_source = RecordingSource::new(
        MountedContainerStateId::new(
            ContainerStateId::new::<u32>(SPLIT_ID, 1),
            NonZeroU64::new(1).expect("non-zero generation"),
        ),
        1,
    );
    let wrong = layout_with_optional_source(&root, viewport, Some(&wrong_source));
    assert_eq!(wrong.rects[&2], fallback.rects[&2]);
    assert_eq!(wrong.rects[&3], fallback.rects[&3]);
}

fn root_children(root: &LayoutNode) -> Vec<SlotChild> {
    let LayoutNode::Container(container) = root else {
        unreachable!("runtime split helper builds a container")
    };
    container.children.clone()
}

fn layout_with_optional_source(
    root: &LayoutNode,
    viewport: Rect,
    source: Option<&dyn LayoutContainerStateReadSource>,
) -> LayoutOutput {
    let mut engine = LayoutEngine::default();
    let mut output = LayoutOutput::default();
    engine.layout_with_state_and_source_into(
        root,
        viewport,
        &LayoutState::default(),
        LayoutDebugOptions::default(),
        source,
        &mut output,
    );
    output
}

#[test]
fn ordinary_container_does_not_query_a_container_state_source() {
    let root = LayoutNode::container(
        10,
        ContainerPolicy {
            kind: ContainerKind::Column,
            ..ContainerPolicy::default()
        },
        vec![child(11, Vector2::new(20.0, 12.0))],
    );
    let source = RecordingSource::new(
        MountedContainerStateId::new(
            ContainerStateId::new::<u32>(10, 1),
            NonZeroU64::new(1).expect("non-zero generation"),
        ),
        23,
    );
    let mut engine = LayoutEngine::default();
    let mut output = LayoutOutput::default();

    engine.layout_with_state_and_source_into(
        &root,
        root_rect(0.0, 0.0, 80.0, 40.0),
        &LayoutState::default(),
        LayoutDebugOptions::default(),
        Some(&source),
        &mut output,
    );

    assert!(source.reads.borrow().is_empty());
}
