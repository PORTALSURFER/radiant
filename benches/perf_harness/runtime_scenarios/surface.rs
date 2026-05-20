#[path = "surface/command_flattening.rs"]
mod command_flattening;
#[path = "surface/nodes.rs"]
mod nodes;

use crate::viewport;
use radiant::{
    layout::{LayoutNode, LayoutOutput, Vector2, layout_tree},
    runtime::{RuntimeBridge, SurfaceRuntime, UiSurface},
    theme::ThemeTokens,
};
use std::{hint::black_box, sync::Arc};

pub(super) fn surface_large_tree() -> impl FnMut() {
    let runtime_surface = StatefulRuntimeSurfaceBench::new();
    move || runtime_surface.step()
}

pub(super) fn text_paint_plan_1k() -> impl FnMut() {
    let text_surface = StatefulTextPaintPlanBench::new();
    move || text_surface.step()
}

pub(super) fn horizontal_scroll_paint_1k() -> impl FnMut() {
    let horizontal_scroll_surface = StatefulHorizontalScrollPaintBench::new();
    move || horizontal_scroll_surface.step()
}

pub(super) fn refresh_large_tree() -> impl FnMut() {
    let mut refresh_large_tree = StatefulRefreshBench::new();
    move || refresh_large_tree.step()
}

pub(super) fn resize_large_tree() -> impl FnMut() {
    let mut resize_large_tree = StatefulResizeBench::new();
    move || resize_large_tree.step()
}

pub(super) fn command_flattening_512() -> impl FnMut() {
    command_flattening::command_flattening_512()
}

struct StatefulRuntimeSurfaceBench {
    surface: UiSurface<()>,
    layout_node: LayoutNode,
    theme: ThemeTokens,
}

impl StatefulRuntimeSurfaceBench {
    fn new() -> Self {
        let surface = UiSurface::<()>::new(nodes::runtime_surface_node(250));
        let layout_node = surface.layout_node();
        Self {
            surface,
            layout_node,
            theme: ThemeTokens::default(),
        }
    }

    fn step(&self) {
        let output = layout_tree(&self.layout_node, viewport(960.0, 720.0));
        let plan = self.surface.paint_plan(&output, &self.theme);
        assert!(output.rects.len() >= 250);
        assert!(!plan.primitives.is_empty());
        black_box((output, plan));
    }
}

struct StatefulTextPaintPlanBench {
    surface: UiSurface<()>,
    layout_node: LayoutNode,
    theme: ThemeTokens,
}

impl StatefulTextPaintPlanBench {
    fn new() -> Self {
        let surface = UiSurface::<()>::new(nodes::text_paint_surface_node(1_000));
        let layout_node = surface.layout_node();
        Self {
            surface,
            layout_node,
            theme: ThemeTokens::default(),
        }
    }

    fn step(&self) {
        let output = layout_tree(&self.layout_node, viewport(960.0, 720.0));
        let plan = self.surface.paint_plan(&output, &self.theme);
        let stats = plan.stats();
        assert!(
            stats.text > 0 && stats.text < 64,
            "text-heavy scroll paint should stay bounded to the visible clip"
        );
        assert!(
            stats.total >= stats.text,
            "text-heavy paint plan should retain all text primitives"
        );
        black_box((output, plan));
    }
}

struct StatefulHorizontalScrollPaintBench {
    surface: UiSurface<()>,
    layout: LayoutOutput,
    theme: ThemeTokens,
}

impl StatefulHorizontalScrollPaintBench {
    fn new() -> Self {
        let surface = UiSurface::<()>::new(nodes::horizontal_scroll_surface_node(1_000));
        let layout_node = surface.layout_node();
        let layout = layout_tree(&layout_node, viewport(320.0, 80.0));
        Self {
            surface,
            layout,
            theme: ThemeTokens::default(),
        }
    }

    fn step(&self) {
        let plan = self.surface.paint_plan(&self.layout, &self.theme);
        let stats = plan.stats();
        assert!(
            stats.text > 0 && stats.text < 16,
            "horizontal clipped row paint should stay bounded to the visible clip"
        );
        black_box(plan);
    }
}

struct StatefulRefreshBench {
    runtime: SurfaceRuntime<RefreshBridge, ()>,
}

impl StatefulRefreshBench {
    fn new() -> Self {
        Self {
            runtime: SurfaceRuntime::new(RefreshBridge { revision: 0 }, Vector2::new(960.0, 720.0)),
        }
    }

    fn step(&mut self) {
        self.runtime.refresh();
        black_box(self.runtime.layout());
    }
}

struct StatefulResizeBench {
    runtime: SurfaceRuntime<RefreshBridge, ()>,
    wide: bool,
}

impl StatefulResizeBench {
    fn new() -> Self {
        Self {
            runtime: SurfaceRuntime::new(RefreshBridge { revision: 0 }, Vector2::new(960.0, 720.0)),
            wide: false,
        }
    }

    fn step(&mut self) {
        self.wide = !self.wide;
        let viewport = if self.wide {
            Vector2::new(960.0, 720.0)
        } else {
            Vector2::new(720.0, 540.0)
        };
        self.runtime.set_viewport(viewport);
        black_box(self.runtime.layout());
    }
}

struct RefreshBridge {
    revision: u64,
}

impl RuntimeBridge<()> for RefreshBridge {
    fn project_surface(&mut self) -> Arc<UiSurface<()>> {
        self.revision = self.revision.wrapping_add(1);
        Arc::new(UiSurface::new(nodes::runtime_surface_node(1_000)))
    }
}
