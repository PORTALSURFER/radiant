use super::*;

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

#[test]
fn invalid_virtualized_descendant_margins_fail_closed_before_scroll_state() {
    let viewport = Rect::from_min_size(Point::new(0.0, 0.0), Vector2::new(220.0, 100.0));
    for (kind, axis) in [
        (ContainerKind::Row, VirtualizationAxis::Horizontal),
        (ContainerKind::Column, VirtualizationAxis::Vertical),
    ] {
        for margin in [
            Insets::all(f32::NAN),
            Insets::all(f32::MAX),
            Insets::all(-f32::MAX),
        ] {
            let valid_root = virtualized_linear_root(kind, axis, Insets::default());
            let invalid_root = virtualized_linear_root(kind, axis, margin);
            let mut engine = LayoutEngine::default();

            let warmed = engine.layout_with_state(
                &valid_root,
                viewport,
                &LayoutState::default(),
                LayoutDebugOptions::default(),
            );
            assert!(warmed.virtual_windows.contains_key(&1));
            assert_eq!(engine.virtual_cache.len(), 1);

            let first = engine.layout_with_state(
                &invalid_root,
                viewport,
                &LayoutState::default(),
                LayoutDebugOptions::all_enabled(),
            );
            assert_invalid_virtualized_layout(&first);
            assert!(engine.virtual_cache.is_empty());

            let second = engine.layout_with_state(
                &invalid_root,
                viewport,
                &LayoutState::default(),
                LayoutDebugOptions::all_enabled(),
            );
            assert_invalid_virtualized_layout(&second);
            assert_eq!(first.diagnostics, second.diagnostics);
            assert!(engine.virtual_cache.is_empty());
        }
    }
}

fn virtualized_linear_root(
    kind: ContainerKind,
    axis: VirtualizationAxis,
    invalid_margin: Insets,
) -> LayoutNode {
    let children = (0..3_u64)
        .map(|index| SlotChild {
            slot: SlotParams {
                size_main: SizeModeMain::Fixed(24.0),
                size_cross: SizeModeCross::Fill,
                constraints: Constraints::unconstrained(),
                margin: if index == 1 {
                    invalid_margin
                } else {
                    Insets::default()
                },
                align_cross_override: None,
                allow_fixed_compress: false,
            },
            child: LayoutNode::widget(index + 10, Vector2::new(40.0, 20.0)),
        })
        .collect();
    let content = LayoutNode::container(
        2,
        ContainerPolicy {
            kind,
            spacing: 1.0,
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
                overscan_px: 0.0,
            }),
            ..ContainerPolicy::default()
        },
        vec![SlotChild::new(SlotParams::fill(), content)],
    )
}

fn assert_invalid_virtualized_layout(output: &super::super::LayoutOutput) {
    assert_eq!(output.rects.len(), 1);
    assert!(output.rects.contains_key(&1));
    for node_id in [2, 10, 11, 12] {
        assert!(
            output.is_omitted(node_id),
            "node {node_id} should be omitted"
        );
    }
    assert!(output.viewport_bounds.is_empty());
    assert!(output.virtual_windows.is_empty());
    assert!(output.overflowed.is_empty());
    assert!(output.overflow_flags.is_empty());
    assert!(
        output.stats.measured_nodes <= 1,
        "invalid virtualized content must not measure descendants"
    );
    assert_eq!(output.stats.laid_out_nodes, 1);
    assert_eq!(output.stats.materialized_nodes, 1);
    assert_eq!(output.diagnostics.len(), 1);
    assert_eq!(output.diagnostics[0].node_id, 1);
    assert_eq!(
        output.diagnostics[0].code,
        LayoutDiagnosticCode::NegativeSizeClamped
    );
    assert_eq!(
        output.diagnostics[0].message,
        "scroll viewport geometry was invalid and descendants were omitted"
    );
    assert!(!output.debug_primitives.iter().any(|primitive| {
        matches!(
            primitive.kind,
            DebugPrimitiveKind::ViewportBounds
                | DebugPrimitiveKind::VirtualWindowBounds
                | DebugPrimitiveKind::CulledRegion
                | DebugPrimitiveKind::OverflowMarker
        )
    }));
}
