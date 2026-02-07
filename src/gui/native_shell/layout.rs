//! Retained view-tree layout and hit-testing for the native shell.

use super::style::StyleTokens;
use crate::gui::types::{Point, Rect, Vector2};

/// Stable identifier for nodes in the retained shell tree.
pub(crate) type ViewNodeId = u64;

/// Semantic node kinds used by the native shell tree.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum ShellNodeKind {
    Root,
    TopBar,
    Sidebar,
    Content,
    WaveformCard,
    BrowserPanel,
    BrowserTabs,
    BrowserTable,
    StatusBar,
}

/// A retained view node with stable identity, geometry, and optional children.
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct ShellNode {
    pub id: ViewNodeId,
    pub kind: ShellNodeKind,
    pub rect: Rect,
    pub children: Vec<ShellNode>,
}

impl ShellNode {
    fn hit_test(&self, point: Point) -> Option<ShellNodeKind> {
        if !self.rect.contains(point) {
            return None;
        }
        for child in self.children.iter().rev() {
            if let Some(hit) = child.hit_test(point) {
                return Some(hit);
            }
        }
        Some(self.kind)
    }
}

/// Computed shell layout for one viewport size.
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct ShellLayout {
    pub root: ShellNode,
    pub top_bar: Rect,
    pub top_bar_title_row: Rect,
    pub top_bar_controls_row: Rect,
    pub top_bar_title_cluster: Rect,
    pub top_bar_action_cluster: Rect,
    pub sidebar: Rect,
    pub sidebar_header: Rect,
    pub sidebar_rows: Rect,
    pub sidebar_footer: Rect,
    pub content: Rect,
    pub waveform_card: Rect,
    pub waveform_header: Rect,
    pub waveform_plot: Rect,
    pub browser_panel: Rect,
    pub browser_tabs: Rect,
    pub browser_toolbar: Rect,
    pub browser_table_header: Rect,
    pub browser_rows: Rect,
    pub browser_footer: Rect,
    pub columns: [Rect; 3],
    pub column_headers: [Rect; 3],
    pub column_rows: [Rect; 3],
    pub status_bar: Rect,
    pub status_left_segment: Rect,
    pub status_center_segment: Rect,
    pub status_right_segment: Rect,
}

impl ShellLayout {
    /// Build shell layout for the provided logical viewport dimensions.
    pub(crate) fn build(viewport: Vector2) -> Self {
        let viewport_width = viewport.x.max(620.0);
        let style = StyleTokens::for_viewport_width(viewport_width);
        Self::build_with_style(viewport, &style)
    }

