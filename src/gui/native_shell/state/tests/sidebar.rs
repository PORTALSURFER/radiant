use super::*;

#[test]
fn folder_row_label_rect_indents_children_beyond_root() {
    let layout = ShellLayout::build(Vector2::new(1280.0, 720.0));
    let style = style_for_layout(&layout);
    let mut model = AppModel::default();
    model.sources.folder_rows.push(FolderRowModel::new(
        "Root",
        String::new(),
        0,
        false,
        false,
        true,
        true,
        true,
    ));
    model.sources.folder_rows.push(FolderRowModel::new(
        "Drums",
        String::new(),
        1,
        false,
        true,
        false,
        true,
        true,
    ));

    let rows = rendered_folder_row_rects(&layout, &style, &model);
    let root_layout = compute_sidebar_folder_row_layout(
        rows[0],
        style.sizing,
        compute_sidebar_folder_row_depth_indent(rows[0], style.sizing, 0),
    );
    let child_layout = compute_sidebar_folder_row_layout(
        rows[1],
        style.sizing,
        compute_sidebar_folder_row_depth_indent(rows[1], style.sizing, 1),
    );

    assert!(child_layout.label_rect.min.x > root_layout.label_rect.min.x);
    assert!(child_layout.disclosure_rect.min.x > root_layout.disclosure_rect.min.x);
}

#[test]
fn folder_rows_render_plain_labels_without_fallback_glyph_prefixes() {
    let layout = ShellLayout::build(Vector2::new(1280.0, 720.0));
    let mut model = AppModel::default();
    model.sources.folder_rows.push(FolderRowModel::new(
        "Root",
        String::new(),
        0,
        false,
        false,
        true,
        true,
        true,
    ));
    model.sources.folder_rows.push(FolderRowModel::new(
        "Drums",
        String::new(),
        1,
        false,
        true,
        false,
        true,
        true,
    ));

    let mut state = NativeShellState::new();
    let frame = state.build_frame(&layout, &model);

    assert!(frame.text_runs.iter().any(|run| run.text == "Drums"));
    assert!(frame.text_runs.iter().all(|run| !matches!(
        run.text.as_str(),
        "• Root" | "▶ Drums" | "▼ Drums" | "· Drums"
    )));
}

#[test]
fn disclosure_gutter_hit_target_is_reserved_only_for_expandable_rows() {
    let layout = ShellLayout::build(Vector2::new(1280.0, 720.0));
    let mut model = AppModel::default();
    model.sources.folder_rows.push(FolderRowModel::new(
        "Root",
        String::new(),
        0,
        false,
        false,
        true,
        true,
        true,
    ));
    model.sources.folder_rows.push(FolderRowModel::new(
        "Drums",
        String::new(),
        1,
        false,
        true,
        false,
        true,
        true,
    ));
    model.sources.folder_rows.push(FolderRowModel::new(
        "OneShots",
        String::new(),
        2,
        false,
        false,
        false,
        false,
        false,
    ));

    let mut state = NativeShellState::new();
    assert!(
        state
            .folder_row_disclosure_rect(&layout, &model, 1)
            .expect("expandable folder should reserve a gutter")
            .width()
            > 1.0
    );

    let leaf_rect = state
        .folder_row_disclosure_rect(&layout, &model, 2)
        .expect("leaf rows still compute gutter geometry");
    let leaf_point = Point::new(
        (leaf_rect.min.x + leaf_rect.max.x) * 0.5,
        (leaf_rect.min.y + leaf_rect.max.y) * 0.5,
    );
    assert_eq!(
        state.folder_row_disclosure_at_point(&layout, &model, leaf_point),
        None
    );
}

