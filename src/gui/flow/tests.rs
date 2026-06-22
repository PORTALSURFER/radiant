use super::packer::collect_flow_group;
use super::*;

fn metrics() -> FlowLayoutMetrics {
    FlowLayoutMetrics::new(3.0, 5.0, 18.0)
}

#[test]
fn pack_flow_rows_wraps_variable_width_items() {
    let rows = pack_flow_rows(
        [
            FlowItem::new("one", 40.0),
            FlowItem::new("two", 40.0),
            FlowItem::new("three", 40.0),
        ],
        90.0,
        metrics(),
    );

    assert_eq!(rows, vec![vec!["one", "two"], vec!["three"]]);
}

#[test]
fn flow_rows_height_includes_line_gaps_between_rows() {
    assert_eq!(flow_rows_height(0, metrics()), 0.0);
    assert_eq!(flow_rows_height(1, metrics()), 18.0);
    assert_eq!(flow_rows_height(3, metrics()), 64.0);
}

#[test]
fn flow_field_metrics_clamps_content_width_and_visible_rows() {
    let field = FlowFieldMetrics::new(metrics(), 26.0, 6.0, 120.0, 3);

    assert_eq!(field.content_width(400.0), 374.0);
    assert_eq!(field.content_width(80.0), 120.0);
    assert_eq!(field.visible_row_count(0), 1);
    assert_eq!(field.visible_row_count(2), 2);
    assert_eq!(field.visible_row_count(8), 3);
    assert_eq!(field.visible_rows_height(8), 64.0);
    assert_eq!(field.visible_field_height(8), 70.0);
    assert!(field.requires_scroll(4));
    assert!(!field.requires_scroll(3));
}

#[test]
fn flow_field_layout_resolves_content_height_and_scroll_policy() {
    let field = FlowFieldMetrics::new(metrics(), 26.0, 6.0, 120.0, 3);

    assert_eq!(
        field.layout(400.0, 8),
        FlowFieldLayout {
            content_width: 374.0,
            row_count: 8,
            visible_row_count: 3,
            content_height: 64.0,
            field_height: 70.0,
            requires_scroll: true,
        }
    );
    assert_eq!(field.layout(80.0, 0).content_width, 120.0);
    assert_eq!(field.layout_for_content_width(80.0, 0).content_width, 120.0);
}

#[test]
fn capped_flow_rows_height_clamps_rows_and_adds_chrome() {
    assert_eq!(capped_flow_rows_height(0, 1, 6, 6.0, metrics()), 24.0);
    assert_eq!(capped_flow_rows_height(3, 1, 6, 6.0, metrics()), 70.0);
    assert_eq!(capped_flow_rows_height(9, 1, 6, 6.0, metrics()), 139.0);
    assert_eq!(capped_flow_rows_height(2, 4, 3, -12.0, metrics()), 64.0);
}

