use super::super::model::{MixerChannel, SEND_COUNT};
use super::super::paint::{
    blend_color, group_color, push_rect, push_stroke, push_text, rgba, send_color, translucent,
};
use super::super::panel::MixerPanelWidget;
use super::fader;
use super::meter;
use radiant::prelude::*;

pub(super) fn append_strip(
    widget: &MixerPanelWidget,
    primitives: &mut Vec<PaintPrimitive>,
    bounds: Rect,
    channel_index: usize,
    theme: &ThemeTokens,
) {
    let channel = widget.channels[channel_index];
    let strip = widget.strip_rect(bounds, channel_index);
    let solo_active = widget.channels.iter().any(|channel| channel.solo);
    let solo_dimmed = channel.is_visually_dimmed_by_solo(solo_active);
    append_strip_shell(
        widget,
        primitives,
        channel_index,
        channel,
        strip,
        solo_dimmed,
        theme,
    );
    meter::append_meter(widget, primitives, channel, strip, solo_dimmed, theme);
    fader::append_fader(widget, primitives, channel_index, strip, solo_dimmed, theme);
    append_sends(widget, primitives, channel_index, strip, solo_dimmed, theme);
    append_channel_buttons(widget, primitives, channel, strip, theme);
    append_strip_footer(
        widget,
        primitives,
        channel_index,
        channel,
        strip,
        solo_dimmed,
        theme,
    );
}

fn append_strip_shell(
    widget: &MixerPanelWidget,
    primitives: &mut Vec<PaintPrimitive>,
    channel_index: usize,
    channel: MixerChannel,
    strip: Rect,
    solo_dimmed: bool,
    theme: &ThemeTokens,
) {
    push_rect(
        primitives,
        widget.common.id,
        strip,
        strip_fill(widget, channel_index, channel, solo_dimmed, theme),
    );
    push_rect(
        primitives,
        widget.common.id,
        Rect::from_min_max(
            Point::new(strip.min.x, strip.min.y),
            Point::new(strip.max.x, strip.min.y + 4.0),
        ),
        strip_group_color(channel, solo_dimmed, theme),
    );
    push_stroke(primitives, widget.common.id, strip, theme.border, 1.0);
    push_text(
        primitives,
        widget.common.id,
        channel.label,
        Rect::from_min_size(
            Point::new(strip.min.x + 8.0, strip.min.y + 10.0),
            Vector2::new(strip.width() - 16.0, 22.0),
        ),
        strip_label_color(solo_dimmed, theme),
        PaintTextAlign::Center,
    );
}

fn append_sends(
    widget: &MixerPanelWidget,
    primitives: &mut Vec<PaintPrimitive>,
    channel_index: usize,
    strip: Rect,
    solo_dimmed: bool,
    theme: &ThemeTokens,
) {
    for send in 0..SEND_COUNT {
        let rect = widget.send_rect(strip, send);
        push_rect(
            primitives,
            widget.common.id,
            rect,
            send_track_color(solo_dimmed, theme),
        );
        let fill = Rect::from_min_max(
            rect.min,
            Point::new(
                rect.min.x + rect.width() * widget.send_display_ratio(channel_index, send),
                rect.max.y,
            ),
        );
        push_rect(
            primitives,
            widget.common.id,
            fill,
            send_fill_color(send, solo_dimmed, theme),
        );
        push_stroke(primitives, widget.common.id, rect, theme.border, 1.0);
    }
}

fn append_channel_buttons(
    widget: &MixerPanelWidget,
    primitives: &mut Vec<PaintPrimitive>,
    channel: MixerChannel,
    strip: Rect,
    theme: &ThemeTokens,
) {
    for (index, label, active, active_color) in [
        (0, "M", channel.muted, theme.accent_danger),
        (1, "S", channel.solo, theme.accent_warning),
        (2, "R", channel.armed, theme.highlight_cyan),
    ] {
        let rect = widget.button_rect(strip, index);
        push_rect(
            primitives,
            widget.common.id,
            rect,
            if active {
                active_color
            } else {
                theme.bg_tertiary
            },
        );
        push_stroke(primitives, widget.common.id, rect, theme.border, 1.0);
        push_text(
            primitives,
            widget.common.id,
            label,
            rect,
            theme.text_primary,
            PaintTextAlign::Center,
        );
    }
}

fn append_strip_footer(
    widget: &MixerPanelWidget,
    primitives: &mut Vec<PaintPrimitive>,
    channel_index: usize,
    channel: MixerChannel,
    strip: Rect,
    solo_dimmed: bool,
    theme: &ThemeTokens,
) {
    push_text(
        primitives,
        widget.common.id,
        format!("{:+.1} dB", widget.fader_display_db(channel_index)),
        Rect::from_min_size(
            Point::new(strip.min.x + 4.0, strip.max.y - 44.0),
            Vector2::new(strip.width() - 12.0, 18.0),
        ),
        footer_gain_color(solo_dimmed, theme),
        PaintTextAlign::Center,
    );
    push_text(
        primitives,
        widget.common.id,
        format!("{:+.0}", channel.pan * 100.0),
        Rect::from_min_size(
            Point::new(strip.min.x + 4.0, strip.max.y - 24.0),
            Vector2::new(strip.width() - 12.0, 18.0),
        ),
        theme.text_muted,
        PaintTextAlign::Center,
    );
}

fn strip_fill(
    widget: &MixerPanelWidget,
    channel_index: usize,
    channel: MixerChannel,
    solo_dimmed: bool,
    theme: &ThemeTokens,
) -> Rgba8 {
    if widget.selection.is_selected(channel_index) {
        blend_color(theme.surface_raised, theme.highlight_blue, 0.20)
    } else if solo_dimmed {
        blend_color(theme.surface_base, theme.bg_primary, 0.42)
    } else {
        blend_color(
            theme.surface_base,
            group_color(channel.group(), theme),
            0.10,
        )
    }
}

fn strip_group_color(channel: MixerChannel, solo_dimmed: bool, theme: &ThemeTokens) -> Rgba8 {
    if solo_dimmed {
        rgba(78, 82, 88, 180)
    } else {
        group_color(channel.group(), theme)
    }
}

fn strip_label_color(solo_dimmed: bool, theme: &ThemeTokens) -> Rgba8 {
    if solo_dimmed {
        theme.text_muted
    } else {
        theme.text_primary
    }
}

fn send_track_color(solo_dimmed: bool, theme: &ThemeTokens) -> Rgba8 {
    if solo_dimmed {
        rgba(24, 26, 29, 255)
    } else {
        theme.bg_tertiary
    }
}

fn send_fill_color(send: usize, solo_dimmed: bool, theme: &ThemeTokens) -> Rgba8 {
    if solo_dimmed {
        rgba(86, 92, 100, 170)
    } else {
        send_color(send, theme)
    }
}

fn footer_gain_color(solo_dimmed: bool, theme: &ThemeTokens) -> Rgba8 {
    if solo_dimmed {
        translucent(theme.text_muted, 150)
    } else {
        theme.text_muted
    }
}
