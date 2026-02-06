//! Mutable interaction state and paint generation for the native shell.

use super::{
    layout::{ShellLayout, ShellNodeKind},
    paint::{FillCircle, FillRect, NativeViewFrame, Primitive, TextAlign, TextRun},
    style::StyleTokens,
};
use crate::app::{AppModel, BrowserRowModel};
use crate::gui::{
    input::KeyCode,
    types::{Point, Rect},
};

/// Mutable interaction + animation state for the native shell.
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct NativeShellState {
    selected_column: usize,
    hovered: Option<ShellNodeKind>,
    transport_running: bool,
    pulse_phase: f32,
}

impl NativeShellState {
    /// Create a default shell state.
    pub(crate) fn new() -> Self {
        Self {
            selected_column: 1,
            hovered: None,
            transport_running: true,
            pulse_phase: 0.0,
        }
    }

    /// Return whether the shell currently needs continuous animation.
    pub(crate) fn needs_animation(&self) -> bool {
        self.transport_running
    }

    /// Synchronize local interaction state from the latest app model.
    pub(crate) fn sync_from_model(&mut self, model: &AppModel) {
        self.selected_column = model.selected_column.min(2);
        self.transport_running = model.transport_running;
    }

    /// Update animation clocks by a frame delta.
    pub(crate) fn tick(&mut self, delta_seconds: f32) {
        if self.transport_running {
            self.pulse_phase =
                (self.pulse_phase + delta_seconds * 2.6).rem_euclid(std::f32::consts::TAU);
        }
    }

    /// Handle pointer movement and update hovered view target.
    pub(crate) fn handle_cursor_move(&mut self, layout: &ShellLayout, point: Point) -> bool {
        let next_hover = layout.hit_test(point);
        if next_hover == self.hovered {
            return false;
        }
        self.hovered = next_hover;
        true
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
        &self,
        layout: &ShellLayout,
        model: &AppModel,
        point: Point,
    ) -> Option<usize> {
        if model.sources.rows.is_empty() {
            return None;
        }
        let style = style_for_layout(layout);
        let rendered_rows = model.sources.rows.len().min(MAX_RENDERED_SOURCE_ROWS);
        build_stacked_rows(
            layout.sidebar_rows,
            rendered_rows,
            style.sizing.source_row_gap,
            style.sizing.source_row_height,
        )
        .iter()
        .position(|rect| rect.contains(point))
    }

    /// Resolve a rendered browser visible-row index for a point in the triage pane.
    pub(crate) fn browser_row_at_point(
        &self,
        layout: &ShellLayout,
        model: &AppModel,
        point: Point,
    ) -> Option<usize> {
        rendered_browser_rows(layout, model, &style_for_layout(layout))
            .into_iter()
            .find(|row| row.rect.contains(point))
            .map(|row| row.visible_row)
    }

