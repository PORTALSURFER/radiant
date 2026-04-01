use super::*;
use crate::app::{FolderPaneIdModel, FolderRowKind};

impl NativeShellState {
    /// Resolve a rendered folder-row index for a point within the sidebar.
    pub(crate) fn folder_row_at_point(
        &mut self,
        layout: &ShellLayout,
        model: &AppModel,
        point: Point,
    ) -> Option<(FolderPaneIdModel, usize)> {
        let style = style_for_layout(layout);
        for pane in [FolderPaneIdModel::Upper, FolderPaneIdModel::Lower] {
            if let Some(row_index) = self
                .cached_folder_rows(layout, &style, model, pane)
                .iter()
                .find(|row| row.rect.contains(point))
                .map(|row| row.row_index)
            {
                return Some((pane, row_index));
            }
        }
        None
    }

    /// Return the folder pane whose header or rows band contains the point.
    pub(crate) fn folder_panel_at_point(
        &mut self,
        layout: &ShellLayout,
        model: &AppModel,
        point: Point,
    ) -> Option<FolderPaneIdModel> {
        let style = style_for_layout(layout);
        let sections = sidebar_sections(layout, &style, model);
        [FolderPaneIdModel::Upper, FolderPaneIdModel::Lower]
            .into_iter()
            .find(|pane| {
                let folder_sections = sections.folder_header(*pane);
                folder_sections.contains(point) || sections.folder_rows(*pane).contains(point)
            })
    }

    /// Return whether a point falls within either folder pane.
    pub(crate) fn folder_panel_contains_point(
        &mut self,
        layout: &ShellLayout,
        model: &AppModel,
        point: Point,
    ) -> bool {
        self.folder_panel_at_point(layout, model, point).is_some()
    }

    /// Return the folder-visibility toggle button rect for tests.
    #[cfg(test)]
    pub(crate) fn folder_visibility_toggle_button_rect(
        &self,
        layout: &ShellLayout,
        model: &AppModel,
    ) -> Option<Rect> {
        let style = style_for_layout(layout);
        let pane = model.sources.active_folder_pane;
        let pane_model = model.sources.folder_pane(pane);
        compute_sidebar_folder_header_layout(
            sidebar_sections(layout, &style, model).folder_header(pane),
            style.sizing,
            pane_model.folder_recovery.in_progress,
            pane_model.folder_recovery.entry_count,
            pane_model.show_all_folders,
            pane_model.can_toggle_show_all_folders,
            pane_model.flattened_view,
            pane_model.can_toggle_flattened_view,
        )
        .visibility_toggle_button
        .map(|button| button.rect)
    }

    /// Return the folder-flatten toggle button rect for tests.
    #[cfg(test)]
    pub(crate) fn folder_flatten_toggle_button_rect(
        &self,
        layout: &ShellLayout,
        model: &AppModel,
    ) -> Option<Rect> {
        let style = style_for_layout(layout);
        let pane = model.sources.active_folder_pane;
        let pane_model = model.sources.folder_pane(pane);
        compute_sidebar_folder_header_layout(
            sidebar_sections(layout, &style, model).folder_header(pane),
            style.sizing,
            pane_model.folder_recovery.in_progress,
            pane_model.folder_recovery.entry_count,
            pane_model.show_all_folders,
            pane_model.can_toggle_show_all_folders,
            pane_model.flattened_view,
            pane_model.can_toggle_flattened_view,
        )
        .flatten_toggle_button
        .map(|button| button.rect)
    }

    /// Return the projected inline folder-edit row index in the active pane, when present.
    pub(crate) fn folder_create_row_index(&self, model: &AppModel) -> Option<usize> {
        let pane_model = model.sources.active_folder_pane_model();
        pane_model
            .folder_rows
            .iter()
            .position(|row| row.kind == FolderRowKind::RenameDraft)
            .or_else(|| {
                pane_model
                    .folder_rows
                    .iter()
                    .position(|row| row.kind == FolderRowKind::CreateDraft)
            })
    }

    /// Return the folder-create input field rect for the active inline edit row.
    pub(crate) fn folder_create_input_rect(
        &mut self,
        layout: &ShellLayout,
        model: &AppModel,
    ) -> Option<Rect> {
        let style = style_for_layout(layout);
        let pane = model.sources.active_folder_pane;
        let row_index = self.folder_create_row_index(model)?;
        let row = model.sources.active_folder_pane_model().folder_rows.get(row_index)?;
        let row_rect = self
            .cached_folder_rows(layout, &style, model, pane)
            .iter()
            .find(|rendered_row| rendered_row.row_index == row_index)?
            .rect;
        Some(folder_create_field_rect(row_rect, style.sizing, row.depth))
    }