#[test]
fn folder_rows_use_single_pixel_shared_separator() {
    let layout = ShellLayout::build(Vector2::new(1280.0, 720.0));
    let style = style_for_layout(&layout);
    let mut model = AppModel::default();
    model.sources.folder_rows.push(FolderRowModel::new(
        "folder_a",
        String::new(),
        0,
        false,
        false,
        false,
        true,
        true,
    ));
    model.sources.folder_rows.push(FolderRowModel::new(
        "folder_b",
        String::new(),
        0,
        false,
        false,
        false,
        true,
        true,
    ));

    let folder_rows = rendered_folder_row_rects(&layout, &style, &model);
    assert!(folder_rows.len() >= 2, "expected at least two folder rows");
    let first_visual_rect = folder_row_visual_rect(folder_rows[0], style.sizing);
    let shared_boundary_y = folder_rows[1].min.y;
    let stroke = style.sizing.border_width.max(1.0);

    let mut state = NativeShellState::new();
    let frame = state.build_frame(&layout, &model);

    let top_separator_count = frame
        .primitives
        .iter()
        .filter(|primitive| {
            matches!(
                primitive,
                Primitive::Rect(FillRect { rect, color })
                    if *color == style.border
                        && rect.min.x == first_visual_rect.min.x
                        && rect.max.x == first_visual_rect.max.x
                        && rect.min.y == shared_boundary_y
                        && rect.max.y == shared_boundary_y + stroke
            )
        })
        .count();
    let lower_stacked_separator_count = frame
        .primitives
        .iter()
        .filter(|primitive| {
            matches!(
                primitive,
                Primitive::Rect(FillRect { rect, color })
                    if *color == style.border
                        && rect.min.x == first_visual_rect.min.x
                        && rect.max.x == first_visual_rect.max.x
                        && rect.min.y == shared_boundary_y - stroke
                        && rect.max.y == shared_boundary_y
            )
        })
        .count();

    assert_eq!(
        top_separator_count, 1,
        "expected one shared folder-row separator"
    );
    assert_eq!(
        lower_stacked_separator_count, 0,
        "folder rows should not stack a second border under the shared separator"
    );
}

#[test]
fn plain_folder_row_fill_insets_from_sidebar_seams() {
    let layout = ShellLayout::build(Vector2::new(1280.0, 720.0));
    let style = style_for_layout(&layout);
    let mut model = AppModel::default();
    model.sources.folder_rows.push(FolderRowModel::new(
        "folder_plain",
        String::new(),
        0,
        false,
        false,
        false,
        true,
        true,
    ));

    let row_rect = rendered_folder_row_rects(&layout, &style, &model)[0];
    let visual_rect = folder_row_visual_rect(row_rect, style.sizing);
    let mut state = NativeShellState::new();
    let frame = state.build_frame(&layout, &model);

    assert!(visual_rect.min.x > row_rect.min.x);
    assert!(visual_rect.max.x < row_rect.max.x);
    assert!(frame.primitives.iter().any(|primitive| {
        matches!(
            primitive,
            Primitive::Rect(FillRect { rect, color })
                if *rect == visual_rect && *color == style.surface_base
        )
    }));
    assert!(frame.primitives.iter().all(|primitive| {
        !matches!(
            primitive,
            Primitive::Rect(FillRect { rect, color })
                if *rect == row_rect && *color == style.surface_base
        )
    }));
}

#[test]
fn selected_folder_row_fill_insets_from_sidebar_seams() {
    let layout = ShellLayout::build(Vector2::new(1280.0, 720.0));
    let style = style_for_layout(&layout);
    let mut model = AppModel::default();
    model.sources.folder_rows.push(FolderRowModel::new(
        "folder_selected",
        String::new(),
        0,
        true,
        false,
        false,
        true,
        true,
    ));

    let row_rect = rendered_folder_row_rects(&layout, &style, &model)[0];
    let visual_rect = folder_row_visual_rect(row_rect, style.sizing);
    let expected_fill = translucent_overlay_color(
        style.bg_tertiary,
        style.grid_soft,
        style.state_selected_blend,
    );
    let mut state = NativeShellState::new();
    let frame = state.build_frame(&layout, &model);

    assert!(frame.primitives.iter().any(|primitive| {
        matches!(
            primitive,
            Primitive::Rect(FillRect { rect, color })
                if *rect == visual_rect && *color == expected_fill
        )
    }));
    assert!(frame.primitives.iter().all(|primitive| {
        !matches!(
            primitive,
            Primitive::Rect(FillRect { rect, color })
                if *rect == row_rect && *color == expected_fill
        )
    }));
}

