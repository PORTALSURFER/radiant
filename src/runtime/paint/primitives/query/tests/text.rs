use crate::{
    gui::types::{Point, Rect, Vector2},
    runtime::{PaintPrimitive, PaintTextAlign, PaintTextRun, SurfacePaintPlan},
    theme::ThemeTokens,
};

#[test]
fn text_queries_return_runs_and_inputs_in_paint_order() {
    let theme = ThemeTokens::default();
    let mut plan = SurfacePaintPlan::empty(&theme);
    plan.primitives.push(PaintPrimitive::Text(PaintTextRun {
        widget_id: 11,
        text: "Status".into(),
        rect: Rect::from_min_size(Point::new(4.0, 5.0), Vector2::new(80.0, 16.0)),
        font_size: 12.0,
        baseline: None,
        color: theme.text_primary,
        align: PaintTextAlign::Left,
        wrap: crate::widgets::TextWrap::None,
    }));
    plan.primitives
        .push(PaintPrimitive::TextInput(crate::runtime::PaintTextInput {
            widget_id: 12,
            rect: Rect::from_min_size(Point::new(8.0, 24.0), Vector2::new(96.0, 18.0)),
            placeholder: Some("Search".into()),
            completion_suffix: None,
            state: crate::widgets::TextInputState::from_value(String::from("ki")),
            font_size: 12.0,
            baseline: None,
            color: theme.text_primary,
            placeholder_color: theme.text_muted,
            completion_color: theme.text_muted,
            selection_color: theme.accent_mint,
            caret_color: theme.text_primary,
            focused: true,
        }));

    let run = plan.first_text_run("Status").expect("status text run");
    assert_eq!(run.widget_id, 11);
    assert_eq!(run.rect.min, Point::new(4.0, 5.0));
    assert!(plan.contains_text("Status"));
    assert!(!plan.contains_text("Missing"));
    assert_eq!(plan.first_text_rect("Status"), Some(run.rect));
    assert_eq!(plan.first_text_color("Status"), Some(theme.text_primary));
    assert!(plan.contains_text_after_x("Status", 3.0));
    assert!(!plan.contains_text_after_x("Status", 5.0));
    assert_eq!(
        plan.first_text_run_after_x("Status", 3.0)
            .map(|run| run.widget_id),
        Some(11)
    );
    assert_eq!(
        plan.text_runs()
            .map(|run| run.text.as_str())
            .collect::<Vec<_>>(),
        vec!["Status"]
    );
    assert_eq!(plan.text_labels().collect::<Vec<_>>(), vec!["Status"]);
    assert_eq!(plan.text_label_strings(), vec![String::from("Status")]);
    assert_eq!(
        plan.text_inputs()
            .map(|input| input.widget_id)
            .collect::<Vec<_>>(),
        vec![12]
    );
    assert_eq!(
        plan.first_text_input().map(|input| input.widget_id),
        Some(12)
    );
    assert!(plan.contains_text_input());
    assert_eq!(
        plan.primitives[0].text_run().map(|run| run.widget_id),
        Some(11)
    );
    assert_eq!(
        plan.primitives[1].text_input().map(|input| input.widget_id),
        Some(12)
    );
}
