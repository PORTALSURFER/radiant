//! Core static-frame and state-overlay builders extracted from native shell state.

use super::*;

mod browser;
mod chrome;
mod map;
mod overlay;
mod status_bar;
mod waveform;

use self::{browser::*, chrome::*, map::*, overlay::*, status_bar::*, waveform::*};

struct StaticFrameCtx<'a> {
    layout: &'a ShellLayout,
    style: &'a StyleTokens,
    model: &'a AppModel,
    sizing: SizingTokens,
    motion_wave: f32,
}

struct BrowserFrameData {
    buttons: Vec<ActionButton>,
    column_chips: Vec<BrowserColumnChip>,
    rows: Vec<CachedBrowserRow>,
}

struct SidebarFrameData {
    source_row_rects: Vec<Rect>,
    upper_folder_rows: Vec<CachedFolderRow>,
    lower_folder_rows: Vec<CachedFolderRow>,
}

impl NativeShellState {
    pub(super) fn build_frame_with_style_into_with_motion_sinks(
        &mut self,
        layout: &ShellLayout,
        style: &StyleTokens,
        model: &AppModel,
        primitives: &mut impl PrimitiveSink,
        text_runs: &mut impl TextRunSink,
        pulse_phase: f32,
        include_overlays: bool,
        motion_model: Option<&NativeMotionModel>,
        static_segment_filter: Option<StaticFrameSegment>,
    ) {
        let sizing = style.sizing;
        let motion_wave = interaction_wave(pulse_phase);
        let ctx = StaticFrameCtx {
            layout,
            style,
            model,
            sizing,
            motion_wave,
        };
        let build_global_static =
            static_segment_matches(static_segment_filter, StaticFrameSegment::GlobalStatic);
        let build_waveform_overlay =
            static_segment_matches(static_segment_filter, StaticFrameSegment::WaveformOverlay);
        let build_browser_rows_window =
            static_segment_matches(static_segment_filter, StaticFrameSegment::BrowserRowsWindow);
        let build_map_panel =
            static_segment_matches(static_segment_filter, StaticFrameSegment::MapPanel);
        let build_browser_frame =
            static_segment_matches(static_segment_filter, StaticFrameSegment::BrowserFrame);
        let build_status_bar =
            static_segment_matches(static_segment_filter, StaticFrameSegment::StatusBar);
        let build_browser_rows_or_map = build_browser_rows_window || build_map_panel;

        render_static_shell_surfaces(&ctx, primitives);

        if build_waveform_overlay {
            render_waveform_static(self, &ctx, primitives, text_runs, motion_model);
        }

        let browser_toolbar = browser_toolbar_layout(layout, style);
        let browser_buttons = browser_action_buttons(layout, style, model, &browser_toolbar);
        let browser_frame_data = BrowserFrameData {
            column_chips: browser_column_chips(layout, style, model, &browser_buttons),
            buttons: browser_buttons,
            rows: if build_browser_rows_or_map {
                rendered_browser_rows(layout, model, style)
            } else {
                Vec::new()
            },
        };
        let sidebar_data = SidebarFrameData {
            source_row_rects: if build_global_static {
                rendered_source_row_rects(layout, style, model)
            } else {
                Vec::new()
            },
            upper_folder_rows: if build_global_static {
                self.cached_folder_rows(layout, style, model, crate::app::FolderPaneIdModel::Upper)
                    .to_vec()
            } else {
                Vec::new()
            },
            lower_folder_rows: if build_global_static {
                self.cached_folder_rows(layout, style, model, crate::app::FolderPaneIdModel::Lower)
                    .to_vec()
            } else {
                Vec::new()
            },
        };
        if build_browser_rows_or_map {
            if model.map.active && build_map_panel {
                render_map_panel(&ctx, primitives);
            } else if !model.map.active && build_browser_rows_window {
                render_browser_rows_window(&ctx, primitives, text_runs, &browser_frame_data.rows);
            }
        }

        render_shell_borders(&ctx, primitives);

        if build_global_static {
            render_top_bar_controls(self, &ctx, primitives, text_runs);
        }
        if build_browser_frame {
            render_browser_frame(self, &ctx, primitives, text_runs, &browser_frame_data);
        }
        if build_global_static {
            render_sidebar(self, &ctx, primitives, text_runs, &sidebar_data);
        }
        // Waveform summary text is produced during overlay rendering so it can
        // update while transport advances without invalidating the static scene.
        if model.map.active && build_map_panel {
            render_map_header(&ctx, text_runs);
        } else if build_browser_frame {
            render_browser_table_header(&ctx, primitives, text_runs);
        }
        if build_browser_frame {
            render_browser_footer(&ctx, text_runs);
        }

        if build_status_bar {
            render_status_bar(self, layout, style, model, primitives, text_runs);
        }

        if include_overlays {
            render_modal_overlays(primitives, text_runs, layout, style, model);
        }
    }

    /// Build only state-driven overlays into reusable buffers.
    pub(crate) fn build_state_overlay_into(
        &mut self,
        layout: &ShellLayout,
        style: &StyleTokens,
        model: &AppModel,
        frame: &mut NativeViewFrame,
    ) {
        frame.primitives.clear();
        frame.text_runs.clear();
        let primitives = &mut frame.primitives;
        let text_runs = &mut frame.text_runs;
        render_state_overlay(self, layout, style, model, primitives, text_runs);

        frame.clear_color = style.clear_color;
    }
}
