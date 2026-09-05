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

#[test]
fn automatic_menus_scale_geometry_and_share_visible_semantic_command_identity() {
    use crate::{
        application::{ApplicationEnvironment, LocaleId, TextScale, WritingDirection},
        gui::automation::AutomationRole,
    };
    for scale in [1.0, 1.5, 2.0] {
        for direction in [WritingDirection::Ltr, WritingDirection::Rtl] {
            let environment = ApplicationEnvironment::new(LocaleId::english())
                .with_text_scale(TextScale::new(scale).unwrap())
                .with_writing_direction(direction);
            let surface = UiSurface::new(
                context_menu(
                    "Actions",
                    [MenuCommand::new("Open", MenuMessage::Open).hotkey_hint("Cmd-O")],
                )
                .anchor(Point::new(80.0, 90.0))
                .view()
                .into_node(),
            )
            .with_application_environment(environment);
            let frame = surface.frame(
                Rect::from_xy_size(0.0, 0.0, 1280.0, 720.0),
                &Default::default(),
            );
            let label = frame.paint_plan.first_text_run("Open").unwrap();
            let hint = frame.paint_plan.first_text_run("Cmd-O").unwrap();
            assert_eq!(label.font_size, 13.0 * scale);
            assert_eq!(hint.font_size, label.font_size);
            assert_eq!(label.widget_id, hint.widget_id);
            assert_eq!(frame.layout.rects[&label.widget_id].height(), 28.0 * scale);
            let semantics = surface
                .find_widget(label.widget_id)
                .unwrap()
                .widget()
                .automation_semantics();
            assert_eq!(semantics.role, AutomationRole::Button);
            assert_eq!(semantics.label.as_deref(), Some("Open"));
            assert_eq!(semantics.description.as_deref(), Some("Cmd-O"));
            assert!(semantics.focusable);
            match direction {
                WritingDirection::Ltr => {
                    assert_eq!(label.align, PaintTextAlign::Left);
                    assert_eq!(hint.align, PaintTextAlign::Right);
                    assert!(
                        label.rect.max.x + MENU_LABEL_HOTKEY_GAP * scale <= hint.rect.min.x + 0.01
                    );
                }
                WritingDirection::Rtl => {
                    assert_eq!(label.align, PaintTextAlign::Right);
                    assert_eq!(hint.align, PaintTextAlign::Left);
                    assert!(
                        hint.rect.max.x + MENU_LABEL_HOTKEY_GAP * scale <= label.rect.min.x + 0.01
                    );
                }
            }
            let title = frame.paint_plan.first_text_run("Actions").unwrap();
            assert_eq!(frame.layout.rects[&title.widget_id].height(), 22.0 * scale);
            assert_eq!(label.rect.height(), 28.0 * scale);
        }
    }
}
