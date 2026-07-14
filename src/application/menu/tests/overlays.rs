use super::{MenuMessage, click, painted_menu_rect};
use crate::{
    application::{
        IntoView, MenuCommand, MessageMenuWidthPolicy, context_menu, message_menu_height,
    },
    gui::{
        text_layout::TextWidthEstimate,
        types::{Point, Rect},
    },
    layout::Vector2,
    runtime::{DeclarativeOwnedRuntimeBridge, PaintPrimitive, SurfaceRuntime, UiSurface},
};

#[test]
fn dismissible_context_menu_backing_emits_dismiss_message() {
    let bridge = DeclarativeOwnedRuntimeBridge::new(
        Vec::<MenuMessage>::new(),
        |_| {
            UiSurface::new(
                context_menu("Actions", [MenuCommand::new("Open", MenuMessage::Open)])
                    .anchor(Point::new(80.0, 90.0))
                    .size(Vector2::new(200.0, 96.0))
                    .dismiss_on(MenuMessage::Close)
                    .view()
                    .into_node(),
            )
        },
        |messages: &mut Vec<MenuMessage>, message| messages.push(message),
    );
    let mut runtime = SurfaceRuntime::new(bridge, Vector2::new(640.0, 360.0));

    click(&mut runtime, Point::new(12.0, 12.0));

    assert_eq!(runtime.bridge().state(), &[MenuMessage::Close]);
}

#[test]
fn dismissible_context_menu_with_width_uses_standard_menu_height() {
    let frame = UiSurface::new(
        context_menu(
            "Actions",
            [
                MenuCommand::new("Open", MenuMessage::Open),
                MenuCommand::new("Delete", MenuMessage::Delete).danger(),
            ],
        )
        .anchor(Point::new(80.0, 90.0))
        .width(200.0)
        .dismiss_on(MenuMessage::Close)
        .view()
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

    assert!(menu_rects.iter().any(|rect| {
        rect.min.x >= 80.0 && (rect.height() - message_menu_height(2)).abs() < 0.01
    }));
}

#[test]
fn message_context_menu_overlay_opens_down_when_space_below() {
    let size = Vector2::new(200.0, message_menu_height(2));
    let frame = menu_overlay_frame(Point::new(80.0, 90.0), size);

    assert_eq!(
        painted_menu_rect(&frame.paint_plan.primitives, size).min,
        Point::new(80.0, 90.0)
    );
}

#[test]
fn message_context_menu_overlay_flips_up_when_bottom_would_clip() {
    let size = Vector2::new(200.0, message_menu_height(2));
    let frame = menu_overlay_frame(Point::new(80.0, 330.0), size);
    let menu_rect = painted_menu_rect(&frame.paint_plan.primitives, size);

    assert_eq!(menu_rect.min, Point::new(80.0, 226.0));
    assert_eq!(menu_rect.max.y, 330.0);
}

#[test]
fn flipped_message_context_menu_routes_clicks_to_painted_items() {
    let size = Vector2::new(200.0, message_menu_height(2));
    let bridge = DeclarativeOwnedRuntimeBridge::new(
        Vec::<MenuMessage>::new(),
        move |_| UiSurface::new(menu_overlay(Point::new(80.0, 330.0), size).into_node()),
        |messages: &mut Vec<MenuMessage>, message| messages.push(message),
    );
    let mut runtime = SurfaceRuntime::new(bridge, Vector2::new(640.0, 360.0));

    click(&mut runtime, Point::new(100.0, 306.0));

    assert_eq!(runtime.bridge().state(), &[MenuMessage::Delete]);
}

#[test]
fn dismissible_context_menu_with_width_policy_sizes_from_longest_label() {
    let policy = MessageMenuWidthPolicy::new(TextWidthEstimate::new(8.0, 24.0), 100.0, 240.0);
    let commands = [
        MenuCommand::new("Open", MenuMessage::Open),
        MenuCommand::new("Remove from collection", MenuMessage::Delete).danger(),
    ];
    let expected_width = policy.width_for_title_and_commands("Actions", &commands);
    let frame = UiSurface::new(
        context_menu("Actions", commands)
            .anchor(Point::new(80.0, 90.0))
            .width_policy(policy)
            .dismiss_on(MenuMessage::Close)
            .view()
            .into_node(),
    )
    .frame(
        Rect::from_min_size(Point::new(0.0, 0.0), Vector2::new(640.0, 360.0)),
        &Default::default(),
    );

    assert!(frame.paint_plan.primitives.iter().any(|primitive| {
        matches!(primitive, PaintPrimitive::FillRect(fill) if (fill.rect.width() - expected_width).abs() < 0.01)
    }));
}

fn menu_overlay_frame(anchor: Point, size: Vector2) -> crate::runtime::SurfaceFrame {
    UiSurface::new(menu_overlay(anchor, size).into_node()).frame(
        Rect::from_min_size(Point::new(0.0, 0.0), Vector2::new(640.0, 360.0)),
        &Default::default(),
    )
}

fn menu_overlay(anchor: Point, size: Vector2) -> crate::application::ViewNode<MenuMessage> {
    context_menu(
        "Actions",
        [
            MenuCommand::new("Open", MenuMessage::Open),
            MenuCommand::new("Delete", MenuMessage::Delete).danger(),
        ],
    )
    .anchor(anchor)
    .size(size)
    .view()
}
