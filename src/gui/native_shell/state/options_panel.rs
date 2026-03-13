//! Top-right options button and options-panel helpers for the native shell.

use super::*;

#[derive(Clone, Debug, PartialEq)]
pub(super) struct OptionsPanelLayout {
    pub(super) panel_rect: Rect,
    pub(super) title_rect: Rect,
    pub(super) buttons: Vec<ActionButton>,
}

pub(super) fn status_options_button_rect(segment: Rect, sizing: SizingTokens) -> Option<Rect> {
    if segment.width() <= 1.0 || segment.height() <= 1.0 {
        return None;
    }
    let inset_x = sizing.text_inset_x.max(3.0);
    let inset_y = sizing.text_inset_y.max(2.0);
    let side = (segment.height() - (inset_y * 2.0))
        .floor()
        .clamp(12.0, 20.0);
    if side <= 0.0 || segment.width() <= side + inset_x {
        return None;
    }
    let min_x = (segment.max.x - inset_x - side).max(segment.min.x);
    let min_y = (segment.min.y + ((segment.height() - side) * 0.5)).max(segment.min.y);
    let max_x = (min_x + side).min(segment.max.x);
    let max_y = (min_y + side).min(segment.max.y);
    (max_x > min_x && max_y > min_y).then_some(Rect::from_min_max(
        Point::new(min_x, min_y),
        Point::new(max_x, max_y),
    ))
}

pub(super) fn status_right_text_rect(
    segment: Rect,
    sizing: SizingTokens,
    button_rect: Option<Rect>,
) -> Rect {
    let text_segment = if let Some(button_rect) = button_rect {
        let max_x = (button_rect.min.x - sizing.text_inset_x.max(3.0)).max(segment.min.x);
        Rect::from_min_max(segment.min, Point::new(max_x, segment.max.y))
    } else {
        segment
    };
    compute_status_text_line_rect(text_segment, sizing, sizing.font_status)
}

pub(super) fn options_panel_layout(
    layout: &ShellLayout,
    style: &StyleTokens,
    model: &AppModel,
) -> Option<OptionsPanelLayout> {
    if !model.options_panel.visible {
        return None;
    }
    let sizing = style.sizing;
    let panel_padding = sizing.overlay_padding.max(10.0);
    let title_height = sizing.overlay_button_height.max(22.0);
    let button_height = sizing.overlay_button_height.max(22.0);
    let button_gap = sizing.action_button_gap.max(4.0);
    let button_width = 236.0_f32.min((layout.content.width() - panel_padding * 2.0).max(160.0));
    let panel_width = button_width + (panel_padding * 2.0);
    let definitions = options_panel_button_defs(model);
    let panel_height = panel_padding
        + title_height
        + button_gap
        + (button_height * definitions.len() as f32)
        + (button_gap * definitions.len().saturating_sub(1) as f32)
        + panel_padding;
    let inset = sizing.panel_inset.max(6.0);
    let max_x = layout.top_bar.max.x - inset;
    let min_x = (max_x - panel_width).max(layout.content.min.x + inset);
    let min_y = layout.top_bar.max.y + inset;
    let max_y = (min_y + panel_height).min(layout.status_bar.min.y - inset);
    let min_y = (max_y - panel_height).max(layout.top_bar.max.y + inset);
    let panel_rect = Rect::from_min_max(
        Point::new(min_x, min_y),
        Point::new(min_x + panel_width, max_y),
    );
    let title_rect = Rect::from_min_max(
        Point::new(
            panel_rect.min.x + panel_padding,
            panel_rect.min.y + panel_padding,
        ),
        Point::new(
            panel_rect.max.x - panel_padding,
            panel_rect.min.y + panel_padding + title_height,
        ),
    );
    let button_x = panel_rect.min.x + panel_padding;
    let mut button_y = title_rect.max.y + button_gap;
    let mut buttons = Vec::with_capacity(definitions.len());
    for (label, action) in definitions {
        let rect = Rect::from_min_max(
            Point::new(button_x, button_y),
            Point::new(button_x + button_width, button_y + button_height),
        );
        buttons.push(ActionButton {
            rect,
            label,
            enabled: true,
            active: false,
            action,
            text_color: style.text_primary,
        });
        button_y += button_height + button_gap;
    }
    Some(OptionsPanelLayout {
        panel_rect,
        title_rect,
        buttons,
    })
}

pub(super) fn options_panel_contains_point(
    layout: &ShellLayout,
    style: &StyleTokens,
    model: &AppModel,
    point: Point,
) -> bool {
    options_panel_layout(layout, style, model).is_some_and(|panel| panel.panel_rect.contains(point))
}

pub(super) fn options_panel_action_at_point(
    layout: &ShellLayout,
    style: &StyleTokens,
    model: &AppModel,
    point: Point,
) -> Option<UiAction> {
    let panel = options_panel_layout(layout, style, model)?;
    panel
        .buttons
        .into_iter()
        .find(|button| button.rect.contains(point))
        .map(|button| button.action)
}

pub(super) fn render_status_options_button(
    primitives: &mut impl PrimitiveSink,
    style: &StyleTokens,
    sizing: SizingTokens,
    button_rect: Rect,
    hovered: bool,
    flashed: bool,
    motion_wave: f32,
) {
    let fill = status_options_button_fill(style, hovered, flashed, motion_wave);
    let border = status_options_button_border(style, hovered, flashed, motion_wave);
    let icon_color = status_options_button_icon_color(style, hovered, flashed, motion_wave);
    emit_primitive(
        primitives,
        Primitive::Rect(FillRect {
            rect: button_rect,
            color: fill,
        }),
    );
    push_border(primitives, button_rect, border, sizing.border_width);
    let icon_rect = inset_rect(
        button_rect,
        sizing.text_inset_x.max(3.0),
        sizing.text_inset_y.max(2.0),
    );
    let _ = emit_toolbar_svg_icon(primitives, WaveformToolbarIcon::Cog, icon_rect, icon_color);
}

