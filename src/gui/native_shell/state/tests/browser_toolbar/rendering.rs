use super::*;

fn rect_contains_rect(outer: Rect, inner: Rect) -> bool {
    inner.min.x >= outer.min.x
        && inner.min.y >= outer.min.y
        && inner.max.x <= outer.max.x
        && inner.max.y <= outer.max.y
}

#[test]
fn browser_filter_icons_replace_legacy_age_and_mark_labels() {
    let layout = ShellLayout::build(Vector2::new(1280.0, 720.0));
    let model = AppModel::default();
    let mut state = NativeShellState::new();
    let frame = state.build_frame(&layout, &model);

    assert!(
        !frame
            .text_runs
            .iter()
            .any(|run| matches!(run.text.as_str(), "NVR" | "1M" | "1W" | "MARK"))
    );

    for chip in [
        crate::app::PlaybackAgeFilterChip::NeverPlayed,
        crate::app::PlaybackAgeFilterChip::OlderThanMonth,
        crate::app::PlaybackAgeFilterChip::OlderThanWeek,
    ] {
        let chip_rect = state
            .browser_playback_age_filter_chip_rect(&layout, &model, chip)
            .expect("playback-age chip should render");
        assert!(frame.primitives.iter().any(|primitive| {
            matches!(
                primitive,
                Primitive::Image(image) if rect_contains_rect(chip_rect, image.rect)
            )
        }));
    }

    let marked_chip = state
        .browser_marked_filter_chip_rect(&layout, &model)
        .expect("marked filter chip should render");
    assert!(frame.primitives.iter().any(|primitive| {
        matches!(
            primitive,
            Primitive::Image(image) if rect_contains_rect(marked_chip, image.rect)
        )
    }));
}
