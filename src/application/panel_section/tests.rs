use super::*;
use crate::{
    application::{IntoView, column, spacer, text},
    gui::types::Point,
    layout::Vector2,
    widgets::{DragHandleMessage, WidgetOutput, WidgetStyle, WidgetTone},
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
fn panel_section_header_parts_builds_custom_header_section() {
    const HEADER_ID: u64 = 4_201;
    const BODY_ID: u64 = 4_202;

    let header =
        panel_section_resize_header("custom-header-resize", 5.0, |_| "resize").id(HEADER_ID);
    let body = text("Body").id(BODY_ID).fill_width().height(20.0);
    let section = panel_section_from_header_parts(
        PanelSectionHeaderParts::new(header, body)
            .padding(6.0)
            .spacing(1.0)
            .height(80.0),
    );
    let layout = section.view_layout_at_size(Vector2::new(240.0, 100.0));
    let header_rect = layout
        .rects
        .get(&HEADER_ID)
        .expect("custom panel header should be laid out");
    let body_rect = layout
        .rects
        .get(&BODY_ID)
        .expect("custom panel body should be laid out");

    assert!(header_rect.width() >= 228.0);
    assert_eq!(header_rect.height(), 5.0);
    assert_eq!(body_rect.min.y, header_rect.max.y + 1.0);
}

#[test]
fn panel_section_header_parts_routes_custom_header_input() {
    const HEADER_ID: u64 = 4_203;

    let header =
        panel_section_resize_header("custom-header-resize", 5.0, |_| "resize").id(HEADER_ID);
    let section = panel_section_from_header_parts(
        PanelSectionHeaderParts::new(header, text("Body")).height(80.0),
    );

    assert_eq!(
        section.view_dispatch_widget_output(
            HEADER_ID,
            WidgetOutput::typed(DragHandleMessage::started(Point::new(8.0, 8.0))),
        ),
        Some("resize")
    );
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
fn panel_section_dialog_parts_use_standard_dialog_chrome() {
    let parts: PanelSectionParts<()> =
        PanelSectionParts::dialog("Confirm", text("Body"), WidgetTone::Warning);

    assert_eq!(parts.title, "Confirm");
    assert_eq!(parts.style, WidgetStyle::strong(WidgetTone::Warning));
    assert_eq!(parts.padding, 8.0);
    assert_eq!(parts.spacing, 6.0);
    assert_eq!(parts.title_height, 24.0);
    assert_eq!(parts.geometry().header_only_height(), 40.0);
}

#[test]
fn dialog_layer_projects_standard_dialog_panel() {
    let frame = dialog_layer::<()>(
        "Confirm",
        text("Body"),
        WidgetTone::Warning,
        Vector2::new(220.0, 120.0),
    )
    .view_frame_at_size_with_default_theme(Vector2::new(320.0, 220.0));

    assert!(frame.paint_plan.contains_text("Confirm"));
    assert!(frame.paint_plan.contains_text("Body"));
}

#[test]
fn closeable_dialog_layer_projects_standard_dialog_panel() {
    let frame = closeable_dialog_layer(
        "Settings",
        text("Body"),
        WidgetTone::Neutral,
        Vector2::new(220.0, 120.0),
        "close",
    )
    .view_frame_at_size_with_default_theme(Vector2::new(320.0, 220.0));

    assert!(frame.paint_plan.contains_text("Settings"));
    assert!(frame.paint_plan.contains_text("Body"));
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