    /// Return the folder-create input text rect for the active inline edit row.
    pub(crate) fn folder_create_text_rect(
        &mut self,
        layout: &ShellLayout,
        model: &AppModel,
    ) -> Option<Rect> {
        let style = style_for_layout(layout);
        let pane = model.sources.active_folder_pane;
        let row_index = self.folder_create_row_index(model)?;
        let row = model.sources.active_folder_pane_model().folder_rows.get(row_index)?;
        let row_rect = self
            .cached_folder_rows(layout, &style, model, pane)
            .iter()
            .find(|rendered_row| rendered_row.row_index == row_index)?
            .rect;
        let field_rect = folder_create_field_rect(row_rect, style.sizing, row.depth);
        Some(folder_create_text_rect(field_rect, style.sizing))
    }

    /// Return whether a point falls inside the inline folder editor input field.
    pub(crate) fn folder_create_input_at_point(
        &mut self,
        layout: &ShellLayout,
        model: &AppModel,
        point: Point,
    ) -> bool {
        self.folder_create_input_rect(layout, model)
            .is_some_and(|rect| rect.contains(point))
    }

    /// Resolve a rendered folder-row disclosure click target for a point within the sidebar.
    pub(crate) fn folder_row_disclosure_at_point(
        &mut self,
        layout: &ShellLayout,
        model: &AppModel,
        point: Point,
    ) -> Option<(FolderPaneIdModel, usize)> {
        let (pane, row_index) = self.folder_row_at_point(layout, model, point)?;
        let pane_model = model.sources.folder_pane(pane);
        if !pane_model.folder_search_query.trim().is_empty() {
            return None;
        }
        let style = style_for_layout(layout);
        let rendered_row = self
            .cached_folder_rows(layout, &style, model, pane)
            .iter()
            .find(|row| row.row_index == row_index)?;
        let row = pane_model.folder_rows.get(row_index)?;
        if matches!(row.kind, FolderRowKind::CreateDraft | FolderRowKind::RenameDraft)
            || row.is_root
            || !row.has_children
        {
            return None;
        }
        let depth_indent =
            compute_sidebar_folder_row_depth_indent(rendered_row.rect, style.sizing, row.depth);
        let disclosure_rect = compute_sidebar_folder_row_layout(
            rendered_row.rect,
            style.sizing,
            depth_indent,
        )
        .disclosure_rect;
        disclosure_rect.contains(point).then_some((pane, row_index))
    }

    /// Return one rendered folder-row disclosure gutter rect for tests.
    #[cfg(test)]
    pub(crate) fn folder_row_disclosure_rect(
        &mut self,
        layout: &ShellLayout,
        model: &AppModel,
        row_index: usize,
    ) -> Option<Rect> {
        let style = style_for_layout(layout);
        let pane = model.sources.active_folder_pane;
        let row = model.sources.active_folder_pane_model().folder_rows.get(row_index)?;
        let row_rect = self
            .cached_folder_rows(layout, &style, model, pane)
            .iter()
            .find(|rendered_row| rendered_row.row_index == row_index)?
            .rect;
        let depth_indent =
            compute_sidebar_folder_row_depth_indent(row_rect, style.sizing, row.depth);
        Some(
            compute_sidebar_folder_row_layout(row_rect, style.sizing, depth_indent).disclosure_rect,
        )
    }

    /// Return rendered folder-row rectangles for geometry tests.
    #[cfg(test)]
    pub(crate) fn rendered_folder_row_rects(
        &mut self,
        layout: &ShellLayout,
        model: &AppModel,
    ) -> Vec<Rect> {
        let style = style_for_layout(layout);
        let pane = model.sources.active_folder_pane;
        self.cached_folder_rows(layout, &style, model, pane)
            .iter()
            .map(|row| row.rect)
            .collect()
    }

    pub(crate) fn folder_viewport_len(
        &mut self,
        layout: &ShellLayout,
        model: &AppModel,
        pane: FolderPaneIdModel,
    ) -> usize {
        let style = style_for_layout(layout);
        self.cached_folder_rows(layout, &style, model, pane)
            .len()
            .min(model.sources.folder_pane(pane).folder_rows.len())
    }

    pub(crate) fn folder_viewport_start_row(
        &mut self,
        layout: &ShellLayout,
        model: &AppModel,
        pane: FolderPaneIdModel,
    ) -> Option<usize> {
        let style = style_for_layout(layout);
        self.cached_folder_rows(layout, &style, model, pane)
            .first()
            .map(|row| row.row_index)
    }

    pub(crate) fn folder_scrollbar_thumb_offset_at_point(
        &mut self,
        layout: &ShellLayout,
        model: &AppModel,
        point: Point,
    ) -> Option<(FolderPaneIdModel, f32)> {
        for pane in [FolderPaneIdModel::Upper, FolderPaneIdModel::Lower] {
            let (scrollbar, _) = self.cached_folder_scrollbar(layout, model, pane)?;
            let hit_rect = Rect::from_min_max(
                Point::new(
                    scrollbar.track.min.x - FOLDER_SCROLLBAR_THUMB_HIT_SLOP,
                    scrollbar.thumb.min.y - FOLDER_SCROLLBAR_THUMB_HIT_SLOP,
                ),
                Point::new(
                    scrollbar.track.max.x + FOLDER_SCROLLBAR_THUMB_HIT_SLOP,
                    scrollbar.thumb.max.y + FOLDER_SCROLLBAR_THUMB_HIT_SLOP,
                ),
            );
            if hit_rect.contains(point) {
                return Some((
                    pane,
                    (point.y - scrollbar.thumb.min.y).clamp(0.0, scrollbar.thumb.height()),
                ));
            }
        }
        None
    }