#[test]
fn focused_folder_overlay_fill_insets_from_sidebar_seams() {
    let layout = ShellLayout::build(Vector2::new(1280.0, 720.0));
    let style = StyleTokens::for_viewport_width(1280.0);
    let mut model = AppModel::default();
    model.focus_context = crate::app::FocusContextModel::SourceFolders;
    model.sources.folder_rows.push(FolderRowModel::new(
        "folder_focused",
        String::new(),
        0,
        false,
        true,
        false,
        true,
        true,
    ));

    let row_rect = rendered_folder_row_rects(&layout, &style, &model)[0];
    let visual_rect = folder_row_visual_rect(row_rect, style.sizing);
    let expected_fill = translucent_overlay_color(
        style.bg_tertiary,
        style.grid_strong,
        style.state_focus_pulse_blend,
    );
    let mut state = NativeShellState::new();
    state.has_focus_emphasis = true;
    let mut overlay = NativeViewFrame::default();
    state.build_state_overlay_into(&layout, &style, &model, &mut overlay);

    assert!(overlay.primitives.iter().any(|primitive| {
        matches!(
            primitive,
            Primitive::Rect(FillRect { rect, color })
                if *rect == visual_rect && *color == expected_fill
        )
    }));
    assert!(overlay.primitives.iter().all(|primitive| {
        !matches!(
            primitive,
            Primitive::Rect(FillRect { rect, color })
                if *rect == row_rect && *color == expected_fill
        )
    }));
}

#[test]
fn inline_folder_draft_fill_insets_from_sidebar_seams() {
    let layout = ShellLayout::build(Vector2::new(1280.0, 720.0));
    let style = style_for_layout(&layout);
    let mut model = AppModel::default();
    model.sources.folder_rows.push(FolderRowModel::create_draft(
        0,
        String::from("new folder"),
        String::from("New folder name"),
        None,
        true,
    ));

    let row_rect = rendered_folder_row_rects(&layout, &style, &model)[0];
    let visual_rect = folder_row_visual_rect(row_rect, style.sizing);
    let mut state = NativeShellState::new();
    let frame = state.build_frame(&layout, &model);

    assert!(frame.primitives.iter().any(|primitive| {
        matches!(
            primitive,
            Primitive::Rect(FillRect { rect, color })
                if *rect == visual_rect && *color == style.surface_base
        )
    }));
    assert!(frame.primitives.iter().all(|primitive| {
        !matches!(
            primitive,
            Primitive::Rect(FillRect { rect, color })
                if *rect == row_rect && *color == style.surface_base
        )
    }));
}

#[test]
fn waveform_bpm_input_focus_overlay_uses_active_input_chrome() {
    let layout = ShellLayout::build(Vector2::new(1280.0, 720.0));
    let style = StyleTokens::for_viewport_width(1280.0);
    let motion = NativeMotionModel::from_app_model(&AppModel::default());
    let mut state = NativeShellState::new();
    state.set_waveform_bpm_editor_state(true, Some(String::from("128.0")), None);
    let bpm_rect = state
        .waveform_toolbar_button_rect(&layout, &AppModel::default(), "BPM Value")
        .expect("bpm value waveform toolbar widget should be present");

    let mut frame = NativeViewFrame::default();
    state.build_chrome_motion_overlay_into(&layout, &style, &motion, &mut frame);

    let overlay_color = frame
        .primitives
        .iter()
        .find_map(|primitive| match primitive {
            Primitive::Rect(FillRect { rect, color }) if *rect == bpm_rect => Some(*color),
            _ => None,
        })
        .expect("active bpm input should emit a focus overlay fill");

    assert_eq!(
        overlay_color,
        waveform_bpm_input_focus_fill(&style, interaction_wave(0.0))
    );
}

