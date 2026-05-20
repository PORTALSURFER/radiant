use crate::{
    application::DEFAULT_STYLED_CONTAINER_PADDING,
    layout::{ContainerPolicy, CrossAlign, Insets, MainAlign},
    widgets::WidgetStyle,
};

#[cfg(test)]
#[path = "lowering_defaults/tests.rs"]
mod tests;

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
