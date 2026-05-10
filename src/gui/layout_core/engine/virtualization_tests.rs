//! Unit tests for ScrollView virtualization behavior.

use super::{
    DebugPrimitiveKind, LayoutDebugOptions, LayoutDiagnosticCode, LayoutEngine, LayoutState,
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
fn fixed_size_virtualized_scroll_avoids_cold_full_list_measurement() {
    let children = (0..10_000_u64)
        .map(|index| SlotChild {
            slot: SlotParams {
                size_main: SizeModeMain::Fixed(28.0),
                size_cross: SizeModeCross::Fill,
                constraints: Constraints::unconstrained(),
                margin: Default::default(),
                align_cross_override: None,
                allow_fixed_compress: false,
            },
            child: LayoutNode::widget(index + 10, Vector2::new(180.0, 20.0)),
        })
        .collect::<Vec<_>>();
    let root = LayoutNode::container(
        1,
        ContainerPolicy {
            kind: ContainerKind::ScrollView,
            overflow: OverflowPolicy::Scroll,
            virtualization: Some(VirtualizationPolicy {
                enabled: true,
                axis: VirtualizationAxis::Vertical,
                overscan_px: 56.0,
            }),
            ..ContainerPolicy::default()
        },
        vec![SlotChild {
            slot: SlotParams::fill(),
            child: LayoutNode::container(
                2,
                ContainerPolicy {
                    kind: ContainerKind::Column,
                    spacing: 2.0,
                    ..ContainerPolicy::default()
                },
                children,
            ),
        }],
    );
    let mut state = LayoutState::default();
    state.scroll_offsets.insert(1, Vector2::new(0.0, 8_000.0));

    let output = LayoutEngine::default().layout_with_state(
        &root,
        Rect::from_min_size(Point::new(0.0, 0.0), Vector2::new(240.0, 140.0)),
        &state,
        LayoutDebugOptions::default(),
    );

    let window = output
        .virtual_windows
        .get(&1)
        .expect("fixed-size virtual window metadata");
    assert_eq!(window.total_children, 10_000);
    assert!(window.last_index_exclusive - window.first_index < 32);
    assert!(
        output.stats.measured_nodes < 64,
        "fixed-size virtual metrics should not measure every row on a cold layout"
    );
}

#[test]
fn virtualized_metrics_cache_tracks_fixed_row_shape_changes() {
    let mut engine = LayoutEngine::default();
    let mut state = LayoutState::default();
    state.scroll_offsets.insert(1, Vector2::new(0.0, 160.0));
    let viewport = Rect::from_min_size(Point::new(0.0, 0.0), Vector2::new(240.0, 140.0));

    let first = engine.layout_with_state(
        &fixed_virtualized_scroll_root(24.0),
        viewport,
        &state,
        LayoutDebugOptions::default(),
    );
    let second = engine.layout_with_state(
        &fixed_virtualized_scroll_root(40.0),
        viewport,
        &state,
        LayoutDebugOptions::default(),
    );

    assert_eq!(
        first
            .virtual_windows
            .get(&1)
            .expect("first virtual window")
            .resolved_total_main,
        24.0 * 128.0 + 2.0 * 127.0
    );
    assert_eq!(
        second
            .virtual_windows
            .get(&1)
            .expect("second virtual window")
            .resolved_total_main,
        40.0 * 128.0 + 2.0 * 127.0
    );
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

fn fixed_virtualized_scroll_root(row_height: f32) -> LayoutNode {
    let children = (0..128_u64)
        .map(|index| SlotChild {
            slot: SlotParams {
                size_main: SizeModeMain::Fixed(row_height),
                size_cross: SizeModeCross::Fill,
                constraints: Constraints::unconstrained(),
                margin: Default::default(),
                align_cross_override: None,
                allow_fixed_compress: false,
            },
            child: LayoutNode::widget(index + 10, Vector2::new(180.0, 20.0)),
        })
        .collect::<Vec<_>>();

    LayoutNode::container(
        1,
        ContainerPolicy {
            kind: ContainerKind::ScrollView,
            overflow: OverflowPolicy::Scroll,
            virtualization: Some(VirtualizationPolicy {
                enabled: true,
                axis: VirtualizationAxis::Vertical,
                overscan_px: 24.0,
            }),
            ..ContainerPolicy::default()
        },
        vec![SlotChild {
            slot: SlotParams::fill(),
            child: LayoutNode::container(
                2,
                ContainerPolicy {
                    kind: ContainerKind::Column,
                    spacing: 2.0,
                    ..ContainerPolicy::default()
                },
                children,
            ),
        }],
    )
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

#[test]
fn virtualization_supports_non_start_linear_alignment() {
    let mut children = Vec::with_capacity(64);
    for index in 0..64_u64 {
        children.push(SlotChild {
            slot: SlotParams {
                size_main: SizeModeMain::Fixed(24.0),
                size_cross: SizeModeCross::Fill,
                constraints: Constraints::new(0.0, 40.0, 0.0, f32::INFINITY),
                margin: Default::default(),
                align_cross_override: None,
                allow_fixed_compress: false,
            },
            child: LayoutNode::widget(index + 10, Vector2::new(24.0, 16.0)),
        });
    }
    let content = LayoutNode::container(
        2,
        ContainerPolicy {
            kind: ContainerKind::Row,
            align_main: crate::gui::layout_core::model::MainAlign::SpaceBetween,
            spacing: 2.0,
            ..ContainerPolicy::default()
        },
        children,
    );
    let root = LayoutNode::container(
        1,
        ContainerPolicy {
            kind: ContainerKind::ScrollView,
            overflow: OverflowPolicy::Scroll,
            virtualization: Some(VirtualizationPolicy {
                enabled: true,
                axis: VirtualizationAxis::Horizontal,
                overscan_px: 16.0,
            }),
            ..ContainerPolicy::default()
        },
        vec![SlotChild {
            slot: SlotParams {
                size_main: SizeModeMain::Fill(1.0),
                size_cross: SizeModeCross::Fill,
                constraints: Constraints::new(0.0, 3_000.0, 0.0, 120.0),
                margin: Default::default(),
                align_cross_override: None,
                allow_fixed_compress: false,
            },
            child: content,
        }],
    );

    let mut state = LayoutState::default();
    state.scroll_offsets.insert(1, Vector2::new(160.0, 0.0));
    let output = layout_tree_with_state(
        &root,
        Rect::from_min_size(Point::new(0.0, 0.0), Vector2::new(240.0, 96.0)),
        &state,
        LayoutDebugOptions::default(),
    );

    assert!(output.virtual_windows.contains_key(&1));
    assert!(
        !output
            .diagnostics
            .iter()
            .any(|item| item.code == LayoutDiagnosticCode::VirtualizationPolicyIgnored)
    );
}

#[test]
fn virtualized_fill_and_percent_layout_matches_full_layout_for_window_items() {
    let mut children = Vec::with_capacity(120);
    for index in 0..120_u64 {
        let size_main = match index % 3 {
            0 => SizeModeMain::Fixed(18.0),
            1 => SizeModeMain::Percent(0.08),
            _ => SizeModeMain::Fill(1.0),
        };
        children.push(SlotChild {
            slot: SlotParams {
                size_main,
                size_cross: SizeModeCross::Fill,
                constraints: Constraints::new(0.0, f32::INFINITY, 4.0, 80.0),
                margin: Default::default(),
                align_cross_override: None,
                allow_fixed_compress: true,
            },
            child: LayoutNode::widget(index + 10, Vector2::new(40.0, 12.0)),
        });
    }
    let content = LayoutNode::container(
        2,
        ContainerPolicy {
            kind: ContainerKind::Column,
            align_main: crate::gui::layout_core::model::MainAlign::Center,
            spacing: 1.0,
            ..ContainerPolicy::default()
        },
        children,
    );
    let root_virtual = LayoutNode::container(
        1,
        ContainerPolicy {
            kind: ContainerKind::ScrollView,
            overflow: OverflowPolicy::Scroll,
            virtualization: Some(VirtualizationPolicy {
                enabled: true,
                axis: VirtualizationAxis::Vertical,
                overscan_px: 18.0,
            }),
            ..ContainerPolicy::default()
        },
        vec![SlotChild {
            slot: SlotParams {
                size_main: SizeModeMain::Fill(1.0),
                size_cross: SizeModeCross::Fill,
                constraints: Constraints::new(0.0, 260.0, 0.0, 1_800.0),
                margin: Default::default(),
                align_cross_override: None,
                allow_fixed_compress: false,
            },
            child: content.clone(),
        }],
    );
    let root_full = LayoutNode::container(
        1,
        ContainerPolicy {
            kind: ContainerKind::ScrollView,
            overflow: OverflowPolicy::Scroll,
            virtualization: None,
            ..ContainerPolicy::default()
        },
        vec![SlotChild {
            slot: SlotParams {
                size_main: SizeModeMain::Fill(1.0),
                size_cross: SizeModeCross::Fill,
                constraints: Constraints::new(0.0, 260.0, 0.0, 1_800.0),
                margin: Default::default(),
                align_cross_override: None,
                allow_fixed_compress: false,
            },
            child: content,
        }],
    );

    let mut state = LayoutState::default();
    state.scroll_offsets.insert(1, Vector2::new(0.0, 240.0));
    let viewport = Rect::from_min_size(Point::new(0.0, 0.0), Vector2::new(220.0, 120.0));
    let virtual_output = layout_tree_with_state(
        &root_virtual,
        viewport,
        &state,
        LayoutDebugOptions::default(),
    );
    let full_output =
        layout_tree_with_state(&root_full, viewport, &state, LayoutDebugOptions::default());
    let window = virtual_output
        .virtual_windows
        .get(&1)
        .expect("virtual window metadata");

    assert!(
        !virtual_output
            .diagnostics
            .iter()
            .any(|item| item.code == LayoutDiagnosticCode::VirtualizationPolicyIgnored)
    );
    for index in window.first_index..window.last_index_exclusive {
        let node_id = index as u64 + 10;
        assert_eq!(
            virtual_output.rects.get(&node_id),
            full_output.rects.get(&node_id)
        );
    }
}
