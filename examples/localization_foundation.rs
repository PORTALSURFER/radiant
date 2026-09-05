//! Stateful locale-switch example using the explicit application environment source.

use radiant::prelude::*;
use radiant::{
    application::{
        ApplicationEnvironment, LocaleId, TextCatalog, TextKey, TextScale, WritingDirection,
    },
    gui::{
        input::KeyCode,
        shortcuts::{
            ShortcutDisplaySpec, ShortcutGesture, ShortcutKeyLabel, ShortcutPlatform,
            ShortcutPresenter,
        },
    },
};
use std::sync::Arc;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Message {
    ToggleLocale,
    Save,
}

#[derive(Default)]
struct State {
    locale_index: usize,
    saves: usize,
}

fn environment(state: &State) -> ApplicationEnvironment {
    let french = LocaleId::new("fr").expect("example locale is valid");
    let arabic = LocaleId::new("ar").expect("example locale is valid");
    let key = TextKey::new("save", "Save");
    let catalog = TextCatalog::default()
        .insert(french.clone(), key.clone(), "Enregistrer")
        .insert(arabic.clone(), key, "حفظ");
    ApplicationEnvironment::new(match state.locale_index {
        1 => french,
        2 => arabic,
        _ => LocaleId::english(),
    })
    .with_writing_direction(if state.locale_index == 2 {
        WritingDirection::Rtl
    } else {
        WritingDirection::Ltr
    })
    .with_text_scale(TextScale::new(if state.locale_index == 0 { 1.0 } else { 1.5 }).unwrap())
    .with_catalog(Arc::new(catalog))
    .with_shortcut_platform(ShortcutPlatform::Mac)
}

fn view(state: &State) -> radiant::application::ViewNode<Message> {
    let environment = environment(state);
    let localized = environment.localized(&TextKey::new("save", "Save"));
    let label = localized.to_content();
    let shortcut =
        ShortcutPresenter::new(environment.shortcut_platform()).present(&ShortcutDisplaySpec::new(
            ShortcutGesture::with_command(KeyCode::S),
            ShortcutKeyLabel::character('S'),
        ));
    column([
        toolbar([button(label.clone()).message(Message::Save).id(10)]),
        message_menu(
            "Actions",
            [MenuCommand::new(label.clone(), Message::Save)
                .hotkey_hint(shortcut.compact_text().to_owned())],
        ),
        text(format!("{} — {}", label.as_str(), shortcut.spoken_text())),
        text(format!("Save actions: {}", state.saves)),
        button("Next locale: English / French / Arabic")
            .message(Message::ToggleLocale)
            .primary(),
    ])
    .padding(16.0)
    .spacing(8.0)
}

fn update(state: &mut State, message: Message) {
    match message {
        Message::ToggleLocale => state.locale_index = (state.locale_index + 1) % 3,
        Message::Save => state.saves += 1,
    }
}

fn main() -> radiant::Result {
    radiant::app(State::default())
        .title("Radiant Localization Foundation")
        .size(640, 400)
        .view(view)
        .application_environment(environment)
        .update(update)
        .run()
}

#[cfg(test)]
mod tests {
    use super::*;
    use radiant::runtime::{PaintPrimitive, SurfaceRuntime};

    #[test]
    fn live_locale_switch_updates_menu_toolbar_help_and_semantic_names_together() {
        let mut runtime = SurfaceRuntime::new(
            radiant::app(State::default())
                .view(view)
                .application_environment(environment)
                .update(update)
                .into_bridge(),
            Vector2::new(640.0, 400.0),
        );
        for label in ["Save", "Enregistrer", "حفظ", "Save"] {
            let plan = runtime.paint_plan(&Default::default());
            let commands: Vec<_> = plan
                .primitives
                .iter()
                .filter_map(|primitive| match primitive {
                    PaintPrimitive::Text(run) if run.text.as_str() == label => Some(run),
                    _ => None,
                })
                .collect();
            assert_eq!(commands.len(), 2);
            for run in commands {
                assert_eq!(
                    runtime
                        .surface()
                        .find_widget(run.widget_id)
                        .unwrap()
                        .widget()
                        .automation_semantics()
                        .label
                        .as_deref(),
                    Some(label),
                );
            }
            assert!(plan.primitives.iter().any(|primitive| {
                matches!(primitive, PaintPrimitive::Text(run) if run.text.as_str().starts_with(&format!("{label} — ")))
            }));
            runtime.dispatch_message(Message::ToggleLocale);
        }
        runtime.dispatch_message(Message::Save);
        assert!(
            runtime
                .paint_plan(&Default::default())
                .first_text_run("Save actions: 1")
                .is_some()
        );
    }
}
