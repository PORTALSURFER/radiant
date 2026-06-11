//! Inspector/property-panel application-builder helper.

use radiant::prelude::*;

#[derive(Clone, Debug, PartialEq, Eq)]
enum InspectorMessage {
    SelectProperty(String),
    SetLocked(bool),
}

#[derive(Clone, Debug)]
struct InspectorState {
    selected: String,
    locked: bool,
    note: String,
}

impl Default for InspectorState {
    fn default() -> Self {
        Self {
            selected: "name".to_string(),
            locked: false,
            note: "Selected property updates this note".to_string(),
        }
    }
}

impl InspectorState {
    fn rows(&self) -> Vec<PropertyRow> {
        [
            PropertyRow::new("name", "Name", "Layer 12"),
            PropertyRow::new("kind", "Kind", "Signal track"),
            PropertyRow::new("locked", "Locked", if self.locked { "Yes" } else { "No" }),
            PropertyRow::new("blend", "Blend", "Normal"),
        ]
        .into_iter()
        .map(|row| {
            let selected = row.id == self.selected;
            row.selected(selected)
        })
        .collect()
    }
}

fn main() -> radiant::Result {
    radiant::app(InspectorState::default())
        .title("Radiant Inspector Panel")
        .size(520, 280)
        .min_size(420, 220)
        .view(|state| {
            row([
                inspector_panel_view(state.rows())
                    .width(260.0)
                    .fill_height(),
                column([
                    text("Preview").height(24.0).fill_width(),
                    text(state.note.clone()).fill_width().height(28.0),
                    checkbox(state.locked)
                        .message(InspectorMessage::SetLocked)
                        .key("locked-toggle")
                        .height(28.0),
                ])
                .style(WidgetStyle::default())
                .fill_width()
                .fill_height()
                .padding(10.0)
                .spacing(8.0),
            ])
            .fill_width()
            .fill_height()
            .padding(12.0)
            .spacing(10.0)
        })
        .update(update)
        .run()
}

fn inspector_panel_view(rows: Vec<PropertyRow>) -> View<InspectorMessage> {
    column([
        text("Inspector").height(20.0).fill_width(),
        column(rows.into_iter().map(inspector_row))
            .fill_width()
            .spacing(1.0),
    ])
    .style(WidgetStyle::default())
    .fill_width()
    .padding(6.0)
    .spacing(4.0)
}

fn inspector_row(row_data: PropertyRow) -> View<InspectorMessage> {
    let selected = row_data.selected;
    let row_id = row_data.id.clone();
    let mut view = row([
        text(row_data.label)
            .key(format!("property-{row_id}-label"))
            .size(112.0, 20.0),
        button(row_data.value)
            .message(InspectorMessage::SelectProperty(row_id.clone()))
            .key(format!("property-{row_id}-value"))
            .subtle()
            .fill_width()
            .height(20.0),
    ])
    .key(format!("property-row-{row_id}"))
    .fill_width()
    .height(24.0)
    .padding_x(6.0)
    .padding_y(1.0)
    .spacing(6.0)
    .style(if selected {
        WidgetStyle {
            tone: WidgetTone::Accent,
            prominence: WidgetProminence::Subtle,
        }
    } else {
        WidgetStyle::default()
    })
    .hoverable();
    if selected {
        view = view.primary();
    }
    view
}

fn update(state: &mut InspectorState, message: InspectorMessage) {
    match message {
        InspectorMessage::SelectProperty(id) => {
            state.selected = id.clone();
            state.note = format!("Selected property: {id}");
        }
        InspectorMessage::SetLocked(locked) => {
            state.locked = locked;
            state.note = if locked {
                "Layer locked".to_string()
            } else {
                "Layer unlocked".to_string()
            };
        }
    }
}
