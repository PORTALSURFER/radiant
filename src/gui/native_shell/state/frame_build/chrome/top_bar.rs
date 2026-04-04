use super::*;
use crate::app::AudioEngineChipStateModel;

pub(super) fn render_top_bar_controls(
    state: &NativeShellState,
    ctx: &StaticFrameCtx<'_>,
    primitives: &mut impl PrimitiveSink,
    text_runs: &mut impl TextRunSink,
) {
    let top_controls = top_bar_controls_layout(ctx.layout, ctx.sizing);
    let chip_error = ctx.model.audio_engine.chip_state == AudioEngineChipStateModel::Error;
    let chip_label = ctx.model.audio_engine.chip_label.as_str();
    if top_controls.active {
        let top_controls_text = compute_top_bar_controls_text_layout(
            top_controls.options_label,
            top_controls.volume_value,
            top_controls.volume_label,
            ctx.sizing,
        );
        emit_primitive(
            primitives,
            Primitive::Rect(FillRect {
                rect: top_controls.volume_meter,
                color: ctx.style.surface_overlay,
            }),
        );
        push_border(
            primitives,
            top_controls.volume_meter,
            ctx.style.border_emphasis,
            ctx.sizing.border_width,
        );
        let volume_level = ctx.model.volume.clamp(0.0, 1.0);
        let fill_width = (top_controls.volume_meter.width() * volume_level)
            .clamp(1.0, top_controls.volume_meter.width());
        emit_primitive(
            primitives,
            Primitive::Rect(FillRect {
                rect: Rect::from_min_max(
                    top_controls.volume_meter.min,
                    Point::new(
                        top_controls.volume_meter.min.x + fill_width,
                        top_controls.volume_meter.max.y,
                    ),
                ),
                color: blend_color(ctx.style.accent_mint, ctx.style.text_primary, 0.28),
            }),
        );
        emit_text(
            text_runs,
            TextRun {
                text: format!("{volume_level:.2}"),
                position: top_controls_text.volume_value.min,
                font_size: ctx.sizing.font_meta,
                color: ctx.style.text_muted,
                max_width: Some(top_controls_text.volume_value.width().max(20.0)),
                align: TextAlign::Right,
            },
        );
        emit_text(
            text_runs,
            TextRun {
                text: String::from("Vol"),
                position: top_controls_text.volume_label.min,
                font_size: ctx.sizing.font_meta,
                color: ctx.style.text_muted,
                max_width: Some(top_controls_text.volume_label.width().max(18.0)),
                align: TextAlign::Left,
            },
        );
    }
    if let Some(button_rect) =
        status_options_button_rect(ctx.layout.top_bar_action_cluster, ctx.sizing)
    {
        render_status_options_button(
            primitives,
            ctx.style,
            ctx.sizing,
            button_rect,
            chip_label,
            chip_error,
            state.hovered_status_options_button,
            state.status_options_button_flash_ticks > 0,
            ctx.motion_wave,
        );
        render_status_options_button_label(
            text_runs,
            ctx.style,
            ctx.sizing,
            button_rect,
            chip_label,
            chip_error,
            state.hovered_status_options_button,
            state.status_options_button_flash_ticks > 0,
            ctx.motion_wave,
        );
    }
}
