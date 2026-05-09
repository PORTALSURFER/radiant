//! Keyboard focus commands and focusable form controls.

use radiant::prelude::*;

const NAME_ID: u64 = 10;
const SEARCH_ID: u64 = 11;

#[derive(Clone)]
enum FocusMessage {
    FocusName,
    FocusSearch,
    Edited,
}

#[derive(Default)]
struct FocusState {
    name: String,
    search: String,
    status: String,
}

fn main() -> radiant::Result {
    radiant::app(FocusState {
        name: String::from("Radiant"),
        search: String::new(),
        status: String::from("Choose a focus target"),
    })
    .title("Radiant Focus Controls")
    .size(560, 240)
    .min_size(420, 180)
    .view(project_surface)
    .shortcuts(|_, _, press, _| {
        if press == KeyPress::with_command(KeyCode::F) {
            ShortcutResolution::action(FocusMessage::FocusSearch)
        } else if press == KeyPress::with_command(KeyCode::N) {
            ShortcutResolution::action(FocusMessage::FocusName)
        } else {
            ShortcutResolution::unhandled()
        }
    })
    .update_with(update)
    .run()
}

fn project_surface(state: &mut FocusState) -> View<FocusMessage> {
    column([
        text("Focus Controls").height(30.0).fill_width(),
        row([
            button("Focus name")
                .message(FocusMessage::FocusName)
                .min_size(110.0, 32.0),
            button("Focus search")
                .message(FocusMessage::FocusSearch)
                .min_size(120.0, 32.0),
        ])
        .fill_width()
        .spacing(10.0),
        row([
            text("Name").size(80.0, 34.0),
            text_input(state.name.clone())
                .message(|_| FocusMessage::Edited)
                .id(NAME_ID)
                .fill_width(),
        ])
        .fill_width()
        .spacing(10.0),
        row([
            text("Search").size(80.0, 34.0),
            text_input(state.search.clone())
                .message(|_| FocusMessage::Edited)
                .id(SEARCH_ID)
                .fill_width(),
        ])
        .fill_width()
        .spacing(10.0),
        text(state.status.clone()).height(28.0).fill_width(),
    ])
    .padding(16.0)
    .spacing(12.0)
}

fn update(
    state: &mut FocusState,
    message: FocusMessage,
    context: &mut UpdateContext<FocusMessage>,
) {
    match message {
        FocusMessage::FocusName => {
            state.status = String::from("Name field requested focus");
            context.focus(NAME_ID);
            context.request_repaint();
        }
        FocusMessage::FocusSearch => {
            state.status = String::from("Search field requested focus");
            context.focus(SEARCH_ID);
            context.request_repaint();
        }
        FocusMessage::Edited => {
            state.status = String::from("Focused input edited");
            context.request_repaint();
        }
    }
}
