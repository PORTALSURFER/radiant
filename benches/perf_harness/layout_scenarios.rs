//! Layout performance scenarios.

use radiant::layout::{
    ContainerKind, ContainerPolicy, LayoutDebugOptions, LayoutEngine, LayoutNode, LayoutState,
    Point, Rect, SizeModeCross, SizeModeMain, SlotChild, SlotParams, Vector2, VirtualizationAxis,
    VirtualizationPolicy, layout_tree, layout_tree_with_state,
};
use std::hint::black_box;

pub(super) fn deep_nesting() -> impl FnMut() {
    bench_deep_nesting
}

pub(super) fn wrap_1k() -> impl FnMut() {
    bench_wrap_1k
}

pub(super) fn virtualized_10k() -> impl FnMut() {
    bench_virtualized_10k
}

pub(super) fn virtualized_fixed_10k() -> impl FnMut() {
    bench_virtualized_fixed_10k
}

pub(super) fn virtualized_fixed_scroll_10k() -> impl FnMut() {
    let mut fixed_scroll = StatefulVirtualizedScrollBench::new();
    move || fixed_scroll.step()
}

pub(super) fn mark_dirty_subtree_10k() -> impl FnMut() {
    let mut dirty_subtree = StatefulDirtySubtreeBench::new();
    move || dirty_subtree.step()
}

fn bench_deep_nesting() {
    let node = deep_nesting_tree();
    let output = layout_tree(&node, viewport(640.0, 360.0));
    assert!(output.rects.len() >= 301);
    black_box(output);
}

fn deep_nesting_tree() -> LayoutNode {
    let mut node = LayoutNode::widget(9_999, Vector2::new(8.0, 8.0));
    for id in (1..=300).rev() {
        node = LayoutNode::container(
            id,
            ContainerPolicy {
                kind: ContainerKind::PaddingBox,
                padding: radiant::layout::Insets::all(1.0),
                ..ContainerPolicy::default()
            },
            vec![SlotChild::new(SlotParams::fill(), node)],
        );
    }
    node
}

fn bench_wrap_1k() {
    let root = wrap_tree(1_000);
    let output = layout_tree(&root, viewport(1024.0, 768.0));
    assert_eq!(output.rects.len(), 1_001);
    black_box(output);
}

fn wrap_tree(count: u64) -> LayoutNode {
    let children = (0..count)
        .map(|index| {
            SlotChild::new(
                SlotParams::fill(),
                LayoutNode::widget(index + 2, Vector2::new(12.0, 8.0)),
            )
        })
        .collect();
    LayoutNode::container(
        1,
        ContainerPolicy {
            kind: ContainerKind::Wrap,
            wrap: radiant::layout::WrapPolicy {
                item_gap: 1.0,
                line_gap: 1.0,
            },
            ..ContainerPolicy::default()
        },
        children,
    )
}

fn bench_virtualized_10k() {
    let root = virtualized_scroll_tree(10_000, SizeModeMain::Intrinsic);
    let mut state = LayoutState::default();
    state.scroll_offsets.insert(1, Vector2::new(0.0, 20_000.0));
    let output = layout_tree_with_state(
        &root,
        viewport(300.0, 140.0),
        &state,
        LayoutDebugOptions::default(),
    );
    let window = output
        .virtual_windows
        .get(&1)
        .expect("virtual window metadata");
    assert_eq!(window.total_children, 10_000);
    assert!(output.stats.materialized_nodes < 256);
    black_box(output);
}

fn bench_virtualized_fixed_10k() {
    let root = virtualized_scroll_tree(10_000, SizeModeMain::Fixed(28.0));
    let mut state = LayoutState::default();
    state.scroll_offsets.insert(1, Vector2::new(0.0, 20_000.0));
    let output = layout_tree_with_state(
        &root,
        viewport(300.0, 140.0),
        &state,
        LayoutDebugOptions::default(),
    );
    let window = output
        .virtual_windows
        .get(&1)
        .expect("virtual window metadata");
    assert_eq!(window.total_children, 10_000);
    assert!(output.stats.materialized_nodes < 256);
    assert!(output.stats.measured_nodes < 64);
    black_box(output);
}

struct StatefulVirtualizedScrollBench {
    root: LayoutNode,
    engine: LayoutEngine,
    state: LayoutState,
    offset: f32,
}

impl StatefulVirtualizedScrollBench {
    fn new() -> Self {
        let root = virtualized_scroll_tree(10_000, SizeModeMain::Fixed(28.0));
        let mut engine = LayoutEngine::default();
        let state = LayoutState::default();
        let output = engine.layout_with_state(
            &root,
            viewport(300.0, 140.0),
            &state,
            LayoutDebugOptions::default(),
        );
        assert!(output.virtual_windows.contains_key(&1));
        Self {
            root,
            engine,
            state,
            offset: 0.0,
        }
    }

    fn step(&mut self) {
        self.offset = (self.offset + 360.0) % 120_000.0;
        self.state
            .scroll_offsets
            .insert(1, Vector2::new(0.0, self.offset));
        let output = self.engine.layout_with_state(
            &self.root,
            viewport(300.0, 140.0),
            &self.state,
            LayoutDebugOptions::default(),
        );
        let window = output
            .virtual_windows
            .get(&1)
            .expect("virtual window metadata");
        assert_eq!(window.total_children, 10_000);
        assert!(output.stats.materialized_nodes < 256);
        assert!(output.stats.measured_nodes < 64);
        black_box(output);
    }
}

struct StatefulDirtySubtreeBench {
    root: LayoutNode,
    engine: LayoutEngine,
}

impl StatefulDirtySubtreeBench {
    fn new() -> Self {
        Self {
            root: virtualized_scroll_tree(10_000, SizeModeMain::Fixed(28.0)),
            engine: LayoutEngine::default(),
        }
    }

    fn step(&mut self) {
        self.engine.mark_measure_dirty_subtree(&self.root, 2);
        self.engine.clear_dirty();
        black_box(&self.engine);
    }
}

fn virtualized_scroll_tree(count: u64, size_main: SizeModeMain) -> LayoutNode {
    let items = (0..count)
        .map(|index| {
            SlotChild::new(
                SlotParams {
                    size_main,
                    size_cross: SizeModeCross::Fill,
                    constraints: radiant::layout::Constraints::unconstrained(),
                    margin: Default::default(),
                    align_cross_override: None,
                    allow_fixed_compress: false,
                },
                LayoutNode::widget(index + 10, Vector2::new(120.0, 10.0)),
            )
        })
        .collect();

    LayoutNode::container(
        1,
        ContainerPolicy {
            kind: ContainerKind::ScrollView,
            overflow: radiant::layout::OverflowPolicy::Scroll,
            virtualization: Some(VirtualizationPolicy {
                enabled: true,
                axis: VirtualizationAxis::Vertical,
                overscan_px: 16.0,
            }),
            ..ContainerPolicy::default()
        },
        vec![SlotChild::new(
            SlotParams::fill(),
            LayoutNode::container(
                2,
                ContainerPolicy {
                    kind: ContainerKind::Column,
                    spacing: 1.0,
                    ..ContainerPolicy::default()
                },
                items,
            ),
        )],
    )
}

fn viewport(width: f32, height: f32) -> Rect {
    Rect::from_min_size(Point::new(0.0, 0.0), Vector2::new(width, height))
}
