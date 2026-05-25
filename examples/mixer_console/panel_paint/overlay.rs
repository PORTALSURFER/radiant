use super::super::model::CHANNEL_COUNT;
use super::super::paint::{push_rect, push_stroke, send_color, translucent};
use super::super::panel::{MeterReadout, MixerPanelWidget};
use super::fader;
use super::meter;
use radiant::prelude::*;

pub(super) fn append_fader_drag_overlay(
    widget: &MixerPanelWidget,
    primitives: &mut Vec<PaintPrimitive>,
    bounds: Rect,
    source_channel: usize,
    theme: &ThemeTokens,
) {
    for channel_index in 0..CHANNEL_COUNT {
        if !should_paint_fader_overlay_for(widget, source_channel, channel_index) {
            continue;
        }
        let strip = widget.strip_rect(bounds, channel_index);
        let fader_rect = widget.fader_rect(strip);
        append_meter_drag_overlay(widget, primitives, channel_index, strip, theme);
        push_rect(
            primitives,
            widget.common.id,
            Rect::from_min_max(
                Point::new(fader_rect.min.x - 2.0, fader_rect.min.y - 12.0),
                Point::new(fader_rect.max.x + 2.0, fader_rect.max.y + 12.0),
            ),
            translucent(theme.surface_base, 245),
        );
        fader::append_fader_track(
            widget,
            primitives,
            fader_rect,
            fader_rect.center().x,
            false,
            theme,
        );
        append_preview_knob(widget, primitives, channel_index, fader_rect, theme);
    }
}

pub(super) fn append_send_drag_overlay(
    widget: &MixerPanelWidget,
    primitives: &mut Vec<PaintPrimitive>,
    bounds: Rect,
    channel_index: usize,
    send: usize,
    theme: &ThemeTokens,
) {
    let strip = widget.strip_rect(bounds, channel_index);
    let rect = widget.send_rect(strip, send);
    push_rect(primitives, widget.common.id, rect, theme.bg_tertiary);
    let fill = Rect::from_min_max(
        rect.min,
        Point::new(
            rect.x_for_ratio(widget.send_display_ratio(channel_index, send)),
            rect.max.y,
        ),
    );
    push_rect(primitives, widget.common.id, fill, send_color(send, theme));
    push_stroke(
        primitives,
        widget.common.id,
        rect,
        theme.border_emphasis,
        1.0,
    );
}

pub(super) fn append_reorder_drag_overlay(
    widget: &MixerPanelWidget,
    primitives: &mut Vec<PaintPrimitive>,
    bounds: Rect,
    source_channel: usize,
    theme: &ThemeTokens,
) {
    let source = widget.strip_rect(bounds, source_channel);
    push_stroke(
        primitives,
        widget.common.id,
        source,
        translucent(theme.text_primary, 135),
        2.0,
    );
    if let Some(insert) = widget.interaction.reorder_insert {
        let line = widget.insertion_line_rect(bounds, insert);
        push_rect(
            primitives,
            widget.common.id,
            line,
            translucent(theme.highlight_cyan, 235),
        );
        push_stroke(
            primitives,
            widget.common.id,
            line,
            theme.border_emphasis,
            1.0,
        );
    }
}

fn should_paint_fader_overlay_for(
    widget: &MixerPanelWidget,
    source_channel: usize,
    channel: usize,
) -> bool {
    if widget.selection.is_selected(source_channel) && widget.selection.selected_indices().len() > 1
    {
        widget.selection.is_selected(channel)
    } else {
        channel == source_channel
    }
}

fn append_meter_drag_overlay(
    widget: &MixerPanelWidget,
    primitives: &mut Vec<PaintPrimitive>,
    channel_index: usize,
    strip: Rect,
    theme: &ThemeTokens,
) {
    let channel = widget.channels[channel_index];
    let readout = MeterReadout {
        meter_db: widget
            .meter_display_db_for_drag(channel_index)
            .unwrap_or(channel.meter.meter_db),
        peak_db: widget
            .peak_display_db_for_drag(channel_index)
            .unwrap_or(channel.meter.peak_db),
    };
    meter::append_meter_values(widget, primitives, channel, strip, false, readout, theme);
}

fn append_preview_knob(
    widget: &MixerPanelWidget,
    primitives: &mut Vec<PaintPrimitive>,
    channel_index: usize,
    fader: Rect,
    theme: &ThemeTokens,
) {
    let knob_y = fader.y_for_ratio_from_bottom(widget.fader_display_ratio(channel_index));
    let knob = Rect::from_min_size(
        Point::new(fader.min.x, knob_y - 8.0),
        Vector2::new(fader.width(), 16.0),
    );
    push_rect(primitives, widget.common.id, knob, theme.highlight_blue);
    push_stroke(
        primitives,
        widget.common.id,
        knob,
        theme.border_emphasis,
        1.0,
    );
}
