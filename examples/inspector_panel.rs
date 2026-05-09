//! Inspector/property-panel application-builder helper.

use radiant::prelude::*;

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
    fn select_property(&mut self, id: String) {
        self.selected = id.clone();
        self.note = format!("Selected property: {id}");
    }

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
                selectable_property_panel(
                    "Inspector",
                    state.rows(),
                    Some(InspectorState::select_property),
                )
                .width(260.0)
                .fill_height(),
                column([
                    text("Preview").height(24.0).fill_width(),
                    text(state.note.clone()).fill_width().height(28.0),
                    checkbox(state.locked)
                        .on_change(|state: &mut InspectorState, locked| {
                            state.locked = locked;
                            state.note = if locked {
                                "Layer locked".to_string()
                            } else {
                                "Layer unlocked".to_string()
                            };
                        })
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
        .run()
}
