use super::*;
use crate::{application::IntoView, application::text, layout::Vector2};

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

    assert_eq!(geometry.content_top_offset(), 30.0);
    assert_eq!(geometry.section_height_for_content_height(100.0), 136.0);
    assert_eq!(geometry.content_height_for_section_height(136.0), 100.0);
}
