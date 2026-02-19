//! Unit tests for ScrollView virtualization behavior.

use super::{
    DebugPrimitiveKind, LayoutDebugOptions, LayoutDiagnosticCode, LayoutState,
    layout_tree_with_state,
};
use crate::gui::layout_core::constraints::Constraints;
use crate::gui::layout_core::model::{
    ContainerKind, ContainerPolicy, OverflowPolicy, SizeModeCross, SizeModeMain, SlotParams,
    VirtualizationAxis, VirtualizationPolicy, WrapPolicy,
};
use crate::gui::layout_core::tree::{LayoutNode, SlotChild};
use crate::gui::types::{Point, Rect, Vector2};

fn intrinsic_slot() -> SlotParams {
    SlotParams {
        size_main: SizeModeMain::Intrinsic,
        size_cross: SizeModeCross::Fill,
        constraints: Constraints::unconstrained(),
        margin: Default::default(),
        align_cross_override: None,
        allow_fixed_compress: false,
    }
}

fn scroll_with_content(
    content_kind: ContainerKind,
    total_children: u64,
    axis: VirtualizationAxis,
    overscan_px: f32,
) -> LayoutNode {
    let children = (0..total_children)
        .map(|index| SlotChild {
            slot: intrinsic_slot(),
            child: LayoutNode::widget(index + 10, Vector2::new(180.0, 10.0)),
        })
        .collect::<Vec<_>>();
    let content = LayoutNode::container(
        2,
        ContainerPolicy {
            kind: content_kind,
            spacing: 1.0,
            wrap: WrapPolicy {
                item_gap: 2.0,
                line_gap: 2.0,
            },
            ..ContainerPolicy::default()
        },
        children,
    );

    LayoutNode::container(
        1,
        ContainerPolicy {
            kind: ContainerKind::ScrollView,
            overflow: OverflowPolicy::Scroll,
            virtualization: Some(VirtualizationPolicy {
                enabled: true,
                axis,
                overscan_px,
            }),
            ..ContainerPolicy::default()
        },
        vec![SlotChild {
            slot: SlotParams::fill(),
            child: content,
        }],
    )
}

#[test]
fn scroll_virtualization_limits_materialized_nodes_for_large_lists() {
    let root = scroll_with_content(
        ContainerKind::Column,
        10_000,
        VirtualizationAxis::Vertical,
        0.0,
    );
    let mut state = LayoutState::default();
    state.scroll_offsets.insert(1, Vector2::new(0.0, 40_000.0));

    let output = layout_tree_with_state(
        &root,
        Rect::from_min_size(Point::new(0.0, 0.0), Vector2::new(220.0, 120.0)),
        &state,
        LayoutDebugOptions::default(),
    );

    let info = output
        .virtual_windows
        .get(&1)
        .expect("virtual window metadata");
    assert_eq!(info.total_children, 10_000);
    assert!(info.first_index > 0);
    assert!(info.culled_after > 0);
    assert!(info.last_index_exclusive - info.first_index < 64);
    assert!(output.stats.materialized_nodes < 128);
    assert_eq!(output.rects.len(), output.stats.materialized_nodes);
}

#[test]
fn virtualization_policy_is_ignored_for_unsupported_content_kind() {
    let root = scroll_with_content(ContainerKind::Wrap, 128, VirtualizationAxis::Vertical, 0.0);
    let output = layout_tree_with_state(
        &root,
        Rect::from_min_size(Point::new(0.0, 0.0), Vector2::new(240.0, 120.0)),
        &LayoutState::default(),
        LayoutDebugOptions::default(),
    );

    assert!(
        output
            .diagnostics
            .iter()
            .any(|item| item.code == LayoutDiagnosticCode::VirtualizationPolicyIgnored)
    );
    assert!(!output.virtual_windows.contains_key(&1));
}

#[test]
fn virtualization_debug_primitives_are_emitted() {
    let root = scroll_with_content(
        ContainerKind::Column,
        512,
        VirtualizationAxis::Vertical,
        8.0,
    );
    let mut state = LayoutState::default();
    state.scroll_offsets.insert(1, Vector2::new(0.0, 320.0));
    let output = layout_tree_with_state(
        &root,
        Rect::from_min_size(Point::new(0.0, 0.0), Vector2::new(220.0, 100.0)),
        &state,
        LayoutDebugOptions::all_enabled(),
    );

    assert!(
        output
            .debug_primitives
            .iter()
            .any(|item| item.kind == DebugPrimitiveKind::ViewportBounds)
    );
    assert!(
        output
            .debug_primitives
            .iter()
            .any(|item| item.kind == DebugPrimitiveKind::VirtualWindowBounds)
    );
    assert!(
        output
            .debug_primitives
            .iter()
            .any(|item| item.kind == DebugPrimitiveKind::CulledRegion)
    );
}

#[test]
fn invalid_virtualization_overscan_is_clamped() {
    let root = scroll_with_content(
        ContainerKind::Column,
        128,
        VirtualizationAxis::Vertical,
        -32.0,
    );
    let output = layout_tree_with_state(
        &root,
        Rect::from_min_size(Point::new(0.0, 0.0), Vector2::new(220.0, 100.0)),
        &LayoutState::default(),
        LayoutDebugOptions::default(),
    );

    assert!(
        output
            .diagnostics
            .iter()
            .any(|item| item.code == LayoutDiagnosticCode::VirtualizationWindowClamped)
    );
}