#[test]
fn push_flow_row_item_appends_to_existing_rows() {
    #[derive(Clone)]
    struct SizedItem(&'static str, f32);

    impl FlowItemWidth for SizedItem {
        fn flow_width(&self) -> f32 {
            self.1
        }
    }

    let mut rows = vec![vec![SizedItem("one", 40.0)]];
    push_flow_row_item(&mut rows, SizedItem("two", 40.0), 40.0, 90.0, metrics());
    push_flow_row_item(&mut rows, SizedItem("three", 40.0), 40.0, 90.0, metrics());

    assert_eq!(rows.len(), 2);
    assert_eq!(
        rows[0].iter().map(|item| item.0).collect::<Vec<_>>(),
        ["one", "two"]
    );
    assert_eq!(
        rows[1].iter().map(|item| item.0).collect::<Vec<_>>(),
        ["three"]
    );
}

#[test]
fn flow_row_packer_tracks_width_without_rescanning_rows() {
    #[derive(Clone, Debug, PartialEq)]
    struct SizedItem(&'static str, f32);

    let mut packer = FlowRowPacker::new(90.0, metrics());
    packer.push_item(SizedItem("one", 40.0), 40.0);
    assert_eq!(packer.current_row_width(), 40.0);
    packer.push_item(SizedItem("two", 40.0), 40.0);
    assert_eq!(packer.current_row_width(), 83.0);
    packer.push_item(SizedItem("three", 40.0), 40.0);

    assert_eq!(packer.current_row_width(), 40.0);
    assert_eq!(
        packer.into_rows(),
        vec![
            vec![SizedItem("one", 40.0), SizedItem("two", 40.0)],
            vec![SizedItem("three", 40.0)]
        ]
    );
}

#[test]
fn flow_row_packer_keeps_group_items_atomic() {
    #[derive(Clone, Debug, PartialEq)]
    struct SizedItem(&'static str, f32);

    let mut packer = FlowRowPacker::new(150.0, metrics());
    packer.push_item(SizedItem("pill", 86.0), 86.0);
    packer.push_group([
        FlowItem::new(SizedItem("prefix", 50.0), 50.0),
        FlowItem::new(SizedItem("input", 70.0), 70.0),
    ]);

    assert_eq!(packer.current_row_width(), 123.0);
    assert_eq!(
        packer.rows(),
        &[
            vec![SizedItem("pill", 86.0)],
            vec![SizedItem("prefix", 50.0), SizedItem("input", 70.0)]
        ]
    );
}

#[test]
fn flow_group_collection_computes_width_while_extracting_payloads() {
    let (values, width) = collect_flow_group(
        [
            FlowItem::new("prefix", 50.0),
            FlowItem::new("input", 70.0),
            FlowItem::new("negative", -10.0),
        ],
        metrics().item_gap,
    );

    assert_eq!(values, ["prefix", "input", "negative"]);
    assert_eq!(width, 126.0);
}

#[test]
fn pack_flow_rows_with_trailing_item_keeps_editor_on_roomy_row() {
    #[derive(Clone, Debug, PartialEq)]
    struct SizedItem(&'static str, f32);

    impl FlowItemWidth for SizedItem {
        fn flow_width(&self) -> f32 {
            self.1
        }
    }

    let rows = pack_flow_rows_with_trailing_item(
        [FlowItem::new(SizedItem("pill", 38.0), 38.0)],
        FlowTrailingItemParts::new(|width| SizedItem("input", width), 61.0, 180.0, 100.0),
        180.0,
        metrics(),
    );

    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0], [SizedItem("pill", 38.0), SizedItem("input", 61.0)]);
}

#[test]
fn pack_flow_rows_with_trailing_item_uses_standalone_width_on_new_row() {
    #[derive(Clone, Debug, PartialEq)]
    struct SizedItem(&'static str, f32);

    impl FlowItemWidth for SizedItem {
        fn flow_width(&self) -> f32 {
            self.1
        }
    }

    let rows = pack_flow_rows_with_trailing_item(
        [
            FlowItem::new(SizedItem("one", 38.0), 38.0),
            FlowItem::new(SizedItem("two", 42.0), 42.0),
        ],
        FlowTrailingItemParts::new(|width| SizedItem("input", width), 61.0, 180.0, 100.0),
        180.0,
        metrics(),
    );

    assert_eq!(rows.len(), 2);
    assert_eq!(rows[0], [SizedItem("one", 38.0), SizedItem("two", 42.0)]);
    assert_eq!(rows[1], [SizedItem("input", 180.0)]);
}

#[test]
fn pack_flow_rows_with_trailing_item_handles_empty_items() {
    #[derive(Clone, Debug, PartialEq)]
    struct SizedItem(&'static str, f32);

    impl FlowItemWidth for SizedItem {
        fn flow_width(&self) -> f32 {
            self.1
        }
    }

    let rows = pack_flow_rows_with_trailing_item(
        [],
        FlowTrailingItemParts::new(|width| SizedItem("input", width), 61.0, 180.0, 100.0),
        180.0,
        metrics(),
    );

    assert_eq!(rows, vec![vec![SizedItem("input", 180.0)]]);
}

#[test]
fn trailing_item_parts_reserve_following_action_width() {
    let parts = FlowTrailingItemParts::new(|width: f32| width, 120.0, 240.0, 100.0)
        .reserve_following_item(22.0, 3.0, 61.0);

    assert_eq!(parts.width, 95.0);
    assert_eq!(parts.standalone_width, 215.0);
    assert_eq!(
        flow_width_with_following_item_reserved(70.0, 22.0, 3.0, 61.0),
        61.0
    );
    assert_eq!(
        flow_width_with_following_item_reserved(f32::INFINITY, 22.0, 3.0, 61.0),
        0.0
    );
}

#[test]
fn pack_flow_rows_with_trailing_item_and_following_item_keeps_action_after_editor() {
    #[derive(Clone, Debug, PartialEq)]
    struct SizedItem(&'static str, f32);

    impl FlowItemWidth for SizedItem {
        fn flow_width(&self) -> f32 {
            self.1
        }
    }

    let rows = pack_flow_rows_with_trailing_item_and_following_item(
        [FlowItem::new(SizedItem("pill", 38.0), 38.0)],
        FlowTrailingItemParts::new(|width| SizedItem("input", width), 120.0, 180.0, 90.0),
        Some(FlowItem::new(SizedItem("toggle", 22.0), 22.0)),
        61.0,
        220.0,
        metrics(),
    );

    assert_eq!(
        rows,
        vec![vec![
            SizedItem("pill", 38.0),
            SizedItem("input", 95.0),
            SizedItem("toggle", 22.0)
        ]]
    );
}

#[test]
fn pack_flow_rows_with_trailing_item_and_following_item_reserves_standalone_width() {
    #[derive(Clone, Debug, PartialEq)]
    struct SizedItem(&'static str, f32);

    impl FlowItemWidth for SizedItem {
        fn flow_width(&self) -> f32 {
            self.1
        }
    }

    let rows = pack_flow_rows_with_trailing_item_and_following_item(
        [FlowItem::new(SizedItem("wide-pill", 160.0), 160.0)],
        FlowTrailingItemParts::new(|width| SizedItem("input", width), 120.0, 220.0, 90.0),
        Some(FlowItem::new(SizedItem("toggle", 22.0), 22.0)),
        61.0,
        220.0,
        metrics(),
    );

    assert_eq!(
        rows,
        vec![
            vec![SizedItem("wide-pill", 160.0)],
            vec![SizedItem("input", 195.0), SizedItem("toggle", 22.0)]
        ]
    );
}

#[test]
fn push_flow_row_group_keeps_items_together_when_wrapping() {
    #[derive(Clone, Debug, PartialEq)]
    struct SizedItem(&'static str, f32);

    impl FlowItemWidth for SizedItem {
        fn flow_width(&self) -> f32 {
            self.1
        }
    }

    let mut rows = pack_flow_rows(
        [FlowItem::new(SizedItem("pill", 86.0), 86.0)],
        150.0,
        metrics(),
    );
    push_flow_row_group(
        &mut rows,
        [
            FlowItem::new(SizedItem("prefix", 50.0), 50.0),
            FlowItem::new(SizedItem("input", 70.0), 70.0),
        ],
        150.0,
        metrics(),
    );

    assert_eq!(
        rows,
        vec![
            vec![SizedItem("pill", 86.0)],
            vec![SizedItem("prefix", 50.0), SizedItem("input", 70.0)]
        ]
    );
}

#[test]
fn pack_flow_rows_with_trailing_group_wraps_group_atomically() {
    #[derive(Clone, Debug, PartialEq)]
    struct SizedItem(&'static str, f32);

    impl FlowItemWidth for SizedItem {
        fn flow_width(&self) -> f32 {
            self.1
        }
    }

    let rows = pack_flow_rows_with_trailing_group(
        [FlowItem::new(SizedItem("pill", 86.0), 86.0)],
        [
            FlowItem::new(SizedItem("prefix", 50.0), 50.0),
            FlowItem::new(SizedItem("input", 70.0), 70.0),
        ],
        150.0,
        metrics(),
    );

    assert_eq!(
        rows,
        vec![
            vec![SizedItem("pill", 86.0)],
            vec![SizedItem("prefix", 50.0), SizedItem("input", 70.0)]
        ]
    );
}

#[test]
fn trailing_item_moves_when_remaining_width_is_too_small() {
    assert!(flow_trailing_item_starts_new_row(
        [38.0, 42.0],
        61.0,
        100.0,
        180.0,
        metrics()
    ));
    assert!(!flow_trailing_item_starts_new_row(
        [38.0],
        61.0,
        100.0,
        180.0,
        metrics()
    ));
}
