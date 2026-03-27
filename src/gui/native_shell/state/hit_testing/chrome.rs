use super::*;

impl NativeShellState {
    /// Resolve a rendered source-row index for a point within the sidebar.
    pub(crate) fn source_row_at_point(
        &mut self,
        layout: &ShellLayout,
        model: &AppModel,
        point: Point,
    ) -> Option<usize> {
        let style = style_for_layout(layout);
        let source_rows = self.cached_source_row_rects(layout, &style, model);
        compute_row_index_at_point(source_rows, point)
    }

    /// Resolve a rendered folder-row index for a point within the sidebar.
    pub(crate) fn folder_row_at_point(
        &mut self,
        layout: &ShellLayout,
        model: &AppModel,
        point: Point,
    ) -> Option<usize> {
        let style = style_for_layout(layout);
        let folder_rows = self.cached_folder_row_rects(layout, &style, model);
        compute_row_index_at_point(folder_rows, point)
    }

    /// Return the folder-visibility toggle button rect for tests.
    #[cfg(test)]
    pub(crate) fn folder_visibility_toggle_button_rect(
        &self,
        layout: &ShellLayout,
        model: &AppModel,
    ) -> Option<Rect> {
        let style = style_for_layout(layout);
        compute_sidebar_folder_header_layout(
            sidebar_sections(layout, &style, model).folder_header,
            style.sizing,
            model.sources.folder_recovery.in_progress,
            model.sources.folder_recovery.entry_count,
            model.sources.show_all_folders,
            model.sources.can_toggle_show_all_folders,
        )
        .toggle_button
        .map(|button| button.rect)
    }

    /// Return the projected inline folder-edit row index, when present.
    pub(crate) fn folder_create_row_index(&self, model: &AppModel) -> Option<usize> {
        model
            .sources
            .folder_rows
            .iter()
            .position(|row| row.kind == crate::app::FolderRowKind::RenameDraft)
            .or_else(|| {
                model
                    .sources
                    .folder_rows
                    .iter()
                    .position(|row| row.kind == crate::app::FolderRowKind::CreateDraft)
            })
    }

    /// Return the folder-create input field rect for the active inline edit row.
    pub(crate) fn folder_create_input_rect(
        &mut self,
        layout: &ShellLayout,
        model: &AppModel,
    ) -> Option<Rect> {
        let style = style_for_layout(layout);
        let row_index = self.folder_create_row_index(model)?;
        let row = model.sources.folder_rows.get(row_index)?;
        let row_rect = *self
            .cached_folder_row_rects(layout, &style, model)
            .get(row_index)?;
        Some(folder_create_field_rect(row_rect, style.sizing, row.depth))
    }

    /// Return the folder-create input text rect for the active inline edit row.
    pub(crate) fn folder_create_text_rect(
        &mut self,
        layout: &ShellLayout,
        model: &AppModel,
    ) -> Option<Rect> {
        let style = style_for_layout(layout);
        let row_index = self.folder_create_row_index(model)?;
        let row = model.sources.folder_rows.get(row_index)?;
        let row_rect = *self
            .cached_folder_row_rects(layout, &style, model)
            .get(row_index)?;
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
    ) -> Option<usize> {
        if !model.sources.folder_search_query.trim().is_empty() {
            return None;
        }
        let style = style_for_layout(layout);
        let folder_rows = self.cached_folder_row_rects(layout, &style, model);
        let row_index = compute_row_index_at_point(folder_rows, point)?;
        let row = model.sources.folder_rows.get(row_index)?;
        if matches!(
            row.kind,
            crate::app::FolderRowKind::CreateDraft | crate::app::FolderRowKind::RenameDraft
        ) || row.is_root
            || !row.has_children
        {
            return None;
        }
        let row_rect = *folder_rows.get(row_index)?;
        let depth_indent =
            compute_sidebar_folder_row_depth_indent(row_rect, style.sizing, row.depth);
        let disclosure_rect =
            compute_sidebar_folder_row_layout(row_rect, style.sizing, depth_indent).disclosure_rect;
        disclosure_rect.contains(point).then_some(row_index)
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
        let folder_rows = self.cached_folder_row_rects(layout, &style, model);
        let row = model.sources.folder_rows.get(row_index)?;
        let row_rect = *folder_rows.get(row_index)?;
        let depth_indent =
            compute_sidebar_folder_row_depth_indent(row_rect, style.sizing, row.depth);
        Some(
            compute_sidebar_folder_row_layout(row_rect, style.sizing, depth_indent).disclosure_rect,
        )
    }

