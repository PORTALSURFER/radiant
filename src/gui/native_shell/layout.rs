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
    TriageColumn(usize),
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
    pub sidebar: Rect,
    pub sidebar_header: Rect,
    pub sidebar_rows: Rect,
    pub sidebar_footer: Rect,
    pub content: Rect,
    pub waveform_card: Rect,
    pub waveform_header: Rect,
    pub waveform_plot: Rect,
    pub columns: [Rect; 3],
    pub column_headers: [Rect; 3],
    pub column_rows: [Rect; 3],
    pub status_bar: Rect,
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
        let status_bar = Rect::from_min_max(
            Point::new(frame.min.x, frame.max.y - sizing.status_bar_height),
            frame.max,
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

        let triage_top = (waveform_card.max.y + sizing.panel_gap).min(content.max.y - 1.0);
        let triage_rect = Rect::from_min_max(Point::new(content.min.x, triage_top), content.max);
        let base_column_width =
            ((triage_rect.width() - (sizing.column_gap * 2.0)) / 3.0).max(sizing.column_min_width);

        let mut columns = [Rect::default(), Rect::default(), Rect::default()];
        for (index, column) in columns.iter_mut().enumerate() {
            let x0 = triage_rect.min.x + (base_column_width + sizing.column_gap) * index as f32;
            let x1 = if index == 2 {
                triage_rect.max.x
            } else {
                x0 + base_column_width
            };
            *column = Rect::from_min_max(
                Point::new(x0, triage_rect.min.y),
                Point::new(x1, triage_rect.max.y),
            );
        }

        let sidebar_header = band_header(sidebar, sizing.source_header_block_height);
        let sidebar_footer =
            band_footer(sidebar, sizing.source_bottom_padding, sidebar_header.max.y);
        let sidebar_rows = inset_horizontal(
            Rect::from_min_max(
                Point::new(sidebar.min.x, sidebar_header.max.y),
                Point::new(sidebar.max.x, sidebar_footer.min.y),
            ),
            sizing.panel_inset,
        );

        let waveform_header = band_header(waveform_card, sizing.waveform_header_block_height);
        let waveform_inset = waveform_card.inset(sizing.panel_inset);
        let waveform_body_top = waveform_header
            .max
            .y
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
            let rows_bottom = (column.max.y - sizing.column_bottom_padding).max(header.max.y);
            column_headers[index] = header;
            column_rows[index] = inset_horizontal(
                Rect::from_min_max(
                    Point::new(column.min.x, header.max.y),
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
                        let mut children = vec![ShellNode {
                            id: 5,
                            kind: ShellNodeKind::WaveformCard,
                            rect: waveform_card,
                            children: Vec::new(),
                        }];
                        for (index, rect) in columns.iter().copied().enumerate() {
                            children.push(ShellNode {
                                id: 100 + index as u64,
                                kind: ShellNodeKind::TriageColumn(index),
                                rect,
                                children: Vec::new(),
                            });
                        }
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
            sidebar,
            sidebar_header,
            sidebar_rows,
            sidebar_footer,
            content,
            waveform_card,
            waveform_header,
            waveform_plot,
            columns,
            column_headers,
            column_rows,
            status_bar,
        }
    }

    /// Hit-test against the retained tree.
    pub(crate) fn hit_test(&self, point: Point) -> Option<ShellNodeKind> {
        self.root.hit_test(point)
    }

    /// Resolve triage column index for a point, if any.
    pub(crate) fn column_at_point(&self, point: Point) -> Option<usize> {
        match self.hit_test(point) {
            Some(ShellNodeKind::TriageColumn(index)) => Some(index),
            _ => None,
        }
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