    pub(crate) fn folder_scrollbar_view_start_for_drag(
        &mut self,
        layout: &ShellLayout,
        model: &AppModel,
        pane: FolderPaneIdModel,
        pointer_y: f32,
        thumb_pointer_offset_y: f32,
    ) -> Option<usize> {
        let (scrollbar, viewport_len) = self.cached_folder_scrollbar(layout, model, pane)?;
        folder_scrollbar_view_start_for_pointer(
            scrollbar,
            viewport_len,
            model.sources.folder_pane(pane).folder_rows.len(),
            pointer_y,
            thumb_pointer_offset_y,
        )
    }

    pub(crate) fn folder_scrollbar_view_start_at_point(
        &mut self,
        layout: &ShellLayout,
        model: &AppModel,
        point: Point,
    ) -> Option<(FolderPaneIdModel, usize)> {
        for pane in [FolderPaneIdModel::Upper, FolderPaneIdModel::Lower] {
            let (scrollbar, viewport_len) = self.cached_folder_scrollbar(layout, model, pane)?;
            if !scrollbar.track.contains(point) || scrollbar.thumb.contains(point) {
                continue;
            }
            if let Some(view_start) = folder_scrollbar_view_start_for_pointer(
                scrollbar,
                viewport_len,
                model.sources.folder_pane(pane).folder_rows.len(),
                point.y,
                scrollbar.thumb.height() * 0.5,
            ) {
                return Some((pane, view_start));
            }
        }
        None
    }

    pub(crate) fn set_folder_view_start_row(
        &mut self,
        pane: FolderPaneIdModel,
        view_start_row: usize,
    ) -> bool {
        let pane_state = self.folder_pane_runtime_state_mut(pane);
        if pane_state.window_start == view_start_row && !pane_state.autoscroll {
            return false;
        }
        pane_state.window_start = view_start_row;
        pane_state.autoscroll = false;
        pane_state.cache_key = None;
        true
    }

    /// Resolve a click inside any folder-header toggle into a UI action.
    pub(crate) fn folder_header_action_at_point(
        &mut self,
        layout: &ShellLayout,
        model: &AppModel,
        point: Point,
    ) -> Option<UiAction> {
        let style = style_for_layout(layout);
        let sections = sidebar_sections(layout, &style, model);
        for pane in [FolderPaneIdModel::Upper, FolderPaneIdModel::Lower] {
            let pane_model = model.sources.folder_pane(pane);
            let toggle = compute_sidebar_folder_header_layout(
                sections.folder_header(pane),
                style.sizing,
                pane_model.folder_recovery.in_progress,
                pane_model.folder_recovery.entry_count,
                pane_model.show_all_folders,
                pane_model.can_toggle_show_all_folders,
                pane_model.flattened_view,
                pane_model.can_toggle_flattened_view,
            );
            if let Some(button) = toggle.visibility_toggle_button
                && button.enabled
                && button.rect.contains(point)
            {
                return Some(UiAction::ToggleShowAllFolders { pane: Some(pane) });
            }
            if let Some(button) = toggle.flatten_toggle_button
                && button.enabled
                && button.rect.contains(point)
            {
                return Some(UiAction::ToggleFolderFlattenedView { pane: Some(pane) });
            }
        }
        None
    }
}

pub(in crate::gui::native_shell::state) fn folder_create_field_rect(
    row_rect: Rect,
    sizing: SizingTokens,
    depth: usize,
) -> Rect {
    let depth_indent = compute_sidebar_folder_row_depth_indent(row_rect, sizing, depth);
    let label_rect = compute_sidebar_folder_row_layout(row_rect, sizing, depth_indent).label_rect;
    let horizontal_inset = sizing.text_inset_x.max(4.0) * 0.5;
    let vertical_inset = sizing.text_inset_y.max(2.0) * 0.5;
    Rect::from_min_max(
        Point::new(
            (label_rect.min.x - horizontal_inset).max(row_rect.min.x),
            row_rect.min.y + vertical_inset,
        ),
        Point::new(
            row_rect.max.x - horizontal_inset,
            row_rect.max.y - vertical_inset,
        ),
    )
}

pub(in crate::gui::native_shell::state) fn folder_create_text_rect(
    field_rect: Rect,
    sizing: SizingTokens,
) -> Rect {
    compute_action_button_text_rect(field_rect, sizing)
}
