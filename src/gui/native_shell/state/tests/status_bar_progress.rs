use super::*;

#[test]
fn indeterminate_scan_progress_renders_scan_label_and_file_counter() {
    let layout = ShellLayout::build(Vector2::new(1280.0, 720.0));
    let style = StyleTokens::for_viewport_width(1280.0);
    let mut state = NativeShellState::new();
    state.tick_with_style(0.35, &style);
    let model = AppModel {
        progress_overlay: crate::app::ProgressOverlayModel {
            visible: true,
            modal: false,
            title: String::from("Scanning source"),
            detail: Some(String::from("drums/kick.wav")),
            completed: 432,
            total: 0,
            cancelable: true,
            cancel_requested: false,
        },
        ..AppModel::default()
    };

    let frame = state.build_frame_with_style(&layout, &style, &model);

    assert!(
        frame
            .text_runs
            .iter()
            .any(|run| run.text.contains("Scanning source")),
        "status bar should show the scan label"
    );
    assert!(
        frame.text_runs.iter().any(|run| run.text == "432 files"),
        "status bar should show the scanned-file counter"
    );
    assert!(
        frame.primitives.iter().any(|primitive| matches!(
            primitive,
            Primitive::Rect(rect)
                if rect.rect.min.x >= layout.status_center_segment.min.x
                    && rect.rect.max.x <= layout.status_center_segment.max.x
                    && rect.color == blend_color(style.accent_mint, style.text_primary, 0.18)
        )),
        "status bar should render an indeterminate progress fill"
    );
}

#[test]
fn determinate_analysis_progress_keeps_fraction_counter() {
    let layout = ShellLayout::build(Vector2::new(1280.0, 720.0));
    let style = StyleTokens::for_viewport_width(1280.0);
    let mut state = NativeShellState::new();
    let model = AppModel {
        progress_overlay: crate::app::ProgressOverlayModel {
            visible: true,
            modal: false,
            title: String::from("Analyzing samples"),
            detail: Some(String::from("Jobs 2/5 • Samples 3/8")),
            completed: 2,
            total: 5,
            cancelable: true,
            cancel_requested: false,
        },
        ..AppModel::default()
    };

    let frame = state.build_frame_with_style(&layout, &style, &model);

    assert!(
        frame
            .text_runs
            .iter()
            .any(|run| run.text.contains("Analyzing samples")),
        "status bar should show the analysis label"
    );
    assert!(
        frame.text_runs.iter().any(|run| run.text == "2/5"),
        "status bar should keep determinate counters"
    );
}

#[test]
fn status_bar_text_cache_reuses_and_invalidates_on_progress_changes() {
    let layout = ShellLayout::build(Vector2::new(1280.0, 720.0));
    let style = StyleTokens::for_viewport_width(1280.0);
    let mut state = NativeShellState::new();
    let mut model = AppModel {
        progress_overlay: crate::app::ProgressOverlayModel {
            visible: true,
            modal: false,
            title: String::from("Analyzing samples"),
            detail: Some(String::from("Jobs 2/5 • Samples 3/8")),
            completed: 2,
            total: 5,
            cancelable: true,
            cancel_requested: false,
        },
        ..AppModel::default()
    };
    let mut segments = StaticFrameSegments::default();

    state.build_static_segment_with_style_into(
        &layout,
        &style,
        &model,
        None,
        StaticFrameSegment::StatusBar,
        &mut segments,
    );
    let first = state.status_bar_text_frame_counts();
    assert_eq!(first.lookup_count, 1);
    assert_eq!(first.cache_hit_count, 0);
    assert_eq!(first.cache_miss_count, 1);

    state.build_static_segment_with_style_into(
        &layout,
        &style,
        &model,
        None,
        StaticFrameSegment::StatusBar,
        &mut segments,
    );
    let second = state.status_bar_text_frame_counts();
    assert_eq!(second.lookup_count, 1);
    assert_eq!(second.cache_hit_count, 1);
    assert_eq!(second.cache_miss_count, 0);

    model.progress_overlay.completed = 3;
    state.build_static_segment_with_style_into(
        &layout,
        &style,
        &model,
        None,
        StaticFrameSegment::StatusBar,
        &mut segments,
    );
    let third = state.status_bar_text_frame_counts();
    assert_eq!(third.lookup_count, 1);
    assert_eq!(third.cache_hit_count, 0);
    assert_eq!(third.cache_miss_count, 1);
}
