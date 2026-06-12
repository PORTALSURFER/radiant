use super::super::{
    CompactOptionListAnchoredParts, CompactOptionListFloatingAboveParts, CompactOptionListItem,
    CompactOptionListParts, compact_option_list_anchored, compact_option_list_floating_above,
};
use crate::{
    application::{IntoView, LayerHorizontalAnchor, LayerVerticalAnchor, stack, text},
    gui::types::{Point, Rect},
    layout::Vector2,
    runtime::{PaintPrimitive, UiSurface},
};

#[test]
fn compact_option_list_floating_above_positions_popup_before_trigger() {
    let items = vec![CompactOptionListItem::new("Kick").secondary_label("Drum")];
    let list = CompactOptionListParts::new(items, 80.0)
        .row_height(18.0)
        .vertical_chrome(6.0);
    let popup = compact_option_list_floating_above::<()>(CompactOptionListFloatingAboveParts::new(
        list, 10.0, 64.0, 4.0, 160.0,
    ));

    let frame = UiSurface::new(stack([text("").size(220.0, 120.0), popup]).into_node()).frame(
        Rect::from_min_size(Point::new(0.0, 0.0), Vector2::new(220.0, 120.0)),
        &Default::default(),
    );

    let text_rect = frame
        .paint_plan
        .primitives
        .iter()
        .find_map(|primitive| match primitive {
            PaintPrimitive::Text(text) if text.text.as_str() == "Kick" => Some(text.rect),
            _ => None,
        })
        .expect("floating option list should paint item text");

    assert!((text_rect.min.x - 17.0).abs() < 0.01, "{text_rect:?}");
    assert!((text_rect.min.y - 43.0).abs() < 0.01, "{text_rect:?}");
}

#[test]
fn compact_option_list_anchored_positions_popup_from_parent_edges() {
    let items = vec![CompactOptionListItem::new("Kick").secondary_label("Drum")];
    let list = CompactOptionListParts::new(items, 80.0)
        .row_height(18.0)
        .vertical_chrome(6.0);
    let popup = compact_option_list_anchored::<()>(CompactOptionListAnchoredParts::new(
        list,
        160.0,
        LayerHorizontalAnchor::Start,
        LayerVerticalAnchor::End,
        12.0,
        24.0,
    ));

    let frame = UiSurface::new(stack([text("").size(220.0, 120.0), popup]).into_node()).frame(
        Rect::from_min_size(Point::new(0.0, 0.0), Vector2::new(220.0, 120.0)),
        &Default::default(),
    );

    let text_rect = frame
        .paint_plan
        .primitives
        .iter()
        .find_map(|primitive| match primitive {
            PaintPrimitive::Text(text) if text.text.as_str() == "Kick" => Some(text.rect),
            _ => None,
        })
        .expect("anchored option list should paint item text");

    assert!((text_rect.min.x - 19.0).abs() < 0.01, "{text_rect:?}");
    assert!((text_rect.min.y - 79.0).abs() < 0.01, "{text_rect:?}");
}
