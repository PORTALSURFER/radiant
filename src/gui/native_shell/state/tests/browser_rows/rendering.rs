use super::*;

#[test]
fn browser_inline_metadata_prefers_explicit_row_metadata() {
    let layout = ShellLayout::build(Vector2::new(1280.0, 720.0));
    let mut state = NativeShellState::new();
    let mut model = AppModel::default();
    model
        .browser
        .rows
        .push(BrowserRowModel::new(0, "Kick 01", 1, true, true).with_bucket_label("165 BPM"));
    let frame = state.build_frame(&layout, &model);
    assert!(
        frame
            .text_runs
            .iter()
            .any(|run| run.text.contains("165 BPM"))
    );
}

#[test]
fn browser_inline_metadata_tags_render_chip_backgrounds() {
    let layout = ShellLayout::build(Vector2::new(1280.0, 720.0));
    let style = style_for_layout(&layout);
    let mut state = NativeShellState::new();
    let mut model = AppModel::default();
    model.browser.rows.push(
        BrowserRowModel::new(0, "Kick 01", 1, true, true)
            .with_bucket_label("165 BPM · LOOP · LONG"),
    );
    let frame = state.build_frame(&layout, &model);
    let rendered = rendered_browser_rows(&layout, &model, &style);
    let row = rendered.first().expect("browser row should render");
    let row_text_layout = compute_browser_row_text_layout(row.rect, style.sizing);
    let expected_chip_rects = browser_inline_tag_chip_rects(
        row_text_layout.sample_label,
        &row.bucket_label,
        0.0,
        style.sizing,
    );
    assert_eq!(expected_chip_rects.len(), 3);
    for rect in expected_chip_rects {
        assert!(frame.primitives.iter().any(|primitive| {
            matches!(
                primitive,
                Primitive::Rect(FillRect { rect: primitive_rect, color })
                    if *primitive_rect == rect
                        && *color == blend_color(style.surface_overlay, style.bg_tertiary, 0.54)
            )
        }));
    }
    for label in ["165 BPM", "LOOP", "LONG"] {
        assert!(frame.text_runs.iter().any(|run| run.text == label));
    }
}

#[test]
fn browser_header_omits_bucket_label() {
    let layout = ShellLayout::build(Vector2::new(1280.0, 720.0));
    let mut state = NativeShellState::new();
    let model = browser_model_with_rows(24, 8);
    let frame = state.build_frame(&layout, &model);
    assert!(!frame.text_runs.iter().any(|run| run.text == "Bucket"));
}

#[test]
fn static_segments_include_browser_rows_when_list_tab_is_active() {
    let layout = ShellLayout::build(Vector2::new(1280.0, 720.0));
    let style = style_for_layout(&layout);
    let mut state = NativeShellState::new();
    let model = browser_model_with_rows(120, 40);
    let mut segments = StaticFrameSegments::default();
    for segment in StaticFrameSegment::ALL {
        state.build_static_segment_with_style_into(
            &layout,
            &style,
            &model,
            None,
            segment,
            &mut segments,
        );
    }
    let rows_segment = segments.frame(StaticFrameSegment::BrowserRowsWindow);
    let map_segment = segments.frame(StaticFrameSegment::MapPanel);
    assert!(!rows_segment.primitives.is_empty());
    assert!(!rows_segment.text_runs.is_empty());
    assert!(map_segment.primitives.is_empty());
}

