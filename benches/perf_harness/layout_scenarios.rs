//! Layout performance scenarios.

#[path = "layout_scenarios/trees.rs"]
mod trees;

use crate::runner::ScenarioCounters;
use radiant::layout::{
    LayoutDebugOptions, LayoutEngine, LayoutNode, LayoutState, Point, Rect, SizeModeMain, Vector2,
    layout_tree, layout_tree_with_state,
};
use std::hint::black_box;

pub(super) fn deep_nesting() -> impl FnMut() {
    bench_deep_nesting
}

pub(super) fn wrap_1k() -> impl FnMut() {
    bench_wrap_1k
}

pub(super) fn virtualized_10k() -> impl FnMut() -> ScenarioCounters {
    bench_virtualized_10k
}

pub(super) fn virtualized_fixed_10k() -> impl FnMut() -> ScenarioCounters {
    bench_virtualized_fixed_10k
}

pub(super) fn virtualized_fixed_scroll_10k() -> impl FnMut() -> ScenarioCounters {
    let mut fixed_scroll = StatefulVirtualizedScrollBench::new();
    move || fixed_scroll.step()
}

pub(super) fn mark_dirty_subtree_10k() -> impl FnMut() -> ScenarioCounters {
    let mut dirty_subtree = StatefulDirtySubtreeBench::new();
    move || dirty_subtree.step()
}

pub(super) fn dirty_virtual_cache_10k() -> impl FnMut() -> ScenarioCounters {
    let mut dirty_cache = StatefulDirtyVirtualCacheBench::new();
    move || dirty_cache.step()
}

fn bench_deep_nesting() {
    let node = trees::deep_nesting_tree();
    let output = layout_tree(&node, viewport(640.0, 360.0));
    assert!(output.rects.len() >= 301);
    black_box(output);
}

fn bench_wrap_1k() {
    let root = trees::wrap_tree(1_000);
    let output = layout_tree(&root, viewport(1024.0, 768.0));
    assert_eq!(output.rects.len(), 1_001);
    black_box(output);
}

fn bench_virtualized_10k() -> ScenarioCounters {
    let root = trees::virtualized_scroll_tree(10_000, SizeModeMain::Intrinsic);
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
    let materialized = output.stats.materialized_nodes as u64;
    black_box(output);
    ScenarioCounters::default().with_allocation_sensitive_work_count(materialized)
}

fn bench_virtualized_fixed_10k() -> ScenarioCounters {
    let root = trees::virtualized_scroll_tree(10_000, SizeModeMain::Fixed(28.0));
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
    let measured = output.stats.measured_nodes as u64;
    black_box(output);
    ScenarioCounters::default().with_allocation_sensitive_work_count(measured)
}

struct StatefulVirtualizedScrollBench {
    root: LayoutNode,
    engine: LayoutEngine,
    state: LayoutState,
    offset: f32,
}

impl StatefulVirtualizedScrollBench {
    fn new() -> Self {
        let root = trees::virtualized_scroll_tree(10_000, SizeModeMain::Fixed(28.0));
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

    fn step(&mut self) -> ScenarioCounters {
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
        let measured = output.stats.measured_nodes as u64;
        black_box(output);
        ScenarioCounters::default()
            .with_relayout_count(1)
            .with_allocation_sensitive_work_count(measured)
    }
}

struct StatefulDirtySubtreeBench {
    root: LayoutNode,
    engine: LayoutEngine,
}

impl StatefulDirtySubtreeBench {
    fn new() -> Self {
        Self {
            root: trees::virtualized_scroll_tree(10_000, SizeModeMain::Fixed(28.0)),
            engine: LayoutEngine::default(),
        }
    }

    fn step(&mut self) -> ScenarioCounters {
        self.engine.mark_measure_dirty_subtree(&self.root, 2);
        self.engine.clear_dirty();
        black_box(&self.engine);
        ScenarioCounters::default()
            .with_dirty_mark_count(1)
            .with_allocation_sensitive_work_count(1)
    }
}

struct StatefulDirtyVirtualCacheBench {
    root: LayoutNode,
    engine: LayoutEngine,
    state: LayoutState,
}

impl StatefulDirtyVirtualCacheBench {
    fn new() -> Self {
        let root = trees::virtualized_scroll_tree(10_000, SizeModeMain::Fixed(28.0));
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
        }
    }

    fn step(&mut self) -> ScenarioCounters {
        self.engine.mark_measure_dirty_subtree(&self.root, 2);
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
        let materialized = output.stats.materialized_nodes as u64;
        black_box(output);
        ScenarioCounters::default()
            .with_scene_rebuild_count(1)
            .with_allocation_sensitive_work_count(materialized)
    }
}

fn viewport(width: f32, height: f32) -> Rect {
    Rect::from_min_size(Point::new(0.0, 0.0), Vector2::new(width, height))
}
