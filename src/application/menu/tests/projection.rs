use super::MenuMessage;
use crate::{
    application::{
        IntoView, MenuCommand, MessageMenuWidthPolicy, context_menu, message_menu_height,
    },
    gui::{
        text_layout::{TextWidthEstimate, estimated_text_width},
        types::{Point, Rect},
    },
    layout::Vector2,
    runtime::{PaintTextAlign, UiSurface},
};

use super::super::projection::{MENU_HOTKEY_HINT_HORIZONTAL_PADDING, MENU_LABEL_HOTKEY_GAP};

#[test]
fn message_menu_paints_command_labels_and_hotkey_hints_as_columns() {
    let frame = projected_menu_frame(
        280.0,
        [
            MenuCommand::new("Open", MenuMessage::Open).hotkey_hint("Cmd-O"),
            MenuCommand::new("Duplicate Without Shortcut", MenuMessage::Delete),
        ],
    );
    let open = frame.paint_plan.first_text_run("Open").expect("label");
    let duplicate = frame
        .paint_plan
        .first_text_run("Duplicate Without Shortcut")
        .expect("plain label");
    let shortcut = frame.paint_plan.first_text_run("Cmd-O").expect("shortcut");

    assert_eq!(open.align, PaintTextAlign::Left);
    assert_eq!(duplicate.align, PaintTextAlign::Left);
    assert_eq!(shortcut.align, PaintTextAlign::Right);
    let hint_metrics = TextWidthEstimate::new(
        MessageMenuWidthPolicy::compact().metrics.character_advance,
        MENU_HOTKEY_HINT_HORIZONTAL_PADDING,
    );
    assert!(shortcut.rect.width() >= estimated_text_width("Cmd-O", hint_metrics));
    assert!((open.rect.min.x - duplicate.rect.min.x).abs() < 0.01);
    assert!(open.rect.max.x + MENU_LABEL_HOTKEY_GAP <= shortcut.rect.min.x);
    assert!(duplicate.rect.max.x > shortcut.rect.min.x);
}

#[test]
fn message_menu_hotkey_hint_width_contributes_to_auto_width() {
    let policy = MessageMenuWidthPolicy::new(TextWidthEstimate::new(8.0, 24.0), 100.0, 320.0);
    let commands_without_hint = [MenuCommand::new("Open", MenuMessage::Open)];
    let commands_with_hint =
        [MenuCommand::new("Open", MenuMessage::Open).hotkey_hint("Command-Shift-O")];

    assert!(
        policy.width_for_title_and_commands("Actions", &commands_with_hint)
            > policy.width_for_title_and_commands("Actions", &commands_without_hint)
    );
}

#[test]
fn compact_message_menu_fits_folder_delete_label_and_shortcut_hint() {
    let policy = MessageMenuWidthPolicy::compact();
    let commands = [
        MenuCommand::new("Delete Folder", MenuMessage::Delete).hotkey_hint("Delete / Backspace")
    ];
    let width = policy.width_for_title_and_commands("Documents", &commands);
    let frame = projected_menu_frame(width, commands);
    let label = frame
        .paint_plan
        .first_text_run("Delete Folder")
        .expect("label");
    let shortcut = frame
        .paint_plan
        .first_text_run("Delete / Backspace")
        .expect("shortcut");
    let label_metrics = TextWidthEstimate::new(policy.metrics.character_advance, 0.0);
    let shortcut_metrics = TextWidthEstimate::new(
        policy.metrics.character_advance,
        MENU_HOTKEY_HINT_HORIZONTAL_PADDING,
    );

    assert!(width > 320.0);
    assert!(label.rect.width() >= estimated_text_width("Delete Folder", label_metrics));
    assert!(shortcut.rect.width() >= estimated_text_width("Delete / Backspace", shortcut_metrics));
    assert!(label.rect.max.x + MENU_LABEL_HOTKEY_GAP <= shortcut.rect.min.x);
}

#[test]
fn compact_message_menu_width_policy_clamps_to_default_range() {
    let policy = MessageMenuWidthPolicy::compact();
    let short_commands = [MenuCommand::new("Go", MenuMessage::Open)];
    let long_commands = [MenuCommand::new(
        "A very long command label that should clamp",
        MenuMessage::Open,
    )];

    assert_eq!(
        policy.width_for_title_and_commands("A", &short_commands),
        policy.min_width
    );
    assert_eq!(
        policy.width_for_title_and_commands("Actions", &long_commands),
        policy.max_width
    );
}

fn projected_menu_frame<const N: usize>(
    width: f32,
    commands: [MenuCommand<MenuMessage>; N],
) -> crate::runtime::SurfaceFrame {
    UiSurface::new(
        context_menu("Actions", commands)
            .anchor(Point::new(80.0, 90.0))
            .size(Vector2::new(width, message_menu_height(N)))
            .view()
            .into_node(),
    )
    .frame(
        Rect::from_min_size(Point::new(0.0, 0.0), Vector2::new(640.0, 360.0)),
        &Default::default(),
    )
}
