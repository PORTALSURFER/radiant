use super::*;

#[test]
fn browser_action_model_signature_changes_with_action_flags_and_chip_content() {
    let mut baseline = AppModel::default();
    baseline.browser_actions.can_rename = true;
    baseline.browser_actions.can_tag = true;
    baseline.browser_actions.can_delete = false;
    baseline.columns[0].title = String::from("Trash");
    baseline.columns[0].item_count = 10;
    baseline.columns[1].title = String::from("Neutral");
    baseline.columns[1].item_count = 20;
    baseline.columns[2].title = String::from("Keep");
    baseline.columns[2].item_count = 30;

    let baseline_signature = browser_action_model_signature(&baseline);

    let mut changed_flag = baseline.clone();
    changed_flag.browser_actions.can_delete = true;
    assert_ne!(
        baseline_signature,
        browser_action_model_signature(&changed_flag)
    );

    let mut changed_chip = baseline.clone();
    changed_chip.columns[2].title = String::from("Favorites");
    assert_ne!(
        baseline_signature,
        browser_action_model_signature(&changed_chip)
    );
}

#[test]
fn waveform_toolbar_model_flags_change_with_channel_and_toggle_state() {
    let baseline = NativeMotionModel::from_app_model(&AppModel::default());
    let baseline_flags = waveform_toolbar_model_flags(&baseline);

    let mut changed_channel = baseline.clone();
    changed_channel.waveform_channel_view = match baseline.waveform_channel_view {
        crate::app::WaveformChannelViewModel::Mono => crate::app::WaveformChannelViewModel::Stereo,
        crate::app::WaveformChannelViewModel::Stereo => crate::app::WaveformChannelViewModel::Mono,
    };
    assert_ne!(
        baseline_flags,
        waveform_toolbar_model_flags(&changed_channel)
    );

    let mut changed_toggle = baseline.clone();
    changed_toggle.waveform_bpm_snap_enabled = !baseline.waveform_bpm_snap_enabled;
    assert_ne!(
        baseline_flags,
        waveform_toolbar_model_flags(&changed_toggle)
    );
}

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

#[test]
fn waveform_scrollbar_lane_stays_separate_from_waveform_plot() {
    let layout = ShellLayout::build(Vector2::new(1280.0, 720.0));
    assert_eq!(layout.waveform_plot.min.x, layout.waveform_card.min.x);
    assert_eq!(layout.waveform_plot.max.x, layout.waveform_card.max.x);
    assert_eq!(layout.waveform_plot.min.y, layout.waveform_header.max.y);
    assert_eq!(
        layout.waveform_scrollbar_lane.min.x,
        layout.waveform_card.min.x
    );
    assert_eq!(
        layout.waveform_scrollbar_lane.max.x,
        layout.waveform_card.max.x
    );
    assert_eq!(
        layout.waveform_scrollbar_lane.max.y,
        layout.waveform_card.max.y
    );
    assert_eq!(
        layout.waveform_plot.max.y,
        layout.waveform_scrollbar_lane.min.y
    );
    assert!(layout.waveform_scrollbar_lane.height() >= 12.0);
}

#[test]
fn touching_major_panels_render_single_seam_borders() {
    let layout = ShellLayout::build(Vector2::new(1280.0, 720.0));
    let style = style_for_layout(&layout);
    let model = AppModel::default();
    let mut state = NativeShellState::new();
    let frame = state.build_frame(&layout, &model);
    let stroke = style.sizing.border_width.max(1.0);
    let top_body_seam = Rect::from_min_max(
        Point::new(layout.root.rect.min.x, layout.top_bar.max.y - stroke),
        Point::new(layout.root.rect.max.x, layout.top_bar.max.y),
    );
    let sidebar_content_seam = Rect::from_min_max(
        Point::new(layout.sidebar.max.x - stroke, layout.top_bar.max.y),
        Point::new(layout.sidebar.max.x, layout.status_bar.min.y),
    );
    let top_body_matches = frame
        .primitives
        .iter()
        .filter(|primitive| {
            matches!(
                primitive,
                Primitive::Rect(FillRect { rect, color })
                    if *rect == top_body_seam && *color == style.border
            )
        })
        .count();
    let sidebar_content_matches = frame
        .primitives
        .iter()
        .filter(|primitive| {
            matches!(
                primitive,
                Primitive::Rect(FillRect { rect, color })
                    if *rect == sidebar_content_seam && *color == style.border
            )
        })
        .count();
    let status_bar_bottom_seam = Rect::from_min_max(
        Point::new(layout.status_bar.min.x, layout.status_bar.max.y - stroke),
        Point::new(layout.status_bar.max.x, layout.status_bar.max.y),
    );
    let status_bar_bottom_matches = frame
        .primitives
        .iter()
        .filter(|primitive| {
            matches!(
                primitive,
                Primitive::Rect(FillRect { rect, color })
                    if *rect == status_bar_bottom_seam && *color == style.border
            )
        })
        .count();
    assert_eq!(top_body_matches, 1);
    assert_eq!(sidebar_content_matches, 1);
    assert_eq!(status_bar_bottom_matches, 0);
}

