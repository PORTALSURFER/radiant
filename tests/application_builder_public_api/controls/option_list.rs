use radiant::prelude as ui;
use radiant::prelude::IntoView;

#[test]
fn compact_option_list_exports_selected_primary_secondary_rows() {
    let items = vec![
        ui::CompactOptionListItem::new("Kick")
            .secondary_label("Drums")
            .selected(true),
        ui::CompactOptionListItem::new("Bass")
            .secondary_label("Instrument")
            .selected(false),
    ];

    let _surface = ui::compact_option_list::<()>(
        ui::CompactOptionListParts::new(items, 96.0)
            .max_visible_rows(4)
            .row_height(18.0)
            .vertical_chrome(6.0)
            .column_gap(8.0)
            .padding(3.0),
    )
    .view()
    .into_surface();
}

#[test]
fn compact_option_list_exports_fluent_interaction_and_placement() {
    let items = vec![ui::CompactOptionListItem::new("Kick").secondary_label("Drums")];
    let anchor = ui::CompactOptionListAnchor::new(
        180.0,
        ui::LayerHorizontalAnchor::Start,
        ui::LayerVerticalAnchor::End,
    )
    .inset(12.0, 24.0);

    let _surface = ui::compact_option_list(ui::CompactOptionListParts::new(items, 96.0))
        .on_activate(|index| index)
        .on_hover(|index| index)
        .anchored(anchor)
        .view()
        .into_surface();
}

#[test]
fn compact_option_list_exports_conditional_mapping_and_floating_placement() {
    let items = vec![ui::CompactOptionListItem::new("Kick")];
    let _surface = ui::compact_option_list(ui::CompactOptionListParts::new(items, 96.0))
        .filter_map_activate(|_| Some(()))
        .filter_map_hover(|_| None)
        .floating_above(ui::CompactOptionListFloatingAbove::new(
            10.0, 64.0, 4.0, 180.0,
        ))
        .view()
        .into_surface();
}
