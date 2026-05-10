//! Default control styles and interaction states.

use radiant::prelude::*;

#[derive(Clone, Debug, Default)]
struct StylingState {
    toggle_enabled: bool,
}

fn main() -> radiant::Result {
    radiant::app(StylingState::default())
        .title("Radiant Styling")
        .size(360, 220)
        .min_size(280, 180)
        .view(styling_view)
        .run()
}

fn styling_view(state: &mut StylingState) -> View<StateAction<StylingState>> {
    column([
        text("Button styles").size(160.0, 28.0),
        row([
            button("Default").on_click(|_: &mut StylingState| {}).id(10),
            button("Primary")
                .primary()
                .on_click(|_: &mut StylingState| {})
                .id(11),
            button("Danger")
                .danger()
                .on_click(|_: &mut StylingState| {})
                .id(12),
        ])
        .spacing(8.0),
        row([
            button("Subtle")
                .subtle()
                .on_click(|_: &mut StylingState| {})
                .id(13),
            toggle("Toggle", state.toggle_enabled)
                .on_change(|state: &mut StylingState, enabled| {
                    state.toggle_enabled = enabled;
                })
                .id(14),
        ])
        .spacing(8.0),
        row([
            text("Hoverable row").fill_width(),
            button("Open").on_click(|_: &mut StylingState| {}).id(15),
        ])
        .style(WidgetStyle::default())
        .hoverable()
        .padding(8.0)
        .spacing(8.0),
    ])
    .padding(16.0)
    .spacing(12.0)
}

#[cfg(test)]
mod tests {
    use super::*;
    use radiant::prelude::IntoView;
    use radiant::runtime::UiSurface;
    use radiant::widgets::{ToggleWidget, Widget};

    #[test]
    fn styling_example_projects_stateful_toggle() {
        let mut disabled = StylingState {
            toggle_enabled: false,
        };
        let mut enabled = StylingState {
            toggle_enabled: true,
        };

        let disabled_surface = styling_view(&mut disabled).into_surface();
        let enabled_surface = styling_view(&mut enabled).into_surface();
        let disabled_toggle =
            widget_ref::<ToggleWidget, _>(&disabled_surface, 14, "disabled toggle");
        let enabled_toggle = widget_ref::<ToggleWidget, _>(&enabled_surface, 14, "enabled toggle");

        assert!(!disabled_toggle.state.checked);
        assert!(!disabled_toggle.common.state.active);
        assert!(enabled_toggle.state.checked);
        assert!(enabled_toggle.common.state.active);
    }

    fn widget_ref<'a, WidgetType, Message>(
        surface: &'a UiSurface<Message>,
        widget_id: u64,
        label: &str,
    ) -> &'a WidgetType
    where
        WidgetType: Widget + 'static,
    {
        surface
            .find_widget(widget_id)
            .and_then(|widget| widget.widget_object().as_any().downcast_ref::<WidgetType>())
            .unwrap_or_else(|| panic!("{label} should be projected"))
    }
}
