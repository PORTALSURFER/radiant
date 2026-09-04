//! Policy-controlled scrolling with a persisted settled offset.

use radiant::layout::{ScrollAxis, ScrollAxisLock, ScrollPolicy, ScrollbarPlacement, Vector2};
use radiant::prelude::*;

#[derive(Clone)]
enum Message {
    OffsetSettled(Vector2),
}

fn main() -> radiant::Result {
    radiant::app(Vector2::new(0.0, 0.0))
        .title("Radiant Controlled Scroll")
        .size(420, 280)
        .view(|offset| {
            scroll(column((0..40).map(|row| {
                text(format!("Scrollable row {row:02}"))
                    .size(360.0, 28.0)
                    .fill_width()
            })))
            .scroll_policy(
                ScrollPolicy::default()
                    .axes(ScrollAxis::Vertical)
                    .axis_lock(ScrollAxisLock::Vertical)
                    .scrollbar_placement(ScrollbarPlacement::Reserved),
            )
            .initial_offset(*offset)
            .on_offset_settled(Message::OffsetSettled)
        })
        .handle_message(|offset, message, context| match message {
            Message::OffsetSettled(value) => {
                *offset = value;
                context.request_repaint();
            }
        })
        .run()
}
