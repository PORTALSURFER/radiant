use super::*;

#[test]
fn file_columns_start_with_default_visible_set() {
    let state = test_state();

    let visible = state
        .visible_file_columns()
        .into_iter()
        .map(|column| column.id.as_str())
        .collect::<Vec<_>>();

    assert_eq!(visible, ["name", "size", "kind", "modified"]);
}

#[test]
fn toggling_file_column_updates_visible_columns_and_keeps_name_locked() {
    let mut state = test_state();

    state.toggle_file_column(String::from("extension"));
    state.toggle_file_column(String::from("name"));

    assert!(
        state
            .visible_file_columns()
            .iter()
            .any(|column| column.id == "extension")
    );
    assert!(
        state
            .visible_file_columns()
            .iter()
            .any(|column| column.id == "name")
    );
    assert_eq!(state.status, "Name column stays visible");
}

#[test]
fn file_column_resize_clamps_width() {
    let mut state = test_state();

    state.resize_file_column(
        String::from("kind"),
        ui::DragHandleMessage::started(radiant::layout::Point::new(100.0, 0.0)),
    );
    state.resize_file_column(
        String::from("kind"),
        ui::DragHandleMessage::Moved {
            position: radiant::layout::Point::new(-200.0, 0.0),
        },
    );

    let width = state
        .columns
        .file_columns
        .iter()
        .find(|column| column.id == "kind")
        .map(|column| column.width)
        .unwrap();
    assert_eq!(width, MIN_FILE_COLUMN_WIDTH);
}
