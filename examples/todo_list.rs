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
}

impl Default for TodoState {
    fn default() -> Self {
        Self {
            next_id: 4,
            draft: String::new(),
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
    }
}

fn main() -> radiant::Result {
    radiant::app(TodoState::default())
        .title("Radiant Todo List")
        .size(560, 420)
        .min_size(420, 260)
        .view(project_surface)
        .run()
}

fn project_surface(state: &mut TodoState) -> ui::StateView<TodoState> {
    ui::column([header_row(state), input_row(state), todo_list(state)])
        .key("root")
        .padding(16.0)
        .spacing(12.0)
}

fn header_row(state: &TodoState) -> ui::StateView<TodoState> {
    let complete = state.items.iter().filter(|item| item.done).count();
    let total = state.items.len();
    ui::row([
        ui::text("Todos")
            .key("title")
            .size(140.0, 28.0)
            .fill_width()
            .baseline(20.0),
        ui::text(format!("{complete}/{total} done"))
            .key("summary")
            .size(120.0, 28.0)
            .baseline(20.0),
    ])
    .key("header")
    .fill_width()
    .spacing(12.0)
}

fn input_row(state: &TodoState) -> ui::StateView<TodoState> {
    ui::row([
        ui::text_input(state.draft.clone())
            .placeholder("What needs to be done?")
            .bind(|state: &mut TodoState| &mut state.draft)
            .key("draft")
            .min_size(260.0, 46.0)
            .preferred_size(420.0, 46.0)
            .fill_width(),
        ui::button("ADD +")
            .primary()
            .on_click(TodoState::add_draft)
            .key("add")
            .size(136.0, 46.0),
    ])
    .key("input")
    .fill_width()
    .spacing(8.0)
}

fn todo_list(state: &TodoState) -> ui::StateView<TodoState> {
    ui::list(state.items.iter(), todo_row)
        .key("list")
        .spacing(6.0)
}

fn todo_row(item: &TodoItem) -> ui::StateView<TodoState> {
    let id = item.id;
    ui::list_row(
        item.id,
        [
            ui::checkbox(item.done)
                .on_change(move |state: &mut TodoState, done| state.set_done(id, done))
                .key("done")
                .size(22.0, 22.0),
            ui::text(item.title.clone())
                .key("title")
                .min_size(160.0, 24.0)
                .preferred_size(420.0, 24.0)
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

fn state_title(draft: &str) -> String {
    draft.trim().to_owned()
}
