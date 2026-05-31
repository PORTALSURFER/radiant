use super::*;
use crate::gui::types::Vector2;

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
