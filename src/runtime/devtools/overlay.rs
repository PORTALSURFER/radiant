use crate::{
    gui::types::{Point, Rect, Rgba8, Vector2},
    layout::NodeId,
    runtime::{PaintPrimitive, PaintTextAlign, push_fill_rect, push_stroke_rect, push_text},
    theme::ThemeTokens,
};

use super::{DevtoolsInspectorProjection, DevtoolsSnapshot};

const DEVTOOLS_OVERLAY_WIDGET_ID: NodeId = u64::MAX - 2048;
const DEVTOOLS_SELECTED_BOUNDS_WIDGET_ID: NodeId = u64::MAX - 2047;
const DEVTOOLS_TREE_TEXT_WIDGET_ID: NodeId = u64::MAX - 2046;
const DEVTOOLS_DETAIL_TEXT_WIDGET_ID: NodeId = u64::MAX - 2045;
const DEVTOOLS_PANEL_WIDTH: f32 = 680.0;
const DEVTOOLS_PANEL_HEIGHT: f32 = 292.0;
const DEVTOOLS_PANEL_MARGIN: f32 = 12.0;
const DEVTOOLS_PANEL_PADDING: f32 = 12.0;
const DEVTOOLS_TREE_WIDTH: f32 = 276.0;
const DEVTOOLS_ROW_HEIGHT: f32 = 17.0;

pub(super) fn append_devtools_overlay(
    snapshot: &DevtoolsSnapshot,
    theme: &ThemeTokens,
    primitives: &mut Vec<PaintPrimitive>,
) {
    if let Some(bounds) = snapshot.selected_node_bounds() {
        push_stroke_rect(
            primitives,
            DEVTOOLS_SELECTED_BOUNDS_WIDGET_ID,
            bounds,
            Rgba8::new(255, 190, 90, 235),
            2.0,
        );
    }
    append_devtools_inspector_panel(snapshot, theme, primitives);
}

fn append_devtools_inspector_panel(
    snapshot: &DevtoolsSnapshot,
    theme: &ThemeTokens,
    primitives: &mut Vec<PaintPrimitive>,
) {
    let projection = snapshot.inspector_projection();
    let panel = devtools_overlay_panel_rect(snapshot.viewport);
    let shadow = Rect::from_min_max(
        Point::new(panel.min.x + 4.0, panel.min.y + 6.0),
        Point::new(panel.max.x + 4.0, panel.max.y + 6.0),
    );

    push_fill_rect(
        primitives,
        DEVTOOLS_OVERLAY_WIDGET_ID,
        shadow,
        Rgba8::new(0, 0, 0, 96),
    );
    push_fill_rect(
        primitives,
        DEVTOOLS_OVERLAY_WIDGET_ID,
        panel,
        Rgba8::new(
            theme.surface_overlay.r,
            theme.surface_overlay.g,
            theme.surface_overlay.b,
            245,
        ),
    );
    push_stroke_rect(
        primitives,
        DEVTOOLS_OVERLAY_WIDGET_ID,
        panel,
        theme.border_emphasis,
        1.0,
    );

    append_devtools_header(primitives, theme, panel, snapshot);
    append_devtools_tree_rows(primitives, theme, panel, &projection);
    append_devtools_detail_rows(primitives, theme, panel, &projection);
}

fn devtools_overlay_panel_rect(viewport: Rect) -> Rect {
    let width = viewport.width().min(DEVTOOLS_PANEL_WIDTH).max(320.0);
    let height = viewport.height().min(DEVTOOLS_PANEL_HEIGHT).max(156.0);
    Rect::from_min_size(
        Point::new(
            (viewport.max.x - width - DEVTOOLS_PANEL_MARGIN).max(DEVTOOLS_PANEL_MARGIN),
            viewport.min.y + DEVTOOLS_PANEL_MARGIN,
        ),
        Vector2::new(width, height),
    )
}

fn append_devtools_header(
    primitives: &mut Vec<PaintPrimitive>,
    theme: &ThemeTokens,
    panel: Rect,
    snapshot: &DevtoolsSnapshot,
) {
    let header = Rect::from_min_size(
        Point::new(
            panel.min.x + DEVTOOLS_PANEL_PADDING,
            panel.min.y + DEVTOOLS_PANEL_PADDING,
        ),
        Vector2::new(panel.width() - DEVTOOLS_PANEL_PADDING * 2.0, 18.0),
    );
    push_text(
        primitives,
        DEVTOOLS_DETAIL_TEXT_WIDGET_ID,
        format!(
            "Radiant devtools  nodes={}  selected={}  paint={}",
            snapshot.inspector_projection().tree_rows.len(),
            snapshot
                .selected_node_id
                .map(|node_id| format!("#{node_id}"))
                .unwrap_or_else(|| String::from("none")),
            snapshot.paint.total
        ),
        header,
        theme.text_primary,
        PaintTextAlign::Left,
    );
}

