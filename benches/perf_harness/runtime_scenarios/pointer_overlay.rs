use radiant::{
    gui::types::Rgba8,
    layout::{Point, Rect, SizeModeCross, SizeModeMain, SlotParams, Vector2},
    runtime::{
        PaintFillRect, PaintPrimitive, RuntimeBridge, SurfaceChild, SurfaceNode, SurfaceRuntime,
        UiSurface, WidgetMessageMapper,
    },
    theme::ThemeTokens,
    widgets::{
        ButtonWidget, FocusBehavior, Widget, WidgetCommon, WidgetInput, WidgetOutput, WidgetSizing,
    },
};
use std::{hint::black_box, sync::Arc};

const ROWS: u64 = 10_000;
const CURSOR_WIDGET_ID: u64 = 10;

pub(super) fn pointer_overlay_paint_10k() -> impl FnMut() {
    let mut bench = StatefulPointerOverlayPaintBench::new();
    move || bench.step()
}

struct StatefulPointerOverlayPaintBench {
    runtime: SurfaceRuntime<PointerOverlayBridge, ()>,
    theme: ThemeTokens,
    overlay: Vec<PaintPrimitive>,
    cursor_x: f32,
}

impl StatefulPointerOverlayPaintBench {
    fn new() -> Self {
        let mut runtime = SurfaceRuntime::new(PointerOverlayBridge, Vector2::new(360.0, 240.0));
        let first = runtime.dispatch_pointer_move_with_outcome(Point::new(24.0, 18.0));
        assert!(first.routed());
        assert!(first.needs_scene_rebuild());
        Self {
            runtime,
            theme: ThemeTokens::default(),
            overlay: Vec::with_capacity(4),
            cursor_x: 24.0,
        }
    }

    fn step(&mut self) {
        self.cursor_x = if self.cursor_x < 320.0 {
            self.cursor_x + 3.0
        } else {
            24.0
        };
        let outcome = self
            .runtime
            .dispatch_pointer_move_with_outcome(Point::new(self.cursor_x, 18.0));
        assert!(outcome.routed());
        assert!(outcome.paint_only_requested);
        assert!(!outcome.needs_scene_rebuild());

        self.overlay.clear();
        self.runtime
            .runtime_overlay_paint_into(&self.theme, &mut self.overlay);
        assert_eq!(
            self.overlay.len(),
            1,
            "pointer overlay paint should remain a bounded runtime overlay"
        );
        black_box(&self.overlay);
    }
}

struct PointerOverlayBridge;

impl RuntimeBridge<()> for PointerOverlayBridge {
    fn project_surface(&mut self) -> Arc<UiSurface<()>> {
        let mut rows = Vec::with_capacity(ROWS as usize);
        rows.push(SurfaceChild::new(
            fixed_row_slot(36.0),
            SurfaceNode::custom_widget(
                PointerOverlayProbeWidget::new(CURSOR_WIDGET_ID),
                WidgetMessageMapper::none(),
            ),
        ));
        rows.extend((1..ROWS).map(|index| {
            SurfaceChild::new(
                fixed_row_slot(28.0),
                SurfaceNode::widget(
                    ButtonWidget::new(
                        10_000 + index,
                        format!("Row {index:05}"),
                        WidgetSizing::fixed(Vector2::new(180.0, 28.0)),
                    ),
                    WidgetMessageMapper::none(),
                ),
            )
        }));
        Arc::new(UiSurface::new(SurfaceNode::column(1, 0.0, rows)))
    }
}

fn fixed_row_slot(height: f32) -> SlotParams {
    SlotParams {
        size_main: SizeModeMain::Fixed(height),
        size_cross: SizeModeCross::Fill,
        constraints: radiant::layout::Constraints::unconstrained(),
        margin: Default::default(),
        align_cross_override: None,
        allow_fixed_compress: false,
    }
}

#[derive(Clone, Debug)]
struct PointerOverlayProbeWidget {
    common: WidgetCommon,
    cursor_x: f32,
}

impl PointerOverlayProbeWidget {
    fn new(id: u64) -> Self {
        let mut common = WidgetCommon::new(
            id,
            WidgetSizing::fixed(Vector2::new(360.0, 36.0)).with_baseline(24.0),
        );
        common.focus = FocusBehavior::Pointer;
        Self {
            common,
            cursor_x: 0.0,
        }
    }
}

impl Widget for PointerOverlayProbeWidget {
    fn common(&self) -> &WidgetCommon {
        &self.common
    }

    fn common_mut(&mut self) -> &mut WidgetCommon {
        &mut self.common
    }

    fn prefers_pointer_move_paint_only(&self) -> bool {
        true
    }

    fn handle_input(&mut self, bounds: Rect, input: WidgetInput) -> Option<WidgetOutput> {
        if let WidgetInput::PointerMove { position } = input {
            self.common.state.hovered = bounds.contains(position);
            self.cursor_x = position.x.clamp(bounds.min.x, bounds.max.x);
        }
        None
    }

    fn append_paint(
        &self,
        _primitives: &mut Vec<PaintPrimitive>,
        _bounds: Rect,
        _layout: &radiant::layout::LayoutOutput,
        _theme: &ThemeTokens,
    ) {
    }

    fn append_runtime_overlay_paint(
        &self,
        primitives: &mut Vec<PaintPrimitive>,
        bounds: Rect,
        _layout: &radiant::layout::LayoutOutput,
        theme: &ThemeTokens,
    ) {
        primitives.push(PaintPrimitive::FillRect(PaintFillRect {
            widget_id: self.common.id,
            rect: Rect::from_min_max(
                Point::new(self.cursor_x - 0.5, bounds.min.y),
                Point::new(self.cursor_x + 0.5, bounds.max.y),
            )
            .clamp_to(bounds),
            color: Rgba8 {
                r: theme.accent_copper.r,
                g: theme.accent_copper.g,
                b: theme.accent_copper.b,
                a: 220,
            },
        }));
    }
}
