//! Background work through the application builder.

use radiant::prelude::*;
use std::{thread, time::Duration};

#[derive(Clone, Debug, PartialEq, Eq)]
enum LoadingMessage {
    Start,
    Loaded(ResourceLoad<String>),
    Reset,
}

#[derive(Clone, Debug)]
struct LoadingState {
    resource: ResourceSlot<String>,
}

impl Default for LoadingState {
    fn default() -> Self {
        Self {
            resource: ResourceSlot::new("demo-loader"),
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
                text(status_text(&state.resource)).height(24.0).fill_width(),
                text(result_text(&state.resource)).height(28.0).fill_width(),
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
                state.resource.mark_loading();
                context.spawn(
                    "demo-loader",
                    || {
                        thread::sleep(Duration::from_millis(60));
                        ResourceLoad::ready(
                            "demo-loader",
                            "Loaded payload from background work".to_string(),
                        )
                    },
                    LoadingMessage::Loaded,
                );
                context.request_repaint();
            }
            LoadingMessage::Loaded(load) => {
                state.resource.apply(load);
                context.request_repaint();
            }
            LoadingMessage::Reset => {
                state.resource.clear();
                context.request_repaint();
            }
        })
        .run()
}

fn status_text(resource: &ResourceSlot<String>) -> &'static str {
    match resource.state() {
        ResourceLoadState::Idle => "Status: idle",
        ResourceLoadState::Loading => "Status: loading",
        ResourceLoadState::Ready => "Status: ready",
        ResourceLoadState::Failed => "Status: failed",
    }
}

fn result_text(resource: &ResourceSlot<String>) -> String {
    resource
        .value()
        .cloned()
        .or_else(|| resource.error().map(ToOwned::to_owned))
        .unwrap_or_else(|| "Idle".to_string())
}