    /// Resolve one source context-menu action at a pointer location.
    pub(crate) fn source_context_menu_action_at_point(
        &self,
        layout: &ShellLayout,
        model: &AppModel,
        point: Point,
    ) -> Option<UiAction> {
        let style = style_for_layout(layout);
        let (_, buttons) =
            source_context_menu_spec(layout, &style, model, self.source_context_menu)?;
        buttons
            .into_iter()
            .find(|button| button.enabled && button.rect.contains(point))
            .map(|button| button.action.clone())
    }

    /// Return `true` when a point lands inside the visible source context menu panel.
    #[cfg(test)]
    pub(crate) fn source_context_menu_contains_point(
        &self,
        layout: &ShellLayout,
        model: &AppModel,
        point: Point,
    ) -> bool {
        let style = style_for_layout(layout);
        let Some((panel_rect, _)) =
            source_context_menu_spec(layout, &style, model, self.source_context_menu)
        else {
            return false;
        };
        panel_rect.contains(point)
    }

    /// Return rendered source-row rectangles for geometry tests.
    #[cfg(test)]
    pub(crate) fn rendered_source_row_rects(
        &mut self,
        layout: &ShellLayout,
        model: &AppModel,
    ) -> Vec<Rect> {
        let style = style_for_layout(layout);
        self.cached_source_row_rects(layout, &style, model).to_vec()
    }

    /// Return rendered folder-row rectangles for geometry tests.
    #[cfg(test)]
    pub(crate) fn rendered_folder_row_rects(
        &mut self,
        layout: &ShellLayout,
        model: &AppModel,
    ) -> Vec<Rect> {
        let style = style_for_layout(layout);
        self.cached_folder_row_rects(layout, &style, model).to_vec()
    }

    /// Return a source-action button rect for the provided action in tests.
    #[cfg(test)]
    pub(crate) fn source_action_button_rect(
        &self,
        layout: &ShellLayout,
        model: &AppModel,
        action: UiAction,
    ) -> Option<Rect> {
        let style = style_for_layout(layout);
        source_action_buttons(layout, &style, model)
            .into_iter()
            .find(|button| button.action == action)
            .map(|button| button.rect)
    }

    /// Return the sidebar-header add-source button rect in tests.
    #[cfg(test)]
    pub(crate) fn source_add_button_rect(&self, layout: &ShellLayout) -> Option<Rect> {
        source_add_button_rect(layout.sidebar_header, style_for_layout(layout).sizing)
    }

    /// Return the top-right options button rect in tests.
    #[cfg(test)]
    pub(crate) fn status_options_button_rect(&self, layout: &ShellLayout) -> Option<Rect> {
        status_options_button_rect(
            layout.top_bar_action_cluster,
            style_for_layout(layout).sizing,
        )
    }

    /// Return whether a point falls inside the visible options panel.
    #[cfg(test)]
    pub(crate) fn options_panel_contains_point(
        &self,
        layout: &ShellLayout,
        model: &AppModel,
        point: Point,
    ) -> bool {
        options_panel_contains_point(layout, &style_for_layout(layout), model, point)
    }

    /// Return whether a point falls inside the visible options panel.
    pub(crate) fn options_panel_contains_point_live(
        &self,
        layout: &ShellLayout,
        model: &AppModel,
        point: Point,
    ) -> bool {
        options_panel_contains_point(layout, &style_for_layout(layout), model, point)
    }

    /// Resolve a click inside the visible options panel.
    pub(crate) fn options_panel_action_at_point(
        &self,
        layout: &ShellLayout,
        model: &AppModel,
        point: Point,
    ) -> Option<UiAction> {
        options_panel_action_at_point(layout, &style_for_layout(layout), model, point)
    }

    /// Return a source-context-menu button rect for one action in tests.
    #[cfg(test)]
    pub(crate) fn source_context_menu_button_rect(
        &self,
        layout: &ShellLayout,
        model: &AppModel,
        action: UiAction,
    ) -> Option<Rect> {
        let style = style_for_layout(layout);
        let (_, buttons) =
            source_context_menu_spec(layout, &style, model, self.source_context_menu)?;
        buttons
            .into_iter()
            .find(|button| button.action == action)
            .map(|button| button.rect)
    }

    /// Resolve a source-management action button click into a native UI action.
    pub(crate) fn source_action_at_point(
        &mut self,
        layout: &ShellLayout,
        model: &AppModel,
        point: Point,
    ) -> Option<UiAction> {
        let style = style_for_layout(layout);
        if source_add_button_rect(layout.sidebar_header, style.sizing)
            .is_some_and(|rect| rect.contains(point))
        {
            self.trigger_source_add_button_flash();
            return Some(UiAction::OpenAddSourceDialog);
        }
        source_action_buttons(layout, &style, model)
            .into_iter()
            .find(|button| button.enabled && button.rect.contains(point))
            .map(|button| button.action)
    }

