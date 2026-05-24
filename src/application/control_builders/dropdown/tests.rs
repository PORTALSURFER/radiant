use super::*;

#[derive(Clone, Debug, PartialEq)]
enum Message {
    Toggle,
    Select(&'static str),
}

#[test]
fn dropdown_height_tracks_expanded_options() {
    assert_eq!(dropdown_height(false, 3), 24.0);
    assert_eq!(dropdown_height(true, 3), 24.0);
    assert_eq!(dropdown_menu_height(3), 80.0);
}

#[test]
fn dropdown_builder_accepts_toggle_and_options() {
    let _view = dropdown("WASAPI", true)
        .toggle_message(Message::Toggle)
        .option("System default", false, Message::Select("default"))
        .option("WASAPI", true, Message::Select("wasapi"))
        .build();
}
