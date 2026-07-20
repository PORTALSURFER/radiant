use super::*;

#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) struct TodoItem {
    pub(super) id: u64,
    pub(super) title: String,
    pub(super) done: bool,
}

pub(super) struct TodoState {
    next_id: u64,
    pub(super) draft: String,
    pub(super) items: Vec<TodoItem>,
    pub(super) drag: Option<TodoDragState>,
}

#[derive(Clone, Debug, PartialEq)]
pub(super) struct TodoDragState {
    pub(super) item_id: u64,
    pub(super) pointer_x: f32,
    pub(super) pointer_y: f32,
    pub(super) drop_index: usize,
    pub(super) title: String,
}

pub(super) const TODO_ROW_STEP: f32 = 52.0;
pub(super) const TODO_LIST_TOP: f32 = 132.0;
pub(super) const TODO_LIST_LEFT: f32 = 26.0;
pub(super) const TODO_LIST_WIDTH: f32 = 638.0;

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
    pub(super) fn add_draft(&mut self) {
        let title = state_title(&self.draft);
        if title.is_empty() {
            return;
        }
        self.items.insert(
            0,
            TodoItem {
                id: self.next_id,
                title,
                done: false,
            },
        );
        self.next_id += 1;
        self.draft.clear();
    }

    pub(super) fn set_done(&mut self, id: u64, done: bool) {
        if let Some(item) = self.items.iter_mut().find(|item| item.id == id) {
            item.done = done;
        }
    }

    pub(super) fn delete(&mut self, id: u64) {
        self.items.retain(|item| item.id != id);
        if self.drag.as_ref().is_some_and(|drag| drag.item_id == id) {
            self.drag = None;
        }
    }

    pub(super) fn drag_item(&mut self, id: u64, message: ui::DragHandleMessage) {
        match message {
            ui::DragHandleMessage::Started { position, .. } => {
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
            ui::DragHandleMessage::Cancelled { .. } => {
                self.drag = None;
            }
            ui::DragHandleMessage::DoubleActivate { .. } => {}
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

    pub(super) fn projected_rows(&self) -> Vec<TodoListRow<'_>> {
        let Some(drag) = self.drag.as_ref() else {
            return self.items.iter().map(TodoListRow::Item).collect();
        };
        self.items
            .iter()
            .filter(|item| item.id != drag.item_id)
            .map(TodoListRow::Item)
            .collect()
    }

    fn index_of(&self, id: u64) -> Option<usize> {
        self.items.iter().position(|item| item.id == id)
    }
}

pub(super) enum TodoListRow<'a> {
    Item(&'a TodoItem),
}

fn state_title(draft: &str) -> String {
    draft.trim().to_owned()
}

pub(super) fn drag_handle_id(item_id: u64) -> u64 {
    50_000 + item_id
}
