use super::super::super::{
    LayoutDiagnosticCode, LayoutEngine, LayoutState, LayoutStats, layout_tree,
};
use crate::gui::layout_core::{
    Constraints, ConstraintsParts, ContainerKind, ContainerPolicy, Insets, LayoutNode,
    SizeModeCross, SizeModeMain, SlotChild, SlotParams, SplitPaneAxis, SplitPanePolicy,
};
use crate::gui::types::{Point, Rect, Vector2};

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
