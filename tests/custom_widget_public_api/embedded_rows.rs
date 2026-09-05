use radiant::{
    application::{ApplicationEnvironment, LocaleId, TextScale},
    gui::list::{DenseRowChromeParts, DenseRowLabelParts, push_dense_row_label},
    layout::{LayoutNode, LayoutOutput, Rect, Vector2},
    runtime::{PaintPrimitive, ResolvedEnvironment, WindowEnvironment},
    theme::{DpiScale, ThemeTokens},
    widgets::{
        DeclaredTextMetrics, EmbeddedInteractiveRowWidget, InteractiveRowMessage,
        InteractiveRowVisualStateParts, InteractiveRowWidget, Widget, WidgetPaintContext,
        WidgetSizing,
    },
};
use std::{cell::Cell, rc::Rc, sync::Arc};

#[derive(Clone)]
struct LegacyEmbeddedRow {
    row: InteractiveRowWidget,
    paint_calls: Rc<Cell<u32>>,
}

impl EmbeddedInteractiveRowWidget for LegacyEmbeddedRow {
    type Message = InteractiveRowMessage;

    fn interactive_row(&self) -> &InteractiveRowWidget {
        &self.row
    }

    fn interactive_row_mut(&mut self) -> &mut InteractiveRowWidget {
        &mut self.row
    }

    fn append_interactive_row_paint(
        &self,
        primitives: &mut Vec<PaintPrimitive>,
        bounds: Rect,
        _layout: &LayoutOutput,
        _theme: &ThemeTokens,
    ) {
        self.paint_calls.set(self.paint_calls.get() + 1);
        push_dense_row_label(
            primitives,
            self.row.id(),
            bounds,
            DenseRowLabelParts::new("legacy", Default::default()),
        );
    }
}

#[derive(Clone)]
struct OptedInEmbeddedRow {
    row: InteractiveRowWidget,
    declared: DeclaredTextMetrics,
}

impl EmbeddedInteractiveRowWidget for OptedInEmbeddedRow {
    type Message = InteractiveRowMessage;

    fn interactive_row(&self) -> &InteractiveRowWidget {
        &self.row
    }

    fn interactive_row_mut(&mut self) -> &mut InteractiveRowWidget {
        &mut self.row
    }

    fn declared_interactive_row_text_metrics(&self) -> Option<DeclaredTextMetrics> {
        Some(self.declared)
    }

    fn append_interactive_row_paint(
        &self,
        primitives: &mut Vec<PaintPrimitive>,
        bounds: Rect,
        _layout: &LayoutOutput,
        _theme: &ThemeTokens,
    ) {
        self.row.push_dense_labeled_chrome_with_environment(
            primitives,
            bounds,
            DenseRowChromeParts::new(
                self.row
                    .dense_visual_state(InteractiveRowVisualStateParts::default()),
                Default::default(),
            ),
            DenseRowLabelParts::new("opted-in", Default::default()),
            self.declared,
            &ResolvedEnvironment::default(),
        );
    }

    fn append_interactive_row_paint_with_context(&self, context: &mut WidgetPaintContext<'_>) {
        let bounds = context.bounds();
        let environment = context.environment().clone();
        let primitives = context.primitives();
        self.row.push_dense_labeled_chrome_with_environment(
            primitives,
            bounds,
            DenseRowChromeParts::new(
                self.row
                    .dense_visual_state(InteractiveRowVisualStateParts::default()),
                Default::default(),
            ),
            DenseRowLabelParts::new("opted-in", Default::default()),
            self.declared,
            &environment,
        );
    }
}

fn environment(scale: f32) -> ResolvedEnvironment {
    ResolvedEnvironment::from_snapshots(
        WindowEnvironment::new(DpiScale::new(2.0), None, false, false),
        Arc::new(
            ApplicationEnvironment::new(LocaleId::english())
                .with_text_scale(TextScale::new(scale).expect("valid text scale")),
        ),
    )
}

fn row(id: u64, height: f32) -> InteractiveRowWidget {
    InteractiveRowWidget::new(id, WidgetSizing::fixed(Vector2::new(80.0, height)))
}

#[test]
fn old_style_embedded_rows_keep_legacy_layout_paint_and_one_callback() {
    let calls = Rc::new(Cell::new(0));
    let row = LegacyEmbeddedRow {
        row: row(701, 22.0),
        paint_calls: Rc::clone(&calls),
    };
    let bounds = Rect::from_size(80.0, 22.0);
    let environment = environment(2.0);
    let LayoutNode::Widget(node) = Widget::layout_node_with_environment(&row, &environment) else {
        panic!("legacy row should remain a widget leaf");
    };
    assert_eq!(node.intrinsic, Vector2::new(80.0, 22.0));
    assert!(!Widget::capabilities(&row).has_semantics());

    let layout = LayoutOutput::default();
    let theme = ThemeTokens::default();
    let mut legacy = Vec::new();
    row.append_paint(&mut legacy, bounds, &layout, &theme);
    calls.set(0);
    let mut contextual = Vec::new();
    let mut context =
        WidgetPaintContext::new(&mut contextual, bounds, &layout, &theme, &environment);
    Widget::append_paint_with_context(&row, &mut context);

    assert_eq!(contextual, legacy);
    assert_eq!(calls.get(), 1);
}

#[test]
fn opted_in_embedded_rows_resolve_intrinsic_and_paint_metrics_from_one_declaration() {
    let opted = OptedInEmbeddedRow {
        row: row(702, 22.0),
        declared: DeclaredTextMetrics::new(
            WidgetSizing::fixed(Vector2::new(80.0, 22.0)),
            13.0,
            Vector2::new(4.0, 0.0),
        ),
    };
    for (scale, expected) in [
        (1.0, (22.0, 13.0, 4.0)),
        (1.5, (33.0, 19.5, 6.0)),
        (2.0, (44.0, 26.0, 8.0)),
    ] {
        let environment = environment(scale);
        let LayoutNode::Widget(node) = Widget::layout_node_with_environment(&opted, &environment)
        else {
            panic!("opted-in row should remain a widget leaf");
        };
        assert_eq!(node.intrinsic.y, expected.0);

        for height in [11.0, 60.0] {
            let bounds = Rect::from_size(80.0, height);
            let layout = LayoutOutput::default();
            let theme = ThemeTokens::default();
            let mut primitives = Vec::new();
            let mut context =
                WidgetPaintContext::new(&mut primitives, bounds, &layout, &theme, &environment);
            Widget::append_paint_with_context(&opted, &mut context);
            let PaintPrimitive::Text(text) = primitives.last().expect("opted-in label") else {
                panic!("expected opted-in text");
            };
            assert_eq!(text.font_size, expected.1);
            assert_eq!(text.rect.min.x, expected.2);
        }
    }
}
