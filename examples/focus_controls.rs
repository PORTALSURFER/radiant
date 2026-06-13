//! Keyboard focus commands and focusable form controls.

use radiant::prelude::*;

const NAME_ID: u64 = 10;
const SEARCH_ID: u64 = 11;

#[derive(Clone)]
enum FocusMessage {
    FocusName,
    FocusSearch,
    NameChanged(String),
    SearchChanged(String),
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
    .handle_message(update)
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
                .message(FocusMessage::NameChanged)
                .id(NAME_ID)
                .fill_width(),
        ])
        .fill_width()
        .spacing(10.0),
        row([
            text("Search").size(80.0, 34.0),
            text_input(state.search.clone())
                .message(FocusMessage::SearchChanged)
                .id(SEARCH_ID)
                .fill_width(),
        ])
        .fill_width()
        .spacing(10.0),
        text(state.status.clone()).id(12).height(28.0).fill_width(),
    ])
    .padding(16.0)
    .spacing(12.0)
}

fn update(
    state: &mut FocusState,
    message: FocusMessage,
    context: &mut UiUpdateContext<FocusMessage>,
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
        FocusMessage::NameChanged(value) => {
            state.name = value;
            state.status = String::from("Name field edited");
            context.request_repaint();
        }
        FocusMessage::SearchChanged(value) => {
            state.search = value;
            state.status = String::from("Search field edited");
            context.request_repaint();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use radiant::{
        layout::Vector2,
        runtime::{Event, SurfaceRuntime},
        widgets::{TextInputWidget, TextWidget, Widget},
    };

    #[test]
    fn focus_controls_text_inputs_keep_typed_edits_after_programmatic_focus() {
        let bridge = radiant::app(FocusState {
            name: String::from("Radiant"),
            search: String::new(),
            status: String::from("Choose a focus target"),
        })
        .view(project_surface)
        .handle_message(update)
        .into_bridge();
        let mut runtime = SurfaceRuntime::new(bridge, Vector2::new(560.0, 240.0));

        runtime.dispatch_message(FocusMessage::FocusSearch);
        assert_eq!(runtime.focused_widget(), Some(SEARCH_ID));
        assert_eq!(
            runtime.dispatch_event(Event::Character('x')),
            Some(SEARCH_ID)
        );

        let search = widget_ref::<TextInputWidget, _>(runtime.surface(), SEARCH_ID, "search input");
        assert_eq!(search.state.value, "x");
        assert_eq!(
            widget_ref::<TextWidget, _>(runtime.surface(), 12, "status").text,
            "Search field edited"
        );

        runtime.dispatch_message(FocusMessage::FocusName);
        assert_eq!(runtime.focused_widget(), Some(NAME_ID));
        assert_eq!(runtime.dispatch_event(Event::Character('!')), Some(NAME_ID));

        let name = widget_ref::<TextInputWidget, _>(runtime.surface(), NAME_ID, "name input");
        assert_eq!(name.state.value, "Radiant!");
    }

    fn widget_ref<'a, WidgetType, Message>(
        surface: &'a radiant::runtime::UiSurface<Message>,
        widget_id: u64,
        label: &str,
    ) -> &'a WidgetType
    where
        WidgetType: Widget + 'static,
    {
        surface
            .find_widget(widget_id)
            .and_then(|widget| widget.widget_object().as_any().downcast_ref::<WidgetType>())
            .unwrap_or_else(|| panic!("{label} should be projected"))
    }
}
