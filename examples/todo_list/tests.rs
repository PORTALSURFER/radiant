use crate::model::{
    TODO_LIST_LEFT, TODO_LIST_TOP, TodoDragState, TodoListRow, TodoState, drag_handle_id,
};
use crate::view::project_surface;

#[test]
fn add_draft_inserts_new_items_at_top() {
    let mut state = TodoState::default();
    state.draft = String::from("Ship keyboard support");

    state.add_draft();

    assert_eq!(state.items[0].title, "Ship keyboard support");
    assert_eq!(state.items[0].id, 8);
    assert_eq!(state.items[1].title, "Add a reusable example");
    assert!(state.draft.is_empty());
}

#[test]
fn projected_rows_do_not_include_drop_target_gap_while_dragging() {
    let mut state = TodoState::default();
    state.drag = Some(TodoDragState {
        item_id: 2,
        pointer_x: TODO_LIST_LEFT,
        pointer_y: TODO_LIST_TOP,
        drop_index: 3,
        title: String::from("Wire text input and buttons"),
    });

    let rows = state.projected_rows();

    assert_eq!(rows.len(), state.items.len() - 1);
    assert!(rows.iter().all(|row| match row {
        TodoListRow::Item(item) => item.id != 2,
    }));
}

#[test]
fn drag_projection_keeps_active_handle_for_pointer_capture() {
    use radiant::prelude::IntoView;

    let mut state = TodoState::default();
    state.drag = Some(TodoDragState {
        item_id: 2,
        pointer_x: TODO_LIST_LEFT,
        pointer_y: TODO_LIST_TOP,
        drop_index: 3,
        title: String::from("Wire text input and buttons"),
    });

    let surface = project_surface(&mut state).into_surface();

    assert!(surface.find_widget(drag_handle_id(2)).is_some());
}
