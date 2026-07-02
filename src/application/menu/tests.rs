use super::*;
use crate::{
    application::IntoView,
    gui::types::{Point, Rect},
    layout::Vector2,
    runtime::{
        DeclarativeOwnedRuntimeBridge, Event, PaintPrimitive, PaintTextAlign, SurfaceRuntime,
        UiSurface,
    },
    widgets::{PointerButton, PointerModifiers, WidgetTone},
};

#[derive(Clone, Debug, PartialEq)]
enum MenuMessage {
    Open,
    Delete,
    Close,
}

#[test]
fn message_menu_emits_host_messages() {
    let bridge = DeclarativeOwnedRuntimeBridge::new(
        Vec::<MenuMessage>::new(),
        |_| {
            UiSurface::new(
                message_menu(
                    "Actions",
                    [
                        MenuCommand::new("Open", MenuMessage::Open),
                        MenuCommand::new("Delete", MenuMessage::Delete).danger(),
                    ],
                )
                .into_node(),
            )
        },
        |messages: &mut Vec<MenuMessage>, message| messages.push(message),
    );
    let mut runtime = SurfaceRuntime::new(bridge, Vector2::new(240.0, 120.0));
    let delete_command = Point::new(20.0, 70.0);

    runtime.dispatch_event(Event::PointerPress {
        position: delete_command,
        button: PointerButton::Primary,
        modifiers: PointerModifiers::default(),
    });
    runtime.dispatch_event(Event::PointerRelease {
        position: delete_command,
        button: PointerButton::Primary,
        modifiers: PointerModifiers::default(),
    });

    assert_eq!(runtime.bridge().state(), &[MenuMessage::Delete]);
}

#[test]
fn message_menu_applies_command_styles() {
    let frame = UiSurface::new(
        message_menu(
            "Actions",
            [MenuCommand::new("Delete", MenuMessage::Delete).danger()],
        )
        .into_node(),
    )
    .frame(
        Rect::from_min_size(Point::new(0.0, 0.0), Vector2::new(240.0, 120.0)),
        &Default::default(),
    );

    assert!(
        frame.paint_plan.primitives.iter().any(|primitive| {
            matches!(
                primitive,
                PaintPrimitive::FillRect(fill) if fill.color.r > fill.color.g
            )
        }),
        "danger commands should apply the danger-toned button style"
    );
}

#[test]
fn dismissible_context_menu_backing_emits_dismiss_message() {
    let bridge = DeclarativeOwnedRuntimeBridge::new(
        Vec::<MenuMessage>::new(),
        |_| {
            UiSurface::new(
                dismissible_context_menu(
                    Point::new(80.0, 90.0),
                    Vector2::new(200.0, 96.0),
                    "Actions",
                    [MenuCommand::new("Open", MenuMessage::Open)],
                    MenuMessage::Close,
                )
                .into_node(),
            )
        },
        |messages: &mut Vec<MenuMessage>, message| messages.push(message),
    );
    let mut runtime = SurfaceRuntime::new(bridge, Vector2::new(640.0, 360.0));

    runtime.dispatch_event(Event::PointerPress {
        position: Point::new(12.0, 12.0),
        button: PointerButton::Primary,
        modifiers: PointerModifiers::default(),
    });
    runtime.dispatch_event(Event::PointerRelease {
        position: Point::new(12.0, 12.0),
        button: PointerButton::Primary,
        modifiers: PointerModifiers::default(),
    });

    assert_eq!(runtime.bridge().state(), &[MenuMessage::Close]);
}

#[test]
fn dismissible_context_menu_with_width_uses_standard_menu_height() {
    let frame = UiSurface::new(
        dismissible_context_menu_with_width(
            Point::new(80.0, 90.0),
            200.0,
            "Actions",
            [
                MenuCommand::new("Open", MenuMessage::Open),
                MenuCommand::new("Delete", MenuMessage::Delete).danger(),
            ],
            MenuMessage::Close,
        )
        .into_node(),
    )
    .frame(
        Rect::from_min_size(Point::new(0.0, 0.0), Vector2::new(640.0, 360.0)),
        &Default::default(),
    );

    let menu_rects = frame
        .paint_plan
        .primitives
        .iter()
        .filter_map(|primitive| match primitive {
            PaintPrimitive::FillRect(fill) => Some(fill.rect),
            _ => None,
        })
        .collect::<Vec<_>>();

    assert!(
        menu_rects.iter().any(|rect| {
            rect.min.x >= 80.0 && (rect.height() - message_menu_height(2)).abs() < 0.01
        }),
        "standard-width menu should paint at standard compact height: {menu_rects:?}"
    );
}