fn append_devtools_tree_rows(
    primitives: &mut Vec<PaintPrimitive>,
    theme: &ThemeTokens,
    panel: Rect,
    projection: &DevtoolsInspectorProjection,
) {
    let origin = Point::new(
        panel.min.x + DEVTOOLS_PANEL_PADDING,
        panel.min.y + DEVTOOLS_PANEL_PADDING + 26.0,
    );
    push_text(
        primitives,
        DEVTOOLS_TREE_TEXT_WIDGET_ID,
        "Surface tree",
        Rect::from_min_size(
            origin,
            Vector2::new(DEVTOOLS_TREE_WIDTH, DEVTOOLS_ROW_HEIGHT),
        ),
        theme.text_primary,
        PaintTextAlign::Left,
    );

    let max_rows = ((panel.height() - 62.0) / DEVTOOLS_ROW_HEIGHT)
        .floor()
        .max(0.0) as usize;
    for (index, row) in projection.tree_rows.iter().take(max_rows).enumerate() {
        let row_origin = Point::new(
            origin.x,
            origin.y + ((index + 1) as f32 * DEVTOOLS_ROW_HEIGHT),
        );
        let row_rect = Rect::from_min_size(
            row_origin,
            Vector2::new(DEVTOOLS_TREE_WIDTH - 8.0, DEVTOOLS_ROW_HEIGHT),
        );
        if row.selected {
            push_fill_rect(
                primitives,
                DEVTOOLS_OVERLAY_WIDGET_ID,
                row_rect,
                Rgba8::new(
                    theme.highlight_blue_soft.r,
                    theme.highlight_blue_soft.g,
                    theme.highlight_blue_soft.b,
                    92,
                ),
            );
        }
        push_text(
            primitives,
            DEVTOOLS_TREE_TEXT_WIDGET_ID,
            row.label.clone(),
            row_rect,
            if row.selected {
                theme.text_primary
            } else {
                theme.text_muted
            },
            PaintTextAlign::Left,
        );
    }

    push_fill_rect(
        primitives,
        DEVTOOLS_OVERLAY_WIDGET_ID,
        tree_detail_separator(panel, origin),
        theme.border,
    );
}

fn append_devtools_detail_rows(
    primitives: &mut Vec<PaintPrimitive>,
    theme: &ThemeTokens,
    panel: Rect,
    projection: &DevtoolsInspectorProjection,
) {
    let origin = Point::new(
        panel.min.x + DEVTOOLS_PANEL_PADDING + DEVTOOLS_TREE_WIDTH + 12.0,
        panel.min.y + DEVTOOLS_PANEL_PADDING + 26.0,
    );
    let width = panel.max.x - origin.x - DEVTOOLS_PANEL_PADDING;
    let mut y = append_devtools_line(primitives, theme, "Selected node", origin, width, true);
    for line in projection.selected_details.iter().take(8) {
        y = append_devtools_line(
            primitives,
            theme,
            line,
            Point::new(origin.x, y),
            width,
            false,
        );
    }
    y += 8.0;
    y = append_devtools_line(
        primitives,
        theme,
        "Runtime",
        Point::new(origin.x, y),
        width,
        true,
    );
    for line in projection.runtime_details.iter().take(5) {
        y = append_devtools_line(
            primitives,
            theme,
            line,
            Point::new(origin.x, y),
            width,
            false,
        );
    }
    let _ = y;
}

fn tree_detail_separator(panel: Rect, origin: Point) -> Rect {
    Rect::from_min_size(
        Point::new(
            panel.min.x + DEVTOOLS_PANEL_PADDING + DEVTOOLS_TREE_WIDTH,
            origin.y,
        ),
        Vector2::new(1.0, panel.height() - DEVTOOLS_PANEL_PADDING * 2.0 - 26.0),
    )
}

fn append_devtools_line(
    primitives: &mut Vec<PaintPrimitive>,
    theme: &ThemeTokens,
    text: impl Into<String>,
    origin: Point,
    width: f32,
    heading: bool,
) -> f32 {
    push_text(
        primitives,
        DEVTOOLS_DETAIL_TEXT_WIDGET_ID,
        text,
        Rect::from_min_size(origin, Vector2::new(width, DEVTOOLS_ROW_HEIGHT)),
        if heading {
            theme.text_primary
        } else {
            theme.text_muted
        },
        PaintTextAlign::Left,
    );
    origin.y + DEVTOOLS_ROW_HEIGHT
}
