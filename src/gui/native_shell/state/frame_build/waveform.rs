use super::StaticFrameCtx;
use super::*;

pub(super) fn render_waveform_static(
    state: &NativeShellState,
    ctx: &StaticFrameCtx<'_>,
    primitives: &mut impl PrimitiveSink,
    text_runs: &mut impl TextRunSink,
    motion_model: Option<&NativeMotionModel>,
) {
    let waveform_inner = ctx.layout.waveform_plot;
    emit_waveform_bpm_grid(primitives, waveform_inner, ctx.model, ctx.style);
    push_waveform_image(
        primitives,
        waveform_inner,
        ctx.model.waveform.waveform_image.as_deref(),
    );
    let owned_motion_model;
    let motion_model = if let Some(motion_model) = motion_model {
        motion_model
    } else {
        owned_motion_model = NativeMotionModel::from_app_model(ctx.model);
        &owned_motion_model
    };
    let waveform_toolbar_buttons = waveform_toolbar_buttons(
        ctx.layout,
        ctx.style,
        motion_model,
        state.waveform_bpm_input_active,
        state.waveform_bpm_input_display.as_deref(),
    );
    let waveform_toolbar_left = waveform_toolbar_left_edge(
        &waveform_toolbar_buttons,
        ctx.layout.waveform_header.max.x - ctx.sizing.text_inset_x,
    );
    push_waveform_header_overlay(
        primitives,
        text_runs,
        ctx.layout,
        ctx.style,
        motion_model,
        Some(waveform_toolbar_left - ctx.sizing.action_button_gap),
    );
    render_waveform_toolbar_buttons(
        primitives,
        text_runs,
        ctx.style,
        ctx.sizing,
        &waveform_toolbar_buttons,
        state.hovered_waveform_toolbar_hint,
        state.waveform_toolbar_flash.map(|flash| flash.hint),
        ctx.motion_wave,
        state.waveform_bpm_editor_visual.is_some(),
    );
}
