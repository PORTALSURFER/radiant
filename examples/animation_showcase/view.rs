use radiant::prelude::*;

use crate::model::{AnimationMessage, AnimationState};

pub(super) fn animation_view(state: &mut AnimationState) -> View<AnimationMessage> {
    column([
        text("Animation Showcase").height(28.0).fill_width(),
        text(state.status()).id(20).height(26.0).fill_width(),
        crate::phase_meter(state.phase, state.running),
        row([
            button(if state.running { "Pause" } else { "Run" })
                .primary()
                .message(AnimationMessage::Toggle)
                .id(40)
                .width(100.0)
                .height(32.0),
            button("Reset")
                .subtle()
                .message(AnimationMessage::Reset)
                .id(41)
                .width(100.0)
                .height(32.0),
        ])
        .spacing(10.0)
        .fill_width(),
    ])
    .style(WidgetStyle::default())
    .padding(16.0)
    .spacing(10.0)
    .fill_width()
    .fill_height()
}
