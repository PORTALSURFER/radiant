use crate::viewport;
use radiant::{
    layout::{
        ContainerKind, ContainerPolicy, LayoutNode, LayoutOutput, SizeModeCross, SizeModeMain,
        SlotParams, Vector2, layout_tree,
    },
    runtime::{Command, RuntimeBridge, SurfaceChild, SurfaceNode, SurfaceRuntime, UiSurface},
    theme::ThemeTokens,
    widgets::{TextWidget, WidgetSizing},
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
    bench_command_flattening_512
}

struct StatefulRuntimeSurfaceBench {
    surface: UiSurface<()>,
    layout_node: LayoutNode,
    theme: ThemeTokens,
}

impl StatefulRuntimeSurfaceBench {
    fn new() -> Self {
        let surface = UiSurface::<()>::new(runtime_surface_node(250));
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
        let surface = UiSurface::<()>::new(text_paint_surface_node(1_000));
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
        let surface = UiSurface::<()>::new(horizontal_scroll_surface_node(1_000));
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
        Arc::new(UiSurface::new(runtime_surface_node(1_000)))
    }
}

fn runtime_surface_node(count: u64) -> SurfaceNode<()> {
    let rows = (0..count)
        .map(|index| {
            SurfaceChild::new(
                SlotParams {
                    size_main: SizeModeMain::Intrinsic,
                    size_cross: SizeModeCross::Fill,
                    constraints: radiant::layout::Constraints::unconstrained(),
                    margin: Default::default(),
                    align_cross_override: None,
                    allow_fixed_compress: false,
                },
                SurfaceNode::static_widget(TextWidget::new(
                    10_000 + index,
                    format!("Row {index}"),
                    WidgetSizing::fixed(Vector2::new(180.0, 24.0)),
                )),
            )
        })
        .collect();
    SurfaceNode::scroll_area(
        1,
        SurfaceNode::container(
            2,
            ContainerPolicy {
                kind: ContainerKind::Column,
                spacing: 2.0,
                ..ContainerPolicy::default()
            },
            rows,
        ),
    )
}

fn text_paint_surface_node(count: u64) -> SurfaceNode<()> {
    let rows = (0..count)
        .map(|index| {
            SurfaceChild::new(
                SlotParams {
                    size_main: SizeModeMain::Fixed(22.0),
                    size_cross: SizeModeCross::Fill,
                    constraints: radiant::layout::Constraints::unconstrained(),
                    margin: Default::default(),
                    align_cross_override: None,
                    allow_fixed_compress: false,
                },
                SurfaceNode::static_widget(TextWidget::new(
                    40_000 + index,
                    format!(
                        "Track {index:04}  position {index:04}.{:02}  cached text row",
                        index % 97
                    ),
                    WidgetSizing::fixed(Vector2::new(520.0, 22.0)),
                )),
            )
        })
        .collect();
    SurfaceNode::scroll_area(
        30_000,
        SurfaceNode::container(
            30_001,
            ContainerPolicy {
                kind: ContainerKind::Column,
                spacing: 1.0,
                ..ContainerPolicy::default()
            },
            rows,
        ),
    )
}

fn horizontal_scroll_surface_node(count: u64) -> SurfaceNode<()> {
    let items = (0..count)
        .map(|index| {
            SurfaceChild::new(
                SlotParams {
                    size_main: SizeModeMain::Fixed(88.0),
                    size_cross: SizeModeCross::Fill,
                    constraints: radiant::layout::Constraints::unconstrained(),
                    margin: Default::default(),
                    align_cross_override: None,
                    allow_fixed_compress: false,
                },
                SurfaceNode::static_widget(TextWidget::new(
                    90_000 + index,
                    format!("Clip {index:04}"),
                    WidgetSizing::fixed(Vector2::new(88.0, 24.0)),
                )),
            )
        })
        .collect();
    SurfaceNode::scroll_area(
        80_000,
        SurfaceNode::container(
            80_001,
            ContainerPolicy {
                kind: ContainerKind::Row,
                spacing: 2.0,
                ..ContainerPolicy::default()
            },
            items,
        ),
    )
}

fn bench_command_flattening_512() {
    let command = Command::batch((0..512).map(|index| {
        if index % 8 == 0 {
            Command::batch([
                Command::message(index),
                Command::request_repaint(),
                Command::message(index + 10_000),
            ])
        } else if index % 5 == 0 {
            Command::request_paint_only()
        } else {
            Command::message(index)
        }
    }));
    let messages = command.into_messages();
    assert_eq!(messages.len(), 486);
    black_box(messages);
}
