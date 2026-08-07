use super::*;

#[test]
fn launch_builders_expose_embedded_font_policy() {
    let no_state = radiant::window("Main")
        .embedded_font(EmbeddedFont::from_static(b"window-font"))
        .font_path("fonts/Window.ttf")
        .spec("main");
    let stateful = radiant::app(())
        .embedded_font(EmbeddedFont::from_static(b"state-font"))
        .font_path("fonts/State.ttf");

    assert_eq!(
        no_state.native_options().text.embedded_fonts[0].bytes(),
        b"window-font"
    );
    assert_eq!(
        no_state.native_options().text.font_paths[0],
        std::path::PathBuf::from("fonts/Window.ttf")
    );
    let _ = stateful;
}

#[test]
fn window_builder_exposes_typed_frame_rate_policy() {
    let spec = radiant::window("Main")
        .frame_rate(FrameRate::Hz60)
        .spec("main");

    assert_eq!(spec.target_frame_rate(), 60);
}

#[test]
fn launch_builders_expose_devtools_overlay_policy() {
    let default = radiant::window("Default").spec("default");
    let no_state = radiant::window("Main")
        .devtools_overlay(DevtoolsOverlayOptions::enabled())
        .spec("main");
    let stateful = radiant::app(()).devtools_overlay(DevtoolsOverlayOptions::enabled());

    assert!(!default.native_options().frame.devtools.is_enabled());
    assert!(no_state.native_options().frame.devtools.is_enabled());
    let _ = stateful;
}

#[test]
fn launch_builders_expose_prewarmed_popup_policy() {
    let no_state = radiant::window("Popup")
        .prewarmed_popup(-32_000.0, -32_000.0)
        .spec("popup");
    let stateful = radiant::app(())
        .title("Popup")
        .prewarmed_popup(-32_000.0, -32_000.0);

    assert_eq!(no_state.title(), "Popup");
    assert_eq!(
        no_state.popup_options().map(|popup| popup.position),
        Some(Some([-32_000.0, -32_000.0]))
    );
    assert_eq!(
        no_state
            .popup_options()
            .map(|popup| popup.hide_after_first_present),
        Some(true)
    );
    let _ = stateful;
}
