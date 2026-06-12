use super::super::{
    CompactOptionListAnchoredParts, CompactOptionListItem, CompactOptionListParts,
    compact_option_list_anchored_with_activation, compact_option_list_from_parts_with_activation,
    compact_option_list_from_parts_with_interaction,
};
use crate::{
    application::{IntoView, LayerHorizontalAnchor, LayerVerticalAnchor, stack, text},
    gui::types::Point,
    layout::Vector2,
    widgets::WidgetInput,
};

#[test]
fn compact_option_list_activation_maps_clicked_row_index() {
    let bridge = crate::runtime::DeclarativeOwnedRuntimeBridge::new(
        Vec::<usize>::new(),
        |_| {
            let items = vec![
                CompactOptionListItem::new("Kick"),
                CompactOptionListItem::new("Snare").selected(true),
            ];
            let list = CompactOptionListParts::new(items, 80.0);
            compact_option_list_from_parts_with_activation(list, Some).into_surface()
        },
        |state, message| state.push(message),
    );
    let mut runtime = crate::runtime::SurfaceRuntime::new(bridge, Vector2::new(160.0, 80.0));
    let click_rect = runtime
        .frame_with_default_theme()
        .paint_plan
        .first_text_rect("Snare")
        .expect("second option should paint");

    runtime.dispatch_primary_click(click_rect.center());

    assert_eq!(runtime.bridge().state(), &[1]);
}

#[test]
fn compact_option_list_interaction_maps_hovered_row_index() {
    let bridge = crate::runtime::DeclarativeOwnedRuntimeBridge::new(
        Vec::<usize>::new(),
        |_| {
            let items = vec![
                CompactOptionListItem::new("Kick"),
                CompactOptionListItem::new("Snare").selected(true),
            ];
            let list = CompactOptionListParts::new(items, 80.0);
            compact_option_list_from_parts_with_interaction(list, |_| None, Some).into_surface()
        },
        |state, message| state.push(message),
    );
    let mut runtime = crate::runtime::SurfaceRuntime::new(bridge, Vector2::new(160.0, 80.0));
    let hover_rect = runtime
        .frame_with_default_theme()
        .paint_plan
        .first_text_rect("Snare")
        .expect("second option should paint");

    runtime.dispatch_input_at(
        hover_rect.center(),
        WidgetInput::PointerMove {
            position: hover_rect.center(),
        },
    );

    assert_eq!(runtime.bridge().state(), &[1]);
}

#[test]
fn compact_option_list_interaction_maps_hover_across_full_row_width() {
    let bridge = crate::runtime::DeclarativeOwnedRuntimeBridge::new(
        Vec::<usize>::new(),
        |_| {
            let items = vec![
                CompactOptionListItem::new("Kick").secondary_label("Drum"),
                CompactOptionListItem::new("Snare")
                    .secondary_label("Drum")
                    .selected(true),
            ];
            let list = CompactOptionListParts::new(items, 80.0);
            compact_option_list_from_parts_with_interaction(list, |_| None, Some)
                .width(180.0)
                .into_surface()
        },
        |state, message| state.push(message),
    );
    let mut runtime = crate::runtime::SurfaceRuntime::new(bridge, Vector2::new(180.0, 80.0));
    let snare_rect = runtime
        .frame_with_default_theme()
        .paint_plan
        .first_text_rect("Snare")
        .expect("second option should paint");
    let right_side = Point::new(168.0, snare_rect.center().y);

    runtime.dispatch_input_at(
        right_side,
        WidgetInput::PointerMove {
            position: right_side,
        },
    );

    assert_eq!(runtime.bridge().state(), &[1]);
}

#[test]
fn compact_option_list_anchored_activation_maps_clicked_row_index() {
    let bridge = crate::runtime::DeclarativeOwnedRuntimeBridge::new(
        Vec::<usize>::new(),
        |_| {
            let items = vec![
                CompactOptionListItem::new("Kick"),
                CompactOptionListItem::new("Snare").selected(true),
            ];
            let list = CompactOptionListParts::new(items, 80.0);
            let popup = compact_option_list_anchored_with_activation(
                CompactOptionListAnchoredParts::new(
                    list,
                    120.0,
                    LayerHorizontalAnchor::Start,
                    LayerVerticalAnchor::End,
                    8.0,
                    8.0,
                ),
                Some,
            );
            stack([text("").size(160.0, 100.0), popup]).into_surface()
        },
        |state, message| state.push(message),
    );
    let mut runtime = crate::runtime::SurfaceRuntime::new(bridge, Vector2::new(160.0, 100.0));
    let click_rect = runtime
        .frame_with_default_theme()
        .paint_plan
        .first_text_rect("Snare")
        .expect("second anchored option should paint");

    runtime.dispatch_primary_click(click_rect.center());

    assert_eq!(runtime.bridge().state(), &[1]);
}
