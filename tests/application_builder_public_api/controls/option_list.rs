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

    let _surface = ui::compact_option_list_from_parts::<()>(
        ui::CompactOptionListParts::new(items, 96.0)
            .max_visible_rows(4)
            .row_height(18.0)
            .vertical_chrome(6.0)
            .column_gap(8.0)
            .padding(3.0),
    )
    .into_surface();
}
