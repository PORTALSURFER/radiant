use crate::gui::{
    layout_core::{
        constraints::Constraints,
        engine::{LayoutDebugOptions, LayoutEngine, LayoutOutput, LayoutState},
        model::{
            ContainerKind, ContainerPolicy, OverflowPolicy, SizeModeCross, SizeModeMain,
            SlotParams, VirtualizationAxis, VirtualizationPolicy,
        },
        tree::{LayoutNode, SlotChild},
    },
    types::{Point, Rect, Vector2},
};

#[test]
fn layout_engine_reuses_scratch_maps_between_passes() {
    let children = (0..64)
        .map(|index| {
            SlotChild::new(
                SlotParams {
                    size_main: SizeModeMain::Fixed(12.0),
                    size_cross: SizeModeCross::Fill,
                    constraints: Constraints::unconstrained(),
                    margin: Default::default(),
                    align_cross_override: None,
                    allow_fixed_compress: false,
                },
                LayoutNode::widget(index + 10, Vector2::new(40.0, 12.0)),
            )
        })
        .collect();
    let root = LayoutNode::container(
        1,
        ContainerPolicy {
            kind: ContainerKind::ScrollView,
            overflow: OverflowPolicy::Scroll,
            virtualization: Some(VirtualizationPolicy {
                enabled: true,
                axis: VirtualizationAxis::Vertical,
                overscan_px: 8.0,
            }),
            ..ContainerPolicy::default()
        },
        vec![SlotChild::new(
            SlotParams::fill(),
            LayoutNode::container(
                2,
                ContainerPolicy {
                    kind: ContainerKind::Column,
                    ..ContainerPolicy::default()
                },
                children,
            ),
        )],
    );
    let viewport = Rect::from_min_size(Point::new(0.0, 0.0), Vector2::new(120.0, 64.0));
    let mut engine = LayoutEngine::default();

    let first = engine.layout_with_state(
        &root,
        viewport,
        &LayoutState::default(),
        LayoutDebugOptions::default(),
    );
    assert!(first.virtual_windows.contains_key(&1));
    let measured_capacity = engine.scratch.measured.capacity();
    let measured_by_node_capacity = engine.scratch.measured_by_node.capacity();
    let linear_window_capacity = engine.scratch.linear_windows.capacity();
    assert!(measured_capacity > 0);
    assert_eq!(
        measured_by_node_capacity, 0,
        "default layout should not populate measured-by-node debug storage"
    );
    assert!(linear_window_capacity > 0);

    let second = engine.layout_with_state(
        &root,
        viewport,
        &LayoutState::default(),
        LayoutDebugOptions::default(),
    );

    assert!(second.virtual_windows.contains_key(&1));
    assert!(engine.scratch.measured.capacity() >= measured_capacity);
    assert_eq!(engine.scratch.measured_by_node.capacity(), 0);
    assert!(engine.scratch.linear_windows.capacity() >= linear_window_capacity);

    let debug = engine.layout_with_state(
        &root,
        viewport,
        &LayoutState::default(),
        LayoutDebugOptions::all_enabled(),
    );

    assert!(debug.virtual_windows.contains_key(&1));
    assert!(engine.scratch.measured_by_node.capacity() > 0);
}

#[test]
fn layout_engine_can_reuse_caller_owned_output_storage() {
    let root = LayoutNode::container(
        1,
        ContainerPolicy {
            kind: ContainerKind::Column,
            ..ContainerPolicy::default()
        },
        (0..8)
            .map(|index| {
                SlotChild::new(
                    SlotParams {
                        size_main: SizeModeMain::Fixed(12.0),
                        size_cross: SizeModeCross::Fill,
                        constraints: Constraints::unconstrained(),
                        margin: Default::default(),
                        align_cross_override: None,
                        allow_fixed_compress: false,
                    },
                    LayoutNode::widget(index + 10, Vector2::new(40.0, 12.0)),
                )
            })
            .collect(),
    );
    let viewport = Rect::from_min_size(Point::new(0.0, 0.0), Vector2::new(120.0, 120.0));
    let mut engine = LayoutEngine::default();
    let mut output = LayoutOutput::default();

    engine.layout_with_state_into(
        &root,
        viewport,
        &LayoutState::default(),
        LayoutDebugOptions::all_enabled(),
        &mut output,
    );
    let diagnostics_capacity = output.diagnostics.capacity();
    let debug_capacity = output.debug_primitives.capacity();
    assert!(output.rects.contains_key(&17));

    engine.layout_with_state_into(
        &root,
        viewport,
        &LayoutState::default(),
        LayoutDebugOptions::all_enabled(),
        &mut output,
    );

    assert!(output.rects.contains_key(&17));
    assert!(output.diagnostics.capacity() >= diagnostics_capacity);
    assert!(output.debug_primitives.capacity() >= debug_capacity);
}

#[test]
fn layout_engine_reuses_linear_scratch_vectors_between_passes() {
    let root = LayoutNode::container(
        1,
        ContainerPolicy {
            kind: ContainerKind::Column,
            overflow: OverflowPolicy::Shrink,
            ..ContainerPolicy::default()
        },
        vec![
            SlotChild::new(
                SlotParams {
                    size_main: SizeModeMain::Fixed(80.0),
                    size_cross: SizeModeCross::Fill,
                    constraints: Constraints::unconstrained(),
                    margin: Default::default(),
                    align_cross_override: None,
                    allow_fixed_compress: true,
                },
                LayoutNode::widget(10, Vector2::new(40.0, 80.0)),
            ),
            SlotChild::new(
                SlotParams {
                    size_main: SizeModeMain::Fill(1.0),
                    size_cross: SizeModeCross::Fill,
                    constraints: Constraints {
                        min_h: 40.0,
                        ..Constraints::unconstrained()
                    },
                    margin: Default::default(),
                    align_cross_override: None,
                    allow_fixed_compress: false,
                },
                LayoutNode::widget(20, Vector2::new(40.0, 40.0)),
            ),
            SlotChild::new(
                SlotParams {
                    size_main: SizeModeMain::Fixed(80.0),
                    size_cross: SizeModeCross::Fill,
                    constraints: Constraints::unconstrained(),
                    margin: Default::default(),
                    align_cross_override: None,
                    allow_fixed_compress: true,
                },
                LayoutNode::widget(30, Vector2::new(40.0, 80.0)),
            ),
        ],
    );
    let viewport = Rect::from_min_size(Point::new(0.0, 0.0), Vector2::new(120.0, 100.0));
    let mut engine = LayoutEngine::default();

    let first = engine.layout(&root, viewport);
    let linear_sizes_capacity = engine.scratch.linear_sizes.capacity();
    let linear_unresolved_capacity = engine.scratch.linear_unresolved.capacity();

    assert!(first.rects.contains_key(&30));
    assert!(linear_sizes_capacity >= 3);
    assert!(linear_unresolved_capacity >= 1);
    assert!(engine.scratch.linear_sizes.is_empty());
    assert!(engine.scratch.linear_unresolved.is_empty());

    let second = engine.layout(&root, viewport);

    assert!(second.rects.contains_key(&30));
    assert!(engine.scratch.linear_sizes.capacity() >= linear_sizes_capacity);
    assert!(engine.scratch.linear_unresolved.capacity() >= linear_unresolved_capacity);
    assert!(engine.scratch.linear_sizes.is_empty());
    assert!(engine.scratch.linear_unresolved.is_empty());
}