#[test]
fn waveform_bpm_editor_overlay_renders_selection_and_caret() {
    let layout = ShellLayout::build(Vector2::new(1280.0, 720.0));
    let style = StyleTokens::for_viewport_width(1280.0);
    let motion = NativeMotionModel::from_app_model(&AppModel::default());
    let mut state = NativeShellState::new();
    let model = AppModel::default();
    let bpm_rect = state
        .waveform_bpm_input_rect(&layout, &model)
        .expect("bpm field should render");
    let bpm_text = state
        .waveform_bpm_text_rect(&layout, &model)
        .expect("bpm text rect should render");
    state.set_waveform_bpm_editor_state(
        true,
        Some(String::from("128.0")),
        Some(TextFieldVisualState {
            text: String::from("128.0"),
            caret_offset: 22.0,
            selection_offsets: Some((0.0, 16.0)),
        }),
    );

    let mut frame = NativeViewFrame::default();
    state.build_chrome_motion_overlay_into(&layout, &style, &motion, &mut frame);
    let caret_width = style.sizing.border_width.max(1.0);

    assert!(frame.primitives.iter().any(|primitive| {
        matches!(
            primitive,
            Primitive::Rect(FillRect { rect, color })
                if *rect == bpm_rect && *color == browser_search_field_active_fill(&style)
        )
    }));
    assert!(frame.primitives.iter().any(|primitive| {
        matches!(
            primitive,
            Primitive::Rect(FillRect { rect, .. })
                if *rect
                    == Rect::from_min_max(
                        Point::new(bpm_text.min.x, bpm_text.min.y),
                        Point::new(bpm_text.min.x + 16.0, bpm_text.max.y),
                    )
        )
    }));
    assert!(frame.primitives.iter().any(|primitive| {
        matches!(
            primitive,
            Primitive::Rect(FillRect { rect, .. })
                if *rect
                    == Rect::from_min_max(
                        Point::new(bpm_text.min.x + 22.0, bpm_text.min.y),
                        Point::new(bpm_text.min.x + 22.0 + caret_width, bpm_text.max.y),
                    )
        )
    }));
    assert!(frame.text_runs.iter().any(|run| run.text == "128.0"));
}

#[test]
fn folder_create_draft_row_ignores_disclosure_hit_testing() {
    let layout = ShellLayout::build(Vector2::new(1280.0, 720.0));
    let mut model = AppModel::default();
    model.sources.folder_rows.push(FolderRowModel::new(
        "Root",
        String::new(),
        0,
        false,
        false,
        true,
        true,
        true,
    ));
    model.sources.folder_rows.push(FolderRowModel::create_draft(
        1,
        String::from("new folder"),
        String::from("New folder name"),
        None,
        true,
    ));
    let mut state = NativeShellState::new();
    let disclosure = state
        .folder_row_disclosure_rect(&layout, &model, 1)
        .expect("draft row should still have layout geometry");
    let point = Point::new(
        (disclosure.min.x + disclosure.max.x) * 0.5,
        (disclosure.min.y + disclosure.max.y) * 0.5,
    );

    assert_eq!(
        state.folder_row_disclosure_at_point(&layout, &model, point),
        None
    );
}

#[test]
fn folder_rename_draft_row_ignores_disclosure_hit_testing() {
    let layout = ShellLayout::build(Vector2::new(1280.0, 720.0));
    let mut model = AppModel::default();
    model.sources.folder_rows.push(FolderRowModel::new(
        "Root",
        String::new(),
        0,
        false,
        false,
        true,
        true,
        true,
    ));
    model.sources.folder_rows.push(FolderRowModel::rename_draft(
        1,
        String::from("Drums"),
        String::from("Folder name"),
        None,
        true,
    ));
    let mut state = NativeShellState::new();
    let disclosure = state
        .folder_row_disclosure_rect(&layout, &model, 1)
        .expect("rename draft row should still have layout geometry");
    let point = Point::new(
        (disclosure.min.x + disclosure.max.x) * 0.5,
        (disclosure.min.y + disclosure.max.y) * 0.5,
    );

    assert_eq!(
        state.folder_row_disclosure_at_point(&layout, &model, point),
        None
    );
}