pub(super) fn render_options_panel(
    primitives: &mut impl PrimitiveSink,
    text_runs: &mut impl TextRunSink,
    layout: &ShellLayout,
    style: &StyleTokens,
    model: &AppModel,
) {
    let Some(panel) = options_panel_layout(layout, style, model) else {
        return;
    };
    let sizing = style.sizing;
    emit_primitive(
        primitives,
        Primitive::Rect(FillRect {
            rect: panel.panel_rect,
            color: style.surface_overlay,
        }),
    );
    push_border(
        primitives,
        panel.panel_rect,
        blend_color(style.border_emphasis, style.highlight_orange, 0.42),
        sizing.border_width,
    );
    emit_text(
        text_runs,
        TextRun {
            text: String::from("Options"),
            position: panel.title_rect.min,
            font_size: sizing.font_title,
            color: style.text_primary,
            max_width: Some(panel.title_rect.width().max(36.0)),
            align: TextAlign::Left,
        },
    );
    for button in &panel.buttons {
        let label_rect = compute_action_button_text_rect(button.rect, sizing);
        emit_primitive(
            primitives,
            Primitive::Rect(FillRect {
                rect: button.rect,
                color: style.surface_base,
            }),
        );
        push_border(
            primitives,
            button.rect,
            blend_color(style.border_emphasis, style.text_primary, 0.18),
            sizing.border_width,
        );
        emit_text(
            text_runs,
            TextRun {
                text: options_panel_button_text(button.label, model),
                position: label_rect.min,
                font_size: sizing.font_meta,
                color: if button.label == "YOLO Edits" {
                    style.accent_warning
                } else {
                    button.text_color
                },
                max_width: Some(label_rect.width().max(12.0)),
                align: TextAlign::Left,
            },
        );
    }
}

fn options_panel_button_defs(model: &AppModel) -> [(&'static str, UiAction); 7] {
    [
        (
            "Input Monitor",
            UiAction::SetInputMonitoringEnabled {
                enabled: !model.options_panel.input_monitoring_enabled,
            },
        ),
        (
            "Advance After Rating",
            UiAction::SetAdvanceAfterRatingEnabled {
                enabled: !model.options_panel.advance_after_rating_enabled,
            },
        ),
        (
            "YOLO Edits",
            UiAction::SetDestructiveYoloMode {
                enabled: !model.options_panel.destructive_yolo_mode_enabled,
            },
        ),
        (
            "Invert Scroll",
            UiAction::SetInvertWaveformScroll {
                enabled: !model.options_panel.invert_waveform_scroll_enabled,
            },
        ),
        ("Set Trash Folder", UiAction::PickTrashFolder),
        ("Open Trash Folder", UiAction::OpenTrashFolder),
        ("Close", UiAction::CloseOptionsPanel),
    ]
}

fn options_panel_button_text(label: &str, model: &AppModel) -> String {
    match label {
        "Input Monitor" => on_off_text(label, model.options_panel.input_monitoring_enabled),
        "Advance After Rating" => {
            on_off_text(label, model.options_panel.advance_after_rating_enabled)
        }
        "YOLO Edits" => on_off_text(label, model.options_panel.destructive_yolo_mode_enabled),
        "Invert Scroll" => on_off_text(label, model.options_panel.invert_waveform_scroll_enabled),
        "Set Trash Folder" => format!(
            "Trash Folder: {}",
            model
                .options_panel
                .trash_folder_label
                .as_deref()
                .unwrap_or("Not set")
        ),
        _ => String::from(label),
    }
}

fn on_off_text(label: &str, enabled: bool) -> String {
    format!("{label}: {}", if enabled { "On" } else { "Off" })
}

fn status_options_button_fill(
    style: &StyleTokens,
    hovered: bool,
    flashed: bool,
    motion_wave: f32,
) -> Rgba8 {
    let idle = style.surface_overlay;
    let hover = translucent_overlay_color(
        idle,
        style.highlight_orange_soft,
        0.2 + (motion_wave * 0.04),
    );
    let flash = blend_color(hover, style.text_primary, 0.18);
    if flashed {
        flash
    } else if hovered {
        hover
    } else {
        idle
    }
}

fn status_options_button_border(
    style: &StyleTokens,
    hovered: bool,
    flashed: bool,
    motion_wave: f32,
) -> Rgba8 {
    let idle = style.border;
    let hover = blend_color(
        style.border_emphasis,
        style.highlight_orange,
        0.42 + (motion_wave * 0.06),
    );
    let flash = blend_color(hover, style.text_primary, 0.18);
    if flashed {
        flash
    } else if hovered {
        hover
    } else {
        idle
    }
}

fn status_options_button_icon_color(
    style: &StyleTokens,
    hovered: bool,
    flashed: bool,
    motion_wave: f32,
) -> Rgba8 {
    let idle = style.text_muted;
    let hover = blend_color(
        style.text_primary,
        style.highlight_orange,
        0.5 + (motion_wave * 0.08),
    );
    if flashed || hovered { hover } else { idle }
}

fn inset_rect(rect: Rect, inset_x: f32, inset_y: f32) -> Rect {
    let min_x = (rect.min.x + inset_x).min(rect.max.x);
    let max_x = (rect.max.x - inset_x).max(min_x);
    let min_y = (rect.min.y + inset_y).min(rect.max.y);
    let max_y = (rect.max.y - inset_y).max(min_y);
    Rect::from_min_max(Point::new(min_x, min_y), Point::new(max_x, max_y))
}
