use super::*;

#[test]
fn text_input_and_toggle_helpers_map_value_messages() {
    let surface: UiSurface<DemoMessage> = UiSurface::new(SurfaceNode::row(
        5,
        8.0,
        vec![
            SurfaceChild::fill(SurfaceNode::text_input(
                50,
                "Draft",
                WidgetSizing::fixed(Vector2::new(140.0, 28.0)),
                DemoMessage::Rename,
            )),
            SurfaceChild::fill(SurfaceNode::text_input_mapped(
                51,
                "Raw",
                WidgetSizing::fixed(Vector2::new(140.0, 28.0)),
                |message| match message {
                    TextInputMessage::Changed { value } | TextInputMessage::Submitted { value } => {
                        DemoMessage::Rename(format!("raw:{value}"))
                    }
                },
            )),
            SurfaceChild::fill(SurfaceNode::toggle(
                52,
                "Enabled",
                WidgetSizing::fixed(Vector2::new(96.0, 28.0)),
                DemoMessage::SetActive,
            )),
            SurfaceChild::fill(SurfaceNode::toggle_mapped(
                53,
                "Raw toggle",
                WidgetSizing::fixed(Vector2::new(112.0, 28.0)),
                |message| match message {
                    ToggleMessage::ValueChanged { checked } => DemoMessage::SetActive(!checked),
                },
            )),
            SurfaceChild::fill(SurfaceNode::toggle_with_checked(
                54,
                "Done",
                true,
                WidgetSizing::fixed(Vector2::new(96.0, 28.0)),
                DemoMessage::SetActive,
            )),
            SurfaceChild::fill(SurfaceNode::toggle_mapped_with_checked(
                55,
                "Raw done",
                true,
                WidgetSizing::fixed(Vector2::new(112.0, 28.0)),
                |message| match message {
                    ToggleMessage::ValueChanged { checked } => DemoMessage::SetActive(!checked),
                },
            )),
        ],
    ));

    assert_eq!(
        surface.dispatch_widget_output(
            50,
            radiant::widgets::WidgetOutput::typed(TextInputMessage::Changed {
                value: String::from("Edited"),
            })
        ),
        Some(DemoMessage::Rename(String::from("Edited")))
    );
    assert_eq!(
        surface.dispatch_widget_output(
            51,
            radiant::widgets::WidgetOutput::typed(TextInputMessage::Submitted {
                value: String::from("Submitted"),
            })
        ),
        Some(DemoMessage::Rename(String::from("raw:Submitted")))
    );
    assert_eq!(
        surface.dispatch_widget_output(
            52,
            radiant::widgets::WidgetOutput::typed(ToggleMessage::ValueChanged { checked: true })
        ),
        Some(DemoMessage::SetActive(true))
    );
    assert_eq!(
        surface.dispatch_widget_output(
            53,
            radiant::widgets::WidgetOutput::typed(ToggleMessage::ValueChanged { checked: true })
        ),
        Some(DemoMessage::SetActive(false))
    );
    assert_eq!(
        surface
            .find_widget(54)
            .map(|widget| widget.widget().common().state.active),
        Some(true)
    );
    assert_eq!(
        surface
            .find_widget(55)
            .map(|widget| widget.widget().common().state.active),
        Some(true)
    );
}
