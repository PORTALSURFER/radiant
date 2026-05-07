//! Small form showing text input, toggle, and direct state callbacks.

use radiant::prelude::*;

struct FormState {
    name: String,
    enabled: bool,
    submitted: String,
}

impl Default for FormState {
    fn default() -> Self {
        Self {
            name: String::new(),
            enabled: true,
            submitted: String::from("Waiting"),
        }
    }
}

fn main() -> radiant::Result {
    radiant::app(FormState::default())
        .title("Radiant Form")
        .size(460, 220)
        .min_size(360, 180)
        .view(|state| {
            column([
                row([
                    text("Name").size(72.0, 28.0),
                    text_input(state.name.clone())
                        .bind(|state: &mut FormState| &mut state.name)
                        .fill_width(),
                ])
                .fill_width(),
                row([
                    toggle("Enabled", state.enabled).on_change(|state: &mut FormState, enabled| {
                        state.enabled = enabled;
                        state.submitted = if enabled {
                            String::from("Form enabled")
                        } else {
                            String::from("Form disabled")
                        };
                    }),
                    text(if state.enabled {
                        "Status: edits will submit"
                    } else {
                        "Status: submit is blocked"
                    })
                    .fill_width()
                    .height(28.0),
                ])
                .fill_width()
                .spacing(12.0),
                row([
                    button("Submit")
                        .primary()
                        .on_click(|state: &mut FormState| {
                            state.submitted = if state.enabled {
                                state.name.clone()
                            } else {
                                String::from("Blocked: form disabled")
                            };
                        }),
                    text(format!("Submitted: {}", state.submitted)).fill_width(),
                ])
                .fill_width(),
            ])
            .padding(16.0)
            .spacing(10.0)
        })
        .run()
}