#[test]
fn folder_create_editor_overlay_renders_selection_and_caret() {
    let layout = ShellLayout::build(Vector2::new(1280.0, 720.0));
    let style = StyleTokens::for_viewport_width(1280.0);
    let mut state = NativeShellState::new();
    let mut model = AppModel::default();
    model.sources.folder_rows.push(FolderRowModel::new(
        "Root",
        String::new(),
        0,
        false,
        false,
        true,
        true,
        true,
    ));
    model.sources.folder_rows.push(FolderRowModel::create_draft(
        1,
        String::from("new folder"),
        String::from("New folder name"),
        Some(String::from("Folder already exists")),
        true,
    ));
    let input_rect = state
        .folder_create_input_rect(&layout, &model)
        .expect("draft input should render");
    let text_rect = state
        .folder_create_text_rect(&layout, &model)
        .expect("draft text rect should render");
    state.set_folder_create_editor_state(Some(TextFieldVisualState {
        text: String::from("new folder"),
        caret_offset: 18.0,
        selection_offsets: Some((0.0, 12.0)),
    }));

    let mut overlay = NativeViewFrame::default();
    state.build_state_overlay_into(&layout, &style, &model, &mut overlay);
    let caret_width = style.sizing.border_width.max(1.0);

    assert!(overlay.primitives.iter().any(|primitive| {
        matches!(
            primitive,
            Primitive::Rect(FillRect { rect, color })
                if *rect == input_rect && *color == browser_search_field_active_fill(&style)
        )
    }));
    assert!(overlay.primitives.iter().any(|primitive| {
        matches!(
            primitive,
            Primitive::Rect(FillRect { rect, .. })
                if *rect
                    == Rect::from_min_max(
                        Point::new(text_rect.min.x, text_rect.min.y),
                        Point::new(text_rect.min.x + 12.0, text_rect.max.y),
                    )
        )
    }));
    assert!(overlay.primitives.iter().any(|primitive| {
        matches!(
            primitive,
            Primitive::Rect(FillRect { rect, .. })
                if *rect
                    == Rect::from_min_max(
                        Point::new(text_rect.min.x + 18.0, text_rect.min.y),
                        Point::new(text_rect.min.x + 18.0 + caret_width, text_rect.max.y),
                    )
        )
    }));
    assert!(overlay.text_runs.iter().any(|run| run.text == "new folder"));
}

#[test]
fn folder_rename_editor_overlay_renders_selection_and_caret() {
    let layout = ShellLayout::build(Vector2::new(1280.0, 720.0));
    let style = StyleTokens::for_viewport_width(1280.0);
    let mut state = NativeShellState::new();
    let mut model = AppModel::default();
    model.sources.folder_rows.push(FolderRowModel::new(
        "Root",
        String::new(),
        0,
        false,
        false,
        true,
        true,
        true,
    ));
    model.sources.folder_rows.push(FolderRowModel::rename_draft(
        1,
        String::from("drums"),
        String::from("Folder name"),
        Some(String::from("Folder already exists")),
        true,
    ));
    let input_rect = state
        .folder_create_input_rect(&layout, &model)
        .expect("rename draft input should render");
    let text_rect = state
        .folder_create_text_rect(&layout, &model)
        .expect("rename draft text rect should render");
    state.set_folder_create_editor_state(Some(TextFieldVisualState {
        text: String::from("drums"),
        caret_offset: 18.0,
        selection_offsets: Some((0.0, 12.0)),
    }));

    let mut overlay = NativeViewFrame::default();
    state.build_state_overlay_into(&layout, &style, &model, &mut overlay);
    let caret_width = style.sizing.border_width.max(1.0);

    assert!(overlay.primitives.iter().any(|primitive| {
        matches!(
            primitive,
            Primitive::Rect(FillRect { rect, color })
                if *rect == input_rect && *color == browser_search_field_active_fill(&style)
        )
    }));
    assert!(overlay.primitives.iter().any(|primitive| {
        matches!(
            primitive,
            Primitive::Rect(FillRect { rect, .. })
                if *rect
                    == Rect::from_min_max(
                        Point::new(text_rect.min.x, text_rect.min.y),
                        Point::new(text_rect.min.x + 12.0, text_rect.max.y),
                    )
        )
    }));
    assert!(overlay.primitives.iter().any(|primitive| {
        matches!(
            primitive,
            Primitive::Rect(FillRect { rect, .. })
                if *rect
                    == Rect::from_min_max(
                        Point::new(text_rect.min.x + 18.0, text_rect.min.y),
                        Point::new(text_rect.min.x + 18.0 + caret_width, text_rect.max.y),
                    )
        )
    }));
    assert!(overlay.text_runs.iter().any(|run| run.text == "drums"));
}