#[test]
fn message_context_menu_overlay_opens_down_when_space_below() {
    let size = Vector2::new(200.0, message_menu_height(2));
    let frame = UiSurface::new(
        message_context_menu_overlay(
            Point::new(80.0, 90.0),
            size,
            "Actions",
            [
                MenuCommand::new("Open", MenuMessage::Open),
                MenuCommand::new("Delete", MenuMessage::Delete).danger(),
            ],
        )
        .into_node(),
    )
    .frame(
        Rect::from_min_size(Point::new(0.0, 0.0), Vector2::new(640.0, 360.0)),
        &Default::default(),
    );

    let menu_rect = painted_menu_rect(&frame.paint_plan.primitives, size);

    assert_eq!(menu_rect.min, Point::new(80.0, 90.0));
}

#[test]
fn message_context_menu_overlay_flips_up_when_bottom_would_clip() {
    let size = Vector2::new(200.0, message_menu_height(2));
    let frame = UiSurface::new(
        message_context_menu_overlay(
            Point::new(80.0, 330.0),
            size,
            "Actions",
            [
                MenuCommand::new("Open", MenuMessage::Open),
                MenuCommand::new("Delete", MenuMessage::Delete).danger(),
            ],
        )
        .into_node(),
    )
    .frame(
        Rect::from_min_size(Point::new(0.0, 0.0), Vector2::new(640.0, 360.0)),
        &Default::default(),
    );

    let menu_rect = painted_menu_rect(&frame.paint_plan.primitives, size);

    assert_eq!(menu_rect.min, Point::new(80.0, 226.0));
    assert_eq!(menu_rect.max.y, 330.0);
}

#[test]
fn flipped_message_context_menu_routes_clicks_to_painted_items() {
    let size = Vector2::new(200.0, message_menu_height(2));
    let bridge = DeclarativeOwnedRuntimeBridge::new(
        Vec::<MenuMessage>::new(),
        move |_| {
            UiSurface::new(
                message_context_menu_overlay(
                    Point::new(80.0, 330.0),
                    size,
                    "Actions",
                    [
                        MenuCommand::new("Open", MenuMessage::Open),
                        MenuCommand::new("Delete", MenuMessage::Delete).danger(),
                    ],
                )
                .into_node(),
            )
        },
        |messages: &mut Vec<MenuMessage>, message| messages.push(message),
    );
    let mut runtime = SurfaceRuntime::new(bridge, Vector2::new(640.0, 360.0));
    let delete_command = Point::new(100.0, 306.0);

    runtime.dispatch_event(Event::PointerPress {
        position: delete_command,
        button: PointerButton::Primary,
        modifiers: PointerModifiers::default(),
    });
    runtime.dispatch_event(Event::PointerRelease {
        position: delete_command,
        button: PointerButton::Primary,
        modifiers: PointerModifiers::default(),
    });

    assert_eq!(runtime.bridge().state(), &[MenuMessage::Delete]);
}

