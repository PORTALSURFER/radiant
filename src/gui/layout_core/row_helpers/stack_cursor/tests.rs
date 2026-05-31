use super::StackedLayoutCursor;

#[test]
fn stacked_layout_cursor_accumulates_extents_and_gaps() {
    let mut cursor = StackedLayoutCursor::new();

    cursor.advance(20.0, 7.0);
    cursor.advance(18.0, 3.0);

    assert_eq!(cursor.offset(), 48.0);
}

#[test]
fn stacked_layout_cursor_ignores_invalid_negative_segments() {
    let mut cursor = StackedLayoutCursor::new();

    cursor.advance(-12.0, f32::INFINITY);
    cursor.advance(10.0, f32::NAN);

    assert_eq!(cursor.offset(), 10.0);
}

#[test]
fn stacked_layout_cursor_can_continue_from_existing_offset() {
    let mut cursor = StackedLayoutCursor::from_offset(12.0);

    cursor.advance(5.0, 2.0);

    assert_eq!(cursor.offset(), 19.0);
    assert_eq!(StackedLayoutCursor::from_offset(f32::NAN).offset(), 0.0);
}

#[test]
fn stacked_layout_cursor_supports_chainable_optional_rows() {
    let cursor = StackedLayoutCursor::new()
        .advanced(20.0, 7.0)
        .advanced_if(false, 999.0, 999.0)
        .advanced_if(true, 18.0, 3.0);

    assert_eq!(cursor.offset(), 48.0);
}
