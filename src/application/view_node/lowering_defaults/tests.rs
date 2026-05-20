use super::*;

#[test]
fn styled_container_defaults_to_panel_padding() {
    let defaults = ViewNodeContainerDefaults::new(None, None, None, Some(WidgetStyle::default()));
    let policy = defaults.base_policy();

    assert_eq!(
        policy.padding,
        Insets::all(DEFAULT_STYLED_CONTAINER_PADDING)
    );
    assert_eq!(policy.align_main, MainAlign::Start);
    assert_eq!(policy.align_cross, CrossAlign::Stretch);
}

#[test]
fn explicit_container_defaults_override_style_padding_and_alignment() {
    let defaults = ViewNodeContainerDefaults::new(
        Some(Insets::all(3.0)),
        Some(MainAlign::Center),
        Some(CrossAlign::End),
        Some(WidgetStyle::default()),
    );
    let policy = defaults.base_policy();

    assert_eq!(policy.padding, Insets::all(3.0));
    assert_eq!(policy.align_main, MainAlign::Center);
    assert_eq!(policy.align_cross, CrossAlign::End);
}
