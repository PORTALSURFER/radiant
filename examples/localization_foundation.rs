//! Stateful locale-switch example using the explicit application environment source.

use radiant::prelude::*;
use radiant::{
    application::{ApplicationEnvironment, LocaleId, TextCatalog, TextKey},
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
    french: bool,
}

fn environment(state: &State) -> ApplicationEnvironment {
    let french = LocaleId::new("fr").expect("example locale is valid");
    let key = TextKey::new("save", "Save");
    let catalog = TextCatalog::default().insert(french.clone(), key, "Enregistrer");
    ApplicationEnvironment::new(if state.french {
        french
    } else {
        LocaleId::english()
    })
    .with_catalog(Arc::new(catalog))
    .with_shortcut_platform(ShortcutPlatform::Mac)
}

fn main() -> radiant::Result {
    radiant::app(State::default())
        .title("Radiant Localization Foundation")
        .size(420, 160)
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
                button("Toggle locale")
                    .message(Message::ToggleLocale)
                    .primary(),
            ])
            .padding(16.0)
            .spacing(8.0)
        })
        .application_environment(environment)
        .update(|state, Message::ToggleLocale| state.french = !state.french)
        .run()
}
