use super::*;

impl NativeShellState {
    /// Return a browser column-chip rect for one column index in tests.
    #[cfg(test)]
    pub(crate) fn browser_column_chip_rect(
        &self,
        layout: &ShellLayout,
        model: &AppModel,
        column: usize,
    ) -> Option<Rect> {
        let style = style_for_layout(layout);
        let toolbar = browser_toolbar_layout(layout, &style);
        let buttons = browser_action_buttons(layout, &style, model, &toolbar);
        browser_column_chips(layout, &style, model, &buttons)
            .into_iter()
            .find(|chip| chip.column == column)
            .map(|chip| chip.rect)
    }

    /// Resolve a rendered browser visible-row index for a point in the triage pane.
    pub(crate) fn browser_row_at_point(
        &mut self,
        layout: &ShellLayout,
        model: &AppModel,
        point: Point,
    ) -> Option<usize> {
        if model.map.active {
            return None;
        }
        let style = style_for_layout(layout);
        let rows = self.cached_browser_rows(layout, &style, model);
        row_index_for_visible_rows(rows, point, layout.browser_rows)
            .map(|index| rows[index].visible_row)
    }

    /// Resolve the focused-row similarity button into its native action.
    pub(crate) fn browser_row_similarity_action_at_point(
        &mut self,
        layout: &ShellLayout,
        model: &AppModel,
        point: Point,
    ) -> Option<UiAction> {
        if model.map.active || model.browser.duplicate_cleanup_active {
            return None;
        }
        let style = style_for_layout(layout);
        self.cached_browser_rows(layout, &style, model)
            .iter()
            .find(|row| row.focused)
            .and_then(|row| browser_similarity_button_rect(row.rect, style.sizing))
            .filter(|rect| rect.contains(point))
            .map(|_| UiAction::ToggleFindSimilarFocusedSample)
    }

    /// Resolve one browser context-menu action at a pointer location.
    pub(crate) fn browser_context_menu_action_at_point(
        &self,
        layout: &ShellLayout,
        model: &AppModel,
        point: Point,
    ) -> Option<UiAction> {
        let style = style_for_layout(layout);
        let (_, buttons) =
            browser_context_menu_spec(layout, &style, model, self.browser_context_menu)?;
        buttons
            .into_iter()
            .find(|button| button.enabled && button.rect.contains(point))
            .map(|button| button.action)
    }

    /// Return `true` when a point lands inside the visible browser context menu panel.
    #[cfg(test)]
    pub(crate) fn browser_context_menu_contains_point(
        &self,
        layout: &ShellLayout,
        model: &AppModel,
        point: Point,
    ) -> bool {
        let style = style_for_layout(layout);
        let Some((panel_rect, _)) =
            browser_context_menu_spec(layout, &style, model, self.browser_context_menu)
        else {
            return false;
        };
        panel_rect.contains(point)
    }

    /// Return a browser-context-menu button rect for one action in tests.
    #[cfg(test)]
    pub(crate) fn browser_context_menu_button_rect(
        &self,
        layout: &ShellLayout,
        model: &AppModel,
        action: UiAction,
    ) -> Option<Rect> {
        let style = style_for_layout(layout);
        let (_, buttons) =
            browser_context_menu_spec(layout, &style, model, self.browser_context_menu)?;
        buttons
            .into_iter()
            .find(|button| button.action == action)
            .map(|button| button.rect)
    }

    /// Return the current rendered browser viewport length.
    pub(crate) fn browser_viewport_len(&mut self, layout: &ShellLayout, model: &AppModel) -> usize {
        let style = style_for_layout(layout);
        self.cached_browser_rows(layout, &style, model)
            .len()
            .min(model.browser.visible_count)
    }

    /// Return the current rendered browser viewport start row.
    ///
    /// The shell can preserve a previously resolved visible window even when the
    /// host-projected `view_start_row` is briefly stale. Callers that need to
    /// continue scrolling from the rows the user is actually seeing should use
    /// this value instead of the raw model field.
    pub(crate) fn browser_viewport_start_row(
        &mut self,
        layout: &ShellLayout,
        model: &AppModel,
    ) -> Option<usize> {
        let style = style_for_layout(layout);
        self.cached_browser_rows(layout, &style, model)
            .first()
            .map(|row| row.visible_row)
    }

    /// Return the pointer's offset within the browser scrollbar thumb when hovered.
    pub(crate) fn browser_scrollbar_thumb_offset_at_point(
        &mut self,
        layout: &ShellLayout,
        model: &AppModel,
        point: Point,
    ) -> Option<f32> {
        let (scrollbar, _) = self.cached_browser_scrollbar(layout, model)?;
        let hit_rect = Rect::from_min_max(
            Point::new(
                scrollbar.track.min.x - BROWSER_SCROLLBAR_THUMB_HIT_SLOP,
                scrollbar.thumb.min.y - BROWSER_SCROLLBAR_THUMB_HIT_SLOP,
            ),
            Point::new(
                scrollbar.track.max.x + BROWSER_SCROLLBAR_THUMB_HIT_SLOP,
                scrollbar.thumb.max.y + BROWSER_SCROLLBAR_THUMB_HIT_SLOP,
            ),
        );
        hit_rect
            .contains(point)
            .then_some((point.y - scrollbar.thumb.min.y).clamp(0.0, scrollbar.thumb.height()))
    }

