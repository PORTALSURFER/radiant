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
