//! Tree-list and sortable details-list primitives.

#[path = "tree_and_details/data.rs"]
mod data;

use data::{detail_rows_for, tree_children};
use radiant::prelude::*;
use std::collections::HashSet;

#[derive(Clone, Debug)]
struct ExampleState {
    selected_tree_item: String,
    selected_row: Option<String>,
    drag_status: String,
    expanded: HashSet<String>,
    sort: DetailsSort,
}

impl Default for ExampleState {
    fn default() -> Self {
        Self {
            selected_tree_item: "design".to_string(),
            selected_row: None,
            drag_status: "Drag handles report row ids".to_string(),
            expanded: ["workspace", "ui"]
                .into_iter()
                .map(str::to_string)
                .collect(),
            sort: DetailsSort::new("name", SortDirection::Ascending),
        }
    }
}

impl ExampleState {
    fn activate_tree_item(&mut self, id: String) {
        let expandable = tree_children(&id).is_some();
        if expandable && !self.expanded.contains(&id) {
            self.expanded.insert(id.clone());
        } else if expandable && self.selected_tree_item == id {
            self.expanded.remove(&id);
        }
        self.selected_tree_item = id;
        self.selected_row = None;
    }

    fn select_row(&mut self, id: String) {
        self.selected_row = Some(id);
    }

    fn record_tree_drag(&mut self, id: String, message: DragHandleMessage) {
        let phase = match message {
            DragHandleMessage::Started { .. } => "started",
            DragHandleMessage::Moved { .. } => "moved",
            DragHandleMessage::Ended { .. } => "ended",
            DragHandleMessage::DoubleActivate { .. } => "double",
            DragHandleMessage::Cancelled { .. } => "cancelled",
        };
        self.drag_status = format!("Drag {phase}: {id}");
    }

    fn sort_by(&mut self, column_id: String) {
        if self.sort.column_id == column_id {
            self.sort.direction = self.sort.direction.toggled();
        } else {
            self.sort = DetailsSort::new(column_id, SortDirection::Ascending);
        }
    }

    fn tree_items(&self) -> Vec<TreeListItem> {
        let mut items = Vec::new();
        push_tree_item(self, &mut items, "workspace", "Workspace", 0);
        items
    }

    fn rows(&self) -> Vec<DetailsRow> {
        let mut rows = detail_rows_for(&self.selected_tree_item);
        rows.sort_by(|a, b| {
            let index = match self.sort.column_id.as_str() {
                "kind" => 1,
                "state" => 2,
                _ => 0,
            };
            let ordering = a.cells[index].cmp(&b.cells[index]);
            self.sort.direction.apply_ordering(ordering)
        });
        rows.into_iter()
            .map(|row| {
                let selected = self.selected_row.as_deref() == Some(row.id.as_str());
                row.selected(selected)
            })
            .collect()
    }
}

fn main() -> radiant::Result {
    radiant::app(ExampleState::default())
        .title("Radiant Tree and Details")
        .size(680, 360)
        .min_size(520, 260)
        .view(example_view)
        .run()
}

fn example_view(state: &mut ExampleState) -> StateView<ExampleState> {
    row([
        column([
            text("Tree").height(22.0).fill_width(),
            tree_list_with_drag(
                state.tree_items(),
                ExampleState::activate_tree_item,
                ExampleState::activate_tree_item,
                None::<fn(&mut ExampleState, String)>,
                Some(ExampleState::record_tree_drag),
            ),
            text(state.drag_status.clone()).height(22.0).fill_width(),
        ])
        .style(WidgetStyle::default())
        .width(230.0)
        .fill_height()
        .padding(8.0)
        .spacing(4.0),
        column([
            text(format!("Details: {}", state.selected_tree_item))
                .height(22.0)
                .fill_width(),
            selectable_sortable_details_list(
                [
                    DetailsColumn::flexible("name", "Name"),
                    DetailsColumn::fixed("kind", "Kind", 120.0),
                    DetailsColumn::fixed("state", "State", 96.0),
                ],
                state.rows(),
                Some(state.sort.clone()),
                ExampleState::sort_by,
                Some(ExampleState::select_row),
            ),
        ])
        .style(WidgetStyle::default())
        .fill_width()
        .fill_height()
        .padding(8.0)
        .spacing(4.0),
    ])
    .fill_width()
    .fill_height()
    .padding(12.0)
    .spacing(10.0)
}

fn push_tree_item(
    state: &ExampleState,
    items: &mut Vec<TreeListItem>,
    id: &'static str,
    label: &'static str,
    depth: usize,
) {
    let mut item = TreeListItem::new(id, depth, label).selected(state.selected_tree_item == id);
    if id != "workspace" {
        item = item.draggable(true);
    }
    if let Some(children) = tree_children(id) {
        item = item.branch(state.expanded.contains(id));
        items.push(item);
        if state.expanded.contains(id) {
            for (child_id, child_label) in children {
                push_tree_item(state, items, child_id, child_label, depth + 1);
            }
        }
    } else {
        items.push(item);
    }
}
