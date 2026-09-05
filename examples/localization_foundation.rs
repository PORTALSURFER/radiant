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
}

#[derive(Default)]
struct State {
    locale_index: usize,
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

fn main() -> radiant::Result {
    radiant::app(State::default())
        .title("Radiant Localization Foundation")
        .size(520, 320)
        .view(|state| {
            let environment = environment(state);
            let key = TextKey::new("save", "Save");
            let localized = environment.localized(&key);
            let shortcut = ShortcutPresenter::new(environment.shortcut_platform()).present(
                &ShortcutDisplaySpec::new(
                    ShortcutGesture::with_command(KeyCode::S),
                    ShortcutKeyLabel::character('S'),
                ),
            );

            column([
                text(localized.to_content()),
                text(format!(
                    "Menu: {} | Help: {}",
                    shortcut.compact_text(),
                    shortcut.spoken_text()
                )),
                message_menu(
                    "Actions",
                    [
                        MenuCommand::new(localized.to_content(), Message::ToggleLocale)
                            .hotkey_hint(shortcut.compact_text().to_owned()),
                    ],
                ),
                button("Next locale: English / French / Arabic")
                    .message(Message::ToggleLocale)
                    .primary(),
            ])
            .padding(16.0)
            .spacing(8.0)
        })
        .application_environment(environment)
        .update(|state, Message::ToggleLocale| state.locale_index = (state.locale_index + 1) % 3)
        .run()
}
