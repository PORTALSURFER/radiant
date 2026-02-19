//! Mutable interaction state and paint generation for the native shell.

use super::{
    layout::{ShellLayout, ShellNodeKind},
    layout_adapter::{
        SidebarRowCounts, compute_browser_action_button_rects, compute_browser_header_text_layout,
        compute_browser_row_text_layout, compute_browser_toolbar_sections,
        compute_drag_overlay_rect, compute_progress_overlay_sections,
        compute_prompt_overlay_sections, compute_sidebar_action_button_rects,
        compute_sidebar_folder_header_layout, compute_sidebar_row_sections,
        compute_source_section_divider_rect, compute_status_text_line_rect,
        compute_top_bar_controls_sections, compute_update_action_button_rects,
        compute_waveform_header_text_layout,
    },
    paint::{FillCircle, FillRect, NativeViewFrame, Primitive, TextAlign, TextRun},
    style::{SizingTokens, StyleTokens},
};
use crate::app::{AppModel, BrowserRowModel, BrowserTagTarget, NativeMotionModel, UiAction};
use crate::gui::{
    input::KeyCode,
    types::{ImageRgba, Point, Rect, Rgba8},
};

/// Mutable interaction + animation state for the native shell.
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct NativeShellState {
    selected_column: usize,
    hovered: Option<ShellNodeKind>,
    hovered_browser_visible_row: Option<usize>,
    transport_running: bool,
    has_focus_emphasis: bool,
    startup_frame_ticks: u8,
    pulse_phase: f32,
    source_row_rects: Vec<Rect>,
    source_row_cache_key: Option<SidebarRowsCacheKey>,
    folder_row_rects: Vec<Rect>,
    folder_row_cache_key: Option<SidebarRowsCacheKey>,
    browser_rows: Vec<CachedBrowserRow>,
    browser_rows_cache_key: Option<BrowserRowsCacheKey>,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
struct NativeAnimationReasons {
    transport_running: bool,
    startup_frame_tick: bool,
}

impl NativeAnimationReasons {
    fn needs_animation(self) -> bool {
        self.transport_running || self.startup_frame_tick
    }
}

impl NativeShellState {
    /// Create a default shell state.
    pub(crate) fn new() -> Self {
        Self {
            selected_column: 1,
            hovered: None,
            hovered_browser_visible_row: None,
            transport_running: true,
            has_focus_emphasis: false,
            startup_frame_ticks: 2,
            pulse_phase: 0.0,
            source_row_rects: Vec::new(),
            source_row_cache_key: None,
            folder_row_rects: Vec::new(),
            folder_row_cache_key: None,
            browser_rows: Vec::new(),
            browser_rows_cache_key: None,
        }
    }

    /// Return whether the shell currently needs continuous animation.
    /// Focus emphasis is intentionally not included so selection and focus rendering
    /// remains static without forcing redraws when transport is idle.
    pub(crate) fn needs_animation(&self) -> bool {
        self.animation_reasons().needs_animation()
    }

    fn animation_reasons(&self) -> NativeAnimationReasons {
        NativeAnimationReasons {
            transport_running: self.transport_running,
            startup_frame_tick: self.startup_frame_ticks > 0,
        }
    }

    /// Return whether playback transport is currently reported as running.
    pub(crate) fn is_transport_running(&self) -> bool {
        self.transport_running
    }

    /// Synchronize local interaction state from the latest app model.
    pub(crate) fn sync_from_model(&mut self, model: &AppModel) {
        self.selected_column = model.selected_column.min(2);
        self.transport_running = model.transport_running;
        self.startup_frame_ticks = self.startup_frame_ticks.saturating_sub(1);
        if model.map.active {
            self.hovered_browser_visible_row = None;
        }
        self.has_focus_emphasis = model
            .browser
            .rows
            .iter()
            .any(|row| row.focused || row.selected)
            || model.sources.rows.iter().any(|row| row.selected)
            || model
                .sources
                .folder_rows
                .iter()
                .any(|row| row.focused || row.selected)
            || model.confirm_prompt.visible;
    }

    /// Synchronize motion-sensitive state from a dedicated motion model projection.
    pub(crate) fn sync_from_motion_model(&mut self, model: &NativeMotionModel) {
        self.transport_running = model.transport_running;
    }

    /// Update animation clocks by a frame delta using explicit style motion tokens.
    pub(crate) fn tick_with_style(&mut self, delta_seconds: f32, style: &StyleTokens) {
        if self.needs_animation() {
            let speed = if self.transport_running {
                style.motion_speed_transport
            } else {
                style.motion_speed_idle
            };
            self.pulse_phase =
                (self.pulse_phase + delta_seconds * speed).rem_euclid(std::f32::consts::TAU);
        }
    }

    /// Handle pointer movement and update hovered view target.
    pub(crate) fn handle_cursor_move(
        &mut self,
        layout: &ShellLayout,
        model: &AppModel,
        point: Point,
    ) -> bool {
        let next_hover = layout.hit_test(point);
        let next_hovered_browser_row =
            self.resolve_hovered_browser_row(layout, model, point, next_hover);
        let changed = next_hover != self.hovered
            || next_hovered_browser_row != self.hovered_browser_visible_row;
        if !changed {
            return false;
        }
        self.hovered = next_hover;
        self.hovered_browser_visible_row = next_hovered_browser_row;
        true
    }

    fn resolve_hovered_browser_row(
        &mut self,
        layout: &ShellLayout,
        model: &AppModel,
        point: Point,
        hover: Option<ShellNodeKind>,
    ) -> Option<usize> {
        if model.map.active || hover != Some(ShellNodeKind::BrowserTable) {
            return None;
        }
        let style = style_for_layout(layout);
        let rows = self.cached_browser_rows(layout, &style, model);
        row_index_for_visible_rows(rows, point, layout.browser_rows, style.sizing)
            .map(|index| rows[index].visible_row)
    }

    /// Handle a primary button click at the pointer position.
    pub(crate) fn handle_primary_click(&mut self, layout: &ShellLayout, point: Point) -> bool {
        let Some(column) = layout.column_at_point(point) else {
            return false;
        };
        if self.selected_column == column {
            return false;
        }
        self.selected_column = column;
        true
    }

    /// Handle backend-agnostic key input.
    pub(crate) fn handle_key(&mut self, key: KeyCode) -> bool {
        match key {
            KeyCode::ArrowLeft => {
                self.selected_column = (self.selected_column + 2) % 3;
                true
            }
            KeyCode::ArrowRight => {
                self.selected_column = (self.selected_column + 1) % 3;
                true
            }
            KeyCode::Num1 => {
                if self.selected_column == 0 {
                    false
                } else {
                    self.selected_column = 0;
                    true
                }
            }
            KeyCode::Num2 => {
                if self.selected_column == 1 {
                    false
                } else {
                    self.selected_column = 1;
                    true
                }
            }
            KeyCode::Num3 => {
                if self.selected_column == 2 {
                    false
                } else {
                    self.selected_column = 2;
                    true
                }
            }
            KeyCode::Enter => {
                self.transport_running = !self.transport_running;
                true
            }
            _ => false,
        }
    }

    /// Resolve a rendered source-row index for a point within the sidebar.
    pub(crate) fn source_row_at_point(
        &mut self,
        layout: &ShellLayout,
        model: &AppModel,
        point: Point,
    ) -> Option<usize> {
        let style = style_for_layout(layout);
        let source_rows = self.cached_source_row_rects(layout, &style, model);
        row_index_from_stacked_rows(
            source_rows,
            point,
            style.sizing.source_row_height,
            style.sizing.source_row_gap,
        )
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
        row_index_from_stacked_rows(
            folder_rows,
            point,
            style.sizing.folder_row_height,
            style.sizing.folder_row_gap,
        )
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

    /// Return a browser-action button rect for the provided action in tests.
    #[cfg(test)]
    pub(crate) fn browser_action_button_rect(
        &self,
        layout: &ShellLayout,
        model: &AppModel,
        action: UiAction,
    ) -> Option<Rect> {
        let style = style_for_layout(layout);
        browser_action_buttons(layout, &style, model)
            .into_iter()
            .find(|button| button.action == action)
            .map(|button| button.rect)
    }

    /// Resolve a source-management action button click into a native UI action.
    pub(crate) fn source_action_at_point(
        &self,
        layout: &ShellLayout,
        model: &AppModel,
        point: Point,
    ) -> Option<UiAction> {
        let style = style_for_layout(layout);
        source_action_buttons(layout, &style, model)
            .into_iter()
            .find(|button| button.enabled && button.rect.contains(point))
            .map(|button| button.action)
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
        row_index_for_visible_rows(rows, point, layout.browser_rows, style.sizing)
            .map(|index| rows[index].visible_row)
    }

    /// Resolve a browser action-strip click into a native UI action.
    pub(crate) fn browser_action_at_point(
        &self,
        layout: &ShellLayout,
        model: &AppModel,
        point: Point,
    ) -> Option<UiAction> {
        let style = style_for_layout(layout);
        browser_action_buttons(layout, &style, model)
            .into_iter()
            .find(|button| button.enabled && button.rect.contains(point))
            .map(|button| button.action)
    }

    /// Resolve a browser tab click into a list/map tab selection action.
    pub(crate) fn browser_tab_action_at_point(
        &self,
        layout: &ShellLayout,
        point: Point,
    ) -> Option<UiAction> {
        let tabs = browser_tabs_layout(layout, style_for_layout(layout).sizing);
        if tabs.samples.contains(point) {
            return Some(UiAction::SetBrowserTab { map: false });
        }
        if tabs.map.contains(point) {
            return Some(UiAction::SetBrowserTab { map: true });
        }
        None
    }

    /// Resolve a top-bar update action button click.
    pub(crate) fn update_action_at_point(
        &self,
        layout: &ShellLayout,
        model: &AppModel,
        point: Point,
    ) -> Option<UiAction> {
        let style = style_for_layout(layout);
        update_action_buttons(layout, &style, model)
            .into_iter()
            .find(|button| button.enabled && button.rect.contains(point))
            .map(|button| button.action)
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

    /// Build a native frame from state + layout + style tokens.
    pub(crate) fn build_frame_with_style(
        &mut self,
        layout: &ShellLayout,
        style: &StyleTokens,
        model: &AppModel,
    ) -> NativeViewFrame {
        let mut frame = NativeViewFrame {
            clear_color: style.clear_color,
            primitives: Vec::new(),
            text_runs: Vec::new(),
        };
        self.build_frame_with_style_into(layout, style, model, &mut frame);
        frame
    }

    /// Build a native frame from state + layout + style tokens into reusable buffers.
    pub(crate) fn build_frame_with_style_into(
        &mut self,
        layout: &ShellLayout,
        style: &StyleTokens,
        model: &AppModel,
        frame: &mut NativeViewFrame,
    ) {
        self.build_frame_with_style_into_with_motion(
            layout,
            style,
            model,
            frame,
            self.pulse_phase,
            true,
        );
    }

    /// Build a frame without animated values into reusable buffers.
    pub(crate) fn build_frame_with_style_into_static(
        &mut self,
        layout: &ShellLayout,
        style: &StyleTokens,
        model: &AppModel,
        frame: &mut NativeViewFrame,
    ) {
        self.build_frame_with_style_into_with_motion(layout, style, model, frame, 0.0, false);
    }

    /// Build a frame with a caller-supplied motion phase.
    fn build_frame_with_style_into_with_motion(
        &mut self,
        layout: &ShellLayout,
        style: &StyleTokens,
        model: &AppModel,
        frame: &mut NativeViewFrame,
        pulse_phase: f32,
        include_overlays: bool,
    ) {
        let sizing = style.sizing;
        let motion_wave = interaction_wave(pulse_phase);
        let focus_fill_emphasis = focus_fill_blend(style, motion_wave);
        let focus_text_emphasis = focus_text_blend(style, motion_wave);
        frame.primitives.clear();
        frame.text_runs.clear();
        let primitives = &mut frame.primitives;
        let text_runs = &mut frame.text_runs;

        primitives.push(Primitive::Rect(FillRect {
            rect: layout.top_bar,
            color: style.surface_raised,
        }));
        primitives.push(Primitive::Rect(FillRect {
            rect: layout.sidebar,
            color: style.surface_raised,
        }));
        primitives.push(Primitive::Rect(FillRect {
            rect: layout.content,
            color: style.surface_base,
        }));
        primitives.push(Primitive::Rect(FillRect {
            rect: layout.waveform_card,
            color: style.surface_raised,
        }));
        primitives.push(Primitive::Rect(FillRect {
            rect: layout.status_bar,
            color: style.surface_raised,
        }));
        primitives.push(Primitive::Rect(FillRect {
            rect: layout.browser_panel,
            color: style.surface_raised,
        }));
        primitives.push(Primitive::Rect(FillRect {
            rect: layout.browser_tabs,
            color: style.surface_overlay,
        }));
        primitives.push(Primitive::Rect(FillRect {
            rect: layout.browser_toolbar,
            color: style.surface_base,
        }));
        primitives.push(Primitive::Rect(FillRect {
            rect: layout.browser_table_header,
            color: style.surface_overlay,
        }));
        primitives.push(Primitive::Rect(FillRect {
            rect: layout.browser_footer,
            color: style.surface_overlay,
        }));

        let waveform_inner = layout.waveform_plot;
        let scan_step = sizing.waveform_scan_step;
        let mut x = waveform_inner.min.x;
        while x < waveform_inner.max.x {
            let strong = ((x - waveform_inner.min.x) / scan_step).floor() as i32 % 4 == 0;
            let line_color = if strong {
                style.grid_strong
            } else {
                style.grid_soft
            };
            primitives.push(Primitive::Rect(FillRect {
                rect: Rect::from_min_max(
                    Point::new(x, waveform_inner.min.y),
                    Point::new(
                        (x + sizing.border_width).min(waveform_inner.max.x),
                        waveform_inner.max.y,
                    ),
                ),
                color: line_color,
            }));
            x += scan_step;
        }

        push_waveform_image(
            primitives,
            waveform_inner,
            model.waveform.waveform_image.as_ref(),
        );

        let browser_buttons = browser_action_buttons(layout, style, model);
        let source_row_rects = rendered_source_row_rects(layout, style, model);
        let folder_row_rects = rendered_folder_row_rects(layout, style, model);
        let browser_rows = rendered_browser_rows(layout, model, style);
        if model.map.active {
            let canvas = map_canvas_rect(layout.browser_rows, sizing);
            primitives.push(Primitive::Rect(FillRect {
                rect: canvas,
                color: blend_color(style.surface_base, style.bg_secondary, 0.24),
            }));
            push_border(
                primitives,
                canvas,
                style.border_emphasis,
                sizing.border_width,
            );
            for point in &model.map.points {
                let center = map_point_center(canvas, point);
                let color = map_point_color(style, point);
                let radius = if point.focused {
                    4.5
                } else if point.selected {
                    3.8
                } else {
                    2.6
                };
                primitives.push(Primitive::Circle(FillCircle {
                    center,
                    radius,
                    color,
                }));
            }
        } else {
            for row in browser_rows.iter() {
                let row_text_layout = compute_browser_row_text_layout(row.rect, sizing);
                let row_columns = row_text_layout.columns;
                let row_fill = if row.focused {
                    translucent_overlay_color(
                        style.bg_tertiary,
                        style.grid_strong,
                        focus_fill_emphasis,
                    )
                } else if row.selected {
                    translucent_overlay_color(
                        style.bg_tertiary,
                        style.grid_soft,
                        style.state_selected_blend,
                    )
                } else if row.visible_row % 2 == 0 {
                    blend_color(style.surface_base, style.bg_secondary, 0.20)
                } else {
                    blend_color(style.surface_base, style.bg_secondary, 0.10)
                };
                let row_border = if row.focused {
                    blend_color(
                        style.accent_warning,
                        style.text_primary,
                        motion_wave * style.state_focus_pulse_blend,
                    )
                } else if row.selected {
                    blend_color(
                        style.accent_mint,
                        style.text_primary,
                        motion_wave * style.state_selected_blend,
                    )
                } else {
                    style.border
                };
                let row_text_color = if row.focused {
                    blend_color(
                        style.accent_warning,
                        style.text_primary,
                        motion_wave * focus_text_emphasis,
                    )
                } else if row.selected {
                    style.accent_mint
                } else {
                    style.text_primary
                };
                primitives.push(Primitive::Rect(FillRect {
                    rect: row.rect,
                    color: row_fill,
                }));
                for separator_x in [row_columns.index.max.x, row_columns.sample.max.x] {
                    primitives.push(Primitive::Rect(FillRect {
                        rect: Rect::from_min_max(
                            Point::new(separator_x, row.rect.min.y),
                            Point::new(
                                (separator_x + sizing.border_width).min(row.rect.max.x),
                                row.rect.max.y,
                            ),
                        ),
                        color: blend_color(style.border, style.grid_soft, 0.36),
                    }));
                }
                push_border(
                    primitives,
                    row.rect,
                    row_border,
                    if row.focused {
                        sizing.focus_stroke_width
                    } else {
                        sizing.border_width
                    },
                );
                let chip_rect = row_text_layout.bucket_chip;
                let chip_color = match row.column {
                    0 => blend_color(style.accent_warning, style.bg_secondary, 0.54),
                    2 => blend_color(style.accent_mint, style.bg_secondary, 0.54),
                    _ => blend_color(style.text_muted, style.bg_secondary, 0.54),
                };
                primitives.push(Primitive::Rect(FillRect {
                    rect: chip_rect,
                    color: chip_color,
                }));
                push_border(primitives, chip_rect, style.border, sizing.border_width);
                text_runs.push(TextRun {
                    text: row.visible_row.to_string(),
                    position: row_text_layout.index_label.min,
                    font_size: sizing.font_meta,
                    color: style.text_muted,
                    max_width: Some(row_text_layout.index_label.width().max(12.0)),
                    align: TextAlign::Right,
                });
                let label_max_width = row_text_layout.sample_label.width().max(20.0);
                text_runs.push(TextRun {
                    text: row.label.clone(),
                    position: row_text_layout.sample_label.min,
                    font_size: sizing.font_body,
                    color: row_text_color,
                    max_width: Some(label_max_width.max(20.0)),
                    align: TextAlign::Left,
                });
                let bucket_label_max_width = row_text_layout.bucket_label.width().max(10.0);
                text_runs.push(TextRun {
                    text: truncate_to_width(
                        &row.bucket_label,
                        bucket_label_max_width,
                        sizing.font_meta,
                    ),
                    position: row_text_layout.bucket_label.min,
                    font_size: sizing.font_meta,
                    color: style.text_primary,
                    max_width: Some(bucket_label_max_width),
                    align: TextAlign::Center,
                });
            }
        }

        push_border(
            primitives,
            layout.top_bar,
            style.border,
            sizing.border_width,
        );
        push_border(
            primitives,
            layout.sidebar,
            style.border,
            sizing.border_width,
        );
        push_border(
            primitives,
            layout.waveform_card,
            style.border,
            sizing.border_width,
        );
        push_border(
            primitives,
            layout.browser_panel,
            style.border,
            sizing.border_width,
        );
        push_border(
            primitives,
            layout.browser_table_header,
            style.border,
            sizing.border_width,
        );
        push_border(
            primitives,
            layout.status_bar,
            style.border,
            sizing.border_width,
        );

        let lamp_radius = sizing.lamp_radius_base
            + (((self.pulse_phase.sin() + 1.0) * 0.5) * sizing.lamp_radius_amp);
        let lamp_color = if self.transport_running {
            style.accent_mint
        } else {
            style.accent_copper
        };
        primitives.push(Primitive::Circle(FillCircle {
            center: Point::new(
                layout.top_bar.max.x - (sizing.text_inset_x + 14.0),
                layout.top_bar_title_row.min.y + (layout.top_bar_title_row.height() * 0.5),
            ),
            radius: lamp_radius,
            color: lamp_color,
        }));

        let top_text_x =
            layout.top_bar_title_cluster.min.x + sizing.text_inset_x + sizing.header_label_gutter;
        let top_title_y = text_top_in_rect(
            layout.top_bar_title_row,
            sizing.font_title,
            sizing.text_inset_y,
        );
        let top_title_width = (layout.top_bar_title_cluster.width()
            - ((sizing.text_inset_x + sizing.header_label_gutter) * 2.0))
            .max(72.0);
        text_runs.push(TextRun {
            text: truncate_to_width(&model.title, top_title_width, sizing.font_title),
            position: Point::new(top_text_x, top_title_y),
            font_size: sizing.font_title,
            color: style.text_primary,
            max_width: Some(top_title_width),
            align: TextAlign::Left,
        });
        let top_controls = top_bar_controls_layout(layout, sizing);
        if top_controls.active {
            let divider_y = layout.top_bar_controls_row.min.y;
            primitives.push(Primitive::Rect(FillRect {
                rect: Rect::from_min_max(
                    Point::new(layout.top_bar.min.x, divider_y),
                    Point::new(
                        layout.top_bar.max.x,
                        (divider_y + sizing.border_width).min(layout.top_bar.max.y),
                    ),
                ),
                color: style.border,
            }));
            primitives.push(Primitive::Rect(FillRect {
                rect: top_controls.volume_meter,
                color: style.surface_overlay,
            }));
            push_border(
                primitives,
                top_controls.volume_meter,
                style.border_emphasis,
                sizing.border_width,
            );
            let volume_level = model.volume.clamp(0.0, 1.0);
            let fill_width = (top_controls.volume_meter.width() * volume_level)
                .clamp(1.0, top_controls.volume_meter.width());
            primitives.push(Primitive::Rect(FillRect {
                rect: Rect::from_min_max(
                    top_controls.volume_meter.min,
                    Point::new(
                        top_controls.volume_meter.min.x + fill_width,
                        top_controls.volume_meter.max.y,
                    ),
                ),
                color: blend_color(style.accent_mint, style.text_primary, 0.28),
            }));
            text_runs.push(TextRun {
                text: String::from("Options"),
                position: Point::new(
                    top_controls.options_label.min.x,
                    text_top_in_rect(
                        top_controls.options_label,
                        sizing.font_meta,
                        sizing.text_inset_y,
                    ),
                ),
                font_size: sizing.font_meta,
                color: style.text_primary,
                max_width: Some(top_controls.options_label.width().max(24.0)),
                align: TextAlign::Left,
            });
            text_runs.push(TextRun {
                text: format!("{volume_level:.2}"),
                position: Point::new(
                    top_controls.volume_value.min.x,
                    text_top_in_rect(
                        top_controls.volume_value,
                        sizing.font_meta,
                        sizing.text_inset_y,
                    ),
                ),
                font_size: sizing.font_meta,
                color: style.text_muted,
                max_width: Some(top_controls.volume_value.width().max(20.0)),
                align: TextAlign::Right,
            });
            text_runs.push(TextRun {
                text: String::from("Vol"),
                position: Point::new(
                    top_controls.volume_label.min.x,
                    text_top_in_rect(
                        top_controls.volume_label,
                        sizing.font_meta,
                        sizing.text_inset_y,
                    ),
                ),
                font_size: sizing.font_meta,
                color: style.text_muted,
                max_width: Some(top_controls.volume_label.width().max(18.0)),
                align: TextAlign::Left,
            });
        }
        let update_buttons = update_action_buttons(layout, style, model);
        let update_status_text = update_status_text(model);
        let update_hint_text = update_hint_text(model);
        let update_notes_text = update_notes_text(model);
        let update_controls_text = if update_notes_text.is_empty() {
            update_hint_text
        } else if update_hint_text.is_empty() {
            update_notes_text
        } else {
            format!("{update_hint_text} | {update_notes_text}")
        };
        let reserved_button_width = update_buttons
            .iter()
            .map(|button| button.rect.width())
            .sum::<f32>()
            + (update_buttons.len().saturating_sub(1) as f32 * sizing.action_button_gap.max(1.0));
        text_runs.push(TextRun {
            text: truncate_to_width(
                &update_status_text,
                (layout.top_bar_action_cluster.width()
                    - reserved_button_width
                    - (sizing.text_inset_x * 2.0))
                    .max(20.0),
                sizing.font_meta,
            ),
            position: Point::new(
                layout.top_bar_action_cluster.min.x + sizing.text_inset_x,
                text_top_in_rect(
                    layout.top_bar_title_row,
                    sizing.font_meta,
                    sizing.text_inset_y,
                ),
            ),
            font_size: sizing.font_meta,
            color: style.text_muted,
            max_width: Some(
                (layout.top_bar_action_cluster.width()
                    - reserved_button_width
                    - (sizing.text_inset_x * 2.0))
                    .max(20.0),
            ),
            align: TextAlign::Left,
        });
        if !update_controls_text.is_empty() {
            text_runs.push(TextRun {
                text: truncate_to_width(
                    &update_controls_text,
                    (layout.top_bar_action_cluster.width() - (sizing.text_inset_x * 2.0)).max(20.0),
                    sizing.font_meta,
                ),
                position: Point::new(
                    layout.top_bar_action_cluster.min.x + sizing.text_inset_x,
                    text_top_in_rect(
                        layout.top_bar_controls_row,
                        sizing.font_meta,
                        sizing.text_inset_y,
                    ),
                ),
                font_size: sizing.font_meta,
                color: style.text_muted,
                max_width: Some(
                    (layout.top_bar_action_cluster.width() - (sizing.text_inset_x * 2.0)).max(20.0),
                ),
                align: TextAlign::Left,
            });
        }
        for button in &update_buttons {
            primitives.push(Primitive::Rect(FillRect {
                rect: button.rect,
                color: if button.enabled {
                    style.surface_overlay
                } else {
                    style.control_disabled_fill
                },
            }));
            push_border(
                primitives,
                button.rect,
                if button.enabled {
                    blend_color(
                        style.border_emphasis,
                        style.text_primary,
                        style.state_hover_soft,
                    )
                } else {
                    style.border
                },
                sizing.border_width,
            );
            text_runs.push(TextRun {
                text: button.label.to_string(),
                position: Point::new(
                    button.rect.min.x + sizing.text_inset_x,
                    text_top_in_rect(button.rect, sizing.font_meta, sizing.text_inset_y),
                ),
                font_size: sizing.font_meta,
                color: if button.enabled {
                    button.text_color
                } else {
                    style.text_muted
                },
                max_width: Some((button.rect.width() - (sizing.text_inset_x * 2.0)).max(12.0)),
                align: TextAlign::Center,
            });
        }
        for button in &browser_buttons {
            primitives.push(Primitive::Rect(FillRect {
                rect: button.rect,
                color: if button.enabled {
                    style.surface_overlay
                } else {
                    style.control_disabled_fill
                },
            }));
            push_border(
                primitives,
                button.rect,
                if button.enabled {
                    blend_color(
                        style.border_emphasis,
                        style.text_primary,
                        style.state_hover_soft,
                    )
                } else {
                    style.border
                },
                sizing.border_width,
            );
            text_runs.push(TextRun {
                text: button.label.to_string(),
                position: Point::new(
                    button.rect.min.x + sizing.text_inset_x,
                    text_top_in_rect(button.rect, sizing.font_meta, sizing.text_inset_y),
                ),
                font_size: sizing.font_meta,
                color: if button.enabled {
                    button.text_color
                } else {
                    style.text_muted
                },
                max_width: Some((button.rect.width() - (sizing.text_inset_x * 2.0)).max(12.0)),
                align: TextAlign::Center,
            });
        }
        let sources_header = if model.sources.header.is_empty() {
            model.sources_label.as_str()
        } else {
            model.sources.header.as_str()
        };
        let sidebar_sections = sidebar_sections(layout, style, model);
        text_runs.push(TextRun {
            text: truncate_to_width(
                sources_header,
                (layout.sidebar_header.width() - (sizing.text_inset_x * 2.0)).max(72.0),
                sizing.font_header,
            ),
            position: Point::new(
                layout.sidebar_header.min.x + sizing.text_inset_x + sizing.header_label_gutter,
                layout.sidebar_header.min.y + sizing.text_inset_y,
            ),
            font_size: sizing.font_header,
            color: style.text_primary,
            max_width: Some(
                (layout.sidebar_header.width() - (sizing.text_inset_x * 2.0)).max(72.0),
            ),
            align: TextAlign::Left,
        });
        text_runs.push(TextRun {
            text: format!(
                "search: {}",
                if model.sources.search_query.is_empty() {
                    "—"
                } else {
                    model.sources.search_query.as_str()
                }
            ),
            position: Point::new(
                layout.sidebar_header.min.x + sizing.text_inset_x + sizing.header_label_gutter,
                layout.sidebar_header.min.y
                    + sizing.text_inset_y
                    + sizing.font_header
                    + sizing.text_row_gap,
            ),
            font_size: sizing.font_meta,
            color: style.text_muted,
            max_width: Some(
                (layout.sidebar_header.width() - (sizing.text_inset_x * 2.0)).max(72.0),
            ),
            align: TextAlign::Left,
        });
        let rendered_sources = source_row_rects.len();
        for (row_index, row_rect) in source_row_rects.iter().enumerate() {
            let row_rect = *row_rect;
            let row = &model.sources.rows[row_index];
            let row_selected = row.selected
                || model
                    .sources
                    .selected_row
                    .is_some_and(|selected| selected == row_index);
            let row_fill = if row_selected {
                translucent_overlay_color(
                    style.bg_tertiary,
                    style.grid_soft,
                    style.state_selected_blend,
                )
            } else {
                style.surface_base
            };
            primitives.push(Primitive::Rect(FillRect {
                rect: row_rect,
                color: row_fill,
            }));
            push_border(
                primitives,
                row_rect,
                if row_selected {
                    blend_color(
                        style.accent_mint,
                        style.text_primary,
                        motion_wave * style.state_selected_blend,
                    )
                } else if row.missing {
                    style.accent_warning
                } else {
                    style.border
                },
                sizing.border_width,
            );
            text_runs.push(TextRun {
                text: truncate_to_width(
                    &row.label,
                    row_label_width(row_rect, &sizing, 0.0, 24.0),
                    sizing.font_body,
                ),
                position: Point::new(
                    row_label_x(row_rect, &sizing, 0.0),
                    text_top_in_rect(row_rect, sizing.font_body, sizing.text_inset_y),
                ),
                font_size: sizing.font_body,
                color: if row_selected {
                    style.accent_mint
                } else {
                    style.text_primary
                },
                max_width: Some(row_label_width(row_rect, &sizing, 0.0, 24.0)),
                align: TextAlign::Left,
            });
        }
        let rendered_folders = folder_row_rects.len();
        if rendered_folders > 0 {
            if let Some(divider_rect) = compute_source_section_divider_rect(
                sidebar_sections.source_rows,
                sidebar_sections.folder_header,
                sizing,
            ) {
                primitives.push(Primitive::Rect(FillRect {
                    rect: divider_rect,
                    color: style.source_section_divider,
                }));
            }
            let folder_header_layout = compute_sidebar_folder_header_layout(
                sidebar_sections.folder_header,
                sizing,
                model.sources.folder_recovery.in_progress,
                model.sources.folder_recovery.entry_count,
            );
            if let Some(badge) = folder_header_layout.badge.as_ref() {
                primitives.push(Primitive::Rect(FillRect {
                    rect: badge.rect,
                    color: if badge.active {
                        style.source_recovery_badge_active
                    } else {
                        style.source_recovery_badge_idle
                    },
                }));
                push_border(
                    primitives,
                    badge.rect,
                    blend_color(
                        style.border_emphasis,
                        style.text_primary,
                        style.state_hover_soft,
                    ),
                    sizing.border_width,
                );
                text_runs.push(TextRun {
                    text: badge.label.clone(),
                    position: Point::new(
                        badge.rect.min.x + sizing.text_inset_x,
                        text_top_in_rect(badge.rect, sizing.font_meta, sizing.text_inset_y),
                    ),
                    font_size: sizing.font_meta,
                    color: style.text_primary,
                    max_width: Some((badge.rect.width() - (sizing.text_inset_x * 2.0)).max(18.0)),
                    align: TextAlign::Center,
                });
            }
            if folder_header_layout.title_row.width() > 8.0 {
                text_runs.push(TextRun {
                    text: format!("Folders ({})", model.sources.folder_rows.len()),
                    position: Point::new(
                        folder_header_layout.title_row.min.x,
                        folder_header_layout.title_row.min.y,
                    ),
                    font_size: sizing.font_header,
                    color: style.text_primary,
                    max_width: Some(folder_header_layout.title_row.width()),
                    align: TextAlign::Left,
                });
                if let Some(metadata_row) = folder_header_layout
                    .metadata_row
                    .filter(|row| row.width() > 24.0)
                {
                    text_runs.push(TextRun {
                        text: format!(
                            "query: {}",
                            if model.sources.folder_search_query.is_empty() {
                                "—"
                            } else {
                                model.sources.folder_search_query.as_str()
                            }
                        ),
                        position: metadata_row.min,
                        font_size: sizing.font_meta,
                        color: style.text_muted,
                        max_width: Some(metadata_row.width()),
                        align: TextAlign::Left,
                    });
                }
            }
            for (row_index, row_rect) in folder_row_rects.iter().enumerate() {
                let row_rect = *row_rect;
                let row = &model.sources.folder_rows[row_index];
                let row_selected = row.selected || row.focused;
                let row_fill = if row.focused {
                    translucent_overlay_color(
                        style.bg_tertiary,
                        style.grid_strong,
                        (style.state_hover_soft + (motion_wave * style.motion_focus_wave_amp))
                            .clamp(0.0, 1.0),
                    )
                } else if row_selected {
                    translucent_overlay_color(
                        style.bg_tertiary,
                        style.grid_soft,
                        style.state_selected_blend,
                    )
                } else {
                    style.surface_base
                };
                primitives.push(Primitive::Rect(FillRect {
                    rect: row_rect,
                    color: row_fill,
                }));
                push_border(
                    primitives,
                    row_rect,
                    if row.focused {
                        blend_color(
                            style.accent_warning,
                            style.text_primary,
                            motion_wave * style.state_focus_pulse_blend,
                        )
                    } else if row.selected {
                        blend_color(
                            style.accent_mint,
                            style.text_primary,
                            motion_wave * style.state_selected_blend,
                        )
                    } else {
                        style.border
                    },
                    if row.focused {
                        sizing.focus_stroke_width
                    } else {
                        sizing.border_width
                    },
                );
                let glyph = if row.is_root {
                    "•"
                } else if row.has_children {
                    if row.expanded { "▼" } else { "▶" }
                } else {
                    "·"
                };
                let depth_indent = (row.depth as f32 * sizing.folder_indent_step)
                    .min((row_rect.width() * 0.45).max(0.0));
                let label_text = format!("{glyph} {}", row.label);
                let folder_text_width = (row_rect.width()
                    - ((sizing.text_inset_x + sizing.row_corner_inset) * 2.0)
                    - depth_indent)
                    .max(24.0);
                text_runs.push(TextRun {
                    text: truncate_to_width(&label_text, folder_text_width, sizing.font_body),
                    position: Point::new(
                        row_label_x(row_rect, &sizing, depth_indent),
                        text_top_in_rect(row_rect, sizing.font_body, sizing.text_inset_y),
                    ),
                    font_size: sizing.font_body,
                    color: if row.focused {
                        style.accent_warning
                    } else if row.selected {
                        style.accent_mint
                    } else {
                        style.text_primary
                    },
                    max_width: Some(folder_text_width),
                    align: TextAlign::Left,
                });
            }
        }
        for button in source_action_buttons(layout, style, model) {
            primitives.push(Primitive::Rect(FillRect {
                rect: button.rect,
                color: if button.enabled {
                    style.surface_overlay
                } else {
                    style.control_disabled_fill
                },
            }));
            push_border(
                primitives,
                button.rect,
                if button.enabled {
                    blend_color(
                        style.border_emphasis,
                        style.text_primary,
                        style.state_hover_soft,
                    )
                } else {
                    style.border
                },
                sizing.border_width,
            );
            text_runs.push(TextRun {
                text: button.label.to_string(),
                position: Point::new(
                    button.rect.min.x + sizing.text_inset_x,
                    text_top_in_rect(button.rect, sizing.font_meta, sizing.text_inset_y),
                ),
                font_size: sizing.font_meta,
                color: if button.enabled {
                    button.text_color
                } else {
                    style.text_muted
                },
                max_width: Some((button.rect.width() - (sizing.text_inset_x * 2.0)).max(12.0)),
                align: TextAlign::Center,
            });
        }
        if model.sources.rows.len() > rendered_sources {
            text_runs.push(TextRun {
                text: format!("+{} more…", model.sources.rows.len() - rendered_sources),
                position: Point::new(
                    layout.sidebar_footer.min.x + sizing.text_inset_x + sizing.header_label_gutter,
                    layout.sidebar_footer.min.y + sizing.text_inset_y,
                ),
                font_size: sizing.font_meta,
                color: style.text_muted,
                max_width: Some(
                    (layout.sidebar_footer.width() - (sizing.text_inset_x * 2.0)).max(56.0),
                ),
                align: TextAlign::Left,
            });
        }
        if model.sources.folder_rows.len() > rendered_folders {
            text_runs.push(TextRun {
                text: format!(
                    "folders: +{} more…",
                    model.sources.folder_rows.len() - rendered_folders
                ),
                position: Point::new(
                    layout.sidebar_footer.min.x + sizing.text_inset_x + sizing.header_label_gutter,
                    layout.sidebar_footer.min.y
                        + sizing.text_inset_y
                        + sizing.font_meta
                        + sizing.text_row_gap,
                ),
                font_size: sizing.font_meta,
                color: style.text_muted,
                max_width: Some(
                    (layout.sidebar_footer.width() - (sizing.text_inset_x * 2.0)).max(56.0),
                ),
                align: TextAlign::Left,
            });
        } else if model.sources.folder_recovery.entry_count > 0 {
            text_runs.push(TextRun {
                text: format!(
                    "recovery entries: {}",
                    model.sources.folder_recovery.entry_count
                ),
                position: Point::new(
                    layout.sidebar_footer.min.x + sizing.text_inset_x + sizing.header_label_gutter,
                    layout.sidebar_footer.min.y
                        + sizing.text_inset_y
                        + sizing.font_meta
                        + sizing.text_row_gap,
                ),
                font_size: sizing.font_meta,
                color: style.text_muted,
                max_width: Some(
                    (layout.sidebar_footer.width() - (sizing.text_inset_x * 2.0)).max(56.0),
                ),
                align: TextAlign::Left,
            });
        }
        // Waveform summary text is produced during overlay rendering so it can
        // update while transport advances without invalidating the static scene.
        let tabs = browser_tabs_layout(layout, sizing);
        let map_active = model.map.active;
        let list_active = !map_active;
        let (
            samples_fill,
            map_fill,
            samples_border,
            map_border,
            samples_text_color,
            map_text_color,
        ) = if list_active {
            (
                blend_color(
                    style.surface_overlay,
                    style.bg_tertiary,
                    style.state_selected_blend + (motion_wave * 0.1),
                ),
                style.surface_base,
                blend_color(style.accent_mint, style.text_primary, 0.42),
                style.border,
                blend_color(
                    style.accent_mint,
                    style.text_primary,
                    motion_wave * style.state_selected_blend,
                ),
                style.text_muted,
            )
        } else {
            (
                style.surface_base,
                blend_color(
                    style.surface_overlay,
                    style.bg_tertiary,
                    style.state_selected_blend + (motion_wave * 0.1),
                ),
                style.border,
                blend_color(style.accent_mint, style.text_primary, 0.42),
                style.text_muted,
                blend_color(
                    style.accent_mint,
                    style.text_primary,
                    motion_wave * style.state_selected_blend,
                ),
            )
        };
        primitives.push(Primitive::Rect(FillRect {
            rect: tabs.samples,
            color: samples_fill,
        }));
        primitives.push(Primitive::Rect(FillRect {
            rect: tabs.map,
            color: map_fill,
        }));
        push_border(
            primitives,
            tabs.samples,
            samples_border,
            sizing.border_width,
        );
        push_border(primitives, tabs.map, map_border, sizing.border_width);
        let samples_text = format!(
            "{} ({})",
            model.browser_chrome.samples_tab_label,
            model.columns.get(1).map(|c| c.item_count).unwrap_or(0)
        );
        text_runs.push(TextRun {
            text: truncate_to_width(
                &samples_text,
                (tabs.samples.width() - (sizing.text_inset_x * 2.0)).max(40.0),
                sizing.font_header,
            ),
            position: Point::new(
                tabs.samples.min.x + sizing.text_inset_x,
                text_top_in_rect(tabs.samples, sizing.font_header, sizing.text_inset_y),
            ),
            font_size: sizing.font_header,
            color: samples_text_color,
            max_width: Some((tabs.samples.width() - (sizing.text_inset_x * 2.0)).max(40.0)),
            align: TextAlign::Left,
        });
        let map_text = model.browser_chrome.map_tab_label.as_str();
        text_runs.push(TextRun {
            text: String::from(map_text),
            position: Point::new(
                tabs.map.min.x + sizing.text_inset_x,
                text_top_in_rect(tabs.map, sizing.font_header, sizing.text_inset_y),
            ),
            font_size: sizing.font_header,
            color: map_text_color,
            max_width: Some((tabs.map.width() - (sizing.text_inset_x * 2.0)).max(40.0)),
            align: TextAlign::Left,
        });
        let toolbar = browser_toolbar_layout(layout, style, &browser_buttons);
        if toolbar.search_field.width() > 1.0 {
            primitives.push(Primitive::Rect(FillRect {
                rect: toolbar.search_field,
                color: style.surface_overlay,
            }));
            push_border(
                primitives,
                toolbar.search_field,
                blend_color(style.border_emphasis, style.text_primary, 0.35),
                sizing.border_width,
            );
        }
        if toolbar.activity_chip.width() > 1.0 {
            primitives.push(Primitive::Rect(FillRect {
                rect: toolbar.activity_chip,
                color: if model.browser.busy {
                    blend_color(style.accent_warning, style.bg_secondary, 0.45)
                } else {
                    blend_color(style.accent_mint, style.bg_secondary, 0.40)
                },
            }));
            push_border(
                primitives,
                toolbar.activity_chip,
                style.border,
                sizing.border_width,
            );
        }
        if toolbar.sort_chip.width() > 1.0 {
            primitives.push(Primitive::Rect(FillRect {
                rect: toolbar.sort_chip,
                color: style.surface_overlay,
            }));
            push_border(
                primitives,
                toolbar.sort_chip,
                style.border,
                sizing.border_width,
            );
        }
        let search_text = if model.browser.search_query.is_empty() {
            model.browser_chrome.search_placeholder.clone()
        } else {
            format!(
                "{}: {}",
                model.browser_chrome.search_prefix_label, model.browser.search_query
            )
        };
        if toolbar.search_field.width() > 1.0 {
            text_runs.push(TextRun {
                text: truncate_to_width(
                    &search_text,
                    (toolbar.search_field.width() - (sizing.text_inset_x * 2.0)).max(24.0),
                    sizing.font_meta,
                ),
                position: Point::new(
                    toolbar.search_field.min.x + sizing.text_inset_x,
                    text_top_in_rect(toolbar.search_field, sizing.font_meta, sizing.text_inset_y),
                ),
                font_size: sizing.font_meta,
                color: if model.browser.search_query.is_empty() {
                    style.text_muted
                } else {
                    style.text_primary
                },
                max_width: Some(
                    (toolbar.search_field.width() - (sizing.text_inset_x * 2.0)).max(24.0),
                ),
                align: TextAlign::Left,
            });
        }
        if toolbar.activity_chip.width() > 1.0 {
            text_runs.push(TextRun {
                text: if model.browser.busy {
                    model.browser_chrome.activity_busy_label.clone()
                } else {
                    model.browser_chrome.activity_ready_label.clone()
                },
                position: Point::new(
                    toolbar.activity_chip.min.x + sizing.text_inset_x,
                    text_top_in_rect(toolbar.activity_chip, sizing.font_meta, sizing.text_inset_y),
                ),
                font_size: sizing.font_meta,
                color: style.text_primary,
                max_width: Some(
                    (toolbar.activity_chip.width() - (sizing.text_inset_x * 2.0)).max(20.0),
                ),
                align: TextAlign::Center,
            });
        }
        if toolbar.sort_chip.width() > 1.0 {
            let sort_label = if model.browser_chrome.sort_order_label.is_empty() {
                model.browser.sort_label.as_deref().unwrap_or("List order")
            } else {
                model.browser_chrome.sort_order_label.as_str()
            };
            let sort_text = if model.browser_chrome.sort_prefix_label.is_empty() {
                String::from(sort_label)
            } else {
                format!("{}: {}", model.browser_chrome.sort_prefix_label, sort_label)
            };
            text_runs.push(TextRun {
                text: sort_text,
                position: Point::new(
                    toolbar.sort_chip.min.x + sizing.text_inset_x,
                    text_top_in_rect(toolbar.sort_chip, sizing.font_meta, sizing.text_inset_y),
                ),
                font_size: sizing.font_meta,
                color: style.text_muted,
                max_width: Some(
                    (toolbar.sort_chip.width() - (sizing.text_inset_x * 2.0)).max(20.0),
                ),
                align: TextAlign::Center,
            });
        }
        if model.map.active {
            let mode_label = match model.map.render_mode {
                crate::app::MapRenderModeModel::Heatmap => "heatmap",
                crate::app::MapRenderModeModel::Points => "points",
            };
            let legend_text = if model.map.legend_label.is_empty() {
                format!(
                    "{}: {mode_label}",
                    model.browser_chrome.similarity_toggle_label
                )
            } else {
                model.map.legend_label.clone()
            };
            let header_left_text =
                format!("{} | {}", model.browser_chrome.map_tab_label, legend_text);
            let selection_or_error = if let Some(error) = model.map.error.as_deref() {
                (error.to_string(), style.accent_warning)
            } else if !model.map.selection_label.is_empty() {
                (model.map.selection_label.clone(), style.text_muted)
            } else if !model.map.hover_label.is_empty() {
                (model.map.hover_label.clone(), style.text_muted)
            } else {
                (String::from("Selection: —"), style.text_muted)
            };
            text_runs.push(TextRun {
                text: truncate_to_width(
                    &header_left_text,
                    (layout.browser_table_header.width() - (sizing.text_inset_x * 2.0)).max(24.0),
                    sizing.font_meta,
                ),
                position: Point::new(
                    layout.browser_table_header.min.x + sizing.text_inset_x,
                    text_top_in_rect(
                        layout.browser_table_header,
                        sizing.font_meta,
                        sizing.text_inset_y,
                    ),
                ),
                font_size: sizing.font_meta,
                color: style.text_primary,
                max_width: Some(
                    (layout.browser_table_header.width() - (sizing.text_inset_x * 2.0)).max(24.0),
                ),
                align: TextAlign::Left,
            });
            text_runs.push(TextRun {
                text: truncate_to_width(
                    &selection_or_error.0,
                    (layout.browser_table_header.width() * 0.42).max(36.0),
                    sizing.font_meta,
                ),
                position: Point::new(
                    layout.browser_table_header.max.x
                        - ((layout.browser_table_header.width() * 0.42).max(36.0))
                        - sizing.text_inset_x,
                    text_top_in_rect(
                        layout.browser_table_header,
                        sizing.font_meta,
                        sizing.text_inset_y,
                    ),
                ),
                font_size: sizing.font_meta,
                color: selection_or_error.1,
                max_width: Some((layout.browser_table_header.width() * 0.42).max(36.0)),
                align: TextAlign::Right,
            });
        } else {
            let header_text_layout =
                compute_browser_header_text_layout(layout.browser_table_header, sizing);
            let header = header_text_layout.columns;
            for separator_x in [header.index.max.x, header.sample.max.x] {
                primitives.push(Primitive::Rect(FillRect {
                    rect: Rect::from_min_max(
                        Point::new(separator_x, layout.browser_table_header.min.y),
                        Point::new(
                            (separator_x + sizing.border_width)
                                .min(layout.browser_table_header.max.x),
                            layout.browser_table_header.max.y,
                        ),
                    ),
                    color: style.border,
                }));
            }
            text_runs.push(TextRun {
                text: String::from("#"),
                position: header_text_layout.index_label.min,
                font_size: sizing.font_meta,
                color: style.text_muted,
                max_width: Some(header_text_layout.index_label.width().max(12.0)),
                align: TextAlign::Right,
            });
            text_runs.push(TextRun {
                text: String::from("Sample"),
                position: header_text_layout.sample_label.min,
                font_size: sizing.font_meta,
                color: style.text_primary,
                max_width: Some(header_text_layout.sample_label.width().max(24.0)),
                align: TextAlign::Left,
            });
            text_runs.push(TextRun {
                text: String::from("Bucket"),
                position: header_text_layout.bucket_label.min,
                font_size: sizing.font_meta,
                color: style.text_primary,
                max_width: Some(header_text_layout.bucket_label.width().max(20.0)),
                align: TextAlign::Center,
            });
        }
        let footer_text = if model.map.active {
            let mut parts = Vec::new();
            if !model.map.summary.is_empty() {
                parts.push(model.map.summary.clone());
            }
            if !model.map.cluster_label.is_empty() {
                parts.push(model.map.cluster_label.clone());
            }
            if !model.map.hover_label.is_empty() {
                parts.push(model.map.hover_label.clone());
            }
            if !model.map.viewport_label.is_empty() {
                parts.push(model.map.viewport_label.clone());
            }
            if parts.is_empty() {
                model.map.summary.clone()
            } else {
                parts.join(" | ")
            }
        } else {
            format!(
                "{} | {} selected{}",
                model.browser_chrome.item_count_label,
                model.browser.selected_path_count,
                if model.browser.busy {
                    " | filtering…"
                } else {
                    ""
                }
            )
        };
        text_runs.push(TextRun {
            text: truncate_to_width(
                &footer_text,
                (layout.browser_footer.width() - (sizing.text_inset_x * 2.0)).max(36.0),
                sizing.font_meta,
            ),
            position: Point::new(
                layout.browser_footer.min.x + sizing.text_inset_x,
                text_top_in_rect(layout.browser_footer, sizing.font_meta, sizing.text_inset_y),
            ),
            font_size: sizing.font_meta,
            color: style.text_muted,
            max_width: Some(
                (layout.browser_footer.width() - (sizing.text_inset_x * 2.0)).max(36.0),
            ),
            align: TextAlign::Left,
        });

        let status_text = if model.status_text.is_empty() {
            if self.transport_running {
                format!(
                    "Transport: running | Selected column: {}",
                    self.selected_column + 1
                )
            } else {
                format!(
                    "Transport: stopped | Selected column: {}",
                    self.selected_column + 1
                )
            }
        } else {
            model.status_text.clone()
        };
        let browser_summary = format!(
            "rows: {} | selected: {} | anchor: {} | search: {}{}",
            model.browser.visible_count,
            model.browser.selected_path_count,
            model
                .browser
                .anchor_visible_row
                .map(|row| row.to_string())
                .unwrap_or_else(|| String::from("—")),
            if model.browser.search_query.is_empty() {
                "—"
            } else {
                model.browser.search_query.as_str()
            },
            if model.browser.busy {
                " | filtering…"
            } else {
                ""
            }
        );
        let status_left = if model.status.left.is_empty() {
            status_text
        } else {
            model.status.left.clone()
        };
        let status_center = if model.status.center.is_empty() {
            browser_summary
        } else {
            model.status.center.clone()
        };
        let status_right = if model.status.right.is_empty() {
            format!("col: {}/3", self.selected_column + 1)
        } else {
            model.status.right.clone()
        };
        let status_left_text_rect =
            compute_status_text_line_rect(layout.status_left_segment, sizing, sizing.font_status);
        let status_center_text_rect =
            compute_status_text_line_rect(layout.status_center_segment, sizing, sizing.font_status);
        let status_right_text_rect =
            compute_status_text_line_rect(layout.status_right_segment, sizing, sizing.font_status);
        text_runs.push(TextRun {
            text: truncate_to_width(
                &status_left,
                status_left_text_rect.width().max(36.0),
                sizing.font_status,
            ),
            position: status_left_text_rect.min,
            font_size: sizing.font_status,
            color: style.text_muted,
            max_width: Some(status_left_text_rect.width().max(36.0)),
            align: TextAlign::Left,
        });
        text_runs.push(TextRun {
            text: truncate_to_width(
                &status_center,
                status_center_text_rect.width().max(36.0),
                sizing.font_status,
            ),
            position: status_center_text_rect.min,
            font_size: sizing.font_status,
            color: style.text_primary,
            max_width: Some(status_center_text_rect.width().max(36.0)),
            align: TextAlign::Center,
        });
        text_runs.push(TextRun {
            text: truncate_to_width(
                &status_right,
                status_right_text_rect.width().max(36.0),
                sizing.font_status,
            ),
            position: status_right_text_rect.min,
            font_size: sizing.font_status,
            color: style.text_muted,
            max_width: Some(status_right_text_rect.width().max(36.0)),
            align: TextAlign::Right,
        });

        if include_overlays {
            let motion_model = NativeMotionModel::from_app_model(model);
            push_waveform_header_overlay(primitives, text_runs, layout, style, &motion_model);
            render_progress_overlay(primitives, text_runs, layout, style, model);
            render_confirm_prompt(primitives, text_runs, layout, style, model);
            render_drag_overlay(primitives, text_runs, layout, style, model);
        }

        frame.clear_color = style.clear_color;
    }

    /// Build only state-driven overlays into reusable buffers.
    pub(crate) fn build_state_overlay_into(
        &mut self,
        layout: &ShellLayout,
        style: &StyleTokens,
        model: &AppModel,
        frame: &mut NativeViewFrame,
    ) {
        let sizing = style.sizing;
        frame.primitives.clear();
        frame.text_runs.clear();
        let primitives = &mut frame.primitives;
        let text_runs = &mut frame.text_runs;

        if self.hovered == Some(ShellNodeKind::TopBar) {
            primitives.push(Primitive::Rect(FillRect {
                rect: layout.top_bar,
                color: tinted_overlay_color(style.bg_tertiary, style.sizing.hover_fill_alpha),
            }));
        }

        if self.hovered == Some(ShellNodeKind::Sidebar) {
            primitives.push(Primitive::Rect(FillRect {
                rect: layout.sidebar,
                color: tinted_overlay_color(style.bg_tertiary, style.sizing.hover_fill_alpha),
            }));
        }

        if self.hovered == Some(ShellNodeKind::WaveformCard) {
            primitives.push(Primitive::Rect(FillRect {
                rect: layout.waveform_card,
                color: tinted_overlay_color(style.bg_tertiary, style.sizing.hover_fill_alpha),
            }));
        }
        if let Some(hovered_visible_row) = self.hovered_browser_visible_row {
            let browser_rows = self.cached_browser_rows(layout, style, model);
            if let Some(row) = browser_rows
                .iter()
                .find(|row| row.visible_row == hovered_visible_row)
            {
                primitives.push(Primitive::Rect(FillRect {
                    rect: row.rect,
                    color: translucent_overlay_color(
                        style.bg_tertiary,
                        style.grid_soft,
                        style.state_hover_soft,
                    ),
                }));
            }
        }

        if self.has_focus_emphasis {
            {
                let source_row_rects = self.cached_source_row_rects(layout, style, model);
                for (row_index, row_rect) in source_row_rects.iter().enumerate() {
                    let Some(row) = model.sources.rows.get(row_index) else {
                        continue;
                    };
                    let row_selected = row.selected
                        || model
                            .sources
                            .selected_row
                            .is_some_and(|selected| selected == row_index);
                    if !row_selected {
                        continue;
                    }
                    push_border(
                        primitives,
                        *row_rect,
                        blend_color(
                            style.accent_mint,
                            style.text_primary,
                            style.state_selected_blend,
                        ),
                        sizing.border_width,
                    );
                }
            }

            {
                let folder_row_rects = self.cached_folder_row_rects(layout, style, model);
                for (row_index, row_rect) in folder_row_rects.iter().enumerate() {
                    let Some(row) = model.sources.folder_rows.get(row_index) else {
                        continue;
                    };
                    if !(row.selected || row.focused) {
                        continue;
                    }
                    if row.focused {
                        primitives.push(Primitive::Rect(FillRect {
                            rect: *row_rect,
                            color: translucent_overlay_color(
                                style.bg_tertiary,
                                style.grid_strong,
                                style.state_focus_pulse_blend,
                            ),
                        }));
                    }
                    if row.focused || row.selected {
                        push_border(
                            primitives,
                            *row_rect,
                            if row.focused {
                                blend_color(
                                    style.accent_warning,
                                    style.text_primary,
                                    style.state_focus_pulse_blend,
                                )
                            } else {
                                blend_color(
                                    style.accent_mint,
                                    style.text_primary,
                                    style.state_selected_blend,
                                )
                            },
                            if row.focused {
                                sizing.focus_stroke_width
                            } else {
                                sizing.border_width
                            },
                        );
                    }
                    if row.focused {
                        let glyph = if row.is_root {
                            "•"
                        } else if row.has_children {
                            if row.expanded { "▼" } else { "▶" }
                        } else {
                            "·"
                        };
                        let depth_indent = (row.depth as f32 * sizing.folder_indent_step)
                            .min((row_rect.width() * 0.45).max(0.0));
                        let row_text_width = (row_rect.width()
                            - ((sizing.text_inset_x + sizing.row_corner_inset) * 2.0)
                            - depth_indent)
                            .max(24.0);
                        let row_label = format!("{glyph} {}", row.label);
                        text_runs.push(TextRun {
                            text: truncate_to_width(&row_label, row_text_width, sizing.font_body),
                            position: Point::new(
                                row_label_x(*row_rect, &sizing, depth_indent),
                                text_top_in_rect(*row_rect, sizing.font_body, sizing.text_inset_y),
                            ),
                            font_size: sizing.font_body,
                            color: blend_color(
                                style.accent_warning,
                                style.text_primary,
                                style.state_focus_pulse_blend,
                            ),
                            max_width: Some(row_text_width),
                            align: TextAlign::Left,
                        });
                    }
                }
            }

            {
                let browser_rows = self.cached_browser_rows(layout, style, model);
                for row in browser_rows.iter() {
                    if !(row.selected || row.focused) {
                        continue;
                    }
                    let row_text_layout = compute_browser_row_text_layout(row.rect, sizing);
                    if row.focused {
                        primitives.push(Primitive::Rect(FillRect {
                            rect: row.rect,
                            color: translucent_overlay_color(
                                style.bg_tertiary,
                                style.grid_strong,
                                style.state_focus_pulse_blend,
                            ),
                        }));
                    }
                    push_border(
                        primitives,
                        row.rect,
                        if row.focused {
                            blend_color(
                                style.accent_warning,
                                style.text_primary,
                                style.state_focus_pulse_blend,
                            )
                        } else {
                            blend_color(
                                style.accent_mint,
                                style.text_primary,
                                style.state_selected_blend,
                            )
                        },
                        if row.focused {
                            sizing.focus_stroke_width
                        } else {
                            sizing.border_width
                        },
                    );
                    if row.focused {
                        text_runs.push(TextRun {
                            text: row.visible_row.to_string(),
                            position: row_text_layout.index_label.min,
                            font_size: sizing.font_meta,
                            color: blend_color(
                                style.accent_warning,
                                style.text_primary,
                                style.state_focus_pulse_blend,
                            ),
                            max_width: Some(row_text_layout.index_label.width().max(12.0)),
                            align: TextAlign::Right,
                        });
                        text_runs.push(TextRun {
                            text: row.label.clone(),
                            position: row_text_layout.sample_label.min,
                            font_size: sizing.font_body,
                            color: blend_color(
                                style.accent_warning,
                                style.text_primary,
                                style.state_focus_pulse_blend,
                            ),
                            max_width: Some(row_text_layout.sample_label.width().max(20.0)),
                            align: TextAlign::Left,
                        });
                    }
                }
            }
        }

        let tabs = browser_tabs_layout(layout, sizing);
        let (samples_fill, map_fill, samples_text_color, map_text_color) = if !model.map.active {
            (
                blend_color(
                    style.surface_overlay,
                    style.bg_tertiary,
                    style.state_selected_blend + 0.1,
                ),
                style.surface_base,
                blend_color(
                    style.accent_mint,
                    style.text_primary,
                    style.state_selected_blend,
                ),
                style.text_muted,
            )
        } else {
            (
                style.surface_base,
                blend_color(
                    style.surface_overlay,
                    style.bg_tertiary,
                    style.state_selected_blend + 0.1,
                ),
                style.text_muted,
                blend_color(
                    style.accent_mint,
                    style.text_primary,
                    style.state_selected_blend,
                ),
            )
        };
        primitives.push(Primitive::Rect(FillRect {
            rect: tabs.samples,
            color: samples_fill,
        }));
        primitives.push(Primitive::Rect(FillRect {
            rect: tabs.map,
            color: map_fill,
        }));
        push_border(primitives, tabs.samples, style.border, sizing.border_width);
        push_border(
            primitives,
            tabs.map,
            blend_color(style.accent_mint, style.text_primary, 0.42),
            sizing.border_width,
        );
        let samples_text = format!(
            "{} ({})",
            model.browser_chrome.samples_tab_label,
            model
                .columns
                .get(1)
                .map(|column| column.item_count)
                .unwrap_or(0)
        );
        text_runs.push(TextRun {
            text: truncate_to_width(
                &samples_text,
                (tabs.samples.width() - (sizing.text_inset_x * 2.0)).max(40.0),
                sizing.font_header,
            ),
            position: Point::new(
                tabs.samples.min.x + sizing.text_inset_x,
                text_top_in_rect(tabs.samples, sizing.font_header, sizing.text_inset_y),
            ),
            font_size: sizing.font_header,
            color: samples_text_color,
            max_width: Some((tabs.samples.width() - (sizing.text_inset_x * 2.0)).max(40.0)),
            align: TextAlign::Left,
        });
        text_runs.push(TextRun {
            text: String::from(model.browser_chrome.map_tab_label.as_str()),
            position: Point::new(
                tabs.map.min.x + sizing.text_inset_x,
                text_top_in_rect(tabs.map, sizing.font_header, sizing.text_inset_y),
            ),
            font_size: sizing.font_header,
            color: map_text_color,
            max_width: Some((tabs.map.width() - (sizing.text_inset_x * 2.0)).max(40.0)),
            align: TextAlign::Left,
        });

        render_progress_overlay(primitives, text_runs, layout, style, model);
        render_confirm_prompt(primitives, text_runs, layout, style, model);
        render_drag_overlay(primitives, text_runs, layout, style, model);

        frame.clear_color = style.clear_color;
    }

    /// Build only motion-sensitive overlays into reusable buffers.
    pub(crate) fn build_motion_overlay_into(
        &mut self,
        layout: &ShellLayout,
        style: &StyleTokens,
        model: &NativeMotionModel,
        frame: &mut NativeViewFrame,
    ) {
        let sizing = style.sizing;
        let motion_wave = interaction_wave(self.pulse_phase);
        frame.primitives.clear();
        frame.text_runs.clear();
        let primitives = &mut frame.primitives;
        let text_runs = &mut frame.text_runs;

        let lamp_radius = sizing.lamp_radius_base + (motion_wave * sizing.lamp_radius_amp);
        let lamp_color = if self.transport_running {
            style.accent_mint
        } else {
            style.accent_copper
        };
        primitives.push(Primitive::Circle(FillCircle {
            center: Point::new(
                layout.top_bar.max.x - (sizing.text_inset_x + 14.0),
                layout.top_bar_title_row.min.y + (layout.top_bar_title_row.height() * 0.5),
            ),
            radius: lamp_radius,
            color: lamp_color,
        }));

        push_waveform_playhead_overlay(primitives, layout, style, model);
        push_waveform_header_overlay(primitives, text_runs, layout, style, model);

        let tabs = browser_tabs_layout(layout, sizing);
        let (samples_fill, map_fill) = if !model.map_active {
            (
                blend_color(
                    style.surface_overlay,
                    style.bg_tertiary,
                    style.state_selected_blend + (motion_wave * 0.1),
                ),
                style.surface_base,
            )
        } else {
            (
                style.surface_base,
                blend_color(
                    style.surface_overlay,
                    style.bg_tertiary,
                    style.state_selected_blend + (motion_wave * 0.1),
                ),
            )
        };
        primitives.push(Primitive::Rect(FillRect {
            rect: tabs.samples,
            color: samples_fill,
        }));
        primitives.push(Primitive::Rect(FillRect {
            rect: tabs.map,
            color: map_fill,
        }));
        push_border(primitives, tabs.samples, style.border, sizing.border_width);
        push_border(
            primitives,
            tabs.map,
            blend_color(style.accent_mint, style.text_primary, 0.42),
            sizing.border_width,
        );
        Self::push_status_right_motion_overlay(
            primitives,
            text_runs,
            layout,
            style,
            &model.status_right,
        );

        frame.clear_color = style.clear_color;
    }

    /// Build a native frame using default style tokens.
    pub(crate) fn build_frame(
        &mut self,
        layout: &ShellLayout,
        model: &AppModel,
    ) -> NativeViewFrame {
        self.build_frame_with_style(layout, &style_for_layout(layout), model)
    }

    fn push_status_right_motion_overlay(
        primitives: &mut Vec<Primitive>,
        text_runs: &mut Vec<TextRun>,
        layout: &ShellLayout,
        style: &StyleTokens,
        status_right: &str,
    ) {
        if status_right.is_empty() {
            return;
        }
        primitives.push(Primitive::Rect(FillRect {
            rect: layout.status_right_segment,
            color: style.surface_raised,
        }));
        let sizing = style.sizing;
        let status_text_rect =
            compute_status_text_line_rect(layout.status_right_segment, sizing, sizing.font_status);
        text_runs.push(TextRun {
            text: truncate_to_width(
                status_right,
                status_text_rect.width().max(36.0),
                sizing.font_status,
            ),
            position: status_text_rect.min,
            font_size: sizing.font_status,
            color: style.text_muted,
            max_width: Some(status_text_rect.width().max(36.0)),
            align: TextAlign::Right,
        });
    }

    fn cached_source_row_rects(
        &mut self,
        layout: &ShellLayout,
        style: &StyleTokens,
        model: &AppModel,
    ) -> &[Rect] {
        let cache_key = sidebar_rows_cache_key(layout, style, model);
        if self.source_row_cache_key != Some(cache_key) {
            self.source_row_rects = rendered_source_row_rects(layout, style, model);
            self.source_row_cache_key = Some(cache_key);
            self.folder_row_cache_key = Some(cache_key);
        }
        &self.source_row_rects
    }

    fn cached_folder_row_rects(
        &mut self,
        layout: &ShellLayout,
        style: &StyleTokens,
        model: &AppModel,
    ) -> &[Rect] {
        let cache_key = sidebar_rows_cache_key(layout, style, model);
        if self.folder_row_cache_key != Some(cache_key) {
            self.folder_row_rects = rendered_folder_row_rects(layout, style, model);
            self.folder_row_cache_key = Some(cache_key);
            self.source_row_cache_key = Some(cache_key);
        }
        &self.folder_row_rects
    }

    fn cached_browser_rows(
        &mut self,
        layout: &ShellLayout,
        style: &StyleTokens,
        model: &AppModel,
    ) -> &[CachedBrowserRow] {
        let cache_key = browser_rows_cache_key(layout, style, model);
        if self.browser_rows_cache_key != Some(cache_key) {
            self.browser_rows = rendered_browser_rows(layout, model, style);
            self.browser_rows_cache_key = Some(cache_key);
        }
        &self.browser_rows
    }
}

fn push_waveform_playhead_overlay(
    primitives: &mut Vec<Primitive>,
    layout: &ShellLayout,
    style: &StyleTokens,
    model: &NativeMotionModel,
) {
    let sizing = style.sizing;
    let waveform_inner = layout.waveform_plot;
    let to_x = |milli: u16| {
        waveform_inner.min.x + waveform_inner.width() * (f32::from(milli.min(1000)) / 1000.0)
    };
    let to_rect = |x: f32| {
        Rect::from_min_max(
            Point::new(x, waveform_inner.min.y),
            Point::new(
                (x + sizing.border_width.max(1.0)).min(waveform_inner.max.x),
                waveform_inner.max.y,
            ),
        )
    };

    if let Some(selection) = model.waveform_selection_milli {
        let start_ratio = f32::from(selection.start_milli.min(1000)) / 1000.0;
        let end_ratio = f32::from(selection.end_milli.min(1000)) / 1000.0;
        let start_x = waveform_inner.min.x + waveform_inner.width() * start_ratio.min(end_ratio);
        let end_x = waveform_inner.min.x + waveform_inner.width() * start_ratio.max(end_ratio);
        let rect = Rect::from_min_max(
            Point::new(start_x, waveform_inner.min.y),
            Point::new(end_x.max(start_x + 1.0), waveform_inner.max.y),
        );
        primitives.push(Primitive::Rect(FillRect {
            rect,
            color: style.grid_strong,
        }));
        push_border(primitives, rect, style.accent_mint, sizing.border_width);
    }

    if let Some(cursor_milli) = model.waveform_cursor_milli {
        primitives.push(Primitive::Rect(FillRect {
            rect: to_rect(to_x(cursor_milli)),
            color: style.accent_warning,
        }));
    }
    if let Some(playhead_milli) = model.waveform_playhead_milli {
        primitives.push(Primitive::Rect(FillRect {
            rect: to_rect(to_x(playhead_milli)),
            color: style.accent_copper,
        }));
    }
}

fn push_waveform_image(
    primitives: &mut Vec<Primitive>,
    waveform_plot: Rect,
    image: Option<&ImageRgba>,
) {
    let Some(image) = image else {
        return;
    };
    if image.width == 0
        || image.height == 0
        || waveform_plot.width() <= 0.0
        || waveform_plot.height() <= 0.0
    {
        return;
    }

    let plot_width = waveform_plot.width();
    let plot_height = waveform_plot.height();
    let src_width = image.width as f32;
    let src_height = image.height as f32;
    let stride = image.width.saturating_mul(4);

    for x in 0..image.width {
        let x0 = waveform_plot.min.x + (x as f32 * plot_width) / src_width;
        let x1 = waveform_plot.min.x + ((x + 1) as f32 * plot_width) / src_width;
        let mut y = 0usize;
        while y < image.height {
            let first_idx = y * stride + x * 4;
            let y0 = y;
            if image.pixels[first_idx + 3] == 0 {
                y += 1;
                continue;
            }
            let mut y1 = y0;
            let mut red = image.pixels[first_idx];
            let mut green = image.pixels[first_idx + 1];
            let mut blue = image.pixels[first_idx + 2];
            let mut alpha = image.pixels[first_idx + 3];

            while y1 < image.height {
                let span_idx = y1 * stride + x * 4;
                if image.pixels[span_idx + 3] == 0 {
                    break;
                }
                let span_alpha = image.pixels[span_idx + 3];
                if span_alpha > alpha {
                    alpha = span_alpha;
                    red = image.pixels[span_idx];
                    green = image.pixels[span_idx + 1];
                    blue = image.pixels[span_idx + 2];
                }
                y1 += 1;
            }

            let top = waveform_plot.min.y + (y0 as f32 / src_height) * plot_height;
            let bottom = waveform_plot.min.y + (y1 as f32 / src_height) * plot_height;
            if bottom > top {
                primitives.push(Primitive::Rect(FillRect {
                    rect: Rect::from_min_max(
                        Point::new(x0, top),
                        Point::new(x1.min(waveform_plot.max.x), bottom.min(waveform_plot.max.y)),
                    ),
                    color: Rgba8 {
                        r: red,
                        g: green,
                        b: blue,
                        a: alpha,
                    },
                }));
            }
            y = y1 + 1;
        }
    }
}

fn push_waveform_header_overlay(
    primitives: &mut Vec<Primitive>,
    text_runs: &mut Vec<TextRun>,
    layout: &ShellLayout,
    style: &StyleTokens,
    model: &NativeMotionModel,
) {
    let sizing = style.sizing;
    let text_layout = compute_waveform_header_text_layout(layout.waveform_header, sizing);
    primitives.push(Primitive::Rect(FillRect {
        rect: layout.waveform_header,
        color: style.surface_raised,
    }));
    let title_max_width = text_layout.title_row.width().max(72.0);
    text_runs.push(TextRun {
        text: truncate_to_width(
            model.waveform_loaded_label.as_deref().unwrap_or("Waveform"),
            title_max_width,
            sizing.font_header,
        ),
        position: text_layout.title_row.min,
        font_size: sizing.font_header,
        color: style.text_primary,
        max_width: Some(title_max_width),
        align: TextAlign::Left,
    });
    let playhead_text = model
        .waveform_playhead_milli
        .map(format_milli_value)
        .unwrap_or_else(|| String::from("—"));
    let cursor_text = model
        .waveform_cursor_milli
        .map(format_milli_value)
        .unwrap_or_else(|| String::from("—"));
    let view_text = format!(
        "{}..{}",
        format_milli_value(model.waveform_view_start_milli),
        format_milli_value(model.waveform_view_end_milli)
    );
    let tempo_text = model.waveform_tempo_label.as_deref().unwrap_or("— BPM");
    let zoom_text = model.waveform_zoom_label.as_deref().unwrap_or("100%");
    let metadata_max_width = text_layout.metadata_row.width().max(72.0);
    text_runs.push(TextRun {
        text: format!(
            "{} | tempo: {} | zoom: {} | playhead: {} | cursor: {} | view: {}",
            model.waveform_transport_hint,
            tempo_text,
            zoom_text,
            playhead_text,
            cursor_text,
            view_text,
        ),
        position: text_layout.metadata_row.min,
        font_size: sizing.font_meta,
        color: style.text_muted,
        max_width: Some(metadata_max_width),
        align: TextAlign::Left,
    });
}

#[derive(Clone, Debug, PartialEq)]
struct CachedBrowserRow {
    visible_row: usize,
    label: String,
    bucket_label: String,
    column: usize,
    selected: bool,
    focused: bool,
    rect: Rect,
}

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
struct SidebarRowsCacheKey {
    root_min_x: u32,
    root_min_y: u32,
    root_max_x: u32,
    root_max_y: u32,
    sidebar_rows_min_x: u32,
    sidebar_rows_min_y: u32,
    sidebar_rows_max_x: u32,
    sidebar_rows_max_y: u32,
    sidebar_section_gap: u32,
    panel_section_padding_top: u32,
    panel_section_padding_bottom: u32,
    source_rows_min_when_split: u32,
    folder_rows_min: u32,
    source_rows: u32,
    folder_rows: u32,
    source_row_height: u32,
    source_row_gap: u32,
    folder_row_height: u32,
    folder_row_gap: u32,
    folder_header_block_height: u32,
    ui_scale: u32,
}

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
struct BrowserRowsCacheKey {
    root_min_x: u32,
    root_min_y: u32,
    root_max_x: u32,
    root_max_y: u32,
    browser_rows_min_x: u32,
    browser_rows_min_y: u32,
    browser_rows_max_x: u32,
    browser_rows_max_y: u32,
    browser_row_height: u32,
    browser_row_gap: u32,
    browser_rows_max_per_column: u32,
    row_capacity: u32,
    browser_row_count: u32,
    selected_visible_row: u32,
    anchor_visible_row: u32,
    focused_visible_row: u32,
    selected_visible_hint: u32,
    map_active: u32,
    visible_count: u32,
    window_start: u32,
    ui_scale: u32,
}

#[derive(Clone, Debug)]
struct ActionButton {
    rect: Rect,
    label: &'static str,
    enabled: bool,
    action: UiAction,
    text_color: Rgba8,
}

#[derive(Clone, Copy, Debug)]
struct SidebarSections {
    source_rows: Rect,
    folder_header: Rect,
    folder_rows: Rect,
}

#[derive(Clone, Copy, Debug)]
struct BrowserTabsLayout {
    samples: Rect,
    map: Rect,
}

#[derive(Clone, Copy, Debug)]
struct BrowserToolbarLayout {
    search_field: Rect,
    activity_chip: Rect,
    sort_chip: Rect,
}

#[derive(Clone, Copy, Debug)]
struct TopBarControlsLayout {
    active: bool,
    options_label: Rect,
    volume_meter: Rect,
    volume_value: Rect,
    volume_label: Rect,
}

fn format_milli_value(value: u16) -> String {
    format!("{:.3}", f32::from(value.min(1000)) / 1000.0)
}

fn volume_action_for_meter(volume_meter: Rect, point: Point) -> UiAction {
    let width = volume_meter.width().max(1.0);
    let clamped_x = point.x.clamp(volume_meter.min.x, volume_meter.max.x);
    let ratio = ((clamped_x - volume_meter.min.x) / width).clamp(0.0, 1.0);
    UiAction::SetVolume {
        value_milli: (ratio * 1000.0).round() as u16,
    }
}

fn rendered_source_rows(style: &StyleTokens, model: &AppModel) -> usize {
    model.sources.rows.len().min(style.sizing.source_rows_max)
}

fn sidebar_rows_cache_key(
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

fn rendered_folder_rows(style: &StyleTokens, model: &AppModel) -> usize {
    model
        .sources
        .folder_rows
        .len()
        .min(style.sizing.folder_rows_max)
}

fn rendered_source_row_rects(
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

fn rendered_folder_row_rects(
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

fn interaction_wave(pulse_phase: f32) -> f32 {
    ((pulse_phase.sin() + 1.0) * 0.5).clamp(0.0, 1.0)
}

fn focus_fill_blend(style: &StyleTokens, motion_wave: f32) -> f32 {
    (style.state_focus_pulse_blend + (motion_wave * style.motion_focus_wave_amp)).clamp(0.0, 1.0)
}

fn focus_text_blend(style: &StyleTokens, motion_wave: f32) -> f32 {
    (style.state_focus_pulse_blend + (motion_wave * style.motion_focus_text_wave_amp))
        .clamp(0.0, 1.0)
}

fn tinted_overlay_color(color: Rgba8, alpha: f32) -> Rgba8 {
    let alpha = (alpha.clamp(0.0, 1.0) * (color.a as f32 / 255.0) * 255.0)
        .round()
        .clamp(0.0, 255.0);
    Rgba8 {
        r: color.r,
        g: color.g,
        b: color.b,
        a: alpha as u8,
    }
}

fn translucent_overlay_color(base: Rgba8, tint: Rgba8, amount: f32) -> Rgba8 {
    let amount = amount.clamp(0.0, 1.0);
    let mut color = blend_color(base, tint, amount);
    color.a = (amount * (base.a as f32 / 255.0) * (tint.a as f32 / 255.0) * 255.0)
        .round()
        .clamp(0.0, 255.0) as u8;
    color
}

fn blend_color(a: Rgba8, b: Rgba8, amount: f32) -> Rgba8 {
    let amount = amount.clamp(0.0, 1.0);
    let mix = |x: u8, y: u8| -> u8 {
        ((x as f32) + ((y as f32 - x as f32) * amount))
            .round()
            .clamp(0.0, 255.0) as u8
    };
    Rgba8 {
        r: mix(a.r, b.r),
        g: mix(a.g, b.g),
        b: mix(a.b, b.b),
        a: mix(a.a, b.a),
    }
}

fn truncate_to_width(text: &str, max_width: f32, font_size: f32) -> String {
    let max_width = max_width.max(0.0);
    let approx_char_width = (font_size * 0.56).max(1.0);
    let max_chars = (max_width / approx_char_width).floor() as usize;
    if max_chars == 0 {
        return String::new();
    }
    let mut chars = text.chars();
    let mut output = String::with_capacity(max_chars);
    for _ in 0..max_chars {
        match chars.next() {
            Some(ch) => output.push(ch),
            None => return output,
        }
    }
    if chars.next().is_none() {
        return output;
    }
    if max_chars <= 3 {
        return ".".repeat(max_chars);
    }
    let truncated_chars = max_chars.saturating_sub(3);
    let new_len = output
        .char_indices()
        .nth(truncated_chars)
        .map_or(output.len(), |(idx, _)| idx);
    output.truncate(new_len);
    output.push_str("...");
    output
}

fn row_index_from_stacked_rows(
    rows: &[Rect],
    point: Point,
    row_height: f32,
    row_gap: f32,
) -> Option<usize> {
    if rows.is_empty() || row_height <= 0.0 {
        return None;
    }
    let stride = (row_height + row_gap.max(0.0)).max(1.0);
    if point.x < rows[0].min.x || point.x > rows[0].max.x {
        return None;
    }
    if point.y < rows[0].min.y {
        return None;
    }
    let index = ((point.y - rows[0].min.y) / stride).floor() as usize;
    rows.get(index)
        .filter(|rect| rect.contains(point))
        .map(|_| index)
}

fn row_index_for_visible_rows(
    rows: &[CachedBrowserRow],
    point: Point,
    browser_rows: Rect,
    sizing: SizingTokens,
) -> Option<usize> {
    if rows.is_empty() || sizing.browser_row_height <= 0.0 || !browser_rows.contains(point) {
        return None;
    }
    let stride = (sizing.browser_row_height + sizing.browser_row_gap.max(0.0)).max(1.0);
    let index = ((point.y - rows[0].rect.min.y) / stride).floor() as usize;
    rows.get(index)
        .filter(|row| row.rect.contains(point))
        .map(|_| index)
}

fn browser_rows_cache_key(
    layout: &ShellLayout,
    style: &StyleTokens,
    model: &AppModel,
) -> BrowserRowsCacheKey {
    let sizing = style.sizing;
    let rows = model.browser.rows.as_slice();
    let row_capacity = browser_rows_capacity(layout.browser_rows, sizing) as u32;
    let selected_visible_row = model.browser.selected_visible_row.unwrap_or(usize::MAX);
    let anchor_visible_row = model.browser.anchor_visible_row.unwrap_or(usize::MAX);
    let focused_visible_row = rows
        .iter()
        .find(|row| row.focused)
        .map(|row| row.visible_row as u32)
        .unwrap_or(u32::MAX);
    let selected_visible_hint = rows
        .iter()
        .find(|row| row.selected)
        .map(|row| row.visible_row as u32)
        .unwrap_or(u32::MAX);
    let row_start = if model.map.active || rows.is_empty() {
        0
    } else {
        browser_window_start(
            rows,
            browser_rows_capacity(layout.browser_rows, sizing),
            model.browser.selected_visible_row,
            model.browser.anchor_visible_row,
        ) as u32
    };
    BrowserRowsCacheKey {
        root_min_x: f32_to_bits(layout.root.rect.min.x),
        root_min_y: f32_to_bits(layout.root.rect.min.y),
        root_max_x: f32_to_bits(layout.root.rect.max.x),
        root_max_y: f32_to_bits(layout.root.rect.max.y),
        browser_rows_min_x: f32_to_bits(layout.browser_rows.min.x),
        browser_rows_min_y: f32_to_bits(layout.browser_rows.min.y),
        browser_rows_max_x: f32_to_bits(layout.browser_rows.max.x),
        browser_rows_max_y: f32_to_bits(layout.browser_rows.max.y),
        browser_row_height: f32_to_bits(sizing.browser_row_height),
        browser_row_gap: f32_to_bits(sizing.browser_row_gap),
        browser_rows_max_per_column: usize_to_u32(sizing.browser_rows_max_per_column),
        row_capacity,
        browser_row_count: rows.len() as u32,
        selected_visible_row: usize_to_u32(selected_visible_row),
        anchor_visible_row: usize_to_u32(anchor_visible_row),
        focused_visible_row,
        selected_visible_hint,
        map_active: model.map.active as u32,
        visible_count: model.browser.visible_count as u32,
        window_start: row_start,
        ui_scale: f32_to_bits(layout.ui_scale),
    }
}

fn usize_to_u32(value: usize) -> u32 {
    u32::try_from(value).unwrap_or(u32::MAX)
}

fn f32_to_bits(value: f32) -> u32 {
    value.to_bits()
}

fn rendered_browser_rows(
    layout: &ShellLayout,
    model: &AppModel,
    style: &StyleTokens,
) -> Vec<CachedBrowserRow> {
    let sizing = style.sizing;
    if model.map.active || model.browser.rows.is_empty() {
        return Vec::new();
    }

    let window_len = browser_rows_capacity(layout.browser_rows, sizing);
    let window_start = browser_window_start(
        &model.browser.rows,
        window_len,
        model.browser.selected_visible_row,
        model.browser.anchor_visible_row,
    );
    let window_end = (window_start + window_len).min(model.browser.rows.len());
    let window = &model.browser.rows[window_start..window_end];

    let mut rendered = Vec::with_capacity(window.len());
    for (row, rect) in window.iter().zip(build_stacked_rows(
        layout.browser_rows,
        window.len(),
        sizing.browser_row_gap,
        sizing.browser_row_height,
    )) {
        let label_width = row_label_width(rect, &sizing, 0.0, 20.0);
        rendered.push(CachedBrowserRow {
            visible_row: row.visible_row,
            label: truncate_to_width(&row.label, label_width, sizing.font_body),
            bucket_label: row
                .bucket_label
                .clone()
                .unwrap_or_else(|| match row.column {
                    0 => String::from("TRASH"),
                    2 => String::from("KEEP"),
                    _ => String::from("SAMPLE"),
                }),
            column: row.column.min(2),
            selected: row.selected,
            focused: row.focused,
            rect,
        });
    }
    rendered
}

fn browser_rows_capacity(table_rows_rect: Rect, sizing: SizingTokens) -> usize {
    let row_height = sizing.browser_row_height.max(1.0);
    let row_gap = sizing.browser_row_gap.max(0.0);
    let geometric_capacity = ((table_rows_rect.height() + row_gap) / (row_height + row_gap))
        .floor()
        .max(1.0) as usize;
    geometric_capacity
        .max(1)
        .min(sizing.browser_rows_max_per_column.max(1))
}

fn browser_window_start(
    rows: &[BrowserRowModel],
    window_len: usize,
    selected_visible_row: Option<usize>,
    anchor_visible_row: Option<usize>,
) -> usize {
    if rows.len() <= window_len {
        return 0;
    }
    let focus_index = selected_visible_row
        .and_then(|target| rows.iter().position(|row| row.visible_row == target))
        .or_else(|| {
            anchor_visible_row
                .and_then(|target| rows.iter().position(|row| row.visible_row == target))
        })
        .or_else(|| rows.iter().position(|row| row.focused))
        .or_else(|| rows.iter().position(|row| row.selected))
        .unwrap_or(0);
    let half = window_len / 2;
    let max_start = rows.len() - window_len;
    focus_index.saturating_sub(half).min(max_start)
}

fn browser_tabs_layout(layout: &ShellLayout, sizing: SizingTokens) -> BrowserTabsLayout {
    let gap = sizing.action_button_gap.max(1.0);
    let tab_width = ((layout.browser_tabs.width() - gap).max(0.0) * 0.5).max(64.0);
    let samples = Rect::from_min_max(
        layout.browser_tabs.min,
        Point::new(
            (layout.browser_tabs.min.x + tab_width).min(layout.browser_tabs.max.x),
            layout.browser_tabs.max.y,
        ),
    );
    let map = Rect::from_min_max(
        Point::new(
            (samples.max.x + gap).min(layout.browser_tabs.max.x),
            layout.browser_tabs.min.y,
        ),
        layout.browser_tabs.max,
    );
    BrowserTabsLayout { samples, map }
}

fn map_canvas_rect(browser_rows: Rect, sizing: SizingTokens) -> Rect {
    browser_rows.inset((sizing.text_inset_x * 0.5).max(2.0))
}

fn map_point_center(canvas: Rect, point: &crate::app::MapPointModel) -> Point {
    let x_ratio = f32::from(point.x_milli.min(1000)) / 1000.0;
    let y_ratio = f32::from(point.y_milli.min(1000)) / 1000.0;
    Point::new(
        canvas.min.x + (canvas.width() * x_ratio),
        canvas.min.y + (canvas.height() * y_ratio),
    )
}

fn map_point_color(style: &StyleTokens, point: &crate::app::MapPointModel) -> Rgba8 {
    if point.focused {
        return style.accent_warning;
    }
    if point.selected {
        return style.accent_mint;
    }
    match point.cluster_id.map(|id| id.rem_euclid(5)) {
        Some(0) => blend_color(style.accent_mint, style.bg_secondary, 0.42),
        Some(1) => blend_color(style.accent_copper, style.bg_secondary, 0.42),
        Some(2) => blend_color(style.accent_warning, style.bg_secondary, 0.42),
        Some(3) => blend_color(style.text_primary, style.bg_secondary, 0.35),
        Some(_) => blend_color(style.text_muted, style.bg_secondary, 0.35),
        None => blend_color(style.text_muted, style.bg_secondary, 0.5),
    }
}

fn map_sample_id_at_point(layout: &ShellLayout, model: &AppModel, point: Point) -> Option<String> {
    if !model.map.active || model.map.points.is_empty() {
        return None;
    }
    let canvas = map_canvas_rect(layout.browser_rows, style_for_layout(layout).sizing);
    if !canvas.contains(point) {
        return None;
    }

    let mut best: Option<(f32, &str)> = None;
    for map_point in &model.map.points {
        let center = map_point_center(canvas, map_point);
        let radius = if map_point.focused {
            7.0
        } else if map_point.selected {
            6.0
        } else {
            5.0
        };
        let dx = point.x - center.x;
        let dy = point.y - center.y;
        let distance_sq = (dx * dx) + (dy * dy);
        if distance_sq > (radius * radius) {
            continue;
        }
        match best {
            Some((best_distance_sq, _)) if distance_sq >= best_distance_sq => {}
            _ => best = Some((distance_sq, map_point.sample_id.as_str())),
        }
    }
    best.map(|(_, sample_id)| sample_id.to_string())
}

fn update_status_text(model: &AppModel) -> String {
    if !model.update.status_label.is_empty() {
        return model.update.status_label.clone();
    }
    match model.update.status {
        crate::app::UpdateStatusModel::Idle => String::from("Updates: idle"),
        crate::app::UpdateStatusModel::Checking => String::from("Checking updates..."),
        crate::app::UpdateStatusModel::Available => model
            .update
            .available_tag
            .as_deref()
            .map(|tag| format!("Update available: {tag}"))
            .unwrap_or_else(|| String::from("Update available")),
        crate::app::UpdateStatusModel::Error => model
            .update
            .last_error
            .as_deref()
            .map(|err| format!("Update check failed: {err}"))
            .unwrap_or_else(|| String::from("Update check failed")),
    }
}

fn update_hint_text(model: &AppModel) -> String {
    if !model.update.action_hint_label.is_empty() {
        return model.update.action_hint_label.clone();
    }
    match model.update.status {
        crate::app::UpdateStatusModel::Idle => String::from("Action: check"),
        crate::app::UpdateStatusModel::Checking => String::from("Action: waiting"),
        crate::app::UpdateStatusModel::Available => {
            if model.update.available_url.is_some() {
                String::from("Actions: open | install | dismiss")
            } else {
                String::from("Action: dismiss")
            }
        }
        crate::app::UpdateStatusModel::Error => String::from("Action: retry"),
    }
}

fn update_notes_text(model: &AppModel) -> String {
    if !model.update.release_notes_label.is_empty() {
        return model.update.release_notes_label.clone();
    }
    String::new()
}

fn update_action_buttons(
    layout: &ShellLayout,
    style: &StyleTokens,
    model: &AppModel,
) -> Vec<ActionButton> {
    let specs: Vec<(&'static str, bool, UiAction)> = match model.update.status {
        crate::app::UpdateStatusModel::Idle => {
            vec![("Check", true, UiAction::CheckForUpdates)]
        }
        crate::app::UpdateStatusModel::Checking => Vec::new(),
        crate::app::UpdateStatusModel::Available => {
            let has_url = model.update.available_url.is_some();
            vec![
                ("Open", has_url, UiAction::OpenUpdateLink),
                ("Install", has_url, UiAction::InstallUpdate),
                ("Dismiss", true, UiAction::DismissUpdate),
            ]
        }
        crate::app::UpdateStatusModel::Error => {
            vec![("Retry", true, UiAction::CheckForUpdates)]
        }
    };
    if specs.is_empty() {
        return Vec::new();
    }
    let labels: Vec<&str> = specs.iter().map(|(label, _, _)| *label).collect();
    let rects = compute_update_action_button_rects(
        layout.top_bar_title_row,
        layout.top_bar_action_cluster,
        style.sizing,
        &labels,
    );
    let start_index = specs.len().saturating_sub(rects.len());
    rects
        .into_iter()
        .zip(specs.into_iter().skip(start_index))
        .map(|(rect, (label, enabled, action))| ActionButton {
            rect,
            label,
            enabled,
            action,
            text_color: style.text_primary,
        })
        .collect()
}

fn browser_toolbar_layout(
    layout: &ShellLayout,
    style: &StyleTokens,
    browser_buttons: &[ActionButton],
) -> BrowserToolbarLayout {
    let action_left = browser_buttons
        .iter()
        .map(|button| button.rect.min.x)
        .min_by(f32::total_cmp)
        .or(Some(
            layout.browser_toolbar.max.x - style.sizing.text_inset_x,
        ));
    let sections =
        compute_browser_toolbar_sections(layout.browser_toolbar, style.sizing, action_left);
    BrowserToolbarLayout {
        search_field: sections.search_field,
        activity_chip: sections.activity_chip,
        sort_chip: sections.sort_chip,
    }
}

fn top_bar_controls_layout(layout: &ShellLayout, sizing: SizingTokens) -> TopBarControlsLayout {
    let resolved = compute_top_bar_controls_sections(layout, sizing);
    TopBarControlsLayout {
        active: resolved.active,
        options_label: resolved.options_label,
        volume_meter: resolved.volume_meter,
        volume_value: resolved.volume_value,
        volume_label: resolved.volume_label,
    }
}

fn sidebar_sections(
    layout: &ShellLayout,
    style: &StyleTokens,
    model: &AppModel,
) -> SidebarSections {
    let resolved = compute_sidebar_row_sections(
        layout.sidebar_rows,
        style.sizing,
        SidebarRowCounts {
            source_rows: rendered_source_rows(style, model),
            folder_rows: rendered_folder_rows(style, model),
        },
    );
    SidebarSections {
        source_rows: resolved.source_rows,
        folder_header: resolved.folder_header,
        folder_rows: resolved.folder_rows,
    }
}

fn browser_action_buttons(
    layout: &ShellLayout,
    style: &StyleTokens,
    model: &AppModel,
) -> Vec<ActionButton> {
    let definitions = [
        (
            "Rename",
            model.browser_actions.can_rename,
            UiAction::StartBrowserRename,
            style.text_primary,
        ),
        (
            "Trash",
            model.browser_actions.can_tag,
            UiAction::TagBrowserSelection {
                target: BrowserTagTarget::Trash,
            },
            style.accent_warning,
        ),
        (
            "Neutral",
            model.browser_actions.can_tag,
            UiAction::TagBrowserSelection {
                target: BrowserTagTarget::Neutral,
            },
            style.text_muted,
        ),
        (
            "Keep",
            model.browser_actions.can_tag,
            UiAction::TagBrowserSelection {
                target: BrowserTagTarget::Keep,
            },
            style.accent_mint,
        ),
        (
            "Delete",
            model.browser_actions.can_delete,
            UiAction::DeleteBrowserSelection,
            style.accent_copper,
        ),
    ];
    let rects = compute_browser_action_button_rects(
        layout.browser_toolbar,
        style.sizing,
        definitions.len(),
    );
    let start_index = definitions.len().saturating_sub(rects.len());
    rects
        .into_iter()
        .zip(definitions.into_iter().skip(start_index))
        .map(
            |(rect, (label, enabled, action, text_color))| ActionButton {
                rect,
                label,
                enabled,
                action,
                text_color,
            },
        )
        .collect()
}

fn source_action_buttons(
    layout: &ShellLayout,
    style: &StyleTokens,
    model: &AppModel,
) -> Vec<ActionButton> {
    let definitions = [
        (
            "New",
            model.sources.folder_actions.can_create_folder,
            UiAction::StartNewFolder,
            style.text_primary,
        ),
        (
            "Root",
            model.sources.folder_actions.can_create_folder_at_root,
            UiAction::StartNewFolderAtRoot,
            style.text_muted,
        ),
        (
            "Rename",
            model.sources.folder_actions.can_rename_folder,
            UiAction::StartFolderRename,
            style.accent_warning,
        ),
        (
            "Delete",
            model.sources.folder_actions.can_delete_folder,
            UiAction::DeleteFocusedFolder,
            style.accent_copper,
        ),
        (
            "Recovery",
            model.sources.folder_actions.can_clear_recovery_log,
            UiAction::ClearFolderDeleteRecoveryLog,
            style.accent_mint,
        ),
    ];
    let rects =
        compute_sidebar_action_button_rects(layout.sidebar_footer, style.sizing, definitions.len());
    let start_index = definitions.len().saturating_sub(rects.len());
    rects
        .into_iter()
        .zip(definitions.into_iter().skip(start_index))
        .map(
            |(rect, (label, enabled, action, text_color))| ActionButton {
                rect,
                label,
                enabled,
                action,
                text_color,
            },
        )
        .collect()
}

fn progress_cancel_button(layout: &ShellLayout, style: &StyleTokens, modal: bool) -> Rect {
    compute_progress_overlay_sections(layout.content, style.sizing, modal).cancel_button
}

fn prompt_buttons(layout: &ShellLayout, style: &StyleTokens) -> (Rect, Rect) {
    let sections = compute_prompt_overlay_sections(layout.content, style.sizing, false, false);
    (sections.confirm_button, sections.cancel_button)
}

fn prompt_input_rect(layout: &ShellLayout, style: &StyleTokens, model: &AppModel) -> Option<Rect> {
    if model.confirm_prompt.input_value.is_none() {
        return None;
    }
    compute_prompt_overlay_sections(
        layout.content,
        style.sizing,
        true,
        model.confirm_prompt.target_label.is_some(),
    )
    .input
}

fn drag_overlay_rect(layout: &ShellLayout, style: &StyleTokens) -> Rect {
    compute_drag_overlay_rect(layout.content, layout.status_bar, style.sizing)
}

fn render_progress_overlay(
    primitives: &mut Vec<Primitive>,
    text_runs: &mut Vec<TextRun>,
    layout: &ShellLayout,
    style: &StyleTokens,
    model: &AppModel,
) {
    if !model.progress_overlay.visible {
        return;
    }
    let sizing = style.sizing;
    if model.progress_overlay.modal {
        primitives.push(Primitive::Rect(FillRect {
            rect: layout.root.rect,
            color: Rgba8 {
                r: style.bg_primary.r,
                g: style.bg_primary.g,
                b: style.bg_primary.b,
                a: style.scrim_soft_alpha,
            },
        }));
    }
    let overlay_sections =
        compute_progress_overlay_sections(layout.content, sizing, model.progress_overlay.modal);
    let rect = overlay_sections.dialog;
    primitives.push(Primitive::Rect(FillRect {
        rect,
        color: style.surface_overlay,
    }));
    push_border(primitives, rect, style.border, sizing.border_width);

    text_runs.push(TextRun {
        text: model.progress_overlay.title.clone(),
        position: Point::new(
            rect.min.x + sizing.text_inset_x,
            rect.min.y + sizing.text_inset_y,
        ),
        font_size: sizing.font_header,
        color: style.text_primary,
        max_width: Some((rect.width() - (sizing.text_inset_x * 2.0)).max(24.0)),
        align: TextAlign::Left,
    });
    if let Some(detail) = model.progress_overlay.detail.as_ref() {
        text_runs.push(TextRun {
            text: detail.clone(),
            position: Point::new(
                rect.min.x + sizing.text_inset_x,
                rect.min.y + sizing.text_inset_y + sizing.font_header + sizing.text_row_gap,
            ),
            font_size: sizing.font_meta,
            color: style.text_muted,
            max_width: Some((rect.width() - (sizing.text_inset_x * 2.0)).max(24.0)),
            align: TextAlign::Left,
        });
    }
    let fraction = if model.progress_overlay.total == 0 {
        0.0
    } else {
        (model.progress_overlay.completed as f32 / model.progress_overlay.total as f32)
            .clamp(0.0, 1.0)
    };
    let bar_rect = overlay_sections.progress_bar;
    primitives.push(Primitive::Rect(FillRect {
        rect: bar_rect,
        color: style.grid_soft,
    }));
    let filled_width = bar_rect.width() * fraction;
    if filled_width > 0.0 {
        primitives.push(Primitive::Rect(FillRect {
            rect: Rect::from_min_max(
                bar_rect.min,
                Point::new(bar_rect.min.x + filled_width, bar_rect.max.y),
            ),
            color: style.accent_mint,
        }));
    }
    push_border(primitives, bar_rect, style.border, sizing.border_width);

    text_runs.push(TextRun {
        text: format!(
            "{} / {}",
            model.progress_overlay.completed, model.progress_overlay.total
        ),
        position: Point::new(bar_rect.min.x, bar_rect.max.y + sizing.text_row_gap),
        font_size: sizing.font_meta,
        color: style.text_muted,
        max_width: Some(bar_rect.width()),
        align: TextAlign::Right,
    });

    if model.progress_overlay.cancelable {
        let button = overlay_sections.cancel_button;
        primitives.push(Primitive::Rect(FillRect {
            rect: button,
            color: if model.progress_overlay.cancel_requested {
                style.grid_soft
            } else {
                style.bg_tertiary
            },
        }));
        push_border(
            primitives,
            button,
            if model.progress_overlay.cancel_requested {
                style.border
            } else {
                style.accent_warning
            },
            sizing.border_width,
        );
        text_runs.push(TextRun {
            text: if model.progress_overlay.cancel_requested {
                String::from("Cancelling")
            } else {
                String::from("Cancel")
            },
            position: Point::new(
                button.min.x + sizing.text_inset_x,
                text_top_in_rect(button, sizing.font_meta, sizing.text_inset_y),
            ),
            font_size: sizing.font_meta,
            color: if model.progress_overlay.cancel_requested {
                style.text_muted
            } else {
                style.text_primary
            },
            max_width: Some((button.width() - (sizing.text_inset_x * 2.0)).max(12.0)),
            align: TextAlign::Center,
        });
    }
}

fn render_confirm_prompt(
    primitives: &mut Vec<Primitive>,
    text_runs: &mut Vec<TextRun>,
    layout: &ShellLayout,
    style: &StyleTokens,
    model: &AppModel,
) {
    if !model.confirm_prompt.visible {
        return;
    }
    let sizing = style.sizing;
    let confirm_enabled = !prompt_has_validation_error(model);
    let prompt_sections = compute_prompt_overlay_sections(
        layout.content,
        sizing,
        model.confirm_prompt.input_value.is_some(),
        model.confirm_prompt.target_label.is_some(),
    );
    primitives.push(Primitive::Rect(FillRect {
        rect: layout.root.rect,
        color: Rgba8 {
            r: style.bg_primary.r,
            g: style.bg_primary.g,
            b: style.bg_primary.b,
            a: style.scrim_modal_alpha,
        },
    }));
    let dialog = prompt_sections.dialog;
    primitives.push(Primitive::Rect(FillRect {
        rect: dialog,
        color: style.surface_overlay,
    }));
    push_border(
        primitives,
        dialog,
        style.accent_warning,
        sizing.border_width,
    );

    text_runs.push(TextRun {
        text: model.confirm_prompt.title.clone(),
        position: Point::new(
            dialog.min.x + sizing.text_inset_x,
            dialog.min.y + sizing.text_inset_y,
        ),
        font_size: sizing.font_title,
        color: style.text_primary,
        max_width: Some((dialog.width() - (sizing.text_inset_x * 2.0)).max(24.0)),
        align: TextAlign::Left,
    });
    text_runs.push(TextRun {
        text: model.confirm_prompt.message.clone(),
        position: Point::new(
            dialog.min.x + sizing.text_inset_x,
            dialog.min.y + sizing.text_inset_y + sizing.font_title + sizing.text_row_gap,
        ),
        font_size: sizing.font_meta,
        color: style.text_muted,
        max_width: Some((dialog.width() - (sizing.text_inset_x * 2.0)).max(24.0)),
        align: TextAlign::Left,
    });
    if let Some(target) = model.confirm_prompt.target_label.as_ref() {
        text_runs.push(TextRun {
            text: target.clone(),
            position: Point::new(
                dialog.min.x + sizing.text_inset_x,
                dialog.min.y
                    + sizing.text_inset_y
                    + sizing.font_title
                    + sizing.font_meta
                    + (sizing.text_row_gap * 2.0),
            ),
            font_size: sizing.font_meta,
            color: style.accent_copper,
            max_width: Some((dialog.width() - (sizing.text_inset_x * 2.0)).max(24.0)),
            align: TextAlign::Left,
        });
    }
    if let Some(input_rect) = prompt_sections.input {
        primitives.push(Primitive::Rect(FillRect {
            rect: input_rect,
            color: style.surface_base,
        }));
        push_border(
            primitives,
            input_rect,
            if model.confirm_prompt.input_error.is_some() {
                style.accent_warning
            } else {
                style.accent_copper
            },
            sizing.border_width,
        );
        let input_text = model
            .confirm_prompt
            .input_value
            .as_deref()
            .unwrap_or_default();
        let (text, color) = if input_text.is_empty() {
            (
                model
                    .confirm_prompt
                    .input_placeholder
                    .as_deref()
                    .unwrap_or("Type here…"),
                style.text_muted,
            )
        } else {
            (input_text, style.text_primary)
        };
        text_runs.push(TextRun {
            text: text.to_string(),
            position: Point::new(
                input_rect.min.x + sizing.text_inset_x,
                input_rect.min.y + sizing.text_inset_y,
            ),
            font_size: sizing.font_meta,
            color,
            max_width: Some((input_rect.width() - (sizing.text_inset_x * 2.0)).max(24.0)),
            align: TextAlign::Left,
        });
        if let Some(error) = model.confirm_prompt.input_error.as_ref() {
            text_runs.push(TextRun {
                text: error.clone(),
                position: Point::new(
                    input_rect.min.x + sizing.text_inset_x,
                    input_rect.max.y + sizing.text_row_gap,
                ),
                font_size: sizing.font_meta,
                color: style.accent_warning,
                max_width: Some((dialog.width() - (sizing.text_inset_x * 2.0)).max(24.0)),
                align: TextAlign::Left,
            });
        }
    }
    let confirm_button = prompt_sections.confirm_button;
    let cancel_button = prompt_sections.cancel_button;
    for (index, (rect, label, color)) in [
        (
            confirm_button,
            if model.confirm_prompt.confirm_label.is_empty() {
                "Confirm"
            } else {
                model.confirm_prompt.confirm_label.as_str()
            },
            style.accent_mint,
        ),
        (
            cancel_button,
            if model.confirm_prompt.cancel_label.is_empty() {
                "Cancel"
            } else {
                model.confirm_prompt.cancel_label.as_str()
            },
            style.text_muted,
        ),
    ]
    .into_iter()
    .enumerate()
    {
        let enabled = if index == 0 { confirm_enabled } else { true };
        primitives.push(Primitive::Rect(FillRect {
            rect,
            color: if enabled {
                style.surface_overlay
            } else {
                style.control_disabled_fill
            },
        }));
        push_border(
            primitives,
            rect,
            if enabled { color } else { style.border },
            sizing.border_width,
        );
        text_runs.push(TextRun {
            text: label.to_string(),
            position: Point::new(
                rect.min.x + sizing.text_inset_x,
                text_top_in_rect(rect, sizing.font_meta, sizing.text_inset_y),
            ),
            font_size: sizing.font_meta,
            color: if !enabled {
                style.text_muted
            } else if index == 0 {
                style.text_primary
            } else {
                style.text_muted
            },
            max_width: Some((rect.width() - (sizing.text_inset_x * 2.0)).max(12.0)),
            align: TextAlign::Center,
        });
    }
}

fn render_drag_overlay(
    primitives: &mut Vec<Primitive>,
    text_runs: &mut Vec<TextRun>,
    layout: &ShellLayout,
    style: &StyleTokens,
    model: &AppModel,
) {
    if !model.drag_overlay.active {
        return;
    }
    let sizing = style.sizing;
    let rect = drag_overlay_rect(layout, style);
    primitives.push(Primitive::Rect(FillRect {
        rect,
        color: style.surface_overlay,
    }));
    push_border(
        primitives,
        rect,
        if model.drag_overlay.valid_target {
            style.accent_mint
        } else {
            style.accent_warning
        },
        sizing.border_width,
    );
    text_runs.push(TextRun {
        text: if model.drag_overlay.target_label.is_empty() {
            model.drag_overlay.label.clone()
        } else {
            format!(
                "{} -> {}",
                model.drag_overlay.label, model.drag_overlay.target_label
            )
        },
        position: Point::new(
            rect.min.x + sizing.text_inset_x,
            rect.min.y + sizing.text_inset_y,
        ),
        font_size: sizing.font_meta,
        color: if model.drag_overlay.valid_target {
            style.text_primary
        } else {
            style.accent_warning
        },
        max_width: Some((rect.width() - (sizing.text_inset_x * 2.0)).max(24.0)),
        align: TextAlign::Center,
    });
}

fn style_for_layout(layout: &ShellLayout) -> StyleTokens {
    StyleTokens::for_viewport_with_scale(layout.root.rect.width(), layout.ui_scale)
}

fn text_top_in_rect(rect: Rect, font_size: f32, minimum_inset_y: f32) -> f32 {
    let centered_y = rect.min.y + ((rect.height() - font_size).max(0.0) * 0.5);
    centered_y.max(rect.min.y + minimum_inset_y)
}

fn prompt_has_validation_error(model: &AppModel) -> bool {
    model
        .confirm_prompt
        .input_error
        .as_ref()
        .is_some_and(|error| !error.trim().is_empty())
}

fn row_label_x(rect: Rect, sizing: &SizingTokens, extra_indent: f32) -> f32 {
    rect.min.x + sizing.text_inset_x + sizing.row_corner_inset + extra_indent.max(0.0)
}

fn row_label_width(rect: Rect, sizing: &SizingTokens, extra_indent: f32, min_width: f32) -> f32 {
    (rect.width() - ((sizing.text_inset_x + sizing.row_corner_inset) * 2.0) - extra_indent.max(0.0))
        .max(min_width.max(0.0))
}

fn push_border(
    primitives: &mut Vec<Primitive>,
    rect: Rect,
    color: crate::gui::types::Rgba8,
    stroke: f32,
) {
    let stroke = stroke.max(1.0);
    if rect.width() <= stroke * 2.0 || rect.height() <= stroke * 2.0 {
        return;
    }
    primitives.push(Primitive::Rect(FillRect {
        rect: Rect::from_min_max(rect.min, Point::new(rect.max.x, rect.min.y + stroke)),
        color,
    }));
    primitives.push(Primitive::Rect(FillRect {
        rect: Rect::from_min_max(Point::new(rect.min.x, rect.max.y - stroke), rect.max),
        color,
    }));
    primitives.push(Primitive::Rect(FillRect {
        rect: Rect::from_min_max(rect.min, Point::new(rect.min.x + stroke, rect.max.y)),
        color,
    }));
    primitives.push(Primitive::Rect(FillRect {
        rect: Rect::from_min_max(Point::new(rect.max.x - stroke, rect.min.y), rect.max),
        color,
    }));
}

fn build_stacked_rows(column: Rect, rows: usize, gap: f32, row_height: f32) -> Vec<Rect> {
    if rows == 0 {
        return Vec::new();
    }
    let row_height = row_height.max(8.0);
    let mut y = column.min.y;
    let mut output = Vec::with_capacity(rows);
    for _ in 0..rows {
        let max_y = (y + row_height).min(column.max.y);
        if max_y <= y {
            break;
        }
        output.push(Rect::from_min_max(
            Point::new(column.min.x, y),
            Point::new(column.max.x, max_y),
        ));
        y = max_y + gap;
        if y >= column.max.y {
            break;
        }
    }
    output
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::{BrowserRowModel, FolderActionsModel, FolderRowModel, SourceRowModel};
    use crate::gui::types::{ImageRgba, Point, Vector2};

    fn populated_sidebar_model() -> AppModel {
        let mut model = AppModel::default();
        for index in 0..20 {
            model.sources.rows.push(SourceRowModel::new(
                format!("source_{index:02}"),
                format!("detail_{index:02}"),
                index == 2,
                false,
            ));
        }
        for index in 0..24 {
            model.sources.folder_rows.push(FolderRowModel::new(
                format!("folder_{index:02}"),
                String::new(),
                index % 4,
                false,
                index == 3,
                index == 0,
                true,
                true,
            ));
        }
        model.sources.folder_actions = FolderActionsModel {
            can_create_folder: true,
            can_create_folder_at_root: true,
            can_rename_folder: true,
            can_delete_folder: true,
            can_clear_recovery_log: true,
        };
        model
    }

    fn browser_model_with_rows(total: usize, focused_visible_row: usize) -> AppModel {
        let mut model = AppModel::default();
        for visible_row in 0..total {
            model.browser.rows.push(BrowserRowModel::new(
                visible_row,
                format!("row_{visible_row:04}"),
                1,
                false,
                visible_row == focused_visible_row,
            ));
        }
        model.browser.visible_count = model.browser.rows.len();
        model.browser.selected_visible_row = Some(focused_visible_row);
        model.browser.anchor_visible_row = Some(focused_visible_row.saturating_sub(2));
        model
    }

    fn assert_rect_inside(outer: Rect, inner: Rect) {
        assert!(inner.min.x >= outer.min.x);
        assert!(inner.min.y >= outer.min.y);
        assert!(inner.max.x <= outer.max.x);
        assert!(inner.max.y <= outer.max.y);
    }

    fn assert_text_run_inside_band(run: &TextRun, band: Rect) {
        assert!(run.position.x >= band.min.x);
        assert!(run.position.x <= band.max.x);
        assert!(run.position.y >= band.min.y);
        assert!(run.position.y + run.font_size <= band.max.y + 0.5);
    }

    #[test]
    fn sidebar_sections_keep_non_overlapping_bands_across_tiers() {
        let sizes = [
            Vector2::new(820.0, 520.0),
            Vector2::new(1280.0, 720.0),
            Vector2::new(2300.0, 1080.0),
        ];
        let mut state = NativeShellState::new();
        let model = populated_sidebar_model();
        for viewport in sizes {
            let layout = ShellLayout::build(viewport);
            let style = style_for_layout(&layout);
            let sections = sidebar_sections(&layout, &style, &model);
            let rendered_sources = state.rendered_source_row_rects(&layout, &model);
            assert_rect_inside(layout.sidebar_rows, sections.source_rows);
            assert_rect_inside(layout.sidebar_rows, sections.folder_header);
            assert_rect_inside(layout.sidebar_rows, sections.folder_rows);
            assert!(sections.source_rows.max.y <= sections.folder_header.min.y);
            assert!(sections.folder_header.max.y <= sections.folder_rows.min.y);
            assert!(!rendered_sources.is_empty());
        }
    }

    #[test]
    fn sidebar_sections_remain_stable_in_cramped_viewports() {
        let layout = ShellLayout::build(Vector2::new(820.0, 400.0));
        let style = style_for_layout(&layout);
        let model = populated_sidebar_model();
        let sections = sidebar_sections(&layout, &style, &model);
        assert_rect_inside(layout.sidebar_rows, sections.source_rows);
        assert_rect_inside(layout.sidebar_rows, sections.folder_header);
        assert_rect_inside(layout.sidebar_rows, sections.folder_rows);
        assert!(sections.source_rows.max.y <= sections.folder_header.min.y);
        assert!(sections.folder_header.max.y <= sections.folder_rows.min.y);
    }

    #[test]
    fn source_divider_remains_above_folder_rows_in_cramped_viewports() {
        let layout = ShellLayout::build(Vector2::new(820.0, 400.0));
        let style = style_for_layout(&layout);
        let model = populated_sidebar_model();
        let sections = sidebar_sections(&layout, &style, &model);
        let divider = compute_source_section_divider_rect(
            sections.source_rows,
            sections.folder_header,
            style.sizing,
        )
        .expect("divider should exist");
        assert_rect_inside(layout.sidebar_rows, divider);
        assert!(divider.max.y <= sections.folder_rows.min.y);
        assert!(divider.min.y >= sections.source_rows.min.y);
    }

    #[test]
    fn folder_recovery_badge_compacts_label_when_header_is_narrow() {
        let layout = ShellLayout::build(Vector2::new(820.0, 520.0));
        let style = style_for_layout(&layout);
        let header_rect = Rect::from_min_max(
            Point::new(0.0, 0.0),
            Point::new(58.0, style.sizing.folder_header_block_height),
        );
        let header_layout =
            compute_sidebar_folder_header_layout(header_rect, style.sizing, false, 153);
        let badge = header_layout.badge.expect("badge should still render");
        assert_rect_inside(header_rect, badge.rect);
        assert!(badge.label.chars().count() <= 3);
        assert!(!badge.active);
    }

    #[test]
    fn folder_header_text_width_yields_no_overlap_with_recovery_badge() {
        let layout = ShellLayout::build(Vector2::new(820.0, 520.0));
        let style = style_for_layout(&layout);
        let header_rect = Rect::from_min_max(
            Point::new(24.0, 40.0),
            Point::new(120.0, 40.0 + style.sizing.folder_header_block_height),
        );
        let header_layout =
            compute_sidebar_folder_header_layout(header_rect, style.sizing, true, 0);
        let badge = header_layout
            .badge
            .expect("badge should render for active recovery");
        assert!(header_layout.title_row.max.x <= badge.rect.min.x);
        if let Some(metadata_row) = header_layout.metadata_row {
            assert!(metadata_row.max.x <= badge.rect.min.x);
        }
    }

    #[test]
    fn source_action_buttons_stay_inside_sidebar_footer() {
        let model = populated_sidebar_model();
        for viewport in [
            Vector2::new(820.0, 520.0),
            Vector2::new(1280.0, 720.0),
            Vector2::new(2300.0, 1080.0),
        ] {
            let layout = ShellLayout::build(viewport);
            let style = style_for_layout(&layout);
            let buttons = source_action_buttons(&layout, &style, &model);
            assert!(!buttons.is_empty());
            for button in &buttons {
                assert_rect_inside(layout.sidebar_footer, button.rect);
            }
            for pair in buttons.windows(2) {
                assert!(pair[0].rect.max.x <= pair[1].rect.min.x);
            }
        }
    }

    #[test]
    fn browser_action_buttons_stay_inside_toolbar() {
        let mut model = AppModel::default();
        model.browser_actions.can_rename = true;
        model.browser_actions.can_tag = true;
        model.browser_actions.can_delete = true;
        for viewport in [
            Vector2::new(820.0, 520.0),
            Vector2::new(1280.0, 720.0),
            Vector2::new(2300.0, 1080.0),
        ] {
            let layout = ShellLayout::build(viewport);
            let style = style_for_layout(&layout);
            let buttons = browser_action_buttons(&layout, &style, &model);
            assert!(!buttons.is_empty());
            for button in &buttons {
                assert_rect_inside(layout.browser_toolbar, button.rect);
            }
            for pair in buttons.windows(2) {
                assert!(pair[0].rect.max.x <= pair[1].rect.min.x);
            }
        }
    }

    #[test]
    fn browser_toolbar_controls_do_not_overlap_action_cluster() {
        let mut model = AppModel::default();
        model.browser_actions.can_rename = true;
        model.browser_actions.can_tag = true;
        model.browser_actions.can_delete = true;
        let layout = ShellLayout::build(Vector2::new(1280.0, 720.0));
        let style = style_for_layout(&layout);
        let buttons = browser_action_buttons(&layout, &style, &model);
        let controls = browser_toolbar_layout(&layout, &style, &buttons);
        let action_cluster_left = buttons
            .iter()
            .map(|button| button.rect.min.x)
            .min_by(f32::total_cmp)
            .unwrap_or(layout.browser_toolbar.max.x);
        assert_rect_inside(layout.browser_toolbar, controls.search_field);
        if controls.activity_chip.width() > 1.0 {
            assert_rect_inside(layout.browser_toolbar, controls.activity_chip);
        }
        if controls.sort_chip.width() > 1.0 {
            assert_rect_inside(layout.browser_toolbar, controls.sort_chip);
        }
        assert!(controls.search_field.max.x <= action_cluster_left);
        assert!(controls.activity_chip.max.x <= action_cluster_left);
        assert!(controls.sort_chip.max.x <= action_cluster_left);
    }

    #[test]
    fn top_bar_controls_fit_inside_control_row() {
        for viewport in [
            Vector2::new(820.0, 520.0),
            Vector2::new(1280.0, 720.0),
            Vector2::new(2300.0, 1080.0),
        ] {
            let layout = ShellLayout::build(viewport);
            let style = style_for_layout(&layout);
            let controls = top_bar_controls_layout(&layout, style.sizing);
            if !controls.active {
                continue;
            }
            assert_rect_inside(layout.top_bar_controls_row, controls.options_label);
            assert_rect_inside(layout.top_bar_controls_row, controls.volume_meter);
            assert_rect_inside(layout.top_bar_controls_row, controls.volume_value);
            assert_rect_inside(layout.top_bar_controls_row, controls.volume_label);
            assert!(controls.options_label.max.x <= controls.volume_meter.min.x);
            assert!(controls.volume_meter.max.x <= controls.volume_value.min.x);
            assert!(controls.volume_value.max.x <= controls.volume_label.min.x);
        }
    }

    #[test]
    fn browser_virtualization_keeps_focused_row_visible_in_dense_column() {
        let layout = ShellLayout::build(Vector2::new(1280.0, 720.0));
        let style = style_for_layout(&layout);
        let mut model = AppModel::default();
        for visible_row in 0..200 {
            model.browser.rows.push(BrowserRowModel::new(
                visible_row,
                format!("row_{visible_row:03}"),
                1,
                false,
                visible_row == 150,
            ));
        }
        model.browser.visible_count = model.browser.rows.len();
        model.browser.selected_visible_row = Some(150);
        let rendered = rendered_browser_rows(&layout, &model, &style);
        assert!(!rendered.is_empty());
        assert!(rendered.iter().any(|row| row.visible_row == 150));
        assert!(rendered.first().is_some_and(|first| first.visible_row > 0));
    }

    #[test]
    fn browser_virtualization_hit_test_maps_first_middle_last_rendered_rows() {
        let layout = ShellLayout::build(Vector2::new(1280.0, 720.0));
        let style = style_for_layout(&layout);
        let mut state = NativeShellState::new();
        let mut model = AppModel::default();
        for visible_row in 0..200 {
            model.browser.rows.push(BrowserRowModel::new(
                visible_row,
                format!("row_{visible_row:03}"),
                1,
                false,
                visible_row == 120,
            ));
        }
        model.browser.visible_count = model.browser.rows.len();
        model.browser.selected_visible_row = Some(120);
        let rendered = rendered_browser_rows(&layout, &model, &style);
        assert!(rendered.len() > 2);
        let middle = rendered.len() / 2;
        for index in [0, middle, rendered.len() - 1] {
            let row = &rendered[index];
            let point = Point::new(
                (row.rect.min.x + row.rect.max.x) * 0.5,
                (row.rect.min.y + row.rect.max.y) * 0.5,
            );
            assert_eq!(
                state.browser_row_at_point(&layout, &model, point),
                Some(row.visible_row)
            );
        }
    }

    #[test]
    fn browser_virtualization_clamps_tail_without_dropping_last_row() {
        let layout = ShellLayout::build(Vector2::new(1280.0, 720.0));
        let style = style_for_layout(&layout);
        let mut model = browser_model_with_rows(1000, 999);
        model.browser.selected_visible_row = Some(999);
        model.browser.anchor_visible_row = Some(996);

        let rendered = rendered_browser_rows(&layout, &model, &style);
        let expected_len = browser_rows_capacity(layout.browser_rows, style.sizing)
            .min(model.browser.rows.len())
            .max(1);
        assert_eq!(rendered.len(), expected_len);
        assert_eq!(rendered.last().map(|row| row.visible_row), Some(999));
        assert!(rendered.iter().any(|row| row.visible_row == 999));
    }

    #[test]
    fn browser_virtualization_hit_test_is_stable_across_viewport_tiers() {
        let mut state = NativeShellState::new();
        for viewport in [
            Vector2::new(820.0, 520.0),
            Vector2::new(1280.0, 720.0),
            Vector2::new(2300.0, 1080.0),
        ] {
            let layout = ShellLayout::build(viewport);
            let style = style_for_layout(&layout);
            let model = browser_model_with_rows(1200, 940);
            let rendered = rendered_browser_rows(&layout, &model, &style);
            assert!(!rendered.is_empty());
            assert!(rendered.iter().any(|row| row.visible_row == 940));
            let middle = rendered.len() / 2;
            for index in [0, middle, rendered.len() - 1] {
                let row = &rendered[index];
                let point = Point::new(
                    (row.rect.min.x + row.rect.max.x) * 0.5,
                    (row.rect.min.y + row.rect.max.y) * 0.5,
                );
                assert_eq!(
                    state.browser_row_at_point(&layout, &model, point),
                    Some(row.visible_row)
                );
            }
        }
    }

    #[test]
    fn large_dataset_frame_build_is_deterministic_across_tiers() {
        let mut state = NativeShellState::new();
        let model = browser_model_with_rows(5000, 4777);
        state.sync_from_model(&model);
        for viewport in [
            Vector2::new(820.0, 520.0),
            Vector2::new(1280.0, 720.0),
            Vector2::new(2300.0, 1080.0),
        ] {
            let layout = ShellLayout::build(viewport);
            let frame_a = state.build_frame(&layout, &model);
            let frame_b = state.build_frame(&layout, &model);
            assert_eq!(frame_a, frame_b);
            assert!(
                frame_a
                    .text_runs
                    .iter()
                    .any(|run| run.text.contains("row_"))
            );
        }
    }

    #[test]
    fn browser_virtualization_5k_rows_keeps_focus_and_tail_visible() {
        let layout = ShellLayout::build(Vector2::new(1280.0, 720.0));
        let style = style_for_layout(&layout);
        let mut model = browser_model_with_rows(5000, 4999);
        model.browser.selected_visible_row = Some(4999);
        model.browser.anchor_visible_row = Some(4995);

        let rendered = rendered_browser_rows(&layout, &model, &style);
        assert!(!rendered.is_empty());
        assert_eq!(rendered.last().map(|row| row.visible_row), Some(4999));
        assert!(
            rendered
                .iter()
                .any(|row| row.visible_row == 4999 && row.focused)
        );
    }

    #[test]
    fn browser_row_hit_test_is_disabled_when_map_tab_is_active() {
        let layout = ShellLayout::build(Vector2::new(1280.0, 720.0));
        let mut model = browser_model_with_rows(600, 300);
        model.map.active = true;
        let point = Point::new(
            (layout.browser_rows.min.x + layout.browser_rows.max.x) * 0.5,
            (layout.browser_rows.min.y + layout.browser_rows.max.y) * 0.5,
        );
        let mut state = NativeShellState::new();
        assert_eq!(state.browser_row_at_point(&layout, &model, point), None);
    }

    #[test]
    fn browser_bucket_label_prefers_explicit_row_metadata() {
        let layout = ShellLayout::build(Vector2::new(1280.0, 720.0));
        let mut state = NativeShellState::new();
        let mut model = AppModel::default();
        model
            .browser
            .rows
            .push(BrowserRowModel::new(0, "Kick 01", 1, true, true).with_bucket_label("165 BPM"));
        let frame = state.build_frame(&layout, &model);
        assert!(
            frame
                .text_runs
                .iter()
                .any(|run| run.text.contains("165 BPM"))
        );
    }

    #[test]
    fn browser_rows_use_alternating_fill_stripes_for_readability() {
        let layout = ShellLayout::build(Vector2::new(1280.0, 720.0));
        let style = style_for_layout(&layout);
        let mut state = NativeShellState::new();
        let mut model = AppModel::default();
        model
            .browser
            .rows
            .push(BrowserRowModel::new(0, "row_even", 1, false, false));
        model
            .browser
            .rows
            .push(BrowserRowModel::new(1, "row_odd", 1, false, false));
        model.browser.visible_count = model.browser.rows.len();
        let rendered = rendered_browser_rows(&layout, &model, &style);
        assert!(rendered.len() >= 2);

        let frame = state.build_frame(&layout, &model);
        let even_rect = rendered[0].rect;
        let odd_rect = rendered[1].rect;
        let even_fill = frame
            .primitives
            .iter()
            .find_map(|primitive| match primitive {
                Primitive::Rect(rect) if rect.rect == even_rect => Some(rect.color),
                _ => None,
            });
        let odd_fill = frame
            .primitives
            .iter()
            .find_map(|primitive| match primitive {
                Primitive::Rect(rect) if rect.rect == odd_rect => Some(rect.color),
                _ => None,
            });
        assert!(even_fill.is_some());
        assert!(odd_fill.is_some());
        assert_ne!(even_fill, odd_fill);
    }

    #[test]
    fn waveform_title_uses_primary_text_hierarchy_color() {
        let layout = ShellLayout::build(Vector2::new(1280.0, 720.0));
        let style = style_for_layout(&layout);
        let mut state = NativeShellState::new();
        let mut model = AppModel::default();
        model.waveform.loaded_label = Some(String::from("WaveTitle"));
        let frame = state.build_frame(&layout, &model);
        assert!(
            frame
                .text_runs
                .iter()
                .any(|run| run.text == "WaveTitle" && run.color == style.text_primary)
        );
    }

    #[test]
    fn waveform_image_data_renders_non_transparent_span_rectangles() {
        let layout = ShellLayout::build(Vector2::new(1280.0, 720.0));
        let mut state = NativeShellState::new();
        let mut model = AppModel::default();
        model.waveform.waveform_image = Some(ImageRgba::new(1, 1, vec![11, 22, 33, 255]).unwrap());
        let frame = state.build_frame(&layout, &model);
        let expected_color = Rgba8 {
            r: 11,
            g: 22,
            b: 33,
            a: 255,
        };
        let has_waveform_pixel = frame.primitives.iter().any(|primitive| {
            matches!(
                primitive,
                Primitive::Rect(rect) if rect.color == expected_color
            )
        });
        assert!(has_waveform_pixel);
    }

    #[test]
    fn waveform_image_transparent_pixels_do_not_emit_geometry() {
        let layout = ShellLayout::build(Vector2::new(1280.0, 720.0));
        let mut state = NativeShellState::new();
        let mut model = AppModel::default();
        model.waveform.waveform_image = Some(ImageRgba::new(1, 1, vec![11, 22, 33, 0]).unwrap());
        let frame = state.build_frame(&layout, &model);
        let has_expected_waveform_color = frame.primitives.iter().any(|primitive| {
            matches!(
                primitive,
                Primitive::Rect(rect) if rect.color == Rgba8 {
                    r: 1,
                    g: 1,
                    b: 1,
                    a: 255
                }
            )
        });
        assert!(!has_expected_waveform_color);
    }

    #[test]
    fn map_header_prefers_projected_legend_selection_and_viewport_copy() {
        let layout = ShellLayout::build(Vector2::new(1280.0, 720.0));
        let mut state = NativeShellState::new();
        let mut model = AppModel::default();
        model.map.active = true;
        model.map.legend_label = String::from("Render: points");
        model.map.selection_label = String::from("Selection: kick_24.wav");
        model.map.hover_label = String::from("Hover: kick_hover.wav");
        model.map.cluster_label = String::from("Clusters: 7");
        model.map.viewport_label = String::from("zoom 1.75x | pan (12, -8)");
        model.map.summary = String::from("248 points");

        let frame = state.build_frame(&layout, &model);
        assert!(
            frame
                .text_runs
                .iter()
                .any(|run| run.text.contains("Render: points"))
        );
        assert!(
            frame
                .text_runs
                .iter()
                .any(|run| run.text.contains("Selection: kick_24.wav"))
        );
        assert!(
            frame
                .text_runs
                .iter()
                .any(|run| run.text.contains("Clusters: 7"))
        );
        assert!(
            frame
                .text_runs
                .iter()
                .any(|run| run.text.contains("zoom 1.75x | pan (12, -8)"))
        );
        assert!(
            frame
                .text_runs
                .iter()
                .any(|run| run.text.contains("248 points"))
        );
    }

    #[test]
    fn map_header_metadata_stays_within_header_band() {
        let layout = ShellLayout::build(Vector2::new(820.0, 520.0));
        let mut state = NativeShellState::new();
        let mut model = AppModel::default();
        model.map.active = true;
        model.map.legend_label = String::from("Render: points");
        model.map.selection_label = String::from("Selection: very_long_sample_name.wav");
        model.map.cluster_label = String::from("Clusters: 42");

        let frame = state.build_frame(&layout, &model);
        let header_runs = frame
            .text_runs
            .iter()
            .filter(|run| run.text.contains("Render:") || run.text.contains("Selection:"))
            .collect::<Vec<_>>();
        assert!(!header_runs.is_empty());
        for run in header_runs {
            assert_text_run_inside_band(run, layout.browser_table_header);
        }
    }

    #[test]
    fn hovered_top_bar_overlay_uses_hover_alpha() {
        let layout = ShellLayout::build(Vector2::new(1280.0, 720.0));
        let style = StyleTokens::for_viewport_width(1280.0);
        let model = AppModel::default();
        let mut state = NativeShellState::new();
        let mut frame = NativeViewFrame::default();
        state.hovered = Some(ShellNodeKind::TopBar);

        state.build_state_overlay_into(&layout, &style, &model, &mut frame);

        let overlay_color = frame
            .primitives
            .iter()
            .find_map(|primitive| match primitive {
                Primitive::Rect(rect) if rect.rect == layout.top_bar => Some(rect.color),
                _ => None,
            })
            .expect("hovered top bar should emit a fill rectangle");

        let expected_alpha =
            (style.sizing.hover_fill_alpha * (style.bg_tertiary.a as f32 / 255.0) * 255.0)
                .round()
                .clamp(0.0, 255.0) as u8;
        assert_eq!(overlay_color.a, expected_alpha);
        assert_eq!(overlay_color.r, style.bg_tertiary.r);
        assert_eq!(overlay_color.g, style.bg_tertiary.g);
        assert_eq!(overlay_color.b, style.bg_tertiary.b);
        assert!(overlay_color.a < 255);
    }

    #[test]
    fn browser_row_hovered_overlay_uses_hover_fill() {
        let layout = ShellLayout::build(Vector2::new(1280.0, 720.0));
        let mut state = NativeShellState::new();
        let style = StyleTokens::for_viewport_width(1280.0);
        let mut model = AppModel::default();
        model
            .browser
            .rows
            .push(BrowserRowModel::new(0, "hover", 1, false, false));
        model
            .browser
            .rows
            .push(BrowserRowModel::new(1, "hover-2", 1, false, false));
        model.browser.visible_count = model.browser.rows.len();

        let rendered_rows = rendered_browser_rows(&layout, &model, &style);
        let hover_row = rendered_rows[0].rect;
        let cursor = Point::new(
            hover_row.min.x + 4.0,
            (hover_row.min.y + hover_row.max.y) * 0.5,
        );
        state.handle_cursor_move(&layout, &model, cursor);

        let mut frame = NativeViewFrame::default();
        state.build_state_overlay_into(&layout, &style, &model, &mut frame);

        let expected_hover =
            translucent_overlay_color(style.bg_tertiary, style.grid_soft, style.state_hover_soft);
        let overlay_color = frame
            .primitives
            .iter()
            .find_map(|primitive| match primitive {
                Primitive::Rect(rect) if rect.rect == hover_row => Some(rect.color),
                _ => None,
            })
            .expect("hovered browser row should emit a fill rectangle");

        assert_eq!(overlay_color, expected_hover);
    }

    #[test]
    fn source_row_selected_fill_is_translucent_overlay() {
        let layout = ShellLayout::build(Vector2::new(1280.0, 720.0));
        let style = StyleTokens::for_viewport_width(1280.0);
        let mut state = NativeShellState::new();
        let mut model = AppModel::default();
        model.sources.rows.push(SourceRowModel::new(
            "selected source",
            "detail",
            true,
            false,
        ));

        let selected_row = *state
            .rendered_source_row_rects(&layout, &model)
            .first()
            .expect("source row should be rendered");
        let frame = state.build_frame(&layout, &model);

        let row_color = frame
            .primitives
            .iter()
            .find_map(|primitive| match primitive {
                Primitive::Rect(rect) if rect.rect == selected_row => Some(rect.color),
                _ => None,
            })
            .expect("selected source row should emit a fill rectangle");

        assert_eq!(
            row_color,
            translucent_overlay_color(
                style.bg_tertiary,
                style.grid_soft,
                style.state_selected_blend
            )
        );
        assert!(row_color.a < 255);
    }

    #[test]
    fn top_bar_update_prefers_projected_status_and_hint_copy() {
        let layout = ShellLayout::build(Vector2::new(1280.0, 720.0));
        let mut state = NativeShellState::new();
        let mut model = AppModel::default();
        model.update.status = crate::app::UpdateStatusModel::Available;
        model.update.status_label = String::from("Update available: v20.1.0");
        model.update.action_hint_label = String::from("Actions: open | install | dismiss");
        model.update.release_notes_label = String::from("Release: v20.1.0 (2026-02-01)");
        model.update.available_url = Some(String::from("https://example.invalid/release"));

        let frame = state.build_frame(&layout, &model);
        assert!(
            frame
                .text_runs
                .iter()
                .any(|run| run.text.contains("Update available"))
        );
        assert!(
            frame
                .text_runs
                .iter()
                .any(|run| run.text.contains("Actions: open"))
        );
        let controls_run = frame
            .text_runs
            .iter()
            .find(|run| run.text.contains("Actions: open"))
            .expect("combined update controls text should be present");
        assert_text_run_inside_band(controls_run, layout.top_bar_controls_row);
    }

    #[test]
    fn tick_with_style_uses_tier_motion_speed_tokens() {
        let mut model = AppModel::default();
        model.transport_running = true;
        let compact_style = StyleTokens::for_viewport_width(820.0);
        let wide_style = StyleTokens::for_viewport_width(2300.0);

        let mut compact_state = NativeShellState::new();
        compact_state.sync_from_model(&model);
        compact_state.tick_with_style(1.0, &compact_style);

        let mut wide_state = NativeShellState::new();
        wide_state.sync_from_model(&model);
        wide_state.tick_with_style(1.0, &wide_style);

        assert!(compact_state.pulse_phase > 0.0);
        assert!(wide_state.pulse_phase > compact_state.pulse_phase);
    }

    #[test]
    fn top_bar_volume_click_maps_to_set_volume_action() {
        let layout = ShellLayout::build(Vector2::new(1280.0, 720.0));
        let state = NativeShellState::new();
        let controls = top_bar_controls_layout(&layout, style_for_layout(&layout).sizing);
        assert!(controls.active);
        let point = Point::new(
            controls.volume_meter.min.x + (controls.volume_meter.width() * 0.75),
            controls.volume_meter.min.y + (controls.volume_meter.height() * 0.5),
        );
        let action = state
            .top_bar_volume_action_at_point(&layout, point)
            .expect("volume click should produce action");
        assert_eq!(action, UiAction::SetVolume { value_milli: 750 });
    }

    #[test]
    fn top_bar_volume_drag_clamps_beyond_meter_bounds() {
        let layout = ShellLayout::build(Vector2::new(1280.0, 720.0));
        let state = NativeShellState::new();
        let controls = top_bar_controls_layout(&layout, style_for_layout(&layout).sizing);
        assert!(controls.active);
        let left_action = state
            .top_bar_volume_drag_action(
                &layout,
                Point::new(
                    controls.volume_meter.min.x - 40.0,
                    controls.volume_meter.min.y,
                ),
            )
            .expect("left drag action");
        let right_action = state
            .top_bar_volume_drag_action(
                &layout,
                Point::new(
                    controls.volume_meter.max.x + 40.0,
                    controls.volume_meter.min.y,
                ),
            )
            .expect("right drag action");
        assert_eq!(left_action, UiAction::SetVolume { value_milli: 0 });
        assert_eq!(right_action, UiAction::SetVolume { value_milli: 1000 });
    }
}
