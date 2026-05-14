use crate::{
    application::DEFAULT_STYLED_CONTAINER_PADDING,
    layout::{ContainerPolicy, CrossAlign, Insets, MainAlign},
    widgets::WidgetStyle,
};

pub(in crate::application) struct ViewNodeContainerDefaults {
    padding: Insets,
    align_main: MainAlign,
    align_cross: CrossAlign,
}

impl ViewNodeContainerDefaults {
    pub(in crate::application) fn new(
        padding: Option<Insets>,
        align_main: Option<MainAlign>,
        align_cross: Option<CrossAlign>,
        style: Option<WidgetStyle>,
    ) -> Self {
        Self {
            padding: padding.unwrap_or_else(|| default_container_padding(style)),
            align_main: align_main.unwrap_or(MainAlign::Start),
            align_cross: align_cross.unwrap_or(CrossAlign::Stretch),
        }
    }

    pub(in crate::application) fn base_policy(&self) -> ContainerPolicy {
        ContainerPolicy {
            padding: self.padding,
            align_main: self.align_main,
            align_cross: self.align_cross,
            ..ContainerPolicy::default()
        }
    }
}

fn default_container_padding(style: Option<WidgetStyle>) -> Insets {
    if style.is_some() {
        Insets::all(DEFAULT_STYLED_CONTAINER_PADDING)
    } else {
        Insets::default()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn styled_container_defaults_to_panel_padding() {
        let defaults =
            ViewNodeContainerDefaults::new(None, None, None, Some(WidgetStyle::default()));
        let policy = defaults.base_policy();

        assert_eq!(
            policy.padding,
            Insets::all(DEFAULT_STYLED_CONTAINER_PADDING)
        );
        assert_eq!(policy.align_main, MainAlign::Start);
        assert_eq!(policy.align_cross, CrossAlign::Stretch);
    }

    #[test]
    fn explicit_container_defaults_override_style_padding_and_alignment() {
        let defaults = ViewNodeContainerDefaults::new(
            Some(Insets::all(3.0)),
            Some(MainAlign::Center),
            Some(CrossAlign::End),
            Some(WidgetStyle::default()),
        );
        let policy = defaults.base_policy();

        assert_eq!(policy.padding, Insets::all(3.0));
        assert_eq!(policy.align_main, MainAlign::Center);
        assert_eq!(policy.align_cross, CrossAlign::End);
    }
}
