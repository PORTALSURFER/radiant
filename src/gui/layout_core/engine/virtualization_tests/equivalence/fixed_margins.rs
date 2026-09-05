use super::super::*;
use crate::gui::layout_core::WritingDirection;

#[test]
fn virtualized_fixed_rows_with_balanced_margins_match_full_layout() {
    let mut children = Vec::with_capacity(96);
    for index in 0..96_u64 {
        children.push(SlotChild {
            slot: SlotParams {
                size_main: SizeModeMain::Fixed(24.0),
                size_cross: SizeModeCross::Fill,
                constraints: Constraints::new(0.0, f32::INFINITY, 4.0, 80.0),
                margin: crate::gui::layout_core::model::Insets {
                    top: -2.0,
                    bottom: 2.0,
                    ..Default::default()
                },
                align_cross_override: None,
                allow_fixed_compress: false,
            },
            child: LayoutNode::widget(index + 10, Vector2::new(40.0, 12.0)),
        });
    }
    let content = LayoutNode::container(
        2,
        ContainerPolicy {
            kind: ContainerKind::Column,
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
            slot: SlotParams::fill(),
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
            slot: SlotParams::fill(),
            child: content,
        }],
    );

    let mut state = LayoutState::default();
    state.scroll_offsets.insert(1, Vector2::new(0.0, 180.0));
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

    for index in window.first_index..window.last_index_exclusive {
        let node_id = index as u64 + 10;
        assert_eq!(
            virtual_output.rects.get(&node_id),
            full_output.rects.get(&node_id)
        );
    }
}

#[test]
fn virtualized_horizontal_rtl_uses_logical_margins_and_physical_offsets() {
    let children = (0..24_u64)
        .map(|index| SlotChild {
            slot: SlotParams {
                size_main: SizeModeMain::Fixed(24.0),
                size_cross: SizeModeCross::Fill,
                constraints: Constraints::new(0.0, f32::INFINITY, 0.0, 80.0),
                margin: crate::gui::layout_core::model::Insets {
                    left: 2.0,
                    right: 5.0,
                    ..Default::default()
                },
                align_cross_override: None,
                allow_fixed_compress: false,
            },
            child: LayoutNode::widget(index + 10, Vector2::new(24.0, 12.0)),
        })
        .collect::<Vec<_>>();
    let content = LayoutNode::container(
        2,
        ContainerPolicy {
            kind: ContainerKind::Row,
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
                axis: VirtualizationAxis::Horizontal,
                overscan_px: 0.0,
            }),
            ..ContainerPolicy::default()
        },
        vec![SlotChild {
            slot: SlotParams::fill(),
            child: content.clone(),
        }],
    );
    let root_full = LayoutNode::container(
        1,
        ContainerPolicy {
            kind: ContainerKind::ScrollView,
            overflow: OverflowPolicy::Scroll,
            ..ContainerPolicy::default()
        },
        vec![SlotChild {
            slot: SlotParams::fill(),
            child: content,
        }],
    );
    let viewport = Rect::from_size(120.0, 80.0);
    let virtual_output =
        crate::layout::layout_tree_with_direction(&root_virtual, viewport, WritingDirection::Rtl);
    let full_output =
        crate::layout::layout_tree_with_direction(&root_full, viewport, WritingDirection::Rtl);
    let window = virtual_output
        .virtual_windows
        .get(&1)
        .expect("virtual window metadata");
    assert!(window.viewport_main_start.abs() < f32::EPSILON);
    assert!(window.viewport_main_end > window.viewport_main_start);
    for index in window.first_index..window.last_index_exclusive {
        let node_id = index as u64 + 10;
        assert_eq!(
            virtual_output.rects.get(&node_id),
            full_output.rects.get(&node_id)
        );
    }
}
