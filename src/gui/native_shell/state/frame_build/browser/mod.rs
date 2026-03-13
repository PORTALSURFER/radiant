use super::*;
use super::{BrowserFrameData, StaticFrameCtx};

mod panel;
mod rows;
mod tabs;

pub(super) fn render_browser_frame(
    state: &NativeShellState,
    ctx: &StaticFrameCtx<'_>,
    primitives: &mut impl PrimitiveSink,
    text_runs: &mut impl TextRunSink,
    data: &BrowserFrameData,
) {
    panel::render_browser_frame(state, ctx, primitives, text_runs, data);
}

pub(super) fn render_browser_rows_window(
    ctx: &StaticFrameCtx<'_>,
    primitives: &mut impl PrimitiveSink,
    text_runs: &mut impl TextRunSink,
    rows: &[CachedBrowserRow],
) {
    rows::render_browser_rows_window(ctx, primitives, text_runs, rows);
}

pub(super) fn render_browser_footer(ctx: &StaticFrameCtx<'_>, text_runs: &mut impl TextRunSink) {
    tabs::render_browser_footer(ctx, text_runs);
}

pub(super) fn render_browser_table_header(
    ctx: &StaticFrameCtx<'_>,
    primitives: &mut impl PrimitiveSink,
    text_runs: &mut impl TextRunSink,
) {
    tabs::render_browser_table_header(ctx, primitives, text_runs);
}

pub(super) fn render_browser_tabs(
    primitives: &mut impl PrimitiveSink,
    text_runs: &mut impl TextRunSink,
    ctx: &StaticFrameCtx<'_>,
    animated: bool,
) {
    tabs::render_browser_tabs(primitives, text_runs, ctx, animated);
}
