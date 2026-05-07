//! Standalone todo-list app built with Radiant application builders.

use radiant::prelude as ui;

#[derive(Clone, Debug, PartialEq, Eq)]
struct TodoItem {
    id: u64,
    title: String,
    done: bool,
}

struct TodoState {
    next_id: u64,
    draft: String,
    items: Vec<TodoItem>,
    drag: Option<TodoDragState>,
}

#[derive(Clone, Debug, PartialEq)]
struct TodoDragState {
    item_id: u64,
    pointer_x: f32,
    pointer_y: f32,
    drop_index: usize,
    title: String,
}

const TODO_ROW_STEP: f32 = 52.0;
const TODO_LIST_TOP: f32 = 132.0;
const TODO_LIST_LEFT: f32 = 26.0;
const TODO_LIST_WIDTH: f32 = 638.0;

impl Default for TodoState {
    fn default() -> Self {
        Self {
            next_id: 8,
            draft: String::new(),
            drag: None,
            items: vec![
                TodoItem {
                    id: 1,
                    title: String::from("Add a reusable example"),
                    done: true,
                },
                TodoItem {
                    id: 2,
                    title: String::from("Wire text input and buttons"),
                    done: false,
                },
                TodoItem {
                    id: 3,
                    title: String::from("Keep state host-owned"),
                    done: false,
                },
                TodoItem {
                    id: 4,
                    title: String::from("Review public API"),
                    done: false,
                },
                TodoItem {
                    id: 5,
                    title: String::from("Add keyboard shortcuts"),
                    done: false,
                },
                TodoItem {
                    id: 6,
                    title: String::from("Polish animations and transitions"),
                    done: false,
                },
                TodoItem {
                    id: 7,
                    title: String::from("Write documentation"),
                    done: false,
                },
            ],
        }
    }
}

impl TodoState {
    fn add_draft(&mut self) {
        let title = state_title(&self.draft);
        if title.is_empty() {
            return;
        }
        self.items.push(TodoItem {
            id: self.next_id,
            title,
            done: false,
        });
        self.next_id += 1;
        self.draft.clear();
    }

    fn set_done(&mut self, id: u64, done: bool) {
        if let Some(item) = self.items.iter_mut().find(|item| item.id == id) {
            item.done = done;
        }
    }

    fn delete(&mut self, id: u64) {
        self.items.retain(|item| item.id != id);
        if self.drag.as_ref().is_some_and(|drag| drag.item_id == id) {
            self.drag = None;
        }
    }

    fn drag_item(&mut self, id: u64, message: ui::DragHandleMessage) {
        match message {
            ui::DragHandleMessage::Started { position } => {
                if let Some(index) = self.index_of(id) {
                    let title = self.items[index].title.clone();
                    self.drag = Some(TodoDragState {
                        item_id: id,
                        pointer_x: position.x,
                        pointer_y: position.y,
                        drop_index: index,
                        title,
                    });
                }
            }
            ui::DragHandleMessage::Moved { position } => self.update_drag(position.x, position.y),
            ui::DragHandleMessage::Ended { position } => {
                self.update_drag(position.x, position.y);
                self.commit_drag();
            }
        }
    }

    fn update_drag(&mut self, x: f32, y: f32) {
        let Some(mut drag) = self.drag.take() else {
            return;
        };
        drag.pointer_x = x;
        drag.pointer_y = y;
        let candidate = ((y - TODO_LIST_TOP + TODO_ROW_STEP * 0.5) / TODO_ROW_STEP).floor();
        let visible_count = self.items.len().saturating_sub(1);
        drag.drop_index = (candidate as isize).clamp(0, visible_count as isize) as usize;
        self.drag = Some(drag);
    }

    fn commit_drag(&mut self) {
        let Some(drag) = self.drag.take() else {
            return;
        };
        let Some(current) = self.index_of(drag.item_id) else {
            return;
        };
        let item = self.items.remove(current);
        let target = drag.drop_index.min(self.items.len());
        self.items.insert(target, item);
    }

    fn projected_rows(&self) -> Vec<TodoListRow<'_>> {
        let Some(drag) = self.drag.as_ref() else {
            return self.items.iter().map(TodoListRow::Item).collect();
        };
        let mut rows = Vec::with_capacity(self.items.len());
        let mut visible_index = 0usize;
        for item in &self.items {
            if item.id == drag.item_id {
                continue;
            }
            if visible_index == drag.drop_index {
                rows.push(TodoListRow::DropTarget(drag.item_id));
            }
            rows.push(TodoListRow::Item(item));
            visible_index += 1;
        }
        if drag.drop_index >= visible_index {
            rows.push(TodoListRow::DropTarget(drag.item_id));
        }
        rows
    }

    fn index_of(&self, id: u64) -> Option<usize> {
        self.items.iter().position(|item| item.id == id)
    }
}

enum TodoListRow<'a> {
    Item(&'a TodoItem),
    DropTarget(u64),
}

fn main() -> radiant::Result {
    radiant::app(TodoState::default())
        .title("Radiant Todo List")
        .size(700, 480)
        .min_size(520, 340)
        .view(project_surface)
        .run()
}

fn project_surface(state: &mut TodoState) -> ui::StateView<TodoState> {
    let page = ui::column([header_row(state), body_section(state)])
        .key("root")
        .subtle()
        .padding(16.0)
        .spacing(2.0);
    if let Some(drag) = state.drag.as_ref() {
        ui::stack([
            page,
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
            .bind(|state: &mut TodoState| &mut state.draft)
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
        TodoListRow::DropTarget(id) => drop_target_row(id),
    }
}

fn todo_item_row(item: &TodoItem) -> ui::StateView<TodoState> {
    let id = item.id;
    ui::list_row(
        item.id,
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
        ],
    )
}

fn passive_todo_item_row(item: &TodoItem) -> ui::StateView<TodoState> {
    let id = item.id;
    ui::row_key(
        item.id,
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
        ],
    )
    .style(ui::WidgetStyle::default())
    .fill_width()
    .height(52.0)
    .padding_x(18.0)
    .padding_y(10.0)
    .spacing(16.0)
}

fn drop_target_row(id: u64) -> ui::StateView<TodoState> {
    ui::row([
        ui::drag_handle()
            .on_drag(move |state: &mut TodoState, message| state.drag_item(id, message))
            .id(drag_handle_id(id))
            .input_only()
            .size(22.0, 22.0),
        ui::text("").fill_width(),
    ])
    .key("drop-target")
    .fill_width()
    .height(TODO_ROW_STEP)
    .padding_x(18.0)
    .padding_y(10.0)
    .style(ui::WidgetStyle {
        tone: ui::WidgetTone::Accent,
        prominence: ui::WidgetProminence::Subtle,
    })
}

fn state_title(draft: &str) -> String {
    draft.trim().to_owned()
}

fn drag_handle_id(item_id: u64) -> u64 {
    50_000 + item_id
}
