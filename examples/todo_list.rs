//! Standalone todo-list app built on the generic Radiant runtime.

use radiant::{
    layout::Vector2,
    runtime::{
        declarative_command_runtime_bridge, run_native_vello_runtime, Command, NativeRunOptions,
        SurfaceChild, SurfaceNode, UiSurface,
    },
    widgets::WidgetSizing,
};
use std::sync::Arc;

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

fn main() -> Result<(), String> {
    let bridge = declarative_command_runtime_bridge(
        TodoState::default(),
        project_surface,
        |state: &mut TodoState, message| {
            match message {
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
            }
            Command::request_repaint()
        },
    );

    run_native_vello_runtime(
        NativeRunOptions {
            title: String::from("Radiant Todo List"),
            inner_size: Some([560.0, 360.0]),
            min_inner_size: Some([420.0, 260.0]),
            ..NativeRunOptions::default()
        },
        bridge,
    )
}

fn project_surface(state: &mut TodoState) -> Arc<UiSurface<TodoMessage>> {
    Arc::new(UiSurface::new(SurfaceNode::column(
        ROOT_ID,
        10.0,
        vec![
            SurfaceChild::fill(header_row(state)),
            SurfaceChild::fill(input_row(state)),
            SurfaceChild::fill(todo_list(state)),
        ],
    )))
}

fn header_row(state: &TodoState) -> SurfaceNode<TodoMessage> {
    let complete = state.items.iter().filter(|item| item.done).count();
    let total = state.items.len();
    SurfaceNode::row(
        HEADER_ROW_ID,
        12.0,
        vec![
            SurfaceChild::fill(SurfaceNode::text(
                TITLE_ID,
                "Todos",
                WidgetSizing::fixed(Vector2::new(140.0, 28.0)).with_baseline(20.0),
            )),
            SurfaceChild::fill(SurfaceNode::text(
                SUMMARY_ID,
                format!("{complete}/{total} done"),
                WidgetSizing::fixed(Vector2::new(120.0, 28.0)).with_baseline(20.0),
            )),
        ],
    )
}

fn input_row(state: &TodoState) -> SurfaceNode<TodoMessage> {
    SurfaceNode::row(
        INPUT_ROW_ID,
        8.0,
        vec![
            SurfaceChild::fill(SurfaceNode::text_input(
                INPUT_ID,
                state.draft.clone(),
                WidgetSizing::new(Vector2::new(260.0, 32.0), Vector2::new(420.0, 32.0)),
                TodoMessage::DraftChanged,
            )),
            SurfaceChild::fill(SurfaceNode::button(
                ADD_BUTTON_ID,
                "Add",
                WidgetSizing::fixed(Vector2::new(80.0, 32.0)),
                TodoMessage::AddDraft,
            )),
        ],
    )
}

fn todo_list(state: &TodoState) -> SurfaceNode<TodoMessage> {
    let rows = state
        .items
        .iter()
        .enumerate()
        .map(|(index, item)| SurfaceChild::fill(todo_row(index as u64, item)))
        .collect::<Vec<_>>();

    SurfaceNode::scroll_area(
        LIST_COLUMN_ID,
        SurfaceNode::column(LIST_CONTENT_COLUMN_ID, 6.0, rows),
    )
}

fn todo_row(index: u64, item: &TodoItem) -> SurfaceNode<TodoMessage> {
    let id = item.id;
    let label = if item.done {
        format!("Done: {}", item.title)
    } else {
        item.title.clone()
    };
    SurfaceNode::row(
        FIRST_ITEM_ROW_ID + index,
        8.0,
        vec![
            SurfaceChild::fill(SurfaceNode::toggle_with_checked(
                FIRST_ITEM_TOGGLE_ID + index,
                label,
                item.done,
                WidgetSizing::new(Vector2::new(260.0, 30.0), Vector2::new(420.0, 30.0)),
                move |done| TodoMessage::SetDone { id, done },
            )),
            SurfaceChild::fill(SurfaceNode::button(
                FIRST_ITEM_DELETE_ID + index,
                "Delete",
                WidgetSizing::fixed(Vector2::new(84.0, 30.0)),
                TodoMessage::Delete { id },
            )),
        ],
    )
}
