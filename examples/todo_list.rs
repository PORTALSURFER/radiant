//! Standalone todo-list app built on the beginner-facing Radiant API.

use radiant::{
    layout::Vector2,
    prelude::{self as beginner, IntoView},
    runtime::SurfaceNode,
    widgets::WidgetSizing,
};

const INPUT_ID: u64 = 100;
const ADD_BUTTON_ID: u64 = 101;
const ROOT_ID: u64 = 1;
const HEADER_ROW_ID: u64 = 2;
const INPUT_ROW_ID: u64 = 3;
const LIST_COLUMN_ID: u64 = 4;
const SUMMARY_ID: u64 = 5;
const TITLE_ID: u64 = 6;
const LIST_CONTENT_COLUMN_ID: u64 = 7;
const FIRST_ITEM_ROW_ID: u64 = 1_000;
const FIRST_ITEM_TOGGLE_ID: u64 = 2_000;
const FIRST_ITEM_DELETE_ID: u64 = 3_000;

#[derive(Clone, Debug, PartialEq, Eq)]
enum TodoMessage {
    DraftChanged(String),
    AddDraft,
    SetDone { id: u64, done: bool },
    Delete { id: u64 },
}

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
            draft: String::from("Review public API"),
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

fn main() -> radiant::Result {
    radiant::app(TodoState::default())
        .title("Radiant Todo List")
        .size(560, 360)
        .min_size(420, 260)
        .view(project_surface)
        .update(|state: &mut TodoState, message| match message {
            TodoMessage::DraftChanged(value) => state.draft = value,
            TodoMessage::AddDraft => {
                let title = state.draft.trim();
                if !title.is_empty() {
                    state.items.push(TodoItem {
                        id: state.next_id,
                        title: title.to_owned(),
                        done: false,
                    });
                    state.next_id += 1;
                    state.draft.clear();
                }
            }
            TodoMessage::SetDone { id, done } => {
                if let Some(item) = state.items.iter_mut().find(|item| item.id == id) {
                    item.done = done;
                }
            }
            TodoMessage::Delete { id } => state.items.retain(|item| item.id != id),
        })
        .run()
}

fn project_surface(state: &mut TodoState) -> beginner::ViewNode<TodoMessage> {
    beginner::column([header_row(state), input_row(state), todo_list(state)])
        .id(ROOT_ID)
        .spacing(10.0)
}

fn header_row(state: &TodoState) -> beginner::ViewNode<TodoMessage> {
    let complete = state.items.iter().filter(|item| item.done).count();
    let total = state.items.len();
    beginner::row([
        beginner::text("Todos")
            .id(TITLE_ID)
            .sizing(WidgetSizing::fixed(Vector2::new(140.0, 28.0)).with_baseline(20.0)),
        beginner::text(format!("{complete}/{total} done"))
            .id(SUMMARY_ID)
            .sizing(WidgetSizing::fixed(Vector2::new(120.0, 28.0)).with_baseline(20.0)),
    ])
    .id(HEADER_ROW_ID)
    .spacing(12.0)
}

fn input_row(state: &TodoState) -> beginner::ViewNode<TodoMessage> {
    beginner::row([
        beginner::text_input(state.draft.clone(), TodoMessage::DraftChanged)
            .id(INPUT_ID)
            .sizing(WidgetSizing::new(
                Vector2::new(260.0, 32.0),
                Vector2::new(420.0, 32.0),
            )),
        beginner::button("Add", TodoMessage::AddDraft)
            .id(ADD_BUTTON_ID)
            .sizing(WidgetSizing::fixed(Vector2::new(80.0, 32.0))),
    ])
    .id(INPUT_ROW_ID)
    .spacing(8.0)
}

fn todo_list(state: &TodoState) -> beginner::ViewNode<TodoMessage> {
    let rows = state
        .items
        .iter()
        .enumerate()
        .map(|(index, item)| todo_row(index as u64, item))
        .collect::<Vec<_>>();

    beginner::ViewNode::from(SurfaceNode::scroll_area(
        LIST_COLUMN_ID,
        beginner::column(rows)
            .id(LIST_CONTENT_COLUMN_ID)
            .spacing(6.0)
            .into_node(),
    ))
}

fn todo_row(index: u64, item: &TodoItem) -> beginner::ViewNode<TodoMessage> {
    let id = item.id;
    let label = if item.done {
        format!("Done: {}", item.title)
    } else {
        item.title.clone()
    };
    beginner::row([
        beginner::toggle(label, item.done, move |done| TodoMessage::SetDone {
            id,
            done,
        })
        .id(FIRST_ITEM_TOGGLE_ID + index)
        .sizing(WidgetSizing::new(
            Vector2::new(260.0, 30.0),
            Vector2::new(420.0, 30.0),
        )),
        beginner::button("Delete", TodoMessage::Delete { id })
            .id(FIRST_ITEM_DELETE_ID + index)
            .sizing(WidgetSizing::fixed(Vector2::new(84.0, 30.0))),
    ])
    .id(FIRST_ITEM_ROW_ID + index)
    .spacing(8.0)
}
