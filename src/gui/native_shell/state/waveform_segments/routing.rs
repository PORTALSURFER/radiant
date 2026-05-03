//! Static segment routing helpers for waveform-related shell primitives.

use super::*;

/// Resolve which static segment owns one primitive.
pub(in crate::gui::native_shell::state) fn static_segment_for_primitive(
    layout: &ShellLayout,
    model: &AppModel,
    primitive: &Primitive,
) -> StaticFrameSegment {
    let anchor = match primitive {
        Primitive::Rect(fill) => fill.rect.center(),
        Primitive::Circle(fill) => fill.center,
        Primitive::LinearGradient(fill) => fill.rect.center(),
        Primitive::Image(image) => image.rect.center(),
    };
    static_segment_for_point(layout, model, anchor)
}

/// Resolve which static segment owns one text run.
pub(in crate::gui::native_shell::state) fn static_segment_for_text(
    layout: &ShellLayout,
    model: &AppModel,
    text_run: &TextRun,
) -> StaticFrameSegment {
    static_segment_for_point(layout, model, text_run.position)
}

/// Resolve the owning static segment for a point in shell coordinates.
fn static_segment_for_point(
    layout: &ShellLayout,
    model: &AppModel,
    point: Point,
) -> StaticFrameSegment {
    if layout.status_bar.contains(point) {
        return StaticFrameSegment::StatusBar;
    }
    if layout.waveform_card.contains(point) {
        return StaticFrameSegment::WaveformOverlay;
    }
    if model.map.active
        && (layout.browser_rows.contains(point) || layout.browser_table_header.contains(point))
    {
        return StaticFrameSegment::MapPanel;
    }
    if layout.browser_rows.contains(point) {
        return StaticFrameSegment::BrowserRowsWindow;
    }
    if layout.browser_panel.contains(point)
        || layout.browser_tabs.contains(point)
        || layout.browser_toolbar.contains(point)
        || layout.browser_table_header.contains(point)
        || layout.browser_footer.contains(point)
    {
        return StaticFrameSegment::BrowserFrame;
    }
    StaticFrameSegment::GlobalStatic
}

/// Return whether one static build pass should include the requested segment.
pub(in crate::gui::native_shell::state) fn static_segment_matches(
    filter: Option<StaticFrameSegment>,
    segment: StaticFrameSegment,
) -> bool {
    filter.is_none_or(|target| target == segment)
}