#[test]
fn static_segments_include_map_panel_when_map_tab_is_active() {
    let layout = ShellLayout::build(Vector2::new(1280.0, 720.0));
    let style = style_for_layout(&layout);
    let mut state = NativeShellState::new();
    let mut model = browser_model_with_rows(120, 40);
    model.map.active = true;
    model.map.summary = String::from("Map summary");
    model.map.selected_sample_id = Some(String::from("kick"));
    model.map.focused_sample_id = Some(String::from("kick"));
    model.map.points = std::sync::Arc::from(vec![crate::app::MapPointModel {
        sample_id: std::sync::Arc::<str>::from("kick"),
        x_milli: 512,
        y_milli: 480,
        cluster_id: Some(1),
    }]);
    let mut segments = StaticFrameSegments::default();
    for segment in StaticFrameSegment::ALL {
        state.build_static_segment_with_style_into(
            &layout,
            &style,
            &model,
            None,
            segment,
            &mut segments,
        );
    }
    let rows_segment = segments.frame(StaticFrameSegment::BrowserRowsWindow);
    let map_segment = segments.frame(StaticFrameSegment::MapPanel);
    assert!(rows_segment.primitives.is_empty());
    assert!(!map_segment.primitives.is_empty());
    assert!(!map_segment.text_runs.is_empty());
}

#[test]
fn browser_rows_use_alternating_fill_stripes_for_readability() {
    let layout = ShellLayout::build(Vector2::new(1280.0, 720.0));
    let style = style_for_layout(&layout);
    let mut state = NativeShellState::new();
    let mut model = AppModel::default();
    model
        .browser
        .rows
        .push(BrowserRowModel::new(0, "row_even", 1, false, false));
    model
        .browser
        .rows
        .push(BrowserRowModel::new(1, "row_odd", 1, false, false));
    model.browser.visible_count = model.browser.rows.len();
    let rendered = rendered_browser_rows(&layout, &model, &style);
    assert!(rendered.len() >= 2);

    let frame = state.build_frame(&layout, &model);
    let even_rect = rendered[0].rect;
    let odd_rect = rendered[1].rect;
    let even_fills: Vec<Rgba8> = frame
        .primitives
        .iter()
        .filter_map(|primitive| match primitive {
            Primitive::Rect(rect) if rect.rect == even_rect => Some(rect.color),
            _ => None,
        })
        .collect();
    let odd_fills: Vec<Rgba8> = frame
        .primitives
        .iter()
        .filter_map(|primitive| match primitive {
            Primitive::Rect(rect) if rect.rect == odd_rect => Some(rect.color),
            _ => None,
        })
        .collect();
    let expected_even = browser_row_stripe_fill(&style, 0);
    let expected_odd = browser_row_stripe_fill(&style, 1);
    assert!(!even_fills.is_empty(), "missing even-row fills");
    assert!(!odd_fills.is_empty(), "missing odd-row fills");
    assert!(
        even_fills.contains(&expected_even),
        "expected even-row fill {expected_even:?}, saw {even_fills:?}"
    );
    assert!(
        odd_fills.contains(&expected_odd),
        "expected odd-row fill {expected_odd:?}, saw {odd_fills:?}"
    );
    assert_ne!(expected_even, expected_odd);
}

#[test]
fn locked_browser_rows_keep_neutral_fill_and_draw_left_marker() {
    let layout = ShellLayout::build(Vector2::new(1280.0, 720.0));
    let style = style_for_layout(&layout);
    let mut state = NativeShellState::new();
    let mut model = AppModel::default();
    model
        .browser
        .rows
        .push(BrowserRowModel::new(0, "locked row", 1, false, false).with_locked(true));

    let rendered = rendered_browser_rows(&layout, &model, &style);
    let row = rendered.first().expect("browser row should render");
    let marker_rect =
        browser_locked_marker_rect(row.rect, style.sizing, 0.0).expect("locked marker");
    let frame = state.build_frame(&layout, &model);
    let row_fills: Vec<Rgba8> = frame
        .primitives
        .iter()
        .filter_map(|primitive| match primitive {
            Primitive::Rect(rect) if rect.rect == row.rect => Some(rect.color),
            _ => None,
        })
        .collect();

    assert!(
        row_fills.contains(&browser_row_stripe_fill(&style, 0)),
        "locked row should keep the standard stripe fill"
    );
    assert!(frame.primitives.iter().any(|primitive| {
        matches!(
            primitive,
            Primitive::Rect(FillRect { rect, color })
                if *rect == marker_rect && *color == style.accent_mint
        )
    }));
}
