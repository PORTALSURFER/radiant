//! Retained view-tree layout and hit-testing for the native shell.

use super::{
    ShellLayoutRuntime,
    layout_adapter::{compute_status_bar_segments, compute_top_bar_band_sections},
    style::StyleTokens,
};
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
    pub waveform_scrollbar_lane: Rect,
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
    /// UI scale factor used to derive the layout’s active token set.
    pub ui_scale: f32,
}

/// Derived metrics used to validate layout parity contracts.
#[derive(Clone, Copy, Debug, PartialEq)]
#[cfg(test)]
pub(crate) struct LayoutContractSnapshot {
    /// Effective viewport width after layout clamping.
    pub viewport_width: f32,
    /// Effective viewport height after layout clamping.
    pub viewport_height: f32,
    /// Sidebar width in logical pixels.
    pub sidebar_width: f32,
    /// Waveform card height in logical pixels.
    pub waveform_height: f32,
    /// Browser table row capacity using active row-height tokens.
    pub browser_row_capacity: usize,
    /// Top-bar height in logical pixels.
    pub top_bar_height: f32,
    /// Status-bar height in logical pixels.
    pub status_bar_height: f32,
}

impl ShellLayout {
    /// Build shell layout for the provided logical viewport dimensions.
    #[cfg(test)]
    pub(crate) fn build(viewport: Vector2) -> Self {
        let style = StyleTokens::for_viewport_width(viewport.x);
        Self::build_with_style(viewport, &style)
    }

    /// Build shell layout for the provided viewport and style token set.
    #[cfg(test)]
    pub(crate) fn build_with_style(viewport: Vector2, style: &StyleTokens) -> Self {
        let mut runtime = ShellLayoutRuntime::default();
        Self::build_with_style_and_runtime(viewport, style, &mut runtime)
    }