    /// Resolve the browser viewport start row for an active scrollbar-thumb drag.
    pub(crate) fn browser_scrollbar_view_start_for_drag(
        &mut self,
        layout: &ShellLayout,
        model: &AppModel,
        pointer_y: f32,
        thumb_pointer_offset_y: f32,
    ) -> Option<usize> {
        let (scrollbar, viewport_len) = self.cached_browser_scrollbar(layout, model)?;
        browser_scrollbar_view_start_for_pointer(
            scrollbar,
            viewport_len,
            model.browser.visible_count,
            pointer_y,
            thumb_pointer_offset_y,
        )
    }

    /// Resolve the browser viewport start for a click inside the scrollbar track.
    ///
    /// Track clicks jump the thumb so its center aligns with the clicked
    /// location, matching the visual expectation that the handle should move to
    /// the requested position immediately.
    pub(crate) fn browser_scrollbar_view_start_at_point(
        &mut self,
        layout: &ShellLayout,
        model: &AppModel,
        point: Point,
    ) -> Option<usize> {
        let (scrollbar, viewport_len) = self.cached_browser_scrollbar(layout, model)?;
        if !scrollbar.track.contains(point) || scrollbar.thumb.contains(point) {
            return None;
        }
        browser_scrollbar_view_start_for_pointer(
            scrollbar,
            viewport_len,
            model.browser.visible_count,
            point.y,
            scrollbar.thumb.height() * 0.5,
        )
    }

    /// Resolve a browser action-strip click into a native UI action.
    pub(crate) fn browser_action_at_point(
        &mut self,
        layout: &ShellLayout,
        model: &AppModel,
        point: Point,
        alt_down: bool,
    ) -> Option<UiAction> {
        let style = style_for_layout(layout);
        let (buttons, chips, toolbar) = self.cached_browser_action_hit_test(layout, &style, model);
        if let Some(level) =
            browser_rating_filter_level_at_point(toolbar.rating_filter_chips, point)
        {
            return Some(UiAction::ToggleBrowserRatingFilter {
                level,
                invert: alt_down,
            });
        }
        if let Some(bucket) =
            browser_playback_age_filter_chip_at_point(toolbar.playback_age_filter_chips, point)
        {
            return Some(UiAction::ToggleBrowserPlaybackAgeFilter {
                bucket,
                invert: alt_down,
            });
        }
        if browser_marked_filter_chip_contains_point(toolbar.marked_filter_chip, point) {
            return Some(UiAction::ToggleBrowserMarkedFilter);
        }
        if toolbar.search_field.width() > 1.0 && toolbar.search_field.contains(point) {
            return Some(UiAction::FocusBrowserSearch);
        }
        if let Some(action) = chips
            .into_iter()
            .find(|chip| chip.rect.contains(point))
            .map(|chip| UiAction::SelectColumn { index: chip.column })
        {
            return Some(action);
        }
        buttons
            .into_iter()
            .find(|button| button.enabled && button.rect.contains(point))
            .map(|button| button.action.clone())
    }

    /// Resolve a browser tab click into a list/map tab selection action.
    pub(crate) fn browser_tab_action_at_point(
        &self,
        layout: &ShellLayout,
        point: Point,
    ) -> Option<UiAction> {
        let tabs: BrowserTabsRects =
            compute_browser_tabs_rects(layout.browser_tabs, style_for_layout(layout).sizing);
        if tabs.samples.contains(point) {
            return Some(UiAction::SetBrowserTab { map: false });
        }
        if tabs.map.contains(point) {
            return Some(UiAction::SetBrowserTab { map: true });
        }
        None
    }

    /// Return the browser-search field rect when the toolbar is available.
    pub(crate) fn browser_search_field_rect(
        &mut self,
        layout: &ShellLayout,
        model: &AppModel,
    ) -> Option<Rect> {
        let style = style_for_layout(layout);
        let (_, _, toolbar) = self.cached_browser_action_hit_test(layout, &style, model);
        (toolbar.search_field.width() > 1.0).then_some(toolbar.search_field)
    }

    /// Return one browser rating-filter chip rect for the given signed level.
    #[cfg(test)]
    pub(crate) fn browser_rating_filter_chip_rect(
        &mut self,
        layout: &ShellLayout,
        model: &AppModel,
        level: i8,
    ) -> Option<Rect> {
        let style = style_for_layout(layout);
        let (_, _, toolbar) = self.cached_browser_action_hit_test(layout, &style, model);
        let index = browser_rating_filter_chip_index(level)?;
        let rect = toolbar.rating_filter_chips[index];
        (rect.width() > 1.0).then_some(rect)
    }

