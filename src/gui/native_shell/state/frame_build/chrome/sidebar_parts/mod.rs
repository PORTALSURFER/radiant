use super::*;

mod footer;
mod folders;
mod header;
mod source_rows;

pub(super) fn render_sidebar(
    _state: &NativeShellState,
    ctx: &StaticFrameCtx<'_>,
    primitives: &mut impl PrimitiveSink,
    text_runs: &mut impl TextRunSink,
    data: &SidebarFrameData,
) {
    header::render_sidebar_header(ctx, primitives, text_runs);
    let rendered_sources = source_rows::render_source_rows(ctx, primitives, text_runs, data);
    let rendered_folders = folders::render_folder_section(ctx, primitives, text_runs, data);
    footer::render_sidebar_footer(ctx, primitives, text_runs, rendered_sources, rendered_folders);
}