#[test]
fn chrome_motion_status_overlay_preserves_status_bar_border_lines() {
    let layout = ShellLayout::build(Vector2::new(1280.0, 720.0));
    let style = style_for_layout(&layout);
    let mut state = NativeShellState::new();
    let mut model = AppModel::default();
    model.status.right = String::from("2/3");
    let motion = NativeMotionModel::from_app_model(&model);
    let overlay_segment = layout.status_right_segment;
    let overlay_rect = status_motion_overlay_rect(overlay_segment, style.sizing.border_width);

    let mut frame = NativeViewFrame::default();
    state.build_chrome_motion_overlay_into(&layout, &style, &motion, &mut frame);

    assert!(
        frame.primitives.iter().any(|primitive| {
            matches!(
                primitive,
                Primitive::Rect(FillRect { rect, color })
                    if *rect == overlay_rect && *color == style.surface_raised
            )
        }),
        "status motion overlay should repaint only the inset text background"
    );
    assert!(
        frame.primitives.iter().all(|primitive| {
            !matches!(
                primitive,
                Primitive::Rect(FillRect { rect, color })
                    if *rect == layout.status_right_segment && *color == style.surface_raised
            )
        }),
        "status motion overlay should not cover the full status segment and erase border lines"
    );
}

#[test]
fn waveform_toolbar_icon_buttons_use_uniform_hit_cell_widths() {
    let layout = ShellLayout::build(Vector2::new(1280.0, 720.0));
    let state = NativeShellState::new();
    let mut model = AppModel::default();
    model.transport_running = false;
    let labels = [
        "Channel", "Norm", "BPM Snap", "Tr Snap", "Show Tr", "Slice", "Loop", "Play", "Rec",
    ];
    let widths: Vec<u32> = labels
        .iter()
        .map(|label| {
            let rect = state
                .waveform_toolbar_button_rect(&layout, &model, label)
                .unwrap_or_else(|| panic!("missing waveform toolbar button rect for {label}"));
            (rect.width() * 100.0).round() as u32
        })
        .collect();
    let min_width = widths.iter().copied().min().unwrap_or(0);
    let max_width = widths.iter().copied().max().unwrap_or(0);
    assert!(
        max_width.saturating_sub(min_width) <= 100,
        "toolbar widths diverged too far: {widths:?}"
    );
}

#[test]
fn waveform_toolbar_renders_without_per_button_rect_chrome() {
    let layout = ShellLayout::build(Vector2::new(1280.0, 720.0));
    let mut model = AppModel::default();
    model.transport_running = false;
    let mut state = NativeShellState::new();
    let button_rects = ["Channel", "Play"]
        .into_iter()
        .map(|label| {
            state
                .waveform_toolbar_button_rect(&layout, &model, label)
                .unwrap_or_else(|| panic!("missing waveform toolbar button rect for {label}"))
        })
        .collect::<Vec<_>>();
    let frame = state.build_frame(&layout, &model);
    for button_rect in button_rects {
        assert!(!frame.primitives.iter().any(|primitive| {
            matches!(primitive, Primitive::Rect(FillRect { rect, .. }) if *rect == button_rect)
        }));
    }
}

#[test]
fn waveform_toolbar_click_sets_flash_in_chrome_motion_fingerprint() {
    let layout = ShellLayout::build(Vector2::new(1280.0, 720.0));
    let mut state = NativeShellState::new();
    let mut model = AppModel::default();
    model.transport_running = false;
    let play = state
        .waveform_toolbar_button_rect(&layout, &model, "Play")
        .expect("play waveform toolbar button should be present");
    let point = Point::new(
        (play.min.x + play.max.x) * 0.5,
        (play.min.y + play.max.y) * 0.5,
    );
    assert_eq!(
        state.waveform_toolbar_action_at_point(&layout, &model, point),
        Some(crate::app::UiAction::ToggleTransport)
    );
    let fingerprint = state.chrome_motion_overlay_fingerprint();
    assert_eq!(
        fingerprint.flashed_waveform_toolbar_hint,
        Some(WaveformToolbarHoverHint::Play)
    );
    assert!(fingerprint.waveform_toolbar_flash_ticks > 0);
}

#[test]
fn waveform_toolbar_hover_uses_theme_highlight_color() {
    let style = StyleTokens::for_viewport_width(1280.0);
    let expected = blend_color(
        blend_color(style.text_muted, style.bg_tertiary, 0.26),
        style.highlight_cyan,
        0.82,
    );

    assert_eq!(
        waveform_toolbar_visual_color(&style, style.highlight_cyan, true, false, true, false, 0.0,),
        expected
    );
}

