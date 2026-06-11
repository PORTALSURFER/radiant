//! Stateful item list with selection-scoped add and remove actions.

use radiant::prelude::*;

#[derive(Clone, Debug, PartialEq, Eq)]
enum ListActionMessage {
    AddAfterSelection,
    AddAfter(u64),
    Select(u64),
    Remove(u64),
}

#[derive(Clone, Debug)]
struct ListItem {
    id: u64,
    title: String,
    detail: String,
}

#[derive(Clone, Debug)]
struct ListActionState {
    items: Vec<ListItem>,
    selected_id: Option<u64>,
    next_id: u64,
}

impl Default for ListActionState {
    fn default() -> Self {
        Self {
            items: vec![
                ListItem {
                    id: 1,
                    title: "Planning".to_string(),
                    detail: "Backlog review".to_string(),
                },
                ListItem {
                    id: 2,
                    title: "Design".to_string(),
                    detail: "Interaction pass".to_string(),
                },
                ListItem {
                    id: 3,
                    title: "Validation".to_string(),
                    detail: "Examples lane".to_string(),
                },
            ],
            selected_id: Some(1),
            next_id: 4,
        }
    }
}

impl ListActionState {
    fn select(&mut self, id: u64) {
        self.selected_id = Some(id);
    }

    fn add_after(&mut self, after_id: Option<u64>) {
        let id = self.next_id;
        self.next_id += 1;
        let item = ListItem {
            id,
            title: format!("New Item {id}"),
            detail: "Describe the item".to_string(),
        };
        let insert_at = after_id
            .and_then(|id| self.items.iter().position(|item| item.id == id))
            .map_or(self.items.len(), |index| index + 1);
        self.items.insert(insert_at, item);
        self.selected_id = Some(id);
    }

    fn remove(&mut self, id: u64) {
        let Some(index) = self.items.iter().position(|item| item.id == id) else {
            return;
        };
        self.items.remove(index);
        self.selected_id = self
            .items
            .get(index)
            .or_else(|| self.items.last())
            .map(|item| item.id);
    }
}

fn main() -> radiant::Result {
    radiant::app(ListActionState::default())
        .title("Radiant List Actions")
        .size(520, 360)
        .min_size(380, 240)
        .view(list_actions_surface)
        .update(update)
        .run()
}

fn list_actions_surface(state: &mut ListActionState) -> View<ListActionMessage> {
    column([
        row([
            text("List Actions").height(30.0).fill_width(),
            button("+")
                .primary()
                .message(ListActionMessage::AddAfterSelection)
                .size(32.0, 32.0),
        ])
        .fill_width()
        .spacing(10.0),
        list(state.items.iter().cloned(), |item| {
            item_row(item, state.selected_id)
        })
        .fill_height(),
        text(selection_summary(state)).height(26.0).fill_width(),
    ])
    .padding(16.0)
    .spacing(12.0)
    .fill()
}

fn item_row(item: ListItem, selected_id: Option<u64>) -> View<ListActionMessage> {
    let id = item.id;
    let selected = selected_id == Some(id);
    list_row_id(
        id,
        [
            selectable(item.title, selected)
                .message(move |_| ListActionMessage::Select(id))
                .fill_width(),
            text(item.detail).height(28.0).fill_width(),
            button("+")
                .subtle()
                .message(ListActionMessage::AddAfter(id))
                .size(32.0, 32.0),
            selected_remove_button(id, selected),
        ],
    )
}

fn selected_remove_button(id: u64, selected: bool) -> View<ListActionMessage> {
    if selected {
        button("-")
            .danger()
            .message(ListActionMessage::Remove(id))
            .size(32.0, 32.0)
    } else {
        text("").size(32.0, 32.0)
    }
}

fn update(state: &mut ListActionState, message: ListActionMessage) {
    match message {
        ListActionMessage::AddAfterSelection => state.add_after(state.selected_id),
        ListActionMessage::AddAfter(id) => state.add_after(Some(id)),
        ListActionMessage::Select(id) => state.select(id),
        ListActionMessage::Remove(id) => state.remove(id),
    }
}

fn selection_summary(state: &ListActionState) -> String {
    match state
        .selected_id
        .and_then(|id| state.items.iter().find(|item| item.id == id))
    {
        Some(item) => format!("Selected: {} ({})", item.title, item.detail),
        None => "No item selected".to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use radiant::prelude::IntoView;

    #[test]
    fn list_actions_adds_after_selection_and_removes_selected_item() {
        let mut state = ListActionState::default();

        state.add_after(state.selected_id);
        assert_eq!(state.items[1].title, "New Item 4");
        assert_eq!(state.selected_id, Some(4));

        state.remove(4);
        assert_eq!(state.items.len(), 3);
        assert_eq!(state.selected_id, Some(2));
    }

    #[test]
    fn list_actions_projects_focusable_row_actions() {
        let mut state = ListActionState::default();
        let surface = list_actions_surface(&mut state).into_surface();

        assert!(surface.keyboard_focus_order().len() >= 8);
    }
}