#[test]
fn message_menu_paints_command_labels_and_hotkey_hints_as_columns() {
    let size = Vector2::new(280.0, message_menu_height(2));
    let frame = UiSurface::new(
        message_context_menu_overlay(
            Point::new(80.0, 90.0),
            size,
            "Actions",
            [
                MenuCommand::new("Open", MenuMessage::Open).hotkey_hint("Cmd-O"),
                MenuCommand::new("Duplicate Without Shortcut", MenuMessage::Delete),
            ],
        )
        .into_node(),
    )
    .frame(
        Rect::from_min_size(Point::new(0.0, 0.0), Vector2::new(640.0, 360.0)),
        &Default::default(),
    );

    let open = frame
        .paint_plan
        .first_text_run("Open")
        .expect("shortcut-backed label should paint");
    let duplicate = frame
        .paint_plan
        .first_text_run("Duplicate Without Shortcut")
        .expect("plain label should paint");
    let shortcut = frame
        .paint_plan
        .first_text_run("Cmd-O")
        .expect("shortcut hint should paint");

    assert_eq!(open.align, PaintTextAlign::Left);
    assert_eq!(duplicate.align, PaintTextAlign::Left);
    assert_eq!(shortcut.align, PaintTextAlign::Right);
    let hint_metrics = crate::gui::text_layout::TextWidthEstimate::new(
        MessageMenuWidthPolicy::compact().metrics.character_advance,
        MENU_HOTKEY_HINT_HORIZONTAL_PADDING,
    );
    let minimum_hint_width = crate::gui::text_layout::estimated_text_width("Cmd-O", hint_metrics);
    assert!(
        shortcut.rect.width() >= minimum_hint_width,
        "shortcut hint column should reserve enough width for the painted text: shortcut={:?}, minimum={minimum_hint_width}",
        shortcut.rect
    );
    assert!(
        (open.rect.min.x - duplicate.rect.min.x).abs() < 0.01,
        "labels should share a left column: open={:?}, duplicate={:?}",
        open.rect,
        duplicate.rect
    );
    assert!(
        duplicate.rect.max.x + MENU_LABEL_HOTKEY_GAP <= shortcut.rect.min.x,
        "label and shortcut columns should not overlap: duplicate={:?}, shortcut={:?}",
        duplicate.rect,
        shortcut.rect
    );
}

#[test]
fn message_menu_hotkey_hint_width_contributes_to_auto_width() {
    let policy = MessageMenuWidthPolicy::new(
        crate::gui::text_layout::TextWidthEstimate::new(8.0, 24.0),
        100.0,
        320.0,
    );
    let commands_without_hint = [MenuCommand::new("Open", MenuMessage::Open)];
    let commands_with_hint =
        [MenuCommand::new("Open", MenuMessage::Open).hotkey_hint("Command-Shift-O")];

    assert!(
        policy.width_for_title_and_commands("Actions", &commands_with_hint)
            > policy.width_for_title_and_commands("Actions", &commands_without_hint),
        "shortcut hints should reserve menu width"
    );
}

#[test]
fn dismissible_context_menu_with_width_policy_sizes_from_longest_label() {
    let policy = MessageMenuWidthPolicy::new(
        crate::gui::text_layout::TextWidthEstimate::new(8.0, 24.0),
        100.0,
        240.0,
    );
    let commands = [
        MenuCommand::new("Open", MenuMessage::Open),
        MenuCommand::new("Remove from collection", MenuMessage::Delete).danger(),
    ];
    let expected_width = policy.width_for_title_and_commands("Actions", &commands);
    let frame = UiSurface::new(
        dismissible_context_menu_with_width_policy(
            Point::new(80.0, 90.0),
            policy,
            "Actions",
            commands,
            MenuMessage::Close,
        )
        .into_node(),
    )
    .frame(
        Rect::from_min_size(Point::new(0.0, 0.0), Vector2::new(640.0, 360.0)),
        &Default::default(),
    );

    let menu_rects = frame
        .paint_plan
        .primitives
        .iter()
        .filter_map(|primitive| match primitive {
            PaintPrimitive::FillRect(fill) => Some(fill.rect),
            _ => None,
        })
        .collect::<Vec<_>>();

    assert!(
        menu_rects
            .iter()
            .any(|rect| (rect.width() - expected_width).abs() < 0.01),
        "context menu should paint at policy-derived width {expected_width}: {menu_rects:?}"
    );
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

#[test]
fn menu_command_style_helpers_are_generic() {
    let command = MenuCommand::new("Open", MenuMessage::Open).primary();
    assert_eq!(command.style.tone, WidgetTone::Accent);
}

fn painted_menu_rect(primitives: &[PaintPrimitive], size: Vector2) -> Rect {
    primitives
        .iter()
        .filter_map(|primitive| match primitive {
            PaintPrimitive::FillRect(fill) => Some(fill.rect),
            _ => None,
        })
        .find(|rect| (rect.width() - size.x).abs() < 0.01 && (rect.height() - size.y).abs() < 0.01)
        .expect("painted compact menu rect")
}
