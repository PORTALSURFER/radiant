//! Sidebar/source-row geometry helpers shared by the native shell.

use super::*;

pub(in crate::gui::native_shell::state) fn rendered_source_rows(
    style: &StyleTokens,
    model: &AppModel,
) -> usize {
    model.sources.rows.len().min(style.sizing.source_rows_max)
}

pub(in crate::gui::native_shell::state) fn sidebar_rows_cache_key(
    layout: &ShellLayout,
    style: &StyleTokens,
    model: &AppModel,
) -> SidebarRowsCacheKey {
    let sizing = style.sizing;
    SidebarRowsCacheKey {
        root_min_x: f32_to_bits(layout.root.rect.min.x),
        root_min_y: f32_to_bits(layout.root.rect.min.y),
        root_max_x: f32_to_bits(layout.root.rect.max.x),
        root_max_y: f32_to_bits(layout.root.rect.max.y),
        sidebar_rows_min_x: f32_to_bits(layout.sidebar_rows.min.x),
        sidebar_rows_min_y: f32_to_bits(layout.sidebar_rows.min.y),
        sidebar_rows_max_x: f32_to_bits(layout.sidebar_rows.max.x),
        sidebar_rows_max_y: f32_to_bits(layout.sidebar_rows.max.y),
        sidebar_section_gap: f32_to_bits(sizing.sidebar_section_gap),
        panel_section_padding_top: f32_to_bits(sizing.panel_section_padding_top),
        panel_section_padding_bottom: f32_to_bits(sizing.panel_section_padding_bottom),
        source_rows_min_when_split: usize_to_u32(sizing.source_rows_min_when_split),
        folder_rows_min: usize_to_u32(sizing.folder_rows_min),
        source_rows: rendered_source_rows(style, model) as u32,
        folder_rows: rendered_folder_rows(style, model) as u32,
        source_row_height: f32_to_bits(sizing.source_row_height),
        source_row_gap: f32_to_bits(sizing.source_row_gap),
        folder_row_height: f32_to_bits(sizing.folder_row_height),
        folder_row_gap: f32_to_bits(sizing.folder_row_gap),
        folder_header_block_height: f32_to_bits(sizing.folder_header_block_height),
        ui_scale: f32_to_bits(layout.ui_scale),
    }
}

pub(in crate::gui::native_shell::state) fn rendered_folder_rows(
    style: &StyleTokens,
    model: &AppModel,
) -> usize {
    model
        .sources
        .folder_rows
        .len()
        .min(style.sizing.folder_rows_max)
}

pub(in crate::gui::native_shell::state) fn rendered_source_row_rects(
    layout: &ShellLayout,
    style: &StyleTokens,
    model: &AppModel,
) -> Vec<Rect> {
    let sections = sidebar_sections(layout, style, model);
    build_stacked_rows(
        sections.source_rows,
        rendered_source_rows(style, model),
        style.sizing.source_row_gap,
        style.sizing.source_row_height,
    )
}

pub(in crate::gui::native_shell::state) fn rendered_folder_row_rects(
    layout: &ShellLayout,
    style: &StyleTokens,
    model: &AppModel,
) -> Vec<Rect> {
    let sections = sidebar_sections(layout, style, model);
    build_stacked_rows(
        sections.folder_rows,
        rendered_folder_rows(style, model),
        style.sizing.folder_row_gap,
        style.sizing.folder_row_height,
    )
}
