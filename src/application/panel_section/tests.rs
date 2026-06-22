use super::*;
use crate::{
    application::{IntoView, column, spacer, text},
    layout::Vector2,
    widgets::{DragHandleMessage, WidgetOutput},
};

#[test]
fn panel_section_parts_adds_trailing_resize_handle() {
    let parts = PanelSectionParts::new("Inspector", text("Body"))
        .trailing_resize_handle("inspector-resize", |_| "resize")
        .height(80.0);

    assert!(parts.trailing.is_some());

    let frame = panel_section_from_parts(parts)
        .view_frame_at_size_with_default_theme(Vector2::new(240.0, 120.0));
    assert!(frame.paint_plan.contains_text("Inspector"));
}

#[test]
fn panel_section_parts_exposes_content_offsets() {
    let parts: PanelSectionParts<()> = PanelSectionParts::new("Inspector", text("Body"))
        .padding(8.0)
        .title_height(22.0)
        .spacing(5.0);

    assert_eq!(parts.geometry().header_only_height(), 38.0);
    assert_eq!(parts.content_top_offset(), 35.0);
    assert_eq!(parts.content_top_inset_from_bottom(120.0), 85.0);
    assert_eq!(parts.content_bottom_inset(), 8.0);
    assert_eq!(parts.section_height_for_content_height(64.0), 107.0);
    assert_eq!(parts.content_height_for_section_height(107.0), 64.0);
}

#[test]
fn panel_section_content_offsets_sanitize_invalid_inputs() {
    let parts: PanelSectionParts<()> = PanelSectionParts::new("Inspector", text("Body"))
        .padding(f32::NAN)
        .title_height(f32::INFINITY)
        .spacing(-4.0);

    assert_eq!(parts.content_top_offset(), 0.0);
    assert_eq!(parts.content_top_inset_from_bottom(f32::NAN), 0.0);
    assert_eq!(parts.content_bottom_inset(), 0.0);
    assert_eq!(parts.section_height_for_content_height(f32::INFINITY), 0.0);
    assert_eq!(parts.content_height_for_section_height(-4.0), 0.0);
}

#[test]
fn panel_section_geometry_exposes_content_height_conversion() {
    let geometry = PanelSectionGeometry::new()
        .padding(6.0)
        .title_height(20.0)
        .spacing(4.0);

    assert_eq!(geometry.header_only_height(), 32.0);
    assert_eq!(geometry.content_top_offset(), 30.0);
    assert_eq!(geometry.section_height_for_content_height(100.0), 136.0);
    assert_eq!(geometry.content_height_for_section_height(136.0), 100.0);
}

#[test]
fn panel_section_resize_header_spans_width_and_routes_drag_messages() {
    const HEADER_ID: u64 = 4_200;

    let header =
        panel_section_resize_header("inspector-header-resize", 24.0, |_| "resize").id(HEADER_ID);
    let layout = column([header, spacer()]).view_layout_at_size(Vector2::new(240.0, 48.0));
    let rect = layout
        .rects
        .get(&HEADER_ID)
        .expect("resize header should be laid out");

    assert_eq!(rect.width(), 240.0);
    assert_eq!(rect.height(), 24.0);

    assert_eq!(
        panel_section_resize_header("inspector-header-resize", 24.0, |_| "resize")
            .id(HEADER_ID)
            .view_dispatch_widget_output(
                HEADER_ID,
                WidgetOutput::typed(DragHandleMessage::started(rect.center())),
            ),
        Some("resize")
    );
}
