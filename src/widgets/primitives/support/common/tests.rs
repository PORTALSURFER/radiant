use super::*;
use crate::layout::Vector2;

#[test]
fn fixed_constructor_sets_identity_and_fixed_sizing() {
    let common = WidgetCommon::fixed(3, 120.0, 40.0);

    assert_eq!(common.id, 3);
    assert_eq!(
        common.sizing,
        WidgetSizing::fixed(Vector2::new(120.0, 40.0))
    );
}

#[test]
fn focus_helpers_expose_pointer_hit_testing_intent() {
    let sizing = WidgetSizing::fixed(Vector2::new(120.0, 40.0));

    assert_eq!(
        WidgetCommon::new(1, sizing).with_pointer_focus().focus,
        FocusBehavior::Pointer
    );
    assert_eq!(
        WidgetCommon::new(2, sizing).with_keyboard_focus().focus,
        FocusBehavior::Keyboard
    );
}

#[test]
fn default_chrome_helper_keeps_focus_contract_but_disables_builtin_paint() {
    let sizing = WidgetSizing::fixed(Vector2::new(120.0, 40.0));
    let common = WidgetCommon::new(1, sizing)
        .with_pointer_focus()
        .without_default_chrome();

    assert_eq!(common.focus, FocusBehavior::Pointer);
    assert!(!common.paint.paints_focus);
    assert!(!common.paint.paints_state_layers);
}

#[test]
fn state_query_helpers_expose_shared_interaction_state() {
    let sizing = WidgetSizing::fixed(Vector2::new(120.0, 40.0));
    let mut common = WidgetCommon::new(1, sizing);

    common.state.hovered = true;
    common.state.pressed = true;
    common.state.focused = true;
    common.state.selected = true;
    common.state.active = true;
    common.state.disabled = true;
    common.state.read_only = true;

    assert!(common.state.is_hovered());
    assert!(common.state.is_pressed());
    assert!(common.state.is_focused());
    assert!(common.state.is_selected());
    assert!(common.state.is_active());
    assert!(common.state.is_disabled());
    assert!(common.state.is_read_only());

    assert!(common.is_hovered());
    assert!(common.is_pressed());
    assert!(common.is_focused());
    assert!(common.is_selected());
    assert!(common.is_active());
    assert!(common.is_disabled());
    assert!(common.is_read_only());
}