    /// Resolve a click inside the folder-header visibility toggle into a UI action.
    pub(crate) fn folder_header_action_at_point(
        &mut self,
        layout: &ShellLayout,
        model: &AppModel,
        point: Point,
    ) -> Option<UiAction> {
        let style = style_for_layout(layout);
        let sections = sidebar_sections(layout, &style, model);
        let toggle = compute_sidebar_folder_header_layout(
            sections.folder_header,
            style.sizing,
            model.sources.folder_recovery.in_progress,
            model.sources.folder_recovery.entry_count,
            model.sources.show_all_folders,
            model.sources.can_toggle_show_all_folders,
        )
        .toggle_button?;
        if !toggle.enabled || !toggle.rect.contains(point) {
            return None;
        }
        Some(UiAction::ToggleShowAllFolders)
    }

    /// Resolve a sidebar background click into a section-focus action.
    pub(crate) fn sidebar_focus_action_at_point(
        &mut self,
        layout: &ShellLayout,
        model: &AppModel,
        point: Point,
    ) -> Option<UiAction> {
        let style = style_for_layout(layout);
        let sections = sidebar_sections(layout, &style, model);
        if sections.source_rows.contains(point) {
            return Some(UiAction::FocusSourcesPanel);
        }
        if sections.folder_header.contains(point) || sections.folder_rows.contains(point) {
            return Some(UiAction::FocusFolderPanel);
        }
        None
    }

    /// Resolve a click inside the status-bar options button to a native options action.
    pub(crate) fn status_options_action_at_point(
        &mut self,
        layout: &ShellLayout,
        model: &AppModel,
        point: Point,
    ) -> Option<UiAction> {
        let Some(button_rect) = status_options_button_rect(
            layout.top_bar_action_cluster,
            style_for_layout(layout).sizing,
        ) else {
            return None;
        };
        if !button_rect.contains(point) {
            return None;
        }
        self.trigger_status_options_button_flash();
        Some(if model.options_panel.visible {
            UiAction::CloseOptionsPanel
        } else {
            UiAction::OpenOptionsMenu
        })
    }

    /// Resolve a click inside the top-bar volume meter to a volume action.
    pub(crate) fn top_bar_volume_action_at_point(
        &self,
        layout: &ShellLayout,
        point: Point,
    ) -> Option<UiAction> {
        let controls = top_bar_controls_layout(layout, style_for_layout(layout).sizing);
        if !controls.active || !controls.volume_meter.contains(point) {
            return None;
        }
        Some(volume_action_for_meter(controls.volume_meter, point))
    }

    /// Resolve a drag point against the top-bar volume meter.
    ///
    /// The x-position is clamped to the meter width so dragging beyond the
    /// edges still emits a stable `SetVolume` action.
    pub(crate) fn top_bar_volume_drag_action(
        &self,
        layout: &ShellLayout,
        point: Point,
    ) -> Option<UiAction> {
        let controls = top_bar_controls_layout(layout, style_for_layout(layout).sizing);
        if !controls.active {
            return None;
        }
        Some(volume_action_for_meter(controls.volume_meter, point))
    }

    /// Resolve a modal confirm prompt button click into confirm/cancel actions.
    pub(crate) fn prompt_action_at_point(
        &self,
        layout: &ShellLayout,
        model: &AppModel,
        point: Point,
    ) -> Option<UiAction> {
        if !model.confirm_prompt.visible {
            return None;
        }
        let style = style_for_layout(layout);
        let (confirm_button, cancel_button) = prompt_buttons(layout, &style);
        if confirm_button.contains(point) {
            if prompt_has_validation_error(model) {
                return None;
            }
            return Some(UiAction::ConfirmPrompt);
        }
        if cancel_button.contains(point) {
            return Some(UiAction::CancelPrompt);
        }
        None
    }

    /// Return whether a point falls inside the active prompt text input rect.
    pub(crate) fn prompt_input_at_point(
        &self,
        layout: &ShellLayout,
        model: &AppModel,
        point: Point,
    ) -> bool {
        if !model.confirm_prompt.visible {
            return false;
        }
        let style = style_for_layout(layout);
        prompt_input_rect(layout, &style, model).is_some_and(|rect| rect.contains(point))
    }

    /// Resolve a progress-overlay cancel click.
    pub(crate) fn progress_action_at_point(
        &self,
        layout: &ShellLayout,
        model: &AppModel,
        point: Point,
    ) -> Option<UiAction> {
        if !model.progress_overlay.visible
            || !model.progress_overlay.modal
            || !model.progress_overlay.cancelable
            || model.progress_overlay.cancel_requested
        {
            return None;
        }
        let style = style_for_layout(layout);
        progress_cancel_button(layout, &style, model.progress_overlay.modal)
            .contains(point)
            .then_some(UiAction::CancelProgress)
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
