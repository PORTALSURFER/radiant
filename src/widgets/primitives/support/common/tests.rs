use super::*;
use crate::layout::Vector2;

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
