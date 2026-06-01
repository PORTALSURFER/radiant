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

#[test]
fn drop_target_mode_configures_hover_and_drop_only_states() {
    let inactive = InteractiveRowWidget::new(7, WidgetSizing::fixed(Vector2::new(120.0, 22.0)))
        .with_drop_target(true)
        .with_drop_target_mode(false, true);
    assert!(!inactive.props.droppable);
    assert!(!inactive.props.drag_active);
    assert!(!inactive.props.drop_hover);

    let hover = InteractiveRowWidget::new(8, WidgetSizing::fixed(Vector2::new(120.0, 22.0)))
        .with_drop_target_mode(true, true);
    assert!(hover.props.droppable);
    assert!(hover.props.drag_active);
    assert!(hover.props.drop_hover);

    let drop_only = InteractiveRowWidget::new(9, WidgetSizing::fixed(Vector2::new(120.0, 22.0)))
        .with_drop_target_mode(true, false);
    assert!(drop_only.props.droppable);
    assert!(drop_only.props.drag_active);
    assert!(!drop_only.props.drop_hover);
}
