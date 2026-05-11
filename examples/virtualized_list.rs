//! Large virtualized list using application builders.

use radiant::prelude::*;

const ROW_COUNT: usize = 10_000;

#[derive(Clone, Debug, PartialEq, Eq)]
enum Message {
    Select(usize),
}

#[derive(Default)]
struct DemoState {
    selected: Option<usize>,
}

fn main() -> radiant::Result {
    radiant::app(DemoState::default())
        .title("Radiant Virtualized List")
        .size(420, 420)
        .min_size(320, 260)
        .view(project_surface)
        .update(|state, message| {
            let Message::Select(index) = message;
            state.selected = Some(index);
        })
        .run()
}

fn project_surface(state: &mut DemoState) -> View<Message> {
    let selected = state
        .selected
        .map(|index| format!("Selected: {index:05}"))
        .unwrap_or_else(|| String::from("Select a row"));

    column([
        text(selected).height(28.0).fill_width(),
        virtual_list(
            0..ROW_COUNT,
            |index| {
                let label = if Some(index) == state.selected {
                    format!("Selected row {index:05}")
                } else {
                    format!("Row {index:05}")
                };
                selectable(label, Some(index) == state.selected)
                    .message(move |_| Message::Select(index))
                    .id(index as u64 + 10_000)
                    .fill_width()
                    .height(32.0)
            },
            96.0,
        )
        .fill_height(),
    ])
    .padding(16.0)
    .spacing(10.0)
}