    /// Build a native frame from state + layout + style tokens.
    pub(crate) fn build_frame_with_style(
        &self,
        layout: &ShellLayout,
        style: &StyleTokens,
        model: &AppModel,
    ) -> NativeViewFrame {
        let sizing = style.sizing;
        let mut primitives = Vec::new();
        let mut text_runs = Vec::new();

        primitives.push(Primitive::Rect(FillRect {
            rect: layout.top_bar,
            color: if self.hovered == Some(ShellNodeKind::TopBar) {
                style.bg_tertiary
            } else {
                style.bg_secondary
            },
        }));
        primitives.push(Primitive::Rect(FillRect {
            rect: layout.sidebar,
            color: if self.hovered == Some(ShellNodeKind::Sidebar) {
                style.bg_tertiary
            } else {
                style.bg_secondary
            },
        }));
        primitives.push(Primitive::Rect(FillRect {
            rect: layout.content,
            color: style.bg_primary,
        }));
        primitives.push(Primitive::Rect(FillRect {
            rect: layout.waveform_card,
            color: if self.hovered == Some(ShellNodeKind::WaveformCard) {
                style.bg_tertiary
            } else {
                style.bg_secondary
            },
        }));
        primitives.push(Primitive::Rect(FillRect {
            rect: layout.status_bar,
            color: style.bg_secondary,
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

        if let Some(selection) = model.waveform.selection_milli {
            let start_ratio = f32::from(selection.start_milli.min(1000)) / 1000.0;
            let end_ratio = f32::from(selection.end_milli.min(1000)) / 1000.0;
            let start_x =
                waveform_inner.min.x + waveform_inner.width() * start_ratio.min(end_ratio);
            let end_x = waveform_inner.min.x + waveform_inner.width() * start_ratio.max(end_ratio);
            let rect = Rect::from_min_max(
                Point::new(start_x, waveform_inner.min.y),
                Point::new(end_x.max(start_x + 1.0), waveform_inner.max.y),
            );
            primitives.push(Primitive::Rect(FillRect {
                rect,
                color: style.grid_strong,
            }));
            push_border(
                &mut primitives,
                rect,
                style.accent_mint,
                sizing.border_width,
            );
        }

        if let Some(cursor_milli) = model.waveform.cursor_milli {
            let ratio = f32::from(cursor_milli.min(1000)) / 1000.0;
            let cursor_x = waveform_inner.min.x + waveform_inner.width() * ratio;
            let cursor_rect = Rect::from_min_max(
                Point::new(cursor_x, waveform_inner.min.y),
                Point::new(
                    (cursor_x + sizing.border_width.max(1.0)).min(waveform_inner.max.x),
                    waveform_inner.max.y,
                ),
            );
            primitives.push(Primitive::Rect(FillRect {
                rect: cursor_rect,
                color: style.accent_warning,
            }));
        }

        if let Some(playhead_milli) = model.waveform.playhead_milli {
            let ratio = f32::from(playhead_milli.min(1000)) / 1000.0;
            let playhead_x = waveform_inner.min.x + waveform_inner.width() * ratio;
            let playhead_rect = Rect::from_min_max(
                Point::new(playhead_x, waveform_inner.min.y),
                Point::new(
                    (playhead_x + sizing.border_width.max(1.0)).min(waveform_inner.max.x),
                    waveform_inner.max.y,
                ),
            );
            primitives.push(Primitive::Rect(FillRect {
                rect: playhead_rect,
                color: style.accent_copper,
            }));
        }

        for (index, column_rect) in layout.columns.iter().copied().enumerate() {
            let hovered = self.hovered == Some(ShellNodeKind::TriageColumn(index));
            let selected = self.selected_column == index;
            let fill = if selected {
                style.bg_tertiary
            } else {
                style.bg_secondary
            };
            primitives.push(Primitive::Rect(FillRect {
                rect: column_rect,
                color: fill,
            }));
            push_border(
                &mut primitives,
                column_rect,
                if hovered {
                    style.accent_warning
                } else if selected {
                    style.accent_mint
                } else {
                    style.border
                },
                sizing.border_width,
            );

            let has_rendered_rows = model
                .browser
                .rows
                .iter()
                .any(|row| row.column.min(2) == index);
            if !has_rendered_rows {
                let row_count = model.columns[index].item_count.clamp(1, 10);
                for row_rect in build_stacked_rows(
                    layout.column_rows[index],
                    row_count,
                    sizing.browser_row_gap,
                    sizing.browser_row_height,
                ) {
                    primitives.push(Primitive::Rect(FillRect {
                        rect: row_rect,
                        color: if selected {
                            style.grid_strong
                        } else {
                            style.grid_soft
                        },
                    }));
                }
            }
        }

        for row in rendered_browser_rows(layout, model, style) {
            primitives.push(Primitive::Rect(FillRect {
                rect: row.rect,
                color: if row.selected || row.focused {
                    style.bg_tertiary
                } else {
                    style.bg_primary
                },
            }));
            push_border(
                &mut primitives,
                row.rect,
                if row.focused {
                    style.accent_warning
                } else if row.selected {
                    style.accent_mint
                } else {
                    style.border
                },
                sizing.border_width,
            );
            text_runs.push(TextRun {
                text: row.label,
                position: Point::new(
                    row.rect.min.x + sizing.text_inset_x,
                    row.rect.min.y + sizing.text_inset_y,
                ),
                font_size: sizing.font_body,
                color: if row.focused {
                    style.accent_warning
                } else if row.selected {
                    style.accent_mint
                } else {
                    style.text_primary
                },
                max_width: Some((row.rect.width() - (sizing.text_inset_x * 2.0)).max(20.0)),
                align: TextAlign::Left,
            });
        }

        push_border(
            &mut primitives,
            layout.top_bar,
            style.border,
            sizing.border_width,
        );
        push_border(
            &mut primitives,
            layout.sidebar,
            style.border,
            sizing.border_width,
        );
        push_border(
            &mut primitives,
            layout.waveform_card,
            style.border,
            sizing.border_width,
        );
        push_border(
            &mut primitives,
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
                layout.top_bar.min.y + (layout.top_bar.height() * 0.5),
            ),
            radius: lamp_radius,
            color: lamp_color,
        }));

        let top_text_x = layout.top_bar.min.x + sizing.text_inset_x + 4.0;
        let top_title_y = layout.top_bar.min.y + sizing.text_inset_y;
        let top_meta_y = top_title_y + sizing.font_title + sizing.text_row_gap;
        text_runs.push(TextRun {
            text: model.title.clone(),
            position: Point::new(top_text_x, top_title_y),
            font_size: sizing.font_title,
            color: style.text_primary,
            max_width: Some((layout.top_bar.width() - 112.0).max(90.0)),
            align: TextAlign::Left,
        });
        text_runs.push(TextRun {
            text: model.backend_label.clone(),
            position: Point::new(top_text_x, top_meta_y),
            font_size: sizing.font_meta,
            color: style.text_muted,
            max_width: Some((layout.top_bar.width() - (sizing.text_inset_x * 2.0)).max(90.0)),
            align: TextAlign::Right,
        });
        let sources_header = if model.sources.header.is_empty() {
            model.sources_label.as_str()
        } else {
            model.sources.header.as_str()
        };
        text_runs.push(TextRun {
            text: sources_header.to_string(),
            position: Point::new(
                layout.sidebar_header.min.x + sizing.text_inset_x + 4.0,
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
                layout.sidebar_header.min.x + sizing.text_inset_x + 4.0,
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
        let rendered_sources = model.sources.rows.len().min(MAX_RENDERED_SOURCE_ROWS);
        for (row_index, row_rect) in build_stacked_rows(
            layout.sidebar_rows,
            rendered_sources,
            sizing.source_row_gap,
            sizing.source_row_height,
        )
        .iter()
        .copied()
        .enumerate()
        {
            let row = &model.sources.rows[row_index];
            let row_selected = row.selected
                || model
                    .sources
                    .selected_row
                    .is_some_and(|selected| selected == row_index);
            primitives.push(Primitive::Rect(FillRect {
                rect: row_rect,
                color: if row_selected {
                    style.bg_tertiary
                } else {
                    style.bg_primary
                },
            }));
            push_border(
                &mut primitives,
                row_rect,
                if row_selected {
                    style.accent_mint
                } else if row.missing {
                    style.accent_warning
                } else {
                    style.border
                },
                sizing.border_width,
            );
            text_runs.push(TextRun {
                text: row.label.clone(),
                position: Point::new(
                    row_rect.min.x + sizing.text_inset_x,
                    row_rect.min.y + sizing.text_inset_y,
                ),
                font_size: sizing.font_body,
                color: if row_selected {
                    style.accent_mint
                } else {
                    style.text_primary
                },
                max_width: Some((row_rect.width() - (sizing.text_inset_x * 2.0)).max(24.0)),
                align: TextAlign::Left,
            });
        }
        if model.sources.rows.len() > rendered_sources {
            text_runs.push(TextRun {
                text: format!("+{} more…", model.sources.rows.len() - rendered_sources),
                position: Point::new(
                    layout.sidebar_footer.min.x + sizing.text_inset_x + 4.0,
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
        let waveform_title = model.waveform.loaded_label.as_deref().unwrap_or("Waveform");
        text_runs.push(TextRun {
            text: waveform_title.to_string(),
            position: Point::new(
                layout.waveform_header.min.x + sizing.text_inset_x + 4.0,
                layout.waveform_header.min.y + sizing.text_inset_y,
            ),
            font_size: sizing.font_header,
            color: style.text_muted,
            max_width: Some(
                (layout.waveform_header.width() - (sizing.text_inset_x * 2.0)).max(72.0),
            ),
            align: TextAlign::Left,
        });
        let playhead_text = model
            .waveform
            .playhead_milli
            .map(format_milli_value)
            .unwrap_or_else(|| String::from("—"));
        let cursor_text = model
            .waveform
            .cursor_milli
            .map(format_milli_value)
            .unwrap_or_else(|| String::from("—"));
        let view_text = format!(
            "{}..{}",
            format_milli_value(model.waveform.view_start_milli),
            format_milli_value(model.waveform.view_end_milli)
        );
        text_runs.push(TextRun {
            text: format!(
                "loop: {} | playhead: {} | cursor: {} | view: {}",
                if model.waveform.loop_enabled {
                    "on"
                } else {
                    "off"
                },
                playhead_text,
                cursor_text,
                view_text,
            ),
            position: Point::new(
                layout.waveform_header.min.x + sizing.text_inset_x + 4.0,
                layout.waveform_header.min.y
                    + sizing.text_inset_y
                    + sizing.font_header
                    + sizing.text_row_gap,
            ),
            font_size: sizing.font_meta,
            color: style.text_muted,
            max_width: Some(
                (layout.waveform_header.width() - (sizing.text_inset_x * 2.0)).max(72.0),
            ),
            align: TextAlign::Left,
        });
        for (index, column) in layout.column_headers.iter().enumerate() {
            let label = format!(
                "{} ({})",
                model.columns[index].title, model.columns[index].item_count
            );
            text_runs.push(TextRun {
                text: label,
                position: Point::new(
                    column.min.x + sizing.text_inset_x + 3.0,
                    column.min.y + sizing.text_inset_y + 2.0,
                ),
                font_size: sizing.font_header,
                color: if self.selected_column == index {
                    style.accent_mint
                } else {
                    style.text_muted
                },
                max_width: Some((column.width() - (sizing.text_inset_x * 2.0)).max(56.0)),
                align: TextAlign::Left,
            });
        }

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
        text_runs.push(TextRun {
            text: status_left,
            position: Point::new(
                layout.status_bar.min.x + sizing.text_inset_x + 4.0,
                layout.status_bar.min.y + sizing.text_inset_y,
            ),
            font_size: sizing.font_status,
            color: style.text_muted,
            max_width: Some((layout.status_bar.width() - (sizing.text_inset_x * 2.0)).max(72.0)),
            align: TextAlign::Left,
        });
        text_runs.push(TextRun {
            text: status_center,
            position: Point::new(
                layout.status_bar.min.x + sizing.text_inset_x + 4.0,
                layout.status_bar.min.y + sizing.text_inset_y,
            ),
            font_size: sizing.font_status,
            color: style.text_primary,
            max_width: Some((layout.status_bar.width() - (sizing.text_inset_x * 2.0)).max(72.0)),
            align: TextAlign::Center,
        });
        text_runs.push(TextRun {
            text: status_right,
            position: Point::new(
                layout.status_bar.min.x + sizing.text_inset_x + 4.0,
                layout.status_bar.min.y + sizing.text_inset_y,
            ),
            font_size: sizing.font_status,
            color: style.text_muted,
            max_width: Some((layout.status_bar.width() - (sizing.text_inset_x * 2.0)).max(72.0)),
            align: TextAlign::Right,
        });

        NativeViewFrame {
            clear_color: style.clear_color,
            primitives,
            text_runs,
        }
    }

    /// Build a native frame using default style tokens.
    pub(crate) fn build_frame(&self, layout: &ShellLayout, model: &AppModel) -> NativeViewFrame {
        self.build_frame_with_style(layout, &style_for_layout(layout), model)
    }
}

const MAX_RENDERED_SOURCE_ROWS: usize = 10;
const MAX_RENDERED_BROWSER_ROWS_PER_COLUMN: usize = 18;

#[derive(Clone, Debug)]
struct RenderedBrowserRow {
    visible_row: usize,
    label: String,
    selected: bool,
    focused: bool,
    rect: Rect,
}

fn format_milli_value(value: u16) -> String {
    format!("{:.3}", f32::from(value.min(1000)) / 1000.0)
}

fn rendered_browser_rows(
    layout: &ShellLayout,
    model: &AppModel,
    style: &StyleTokens,
) -> Vec<RenderedBrowserRow> {
    let sizing = style.sizing;
    let mut rows_by_column: [Vec<&BrowserRowModel>; 3] = [Vec::new(), Vec::new(), Vec::new()];
    for row in &model.browser.rows {
        let column = row.column.min(2);
        if rows_by_column[column].len() < MAX_RENDERED_BROWSER_ROWS_PER_COLUMN {
            rows_by_column[column].push(row);
        }
    }

    let mut rendered = Vec::new();
    for (column, rows) in rows_by_column.iter().enumerate() {
        if rows.is_empty() {
            continue;
        }
        for (row, rect) in rows.iter().zip(build_stacked_rows(
            layout.column_rows[column],
            rows.len(),
            sizing.browser_row_gap,
            sizing.browser_row_height,
        )) {
            rendered.push(RenderedBrowserRow {
                visible_row: row.visible_row,
                label: row.label.clone(),
                selected: row.selected,
                focused: row.focused,
                rect,
            });
        }
    }
    rendered
}

fn style_for_layout(layout: &ShellLayout) -> StyleTokens {
    StyleTokens::for_viewport_width(layout.root.rect.width())
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
