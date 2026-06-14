use super::*;

#[test]
fn dense_visual_state_merges_host_and_interaction_state() {
    let mut row = InteractiveRowWidget::new(7, WidgetSizing::fixed(Vector2::new(120.0, 22.0)));
    row.common.state.hovered = true;
    row.common.state.pressed = true;

    assert_eq!(
        row.dense_visual_state(InteractiveRowVisualStateParts {
            selected: true,
            active_target: false,
            candidate: true,
        }),
        DenseRowVisualState {
            selected: true,
            hovered: true,
            pressed: true,
            active_target: false,
            candidate: true,
        }
    );
}

#[test]
fn dense_visual_state_preserves_default_host_state() {
    let row = InteractiveRowWidget::new(7, WidgetSizing::fixed(Vector2::new(120.0, 22.0)));

    assert_eq!(
        row.dense_visual_state(InteractiveRowVisualStateParts::default()),
        DenseRowVisualState::default()
    );
}

#[test]
fn paints_interaction_fill_tracks_hover_and_drag_policy() {
    let idle = InteractiveRowWidget::new(7, WidgetSizing::fixed(Vector2::new(120.0, 22.0)));
    assert!(idle.paints_interaction_fill());

    let suppressed = InteractiveRowWidget::new(8, WidgetSizing::fixed(Vector2::new(120.0, 22.0)))
        .suppress_hover(true);
    assert!(!suppressed.paints_interaction_fill());

    let active_non_source =
        InteractiveRowWidget::new(9, WidgetSizing::fixed(Vector2::new(120.0, 22.0)))
            .with_drag_active(true);
    assert!(!active_non_source.paints_interaction_fill());

    let active_source =
        InteractiveRowWidget::new(10, WidgetSizing::fixed(Vector2::new(120.0, 22.0)))
            .with_drag_active(true)
            .with_drag_source(true);
    assert!(active_source.paints_interaction_fill());

    let active_drop_target =
        InteractiveRowWidget::new(11, WidgetSizing::fixed(Vector2::new(120.0, 22.0)))
            .with_drop_target_mode(true, true);
    assert!(active_drop_target.paints_interaction_fill());
}