    /// Return the marked-filter chip rect when the toolbar is available.
    #[cfg(test)]
    pub(crate) fn browser_marked_filter_chip_rect(
        &mut self,
        layout: &ShellLayout,
        model: &AppModel,
    ) -> Option<Rect> {
        let style = style_for_layout(layout);
        let (_, _, toolbar) = self.cached_browser_action_hit_test(layout, &style, model);
        (toolbar.marked_filter_chip.width() > 1.0).then_some(toolbar.marked_filter_chip)
    }

    /// Return one browser playback-age filter chip rect for the given chip.
    #[cfg(test)]
    pub(crate) fn browser_playback_age_filter_chip_rect(
        &mut self,
        layout: &ShellLayout,
        model: &AppModel,
        chip: crate::app::PlaybackAgeFilterChip,
    ) -> Option<Rect> {
        let style = style_for_layout(layout);
        let (_, _, toolbar) = self.cached_browser_action_hit_test(layout, &style, model);
        let index = browser_playback_age_filter_chip_index(chip)?;
        let rect = toolbar.playback_age_filter_chips[index];
        (rect.width() > 1.0).then_some(rect)
    }

    /// Return one browser action-button rect for the given label.
    #[cfg(test)]
    pub(crate) fn browser_action_button_rect(
        &self,
        layout: &ShellLayout,
        model: &AppModel,
        label: &str,
    ) -> Option<Rect> {
        let style = style_for_layout(layout);
        let toolbar = browser_toolbar_layout(layout, &style);
        browser_action_buttons(layout, &style, model, &toolbar)
            .into_iter()
            .find(|button| button.label == label)
            .map(|button| button.rect)
    }

    /// Return the browser-search text rect used for rendering inside the field.
    pub(crate) fn browser_search_text_rect(
        &mut self,
        layout: &ShellLayout,
        model: &AppModel,
    ) -> Option<Rect> {
        let style = style_for_layout(layout);
        let (_, _, toolbar) = self.cached_browser_action_hit_test(layout, &style, model);
        if toolbar.search_field.width() <= 1.0 {
            return None;
        }
        let toolbar_text_layout = compute_browser_toolbar_text_layout(
            toolbar.search_field,
            toolbar.activity_chip,
            toolbar.sort_chip,
            style.sizing,
        );
        Some(toolbar_text_layout.search_label)
    }

    /// Return the focused-row similarity button rect when present.
    #[cfg(test)]
    pub(crate) fn browser_similarity_button_rect(
        &mut self,
        layout: &ShellLayout,
        model: &AppModel,
    ) -> Option<Rect> {
        let style = style_for_layout(layout);
        self.cached_browser_rows(layout, &style, model)
            .iter()
            .find(|row| row.focused)
            .and_then(|row| super::super::browser_similarity_button_rect(row.rect, style.sizing))
    }

    /// Resolve a map-point click to a sample-id action when map tab is active.
    pub(crate) fn map_sample_action_at_point(
        &self,
        layout: &ShellLayout,
        model: &AppModel,
        point: Point,
    ) -> Option<UiAction> {
        if !model.map.active {
            return None;
        }
        map_sample_id_at_point(layout, model, point)
            .map(|sample_id| UiAction::FocusMapSample { sample_id })
    }
}

pub(in crate::gui::native_shell::state) fn browser_action_hit_test_cache_key(
    layout: &ShellLayout,
    model: &AppModel,
) -> BrowserActionHitTestCacheKey {
    BrowserActionHitTestCacheKey {
        browser_toolbar_min_x: f32_to_bits(layout.browser_toolbar.min.x),
        browser_toolbar_min_y: f32_to_bits(layout.browser_toolbar.min.y),
        browser_toolbar_max_x: f32_to_bits(layout.browser_toolbar.max.x),
        browser_toolbar_max_y: f32_to_bits(layout.browser_toolbar.max.y),
        ui_scale: f32_to_bits(layout.ui_scale),
        model_signature: browser_action_model_signature(model),
    }
}

pub(in crate::gui::native_shell::state) fn browser_action_model_signature(model: &AppModel) -> u64 {
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    model.browser_actions.can_rename.hash(&mut hasher);
    model.browser_actions.can_tag.hash(&mut hasher);
    model.browser_actions.can_delete.hash(&mut hasher);
    model
        .browser_actions
        .random_navigation_enabled
        .hash(&mut hasher);
    model
        .browser_actions
        .duplicate_cleanup_active
        .hash(&mut hasher);
    model.browser.active_rating_filters.hash(&mut hasher);
    model.browser.active_playback_age_filters.hash(&mut hasher);
    model.browser.marked_filter_active.hash(&mut hasher);
    model.selected_column.min(2).hash(&mut hasher);
    for index in 0..3 {
        if let Some(column) = model.columns.get(index) {
            column.title.hash(&mut hasher);
            column.item_count.hash(&mut hasher);
        } else {
            index.hash(&mut hasher);
        }
    }
    hasher.finish()
}