#[test]
fn source_divider_remains_above_folder_rows_in_cramped_viewports() {
    let layout = ShellLayout::build(Vector2::new(820.0, 400.0));
    let style = style_for_layout(&layout);
    let model = populated_sidebar_model();
    let sections = sidebar_sections(&layout, &style, &model);
    let divider = compute_source_section_divider_rect(
        sections.source_rows,
        sections.folder_header,
        style.sizing,
    )
    .expect("divider should exist");
    assert_rect_inside(layout.sidebar_rows, divider);
    assert!(divider.max.y <= sections.folder_rows.min.y);
    assert!(divider.min.y >= sections.source_rows.min.y);
}

#[test]
fn folder_recovery_badge_compacts_label_when_header_is_narrow() {
    let layout = ShellLayout::build(Vector2::new(820.0, 520.0));
    let style = style_for_layout(&layout);
    let header_rect = Rect::from_min_max(
        Point::new(0.0, 0.0),
        Point::new(58.0, style.sizing.folder_header_block_height),
    );
    let header_layout = compute_sidebar_folder_header_layout(
        header_rect,
        style.sizing,
        false,
        153,
        true,
        true,
        false,
        true,
    );
    let badge = header_layout.badge.expect("badge should still render");
    assert_rect_inside(header_rect, badge.rect);
    assert!(badge.label.chars().count() <= 3);
    assert!(!badge.active);
}

#[test]
fn folder_header_text_width_yields_no_overlap_with_recovery_badge() {
    let layout = ShellLayout::build(Vector2::new(820.0, 520.0));
    let style = style_for_layout(&layout);
    let header_rect = Rect::from_min_max(
        Point::new(24.0, 40.0),
        Point::new(120.0, 40.0 + style.sizing.folder_header_block_height),
    );
    let header_layout = compute_sidebar_folder_header_layout(
        header_rect,
        style.sizing,
        true,
        0,
        true,
        true,
        false,
        true,
    );
    let badge = header_layout
        .badge
        .expect("badge should render for active recovery");
    assert!(header_layout.title_row.max.x <= badge.rect.min.x);
    if let Some(metadata_row) = header_layout.metadata_row {
        assert!(metadata_row.max.x <= badge.rect.min.x);
    }
}

#[test]
fn source_action_buttons_stay_inside_sidebar_footer() {
    let model = populated_sidebar_model();
    for viewport in [
        Vector2::new(820.0, 520.0),
        Vector2::new(1280.0, 720.0),
        Vector2::new(2300.0, 1080.0),
    ] {
        let layout = ShellLayout::build(viewport);
        let style = style_for_layout(&layout);
        let buttons = source_action_buttons(&layout, &style, &model);
        assert!(!buttons.is_empty());
        for button in &buttons {
            assert_rect_inside(layout.sidebar_footer, button.rect);
        }
        for pair in buttons.windows(2) {
            assert!(pair[0].rect.max.x <= pair[1].rect.min.x);
        }
    }
}

#[test]
fn selected_source_row_uses_mint_label_text() {
    let layout = ShellLayout::build(Vector2::new(1280.0, 720.0));
    let mut state = NativeShellState::new();
    let mut model = AppModel::default();
    model.sources.rows.push(SourceRowModel::new(
        "selected source",
        String::new(),
        true,
        false,
    ));

    let frame = state.build_frame(&layout, &model);
    let selected_label = frame
        .text_runs
        .iter()
        .find(|run| run.text == "selected source")
        .expect("selected source label should render");

    assert_eq!(
        selected_label.color,
        StyleTokens::for_viewport_width(1280.0).accent_mint
    );
}

#[test]
fn folder_recovery_badge_renders_idle_count_label() {
    let layout = ShellLayout::build(Vector2::new(1280.0, 720.0));
    let style = StyleTokens::for_viewport_width(1280.0);
    let mut state = NativeShellState::new();
    let mut model = populated_sidebar_model();
    model.sources.folder_recovery.entry_count = 153;

    let frame = state.build_frame(&layout, &model);
    let badge_label = frame
        .text_runs
        .iter()
        .find(|run| run.text == "153 entries")
        .expect("idle recovery badge label should render");

    assert_eq!(badge_label.color, style.text_primary);
    assert!(frame.primitives.iter().any(|primitive| {
        matches!(
            primitive,
            Primitive::Rect(FillRect { color, .. })
                if *color == style.source_recovery_badge_idle
        )
    }));
}
