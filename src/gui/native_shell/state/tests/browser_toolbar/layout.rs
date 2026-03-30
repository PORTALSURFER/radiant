use super::*;

#[test]
fn browser_action_buttons_stay_inside_toolbar() {
    let mut model = AppModel::default();
    model.browser_actions.can_rename = true;
    model.browser_actions.can_tag = true;
    model.browser_actions.can_delete = true;
    for viewport in [
        Vector2::new(820.0, 520.0),
        Vector2::new(1280.0, 720.0),
        Vector2::new(2300.0, 1080.0),
    ] {
        let layout = ShellLayout::build(viewport);
        let style = style_for_layout(&layout);
        let toolbar = browser_toolbar_layout(&layout, &style);
        let buttons = browser_action_buttons(&layout, &style, &model, &toolbar);
        assert_eq!(buttons.len(), 2);
        assert_eq!(buttons[0].label, "Random");
        assert_eq!(buttons[0].icon, Some(WaveformToolbarIcon::Dice));
        assert!(buttons[0].enabled);
        assert!(!buttons[0].active);
        assert_eq!(buttons[1].label, "Cleanup");
        assert_eq!(buttons[1].icon, Some(WaveformToolbarIcon::Filter));
        assert!(buttons[1].enabled);
        assert!(!buttons[1].active);
        assert_rect_inside(layout.browser_toolbar, buttons[0].rect);
        assert_rect_inside(layout.browser_toolbar, buttons[1].rect);
    }
}

#[test]
fn browser_toolbar_controls_do_not_overlap_action_cluster() {
    let mut model = AppModel::default();
    model.browser_actions.can_rename = true;
    model.browser_actions.can_tag = true;
    model.browser_actions.can_delete = true;
    let layout = ShellLayout::build(Vector2::new(1280.0, 720.0));
    let style = style_for_layout(&layout);
    let controls = browser_toolbar_layout(&layout, &style);
    let buttons = browser_action_buttons(&layout, &style, &model, &controls);
    assert_eq!(buttons.len(), 2);
    assert!(
        controls
            .rating_filter_chips
            .iter()
            .all(|chip| chip.width() > 1.0)
    );
    assert_rect_inside(layout.browser_toolbar, controls.search_field);
    assert!(
        controls.search_field.max.x <= layout.browser_toolbar.max.x - style.sizing.text_inset_x
    );
    assert_eq!(buttons[0].rect, controls.action_slots[0]);
    assert_eq!(buttons[1].rect, controls.action_slots[1]);
    assert!(controls.rating_filter_chips[7].max.x <= buttons[0].rect.min.x);
    assert!(buttons[0].rect.max.x <= buttons[1].rect.min.x);
    assert!(buttons[1].rect.max.x <= controls.search_field.min.x);
    assert!(controls.search_field.width() < layout.browser_toolbar.width());
    assert!(controls.activity_chip.width() <= 0.0);
    assert!(controls.sort_chip.width() <= 0.0);
    assert!(
        controls
            .triage_chips
            .into_iter()
            .all(|chip| chip.width() <= 0.0)
    );
}

#[test]
fn browser_toolbar_places_playback_age_chips_between_rating_and_mark_controls() {
    let layout = ShellLayout::build(Vector2::new(1280.0, 720.0));
    let style = style_for_layout(&layout);
    let controls = browser_toolbar_layout(&layout, &style);

    assert!(
        controls
            .playback_age_filter_chips
            .iter()
            .all(|chip| chip.width() > 1.0)
    );
    assert!(
        controls.rating_filter_chips[7].max.x <= controls.playback_age_filter_chips[0].min.x
    );
    assert!(
        controls.playback_age_filter_chips[2].max.x <= controls.marked_filter_chip.min.x
    );
    assert!(controls.marked_filter_chip.max.x <= controls.action_slots[0].min.x);
}

#[test]
fn browser_toolbar_right_side_does_not_hit_search_field() {
    let layout = ShellLayout::build(Vector2::new(1280.0, 720.0));
    let mut state = NativeShellState::new();
    let model = AppModel::default();
    let style = style_for_layout(&layout);
    let controls = browser_toolbar_layout(&layout, &style);
    let point = Point::new(
        (controls.search_field.max.x + layout.browser_toolbar.max.x) * 0.5,
        (layout.browser_toolbar.min.y + layout.browser_toolbar.max.y) * 0.5,
    );
    assert!(point.x > controls.search_field.max.x);
    assert_eq!(
        state.browser_action_at_point(&layout, &model, point, false),
        None
    );
}

#[test]
fn top_bar_controls_fit_inside_control_row() {
    for viewport in [
        Vector2::new(820.0, 520.0),
        Vector2::new(1280.0, 720.0),
        Vector2::new(2300.0, 1080.0),
    ] {
        let layout = ShellLayout::build(viewport);
        let style = style_for_layout(&layout);
        let controls = top_bar_controls_layout(&layout, style.sizing);
        if !controls.active {
            continue;
        }
        assert_rect_inside(layout.top_bar_title_cluster, controls.volume_meter);
        assert_rect_inside(layout.top_bar_title_cluster, controls.volume_value);
        assert_rect_inside(layout.top_bar_title_cluster, controls.volume_label);
        assert!(controls.volume_meter.max.x <= controls.volume_value.min.x);
        assert!(controls.volume_value.max.x <= controls.volume_label.min.x);
    }
}
