use super::*;

#[test]
fn static_widget_helper_builds_non_emitting_leaf() {
    let title = TextWidget::new(
        30,
        "Status",
        WidgetSizing::fixed(Vector2::new(120.0, 20.0)).with_baseline(14.0),
    );
    let surface: UiSurface<DemoMessage> = UiSurface::new(SurfaceNode::static_widget(title));

    assert!(surface.find_widget(30).is_some());
    assert_eq!(
        surface.dispatch_widget_output(
            30,
            radiant::widgets::WidgetOutput::typed(ButtonMessage::Activate {
                provenance: InteractionProvenance::Programmatic,
            })
        ),
        None
    );
}

#[test]
fn text_and_button_helpers_build_common_leaf_nodes() {
    let surface: UiSurface<DemoMessage> = UiSurface::new(SurfaceNode::row(
        4,
        8.0,
        vec![
            SurfaceChild::fill(SurfaceNode::text(
                40,
                "Counter",
                WidgetSizing::fixed(Vector2::new(120.0, 20.0)).with_baseline(14.0),
            )),
            SurfaceChild::fill(SurfaceNode::button(
                41,
                "Increment",
                WidgetSizing::fixed(Vector2::new(96.0, 28.0)),
                DemoMessage::Increment,
            )),
            SurfaceChild::fill(SurfaceNode::button_mapped(
                42,
                "Rename",
                WidgetSizing::fixed(Vector2::new(96.0, 28.0)),
                |_| DemoMessage::Rename(String::from("Mapped")),
            )),
            SurfaceChild::fill(SurfaceNode::badge(
                43,
                "Active",
                WidgetSizing::fixed(Vector2::new(72.0, 24.0)),
                DemoMessage::SetActive(true),
            )),
            SurfaceChild::fill(SurfaceNode::badge_mapped(
                44,
                "Mapped badge",
                WidgetSizing::fixed(Vector2::new(112.0, 24.0)),
                |_| DemoMessage::Rename(String::from("Badge")),
            )),
        ],
    ));

    assert!(surface.find_widget(40).is_some());
    assert_eq!(
        surface.dispatch_widget_output(
            41,
            radiant::widgets::WidgetOutput::typed(ButtonMessage::Activate {
                provenance: InteractionProvenance::Programmatic,
            })
        ),
        Some(DemoMessage::Increment)
    );
    assert_eq!(
        surface.dispatch_widget_output(
            42,
            radiant::widgets::WidgetOutput::typed(ButtonMessage::Activate {
                provenance: InteractionProvenance::Programmatic,
            })
        ),
        Some(DemoMessage::Rename(String::from("Mapped")))
    );
    assert_eq!(
        surface.dispatch_widget_output(
            43,
            radiant::widgets::WidgetOutput::typed(BadgeMessage::Activate)
        ),
        Some(DemoMessage::SetActive(true))
    );
    assert_eq!(
        surface.dispatch_widget_output(
            44,
            radiant::widgets::WidgetOutput::typed(BadgeMessage::Activate)
        ),
        Some(DemoMessage::Rename(String::from("Badge")))
    );
}

#[test]
fn surface_button_mapped_forwards_complete_provenance_payload() {
    let surface: UiSurface<ButtonMessage> = UiSurface::new(SurfaceNode::button_mapped(
        45,
        "Mapped",
        WidgetSizing::fixed(Vector2::new(96.0, 28.0)),
        |message| message,
    ));
    let expected = ButtonMessage::ActivateWithModifiers {
        provenance: InteractionProvenance::Pointer {
            modifiers: radiant::widgets::PointerModifiers {
                command: true,
                shift: false,
                alt: true,
            },
            timestamp: None,
            sequence_range: None,
        },
    };

    assert_eq!(
        surface.dispatch_widget_output(45, radiant::widgets::WidgetOutput::typed(expected),),
        Some(expected)
    );
}