    /// Build shell layout for the provided viewport/style using a persistent runtime cache.
    pub(crate) fn build_with_style_and_runtime(
        viewport: Vector2,
        style: &StyleTokens,
        runtime: &mut ShellLayoutRuntime,
    ) -> Self {
        let viewport_width = viewport.x.max(style.sizing.min_viewport_width);
        let viewport_height = viewport.y.max(style.sizing.min_viewport_height);
        let sizing = style.sizing;
        let base_style = StyleTokens::for_viewport_width(viewport_width);
        let ui_scale = if base_style.sizing.font_title > 0.0 {
            (sizing.font_title / base_style.sizing.font_title).clamp(1.0, 3.0)
        } else {
            1.0
        };
        let sections =
            runtime.compute_shell_sections(Vector2::new(viewport_width, viewport_height), style);
        let root_rect = sections.root;
        let top_bar = Rect::from_min_max(
            root_rect.min,
            Point::new(root_rect.max.x, root_rect.min.y + sizing.top_bar_height),
        );
        let top_bar_bands = compute_top_bar_band_sections(top_bar, sizing);
        let top_bar_title_row = top_bar_bands.top_bar_title_row;
        let top_bar_controls_row = top_bar_bands.top_bar_controls_row;
        let top_bar_title_cluster = top_bar_bands.top_bar_title_cluster;
        let top_bar_action_cluster = top_bar_bands.top_bar_action_cluster;
        let status_bar = Rect::from_min_max(
            Point::new(root_rect.min.x, root_rect.max.y - sizing.status_bar_height),
            root_rect.max,
        );
        let status_segments = compute_status_bar_segments(status_bar, sizing);
        let status_left_segment = status_segments.left;
        let status_center_segment = status_segments.center;
        let status_right_segment = status_segments.right;
        let body_min_y = top_bar.max.y;
        let body_max_y = status_bar.min.y;
        let sidebar = Rect::from_min_max(
            Point::new(sections.sidebar.min.x, body_min_y),
            Point::new(sections.sidebar.max.x, body_max_y),
        );
        let sidebar_bands = runtime.compute_sidebar_band_sections(sidebar, sizing);
        let sidebar_header = sidebar_bands.sidebar_header;
        let sidebar_footer = sidebar_bands.sidebar_footer;
        let sidebar_rows = Rect::from_min_max(
            Point::new(
                sidebar_bands.sidebar_rows.min.x,
                sidebar_header.max.y.min(sidebar_footer.min.y),
            ),
            Point::new(
                sidebar_bands.sidebar_rows.max.x,
                sidebar_footer
                    .min
                    .y
                    .max(sidebar_header.max.y.min(sidebar_footer.min.y)),
            ),
        );
        let content = Rect::from_min_max(
            Point::new(sidebar.max.x, body_min_y),
            Point::new(root_rect.max.x, body_max_y),
        );
        let waveform_card = Rect::from_min_max(
            Point::new(content.min.x, content.min.y),
            Point::new(
                content.max.x,
                sections.waveform_card.max.y.min(content.max.y),
            ),
        );
        let browser_panel =
            Rect::from_min_max(Point::new(content.min.x, waveform_card.max.y), content.max);
        let browser_bands = runtime.compute_browser_band_sections(browser_panel, sizing);
        let browser_tabs = browser_bands.browser_tabs;
        let browser_footer = browser_bands.browser_footer;
        let browser_toolbar = Rect::from_min_max(
            Point::new(browser_bands.browser_toolbar.min.x, browser_tabs.max.y),
            Point::new(
                browser_bands.browser_toolbar.max.x,
                (browser_tabs.max.y + browser_bands.browser_toolbar.height())
                    .min(browser_footer.min.y),
            ),
        );
        let browser_table_header = Rect::from_min_max(
            Point::new(
                browser_bands.browser_table_header.min.x,
                browser_toolbar.max.y,
            ),
            Point::new(
                browser_bands.browser_table_header.max.x,
                (browser_toolbar.max.y + browser_bands.browser_table_header.height())
                    .min(browser_footer.min.y),
            ),
        );
        let browser_rows = Rect::from_min_max(
            Point::new(browser_bands.browser_rows.min.x, browser_table_header.max.y),
            Point::new(
                browser_bands.browser_rows.max.x,
                browser_footer.min.y.max(browser_table_header.max.y),
            ),
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

        let waveform_header = band_header(waveform_card, sizing.waveform_header_block_height);
        let waveform_body_top = waveform_header
            .max
            .y
            .clamp(waveform_card.min.y, waveform_card.max.y);
        let waveform_body = Rect::from_min_max(
            Point::new(waveform_card.min.x, waveform_body_top),
            waveform_card.max,
        );
        let waveform_scrollbar_lane_height =
            waveform_scrollbar_lane_height(waveform_body, sizing.waveform_header_block_height);
        let waveform_scrollbar_lane = Rect::from_min_max(
            Point::new(
                waveform_body.min.x,
                waveform_body.max.y - waveform_scrollbar_lane_height,
            ),
            waveform_body.max,
        );
        let waveform_plot = Rect::from_min_max(
            waveform_body.min,
            Point::new(waveform_body.max.x, waveform_scrollbar_lane.min.y),
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
            waveform_scrollbar_lane,
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
            ui_scale,
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

    /// Build a compact metric snapshot used by parity/layout contract tests.
    #[cfg(test)]
    pub(crate) fn contract_snapshot(&self, style: &StyleTokens) -> LayoutContractSnapshot {
        let row_stride = (style.sizing.browser_row_height + style.sizing.browser_row_gap).max(1.0);
        LayoutContractSnapshot {
            viewport_width: self.root.rect.width(),
            viewport_height: self.root.rect.height(),
            sidebar_width: self.sidebar.width(),
            waveform_height: self.waveform_card.height(),
            browser_row_capacity: (self.browser_rows.height() / row_stride).floor() as usize,
            top_bar_height: self.top_bar.height(),
            status_bar_height: self.status_bar.height(),
        }
    }
}

fn band_header(panel: Rect, header_height: f32) -> Rect {
    Rect::from_min_max(
        panel.min,
        Point::new(panel.max.x, (panel.min.y + header_height).min(panel.max.y)),
    )
}

fn inset_horizontal(rect: Rect, inset: f32) -> Rect {
    let max_inset = (rect.width() * 0.5).max(0.0);
    let inset = inset.min(max_inset);
    Rect::from_min_max(
        Point::new(rect.min.x + inset, rect.min.y),
        Point::new(rect.max.x - inset, rect.max.y),
    )
}

fn waveform_scrollbar_lane_height(waveform_body: Rect, header_height: f32) -> f32 {
    if waveform_body.height() <= 1.0 {
        return 0.0;
    }
    let desired = (header_height * 0.5).round().clamp(12.0, 18.0);
    desired.min((waveform_body.height() - 1.0).max(0.0))
}
