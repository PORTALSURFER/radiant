//! Stable keys for list rows that can be reordered or removed.

use radiant::prelude::*;

#[derive(Clone)]
struct Item {
    id: u64,
    label: String,
}

struct KeyState {
    items: Vec<Item>,
}

impl Default for KeyState {
    fn default() -> Self {
        Self {
            items: vec![
                Item {
                    id: 1,
                    label: String::from("Alpha"),
                },
                Item {
                    id: 2,
                    label: String::from("Beta"),
                },
                Item {
                    id: 3,
                    label: String::from("Gamma"),
                },
            ],
        }
    }
}

fn main() -> radiant::Result {
    radiant::app(KeyState::default())
        .title("Radiant Keys")
        .size(360, 240)
        .min_size(280, 180)
        .view(|state| {
            column([
                row([
                    text("Stable keys").fill_width().height(32.0),
                    button("Reverse")
                        .primary()
                        .on_click(|state: &mut KeyState| state.items.reverse()),
                ])
                .fill_width(),
                list(state.items.iter(), keyed_row).fill_height(),
            ])
            .padding(16.0)
            .spacing(12.0)
        })
        .run()
}

fn keyed_row(item: &Item) -> StateView<KeyState> {
    let id = item.id;
    list_row(
        id,
        [
            text(item.label.clone()).key("label").fill_width(),
            button("Remove")
                .danger()
                .on_click(move |state: &mut KeyState| {
                    state.items.retain(|item| item.id != id);
                })
                .key("remove"),
        ],
    )
}
