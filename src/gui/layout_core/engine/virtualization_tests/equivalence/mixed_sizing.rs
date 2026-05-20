use super::super::*;

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
