use super::StaticFrameCtx;
use super::*;

mod panel;
mod rows;
mod tabs;

pub(super) fn render_content_frame(
    state: &mut NativeShellState,
    ctx: &StaticFrameCtx<'_>,
    primitives: &mut impl PrimitiveSink,
    text_runs: &mut impl TextRunSink,
) {
    panel::render_content_frame(state, ctx, primitives, text_runs);
}

pub(super) fn render_content_rows_window(
    state: &mut NativeShellState,
    ctx: &StaticFrameCtx<'_>,
    primitives: &mut impl PrimitiveSink,
    text_runs: &mut impl TextRunSink,
) {
    let content_rows = state.cached_content_rows(ctx.layout, ctx.style, ctx.model);
    rows::render_content_rows_window(ctx, primitives, text_runs, content_rows);
}

pub(super) fn render_content_footer(
    state: &mut NativeShellState,
    ctx: &StaticFrameCtx<'_>,
    text_runs: &mut impl TextRunSink,
) {
    tabs::render_content_footer(state, ctx, text_runs);
}

pub(super) fn render_content_table_header(
    ctx: &StaticFrameCtx<'_>,
    primitives: &mut impl PrimitiveSink,
    text_runs: &mut impl TextRunSink,
) {
    tabs::render_content_table_header(ctx, primitives, text_runs);
}

pub(super) fn render_content_tabs(
    primitives: &mut impl PrimitiveSink,
    text_runs: &mut impl TextRunSink,
    ctx: &StaticFrameCtx<'_>,
    animated: bool,
    cached_text: &ContentSegmentTextCacheValue,
) {
    tabs::render_content_tabs(primitives, text_runs, ctx, animated, cached_text);
}
