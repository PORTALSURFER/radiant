//! Small form showing text input, toggle, and direct state callbacks.

use radiant::prelude::*;

#[derive(Default)]
struct FormState {
    name: String,
    enabled: bool,
    submitted: String,
}

fn main() -> radiant::Result {
    radiant::app(FormState::default())
        .title("Radiant Form")
        .size(420, 180)
        .min_size(320, 140)
        .view(|state| {
            column([
                row([
                    text("Name").size(72.0, 28.0),
                    text_input(state.name.clone())
                        .bind(|state: &mut FormState| &mut state.name)
                        .fill_width(),
                ])
                .fill_width(),
                toggle("Enabled", state.enabled)
                    .on_change(|state: &mut FormState, enabled| state.enabled = enabled),
                row([
                    button("Submit")
                        .primary()
                        .on_click(|state: &mut FormState| {
                            state.submitted = if state.enabled {
                                state.name.clone()
                            } else {
                                String::from("Disabled")
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
