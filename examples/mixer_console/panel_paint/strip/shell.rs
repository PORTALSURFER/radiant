use super::{StripPaintState, style};
use crate::mixer_console::paint::{push_rect, push_stroke, push_text};
use crate::mixer_console::panel::MixerPanelWidget;
use radiant::prelude::*;

pub(super) fn append_shell(
    widget: &MixerPanelWidget,
    primitives: &mut Vec<PaintPrimitive>,
    state: StripPaintState,
    theme: &ThemeTokens,
) {
    push_rect(
        primitives,
        widget.common.id,
        state.rect,
        style::strip_fill(widget, state, theme),
    );
    push_rect(
        primitives,
        widget.common.id,
        Rect::from_min_max(
            Point::new(state.rect.min.x, state.rect.min.y),
            Point::new(state.rect.max.x, state.rect.min.y + 4.0),
        ),
        style::strip_group_color(state, theme),
    );
    push_stroke(primitives, widget.common.id, state.rect, theme.border, 1.0);
    push_text(
        primitives,
        widget.common.id,
        state.channel.label,
        Rect::from_min_size(
            Point::new(state.rect.min.x + 8.0, state.rect.min.y + 10.0),
            Vector2::new(state.rect.width() - 16.0, 22.0),
        ),
        style::strip_label_color(state, theme),
        PaintTextAlign::Center,
    );
}
