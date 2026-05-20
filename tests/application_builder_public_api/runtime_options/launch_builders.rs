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