#[test]
fn waveform_toolbar_play_button_uses_transport_accent_when_idle() {
    let layout = ShellLayout::build(Vector2::new(1280.0, 720.0));
    let style = style_for_layout(&layout);
    let mut model = AppModel::default();
    model.transport_running = false;

    let buttons = waveform_toolbar_buttons(
        &layout,
        &style,
        &NativeMotionModel::from_app_model(&model),
        false,
        None,
    );
    let play = buttons
        .iter()
        .find(|button| button.label == "Play")
        .expect("play toolbar button should be present");

    assert_eq!(play.text_color, style.accent_warning);
}

#[test]
fn waveform_toolbar_stop_button_uses_stop_icon_and_escape_when_running() {
    let layout = ShellLayout::build(Vector2::new(1280.0, 720.0));
    let style = style_for_layout(&layout);
    let mut model = AppModel::default();
    model.transport_running = true;

    let buttons = waveform_toolbar_buttons(
        &layout,
        &style,
        &NativeMotionModel::from_app_model(&model),
        false,
        None,
    );
    let stop = buttons
        .iter()
        .find(|button| button.label == "Stop")
        .expect("stop toolbar button should be present while transport runs");

    assert_eq!(stop.icon, Some(WaveformToolbarIcon::Stop));
    assert_eq!(stop.action, Some(UiAction::HandleEscape));
    assert_eq!(stop.text_color, style.highlight_orange_soft);
}

#[test]
fn waveform_toolbar_bpm_snap_button_uses_highlight_when_enabled() {
    let layout = ShellLayout::build(Vector2::new(1280.0, 720.0));
    let style = style_for_layout(&layout);
    let mut model = AppModel::default();
    let buttons_off = waveform_toolbar_buttons(
        &layout,
        &style,
        &NativeMotionModel::from_app_model(&model),
        false,
        None,
    );
    let bpm_snap_off = buttons_off
        .iter()
        .find(|button| button.label == "BPM Snap")
        .expect("bpm snap toolbar button should be present");
    assert_eq!(bpm_snap_off.text_color, style.text_muted);

    model.waveform_chrome.bpm_snap_enabled = true;
    let buttons_on = waveform_toolbar_buttons(
        &layout,
        &style,
        &NativeMotionModel::from_app_model(&model),
        false,
        None,
    );
    let bpm_snap_on = buttons_on
        .iter()
        .find(|button| button.label == "BPM Snap")
        .expect("bpm snap toolbar button should be present");
    assert_eq!(bpm_snap_on.text_color, style.accent_warning);
}

#[test]
fn waveform_toolbar_normalized_audition_button_uses_highlight_when_enabled() {
    let layout = ShellLayout::build(Vector2::new(1280.0, 720.0));
    let style = style_for_layout(&layout);
    let mut model = AppModel::default();
    let buttons_off = waveform_toolbar_buttons(
        &layout,
        &style,
        &NativeMotionModel::from_app_model(&model),
        false,
        None,
    );
    let normalized_off = buttons_off
        .iter()
        .find(|button| button.label == "Norm")
        .expect("normalized audition toolbar button should be present");
    assert_eq!(normalized_off.text_color, style.text_muted);

    model.waveform_chrome.normalized_audition_enabled = true;
    let buttons_on = waveform_toolbar_buttons(
        &layout,
        &style,
        &NativeMotionModel::from_app_model(&model),
        false,
        None,
    );
    let normalized_on = buttons_on
        .iter()
        .find(|button| button.label == "Norm")
        .expect("normalized audition toolbar button should be present");
    assert_eq!(normalized_on.text_color, style.accent_warning);
}

#[test]
fn waveform_toolbar_toggle_buttons_share_warning_accent_when_enabled() {
    let layout = ShellLayout::build(Vector2::new(1280.0, 720.0));
    let style = style_for_layout(&layout);
    let mut model = AppModel::default();
    model.waveform_chrome.transient_snap_enabled = true;
    model.waveform_chrome.transient_markers_enabled = true;
    model.waveform_chrome.slice_mode_enabled = true;
    model.waveform.loop_enabled = true;

    let buttons = waveform_toolbar_buttons(
        &layout,
        &style,
        &NativeMotionModel::from_app_model(&model),
        false,
        None,
    );

    for label in ["Tr Snap", "Show Tr", "Slice", "Loop"] {
        let button = buttons
            .iter()
            .find(|button| button.label == label)
            .unwrap_or_else(|| panic!("{label} toolbar button should be present"));
        assert_eq!(button.text_color, style.accent_warning);
    }
}
