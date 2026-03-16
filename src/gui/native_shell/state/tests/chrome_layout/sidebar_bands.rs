use super::super::*;

#[test]
fn sidebar_sections_keep_non_overlapping_bands_across_tiers() {
    let sizes = [
        Vector2::new(820.0, 520.0),
        Vector2::new(1280.0, 720.0),
        Vector2::new(2300.0, 1080.0),
    ];
    let mut state = NativeShellState::new();
    let model = populated_sidebar_model();
    for viewport in sizes {
        let layout = ShellLayout::build(viewport);
        let style = style_for_layout(&layout);
        let sections = sidebar_sections(&layout, &style, &model);
        let rendered_sources = state.rendered_source_row_rects(&layout, &model);
        assert_rect_inside(layout.sidebar_rows, sections.source_rows);
        assert_rect_inside(layout.sidebar_rows, sections.folder_header);
        assert_rect_inside(layout.sidebar_rows, sections.folder_rows);
        assert!(sections.source_rows.max.y <= sections.folder_header.min.y);
        assert!(sections.folder_header.max.y <= sections.folder_rows.min.y);
        assert!(!rendered_sources.is_empty());
    }
}

#[test]
fn sidebar_sections_remain_stable_in_cramped_viewports() {
    let layout = ShellLayout::build(Vector2::new(820.0, 400.0));
    let style = style_for_layout(&layout);
    let model = populated_sidebar_model();
    let sections = sidebar_sections(&layout, &style, &model);
    assert_rect_inside(layout.sidebar_rows, sections.source_rows);
    assert_rect_inside(layout.sidebar_rows, sections.folder_header);
    assert_rect_inside(layout.sidebar_rows, sections.folder_rows);
    assert!(sections.source_rows.max.y <= sections.folder_header.min.y);
    assert!(sections.folder_header.max.y <= sections.folder_rows.min.y);
}