    /// Build shell layout for the provided viewport and style token set.
    pub(crate) fn build_with_style(viewport: Vector2, style: &StyleTokens) -> Self {
        let viewport_width = viewport.x.max(620.0);
        let viewport_height = viewport.y.max(400.0);
        let sizing = style.sizing;

        let root_rect = Rect::from_min_size(
            Point::new(0.0, 0.0),
            Vector2::new(viewport_width, viewport_height),
        );
        let frame = root_rect.inset(sizing.frame_inset);
        let top_bar = Rect::from_min_max(
            frame.min,
            Point::new(frame.max.x, frame.min.y + sizing.top_bar_height),
        );
        let title_row_height = sizing
            .top_bar_title_row_height
            .max(12.0)
            .min((top_bar.height() - 4.0).max(12.0));
        let top_bar_title_row = Rect::from_min_max(
            top_bar.min,
            Point::new(
                top_bar.max.x,
                (top_bar.min.y + title_row_height).min(top_bar.max.y),
            ),
        );
        let controls_row_top = (top_bar_title_row.max.y + sizing.text_row_gap)
            .min(top_bar.max.y)
            .max(top_bar_title_row.max.y);
        let top_bar_controls_row =
            Rect::from_min_max(Point::new(top_bar.min.x, controls_row_top), top_bar.max);
        let top_bar_inner = inset_horizontal(top_bar_title_row, sizing.panel_inset);
        let desired_action_cluster_width = ((sizing.action_button_width * 5.0)
            + (sizing.action_button_gap * 4.0)
            + (sizing.text_inset_x * 2.0))
            .clamp(
                sizing.top_bar_action_cluster_min_width,
                sizing.top_bar_action_cluster_max_width,
            );
        let max_action_cluster_width = (top_bar_inner.width() - 72.0).max(0.0);
        let action_cluster_width = desired_action_cluster_width.min(max_action_cluster_width);
        let top_bar_action_cluster = if action_cluster_width > 0.0 {
            Rect::from_min_max(
                Point::new(
                    (top_bar_inner.max.x - action_cluster_width).max(top_bar_inner.min.x),
                    top_bar_inner.min.y,
                ),
                top_bar_inner.max,
            )
        } else {
            Rect::from_min_max(top_bar_inner.max, top_bar_inner.max)
        };
        let title_cluster_max_x =
            (top_bar_action_cluster.min.x - sizing.top_bar_cluster_gap).max(top_bar_inner.min.x);
        let top_bar_title_cluster = Rect::from_min_max(
            top_bar_inner.min,
            Point::new(title_cluster_max_x, top_bar_inner.max.y),
        );

        let status_bar = Rect::from_min_max(
            Point::new(frame.min.x, frame.max.y - sizing.status_bar_height),
            frame.max,
        );
        let status_inner = inset_horizontal(status_bar, sizing.panel_inset);
        let status_total_gap = sizing.status_segment_gap * 2.0;
        let status_available = (status_inner.width() - status_total_gap).max(0.0);
        let status_left_width = status_available * 0.30;
        let status_right_width = status_available * 0.22;
        let status_center_width =
            (status_available - status_left_width - status_right_width).max(0.0);
        let status_left_segment = Rect::from_min_max(
            status_inner.min,
            Point::new(status_inner.min.x + status_left_width, status_inner.max.y),
        );
        let status_center_min_x = status_left_segment.max.x + sizing.status_segment_gap;
        let status_center_segment = Rect::from_min_max(
            Point::new(status_center_min_x, status_inner.min.y),
            Point::new(
                status_center_min_x + status_center_width,
                status_inner.max.y,
            ),
        );
        let status_right_min_x = status_center_segment.max.x + sizing.status_segment_gap;
        let status_right_segment = Rect::from_min_max(
            Point::new(status_right_min_x, status_inner.min.y),
            status_inner.max,
        );
        let body = Rect::from_min_max(
            Point::new(frame.min.x, top_bar.max.y + sizing.panel_gap),
            Point::new(frame.max.x, status_bar.min.y - sizing.panel_gap),
        );

        let max_sidebar = (body.width() - sizing.content_min_width).max(sizing.sidebar_min_width);
        let sidebar_width = (body.width() * sizing.sidebar_ratio).clamp(
            sizing.sidebar_min_width,
            sizing.sidebar_max_width.min(max_sidebar),
        );
        let sidebar =
            Rect::from_min_max(body.min, Point::new(body.min.x + sidebar_width, body.max.y));
        let content_min_x = (sidebar.max.x + sizing.panel_gap).min(body.max.x - 64.0);
        let content = Rect::from_min_max(Point::new(content_min_x, body.min.y), body.max);

        let waveform_height = (content.height() * sizing.waveform_ratio)
            .clamp(sizing.waveform_min_height, sizing.waveform_max_height)
            .min((content.height() - 64.0).max(70.0));
        let waveform_card = Rect::from_min_max(
            content.min,
            Point::new(
                content.max.x,
                (content.min.y + waveform_height).min(content.max.y),
            ),
        );

        let browser_top = (waveform_card.max.y + sizing.panel_gap).min(content.max.y - 1.0);
        let browser_panel = Rect::from_min_max(Point::new(content.min.x, browser_top), content.max);
        let browser_tabs_height = sizing
            .browser_tabs_height
            .max(16.0)
            .min(browser_panel.height());
        let browser_tabs = inset_horizontal(
            band_header(browser_panel, browser_tabs_height),
            sizing.panel_inset,
        );
        let browser_toolbar_top =
            (browser_tabs.max.y + sizing.text_row_gap).min(browser_panel.max.y);
        let browser_toolbar_height = sizing
            .browser_toolbar_height
            .max(18.0)
            .min((browser_panel.max.y - browser_toolbar_top).max(0.0));
        let browser_toolbar = inset_horizontal(
            Rect::from_min_max(
                Point::new(browser_panel.min.x, browser_toolbar_top),
                Point::new(
                    browser_panel.max.x,
                    browser_toolbar_top + browser_toolbar_height,
                ),
            ),
            sizing.panel_inset,
        );
        let browser_header_top =
            (browser_toolbar.max.y + sizing.text_row_gap).min(browser_panel.max.y);
        let browser_header_height = sizing
            .browser_table_header_height
            .max(16.0)
            .min((browser_panel.max.y - browser_header_top).max(0.0));
        let browser_table_header = inset_horizontal(
            Rect::from_min_max(
                Point::new(browser_panel.min.x, browser_header_top),
                Point::new(
                    browser_panel.max.x,
                    browser_header_top + browser_header_height,
                ),
            ),
            sizing.panel_inset,
        );
        let browser_footer = band_footer(
            browser_panel,
            sizing.browser_footer_height.clamp(14.0, 28.0),
            browser_table_header.max.y,
        );
        let browser_rows_top = (browser_table_header.max.y + sizing.text_row_gap)
            .min(browser_footer.min.y)
            .max(browser_panel.min.y);
        let browser_rows = inset_horizontal(
            Rect::from_min_max(
                Point::new(browser_panel.min.x, browser_rows_top),
                Point::new(browser_panel.max.x, browser_footer.min.y),
            ),
            sizing.panel_inset,
        );

        // Keep legacy triage partitions as invisible compatibility geometry for
        // routing actions that still speak in triage-column terms.
        let base_column_width =
            ((browser_rows.width() - (sizing.column_gap * 2.0)) / 3.0).max(sizing.column_min_width);
        let mut columns = [Rect::default(), Rect::default(), Rect::default()];
        for (index, column) in columns.iter_mut().enumerate() {
            let x0 = browser_rows.min.x + (base_column_width + sizing.column_gap) * index as f32;
            let x1 = if index == 2 {
                browser_rows.max.x
            } else {
                x0 + base_column_width
            };
            *column = Rect::from_min_max(
                Point::new(x0, browser_rows.min.y),
                Point::new(x1, browser_rows.max.y),
            );
        }

        let sidebar_header = band_header(sidebar, sizing.source_header_block_height);
        let sidebar_footer =
            band_footer(sidebar, sizing.source_bottom_padding, sidebar_header.max.y);
        let sidebar_rows_top = (sidebar_header.max.y + sizing.header_to_rows_gap)
            .min(sidebar_footer.min.y)
            .max(sidebar.min.y);
        let sidebar_rows = inset_horizontal(
            Rect::from_min_max(
                Point::new(sidebar.min.x, sidebar_rows_top),
                Point::new(sidebar.max.x, sidebar_footer.min.y),
            ),
            sizing.panel_inset,
        );

        let waveform_header = band_header(waveform_card, sizing.waveform_header_block_height);
        let waveform_inset = waveform_card.inset(sizing.panel_inset);
        let waveform_body_top = waveform_header.max.y
            + sizing
                .header_to_rows_gap
                .max(waveform_inset.min.y)
                .min(waveform_inset.max.y);
        let waveform_plot = Rect::from_min_max(
            Point::new(waveform_inset.min.x, waveform_body_top),
            waveform_inset.max,
        );

        let mut column_headers = [Rect::default(), Rect::default(), Rect::default()];
        let mut column_rows = [Rect::default(), Rect::default(), Rect::default()];
        for (index, column) in columns.iter().copied().enumerate() {
            let header = band_header(column, sizing.column_header_block_height);
            let rows_top = (header.max.y + sizing.header_to_rows_gap).min(column.max.y);
            let rows_bottom = (column.max.y - sizing.column_bottom_padding).max(header.max.y);
            column_headers[index] = header;
            column_rows[index] = inset_horizontal(
                Rect::from_min_max(
                    Point::new(column.min.x, rows_top),
                    Point::new(column.max.x, rows_bottom),
                ),
                sizing.panel_inset,
            );
        }

        let root = ShellNode {
            id: 1,
            kind: ShellNodeKind::Root,
            rect: root_rect,
            children: vec![
                ShellNode {
                    id: 2,
                    kind: ShellNodeKind::TopBar,
                    rect: top_bar,
                    children: Vec::new(),
                },
                ShellNode {
                    id: 3,
                    kind: ShellNodeKind::Sidebar,
                    rect: sidebar,
                    children: Vec::new(),
                },
                ShellNode {
                    id: 4,
                    kind: ShellNodeKind::Content,
                    rect: content,
                    children: {
                        let children = vec![
                            ShellNode {
                                id: 5,
                                kind: ShellNodeKind::WaveformCard,
                                rect: waveform_card,
                                children: Vec::new(),
                            },
                            ShellNode {
                                id: 100,
                                kind: ShellNodeKind::BrowserPanel,
                                rect: browser_panel,
                                children: vec![
                                    ShellNode {
                                        id: 101,
                                        kind: ShellNodeKind::BrowserTabs,
                                        rect: browser_tabs,
                                        children: Vec::new(),
                                    },
                                    ShellNode {
                                        id: 102,
                                        kind: ShellNodeKind::BrowserTable,
                                        rect: browser_rows,
                                        children: Vec::new(),
                                    },
                                ],
                            },
                        ];
                        children
                    },
                },
                ShellNode {
                    id: 6,
                    kind: ShellNodeKind::StatusBar,
                    rect: status_bar,
                    children: Vec::new(),
                },
            ],
        };

        Self {
            root,
            top_bar,
            top_bar_title_row,
            top_bar_controls_row,
            top_bar_title_cluster,
            top_bar_action_cluster,
            sidebar,
            sidebar_header,
            sidebar_rows,
            sidebar_footer,
            content,
            waveform_card,
            waveform_header,
            waveform_plot,
            browser_panel,
            browser_tabs,
            browser_toolbar,
            browser_table_header,
            browser_rows,
            browser_footer,
            columns,
            column_headers,
            column_rows,
            status_bar,
            status_left_segment,
            status_center_segment,
            status_right_segment,
        }
    }

