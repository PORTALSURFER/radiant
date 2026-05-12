use super::*;
use radiant::{
    runtime::{PaintPrimitive, PaintTextAlign, PaintTextRun},
    widgets::{TextAlign, TextWrap, WidgetCommon, WidgetInput, WidgetOutput},
};

#[derive(Clone)]
struct CustomTextPolicyWidget {
    common: WidgetCommon,
    wrap: TextWrap,
    align: TextAlign,
}

impl CustomTextPolicyWidget {
    fn new(id: u64) -> Self {
        Self {
            common: WidgetCommon::new(id, WidgetSizing::fixed(Vector2::new(120.0, 24.0))),
            wrap: TextWrap::None,
            align: TextAlign::Left,
        }
    }
}

impl Widget for CustomTextPolicyWidget {
    fn common(&self) -> &WidgetCommon {
        &self.common
    }

    fn common_mut(&mut self) -> &mut WidgetCommon {
        &mut self.common
    }

    fn handle_input(&mut self, _bounds: Rect, _input: WidgetInput) -> Option<WidgetOutput> {
        None
    }

    fn set_text_wrap(&mut self, wrap: TextWrap) -> bool {
        self.wrap = wrap;
        true
    }

    fn set_text_align(&mut self, align: TextAlign) -> bool {
        self.align = align;
        true
    }

    fn append_paint(
        &self,
        primitives: &mut Vec<PaintPrimitive>,
        bounds: Rect,
        _layout: &radiant::layout::LayoutOutput,
        theme: &radiant::theme::ThemeTokens,
    ) {
        primitives.push(PaintPrimitive::Text(PaintTextRun {
            widget_id: self.common.id,
            text: "custom".into(),
            rect: bounds,
            font_size: 13.0,
            baseline: Some(17.0),
            color: theme.text_primary,
            align: match self.align {
                TextAlign::Left => PaintTextAlign::Left,
                TextAlign::Center => PaintTextAlign::Center,
                TextAlign::Right => PaintTextAlign::Right,
            },
            wrap: self.wrap,
        }));
    }
}

#[test]
fn application_builder_typography_helpers_lower_text_policies_and_baselines() {
    use radiant::prelude::{self as ui, IntoView};

    let surface: UiSurface<()> = ui::column([
        ui::text("Wrapped text that should stay inside the assigned text rectangle")
            .wrap()
            .id(10)
            .fill_width()
            .height(64.0)
            .baseline(18.0),
        ui::text("Truncated text that keeps one line")
            .truncate()
            .id(11)
            .fill_width()
            .height(28.0)
            .baseline(19.0),
        ui::row([
            ui::text("Name")
                .id(12)
                .size(80.0, 28.0)
                .baseline(19.0)
                .align_text(TextAlign::Right),
            ui::text("Radiant")
                .id(13)
                .fill_width()
                .height(28.0)
                .baseline(19.0)
                .align_text(TextAlign::Center),
        ])
        .id(20)
        .fill_width()
        .spacing(8.0),
    ])
    .id(1)
    .padding(16.0)
    .spacing(10.0)
    .into_surface();
    let layout = layout_tree(
        &surface.layout_node(),
        Rect::from_min_size(Point::new(0.0, 0.0), Vector2::new(360.0, 180.0)),
    );

    let wrapped = widget_ref::<TextWidget, _>(&surface, 10, "wrapped text");
    let truncated = widget_ref::<TextWidget, _>(&surface, 11, "truncated text");
    assert_eq!(wrapped.wrap, TextWrap::Word);
    assert_eq!(wrapped.align, TextAlign::Left);
    assert_eq!(wrapped.common.sizing.baseline, Some(18.0));
    assert_eq!(truncated.wrap, TextWrap::None);
    assert_eq!(truncated.common.sizing.baseline, Some(19.0));
    assert_eq!(
        widget_ref::<TextWidget, _>(&surface, 12, "label").align,
        TextAlign::Right
    );
    assert_eq!(
        widget_ref::<TextWidget, _>(&surface, 13, "value").align,
        TextAlign::Center
    );
    assert_eq!(
        widget_ref::<TextWidget, _>(&surface, 12, "label")
            .common
            .sizing
            .baseline,
        Some(19.0)
    );
    let paint = surface.paint_plan(&layout, &radiant::theme::ThemeTokens::default());
    assert!(paint.primitives.iter().any(|primitive| {
        matches!(
            primitive,
            PaintPrimitive::Text(text)
                if text.widget_id == 12 && text.align == PaintTextAlign::Right
        )
    }));
    assert!(paint.primitives.iter().any(|primitive| {
        matches!(
            primitive,
            PaintPrimitive::Text(text)
                if text.widget_id == 13 && text.align == PaintTextAlign::Center
        )
    }));
    assert_eq!(layout.rects[&10].height(), 64.0);
    assert_eq!(layout.rects[&11].height(), 28.0);
    assert_eq!(layout.rects[&12].height(), layout.rects[&13].height());
    assert!(layout.rects[&13].min.x >= layout.rects[&12].max.x + 8.0);
}

#[test]
fn application_builder_text_policy_modifiers_use_widget_contract() {
    use radiant::prelude::{self as ui, IntoView};

    let surface: UiSurface<()> = ui::widget(CustomTextPolicyWidget::new(0))
        .wrap()
        .align_text(TextAlign::Right)
        .id(10)
        .into_surface();

    let custom = widget_ref::<CustomTextPolicyWidget, _>(&surface, 10, "custom text policy");

    assert_eq!(custom.wrap, TextWrap::Word);
    assert_eq!(custom.align, TextAlign::Right);
}
