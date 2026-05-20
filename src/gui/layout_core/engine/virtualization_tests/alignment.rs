use super::*;

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