    /// Hit-test against the retained tree.
    pub(crate) fn hit_test(&self, point: Point) -> Option<ShellNodeKind> {
        self.root.hit_test(point)
    }

    /// Resolve triage column index for a point, if any.
    pub(crate) fn column_at_point(&self, point: Point) -> Option<usize> {
        if !self.browser_rows.contains(point) {
            return None;
        }
        let ratio = ((point.x - self.browser_rows.min.x) / self.browser_rows.width().max(1.0))
            .clamp(0.0, 0.999_9);
        Some((ratio * 3.0).floor() as usize)
    }
}

fn band_header(panel: Rect, header_height: f32) -> Rect {
    Rect::from_min_max(
        panel.min,
        Point::new(panel.max.x, (panel.min.y + header_height).min(panel.max.y)),
    )
}

fn band_footer(panel: Rect, footer_height: f32, min_y: f32) -> Rect {
    let footer_start = (panel.max.y - footer_height).max(min_y).min(panel.max.y);
    Rect::from_min_max(Point::new(panel.min.x, footer_start), panel.max)
}

fn inset_horizontal(rect: Rect, inset: f32) -> Rect {
    let max_inset = (rect.width() * 0.5).max(0.0);
    let inset = inset.min(max_inset);
    Rect::from_min_max(
        Point::new(rect.min.x + inset, rect.min.y),
        Point::new(rect.max.x - inset, rect.max.y),
    )
}
