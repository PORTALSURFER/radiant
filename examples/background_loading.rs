//! Background work through the application builder.

use radiant::prelude::*;
use std::{thread, time::Duration};

#[derive(Clone, Debug, PartialEq, Eq)]
enum LoadingMessage {
    Start,
    Loaded(String),
    Reset,
}

#[derive(Clone, Debug)]
struct LoadingState {
    loading: bool,
    result: String,
}

impl Default for LoadingState {
    fn default() -> Self {
        Self {
            loading: false,
            result: "Idle".to_string(),
        }
    }
}

fn main() -> radiant::Result {
    radiant::app(LoadingState::default())
        .title("Radiant Background Loading")
        .size(520, 180)
        .min_size(420, 150)
        .view(|state| {
            column([
                text("Background Loading").height(28.0).fill_width(),
                text(if state.loading {
                    "Status: loading"
                } else {
                    "Status: ready"
                })
                .height(24.0)
                .fill_width(),
                text(state.result.clone()).height(28.0).fill_width(),
                row([
                    button("Start")
                        .primary()
                        .message(LoadingMessage::Start)
                        .width(100.0)
                        .height(32.0),
                    button("Reset")
                        .subtle()
                        .message(LoadingMessage::Reset)
                        .width(100.0)
                        .height(32.0),
                ])
                .spacing(10.0)
                .fill_width(),
            ])
            .style(WidgetStyle::default())
            .padding(16.0)
            .spacing(8.0)
        })
        .update_with(|state, message, context| match message {
            LoadingMessage::Start => {
                state.loading = true;
                state.result = "Worker running".to_string();
                context.spawn(
                    "demo-loader",
                    || {
                        thread::sleep(Duration::from_millis(60));
                        "Loaded payload from background work".to_string()
                    },
                    LoadingMessage::Loaded,
                );
                context.request_repaint();
            }
            LoadingMessage::Loaded(result) => {
                state.loading = false;
                state.result = result;
                context.request_repaint();
            }
            LoadingMessage::Reset => {
                state.loading = false;
                state.result = "Idle".to_string();
                context.request_repaint();
            }
        })
        .run()
}
