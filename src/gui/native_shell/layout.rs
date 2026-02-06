//! Retained view-tree layout and hit-testing for the native shell.

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

/// Layout density profile used to build shell geometry.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum LayoutDensity {
    /// Compact, studio-oriented layout with tight panel framing.
    CompactStudio,
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
    pub content: Rect,
    pub waveform_card: Rect,
    pub columns: [Rect; 3],
    pub status_bar: Rect,
}

#[derive(Clone, Copy, Debug, PartialEq)]
struct LayoutMetrics {
    frame_inset: f32,
    panel_gap: f32,
    top_bar_height: f32,
    status_bar_height: f32,
    sidebar_ratio: f32,
    sidebar_min_width: f32,
    sidebar_max_width: f32,
    content_min_width: f32,
    waveform_ratio: f32,
    waveform_min_height: f32,
    waveform_max_height: f32,
    column_gap: f32,
}

impl LayoutMetrics {
    fn compact_for_width(viewport_width: f32) -> Self {
        if viewport_width < 980.0 {
            return Self {
                frame_inset: 6.0,
                panel_gap: 5.0,
                top_bar_height: 34.0,
                status_bar_height: 20.0,
                sidebar_ratio: 0.23,
                sidebar_min_width: 168.0,
                sidebar_max_width: 252.0,
                content_min_width: 180.0,
                waveform_ratio: 0.34,
                waveform_min_height: 120.0,
                waveform_max_height: 220.0,
                column_gap: 5.0,
            };
        }
        if viewport_width > 1700.0 {
            return Self {
                frame_inset: 10.0,
                panel_gap: 8.0,
                top_bar_height: 38.0,
                status_bar_height: 22.0,
                sidebar_ratio: 0.20,
                sidebar_min_width: 190.0,
                sidebar_max_width: 320.0,
                content_min_width: 260.0,
                waveform_ratio: 0.36,
                waveform_min_height: 140.0,
                waveform_max_height: 280.0,
                column_gap: 8.0,
            };
        }
        Self {
            frame_inset: 7.0,
            panel_gap: 6.0,
            top_bar_height: 36.0,
            status_bar_height: 20.0,
            sidebar_ratio: 0.22,
            sidebar_min_width: 176.0,
            sidebar_max_width: 280.0,
            content_min_width: 220.0,
            waveform_ratio: 0.35,
            waveform_min_height: 126.0,
            waveform_max_height: 250.0,
            column_gap: 6.0,
        }
    }
}

impl ShellLayout {
    /// Build shell layout for the provided logical viewport dimensions.
    pub(crate) fn build(viewport: Vector2) -> Self {
        Self::build_with_density(viewport, LayoutDensity::CompactStudio)
    }

    /// Build shell layout for the provided viewport and density profile.
    pub(crate) fn build_with_density(viewport: Vector2, density: LayoutDensity) -> Self {
        let viewport_width = viewport.x.max(620.0);
        let viewport_height = viewport.y.max(400.0);
        let metrics = match density {
            LayoutDensity::CompactStudio => LayoutMetrics::compact_for_width(viewport_width),
        };

        let root_rect = Rect::from_min_size(
            Point::new(0.0, 0.0),
            Vector2::new(viewport_width, viewport_height),
        );
        let frame = root_rect.inset(metrics.frame_inset);
        let top_bar = Rect::from_min_max(
            frame.min,
            Point::new(frame.max.x, frame.min.y + metrics.top_bar_height),
        );
        let status_bar = Rect::from_min_max(
            Point::new(frame.min.x, frame.max.y - metrics.status_bar_height),
            frame.max,
        );
        let body = Rect::from_min_max(
            Point::new(frame.min.x, top_bar.max.y + metrics.panel_gap),
            Point::new(frame.max.x, status_bar.min.y - metrics.panel_gap),
        );

        let max_sidebar = (body.width() - metrics.content_min_width).max(metrics.sidebar_min_width);
        let sidebar_width = (body.width() * metrics.sidebar_ratio).clamp(
            metrics.sidebar_min_width,
            metrics.sidebar_max_width.min(max_sidebar),
        );
        let sidebar =
            Rect::from_min_max(body.min, Point::new(body.min.x + sidebar_width, body.max.y));
        let content_min_x = (sidebar.max.x + metrics.panel_gap).min(body.max.x - 64.0);
        let content = Rect::from_min_max(Point::new(content_min_x, body.min.y), body.max);

        let waveform_height = (content.height() * metrics.waveform_ratio)
            .clamp(metrics.waveform_min_height, metrics.waveform_max_height)
            .min((content.height() - 64.0).max(70.0));
        let waveform_card = Rect::from_min_max(
            content.min,
            Point::new(
                content.max.x,
                (content.min.y + waveform_height).min(content.max.y),
            ),
        );

        let triage_top = (waveform_card.max.y + metrics.panel_gap).min(content.max.y - 1.0);
        let triage_rect = Rect::from_min_max(Point::new(content.min.x, triage_top), content.max);
        let base_column_width =
            ((triage_rect.width() - (metrics.column_gap * 2.0)) / 3.0).max(40.0);

        let mut columns = [Rect::default(), Rect::default(), Rect::default()];
        for (index, column) in columns.iter_mut().enumerate() {
            let x0 = triage_rect.min.x + (base_column_width + metrics.column_gap) * index as f32;
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
            content,
            waveform_card,
            columns,
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
