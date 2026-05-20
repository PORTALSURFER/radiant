use super::*;
use crate::model::{
    TODO_LIST_LEFT, TODO_LIST_TOP, TODO_LIST_WIDTH, TODO_ROW_STEP, TodoItem, TodoListRow,
    TodoState, drag_handle_id,
};

pub(super) fn project_surface(state: &mut TodoState) -> ui::StateView<TodoState> {
    let page = ui::column([header_row(state), body_section(state)])
        .key("root")
        .subtle()
        .padding(16.0)
        .spacing(2.0);
    if let Some(drag) = state.drag.as_ref() {
        ui::stack([
            page,
            drag_capture_handle(drag.item_id),
            ui::drop_marker(
                TODO_LIST_LEFT,
                TODO_LIST_TOP + drag.drop_index as f32 * TODO_ROW_STEP - 2.0,
                TODO_LIST_WIDTH,
                3.0,
            )
            .key("drop-marker"),
            ui::overlay_panel(
                drag.title.clone(),
                (drag.pointer_x - 36.0).clamp(8.0, 560.0),
                (drag.pointer_y - 26.0).max(8.0),
                560.0,
                52.0,
            )
            .primary()
            .key("drag-preview"),
        ])
        .key("root-stack")
    } else {
        page
    }
}

fn header_row(state: &TodoState) -> ui::StateView<TodoState> {
    let complete = state.items.iter().filter(|item| item.done).count();
    let total = state.items.len();
    ui::row([
        ui::text("TODOS")
            .key("title")
            .size(180.0, 32.0)
            .fill_width()
            .baseline(23.0),
        ui::text(format!("{complete} / {total} DONE"))
            .key("summary")
            .size(140.0, 32.0)
            .baseline(23.0),
    ])
    .key("header")
    .style(ui::WidgetStyle::default())
    .fill_width()
    .height(52.0)
    .padding_x(20.0)
    .padding_y(10.0)
    .spacing(12.0)
}

fn body_section(state: &mut TodoState) -> ui::StateView<TodoState> {
    ui::column([input_row(state), todo_list(state)])
        .key("body")
        .style(ui::WidgetStyle::default())
        .fill_width()
        .fill_height()
        .padding(10.0)
        .spacing(10.0)
}

fn input_row(state: &TodoState) -> ui::StateView<TodoState> {
    ui::row([
        ui::text_input(state.draft.clone())
            .placeholder("What needs to be done?")
            .bind_submit(
                |state: &mut TodoState| &mut state.draft,
                TodoState::add_draft,
            )
            .key("draft")
            .min_size(300.0, 42.0)
            .preferred_size(480.0, 42.0)
            .fill_width(),
        ui::button("ADD +")
            .primary()
            .on_click(TodoState::add_draft)
            .key("add")
            .size(120.0, 42.0),
    ])
    .key("input")
    .fill_width()
    .spacing(12.0)
}

fn todo_list(state: &TodoState) -> ui::StateView<TodoState> {
    let dragging = state.drag.is_some();
    ui::scroll(
        ui::column(
            state
                .projected_rows()
                .into_iter()
                .map(move |row| todo_row(row, dragging)),
        )
        .spacing(0.0),
    )
    .key("list")
    .style(ui::WidgetStyle::default())
    .fill_height()
}

fn todo_row(row: TodoListRow<'_>, dragging: bool) -> ui::StateView<TodoState> {
    match row {
        TodoListRow::Item(item) if dragging => passive_todo_item_row(item),
        TodoListRow::Item(item) => todo_item_row(item),
    }
}

fn drag_capture_handle(item_id: u64) -> ui::StateView<TodoState> {
    ui::drag_handle()
        .on_drag(move |state: &mut TodoState, message| state.drag_item(item_id, message))
        .id(drag_handle_id(item_id))
        .input_only()
        .size(1.0, 1.0)
}

fn todo_item_row(item: &TodoItem) -> ui::StateView<TodoState> {
    ui::list_row(item.id, todo_row_children(item))
}

fn passive_todo_item_row(item: &TodoItem) -> ui::StateView<TodoState> {
    ui::row_key(item.id, todo_row_children(item))
        .style(ui::WidgetStyle::default())
        .fill_width()
        .height(52.0)
        .padding_x(18.0)
        .padding_y(10.0)
        .spacing(16.0)
}

fn todo_row_children(item: &TodoItem) -> [ui::StateView<TodoState>; 4] {
    let id = item.id;
    [
        ui::drag_handle()
            .on_drag(move |state: &mut TodoState, message| state.drag_item(id, message))
            .id(drag_handle_id(id))
            .size(22.0, 22.0),
        ui::checkbox(item.done)
            .on_change(move |state: &mut TodoState, done| state.set_done(id, done))
            .key("done")
            .size(22.0, 22.0),
        ui::text(item.title.clone())
            .key("title")
            .min_size(180.0, 24.0)
            .preferred_size(480.0, 24.0)
            .fill_width(),
        ui::button("DELETE")
            .danger()
            .subtle()
            .on_click(move |state: &mut TodoState| state.delete(id))
            .key("delete")
            .size(108.0, 32.0),
    ]
}
