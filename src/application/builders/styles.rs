use crate::widgets::{WidgetProminence, WidgetStyle, WidgetTone};

pub(in crate::application) fn primary_style() -> WidgetStyle {
    WidgetStyle::new(WidgetTone::Accent, WidgetProminence::Strong)
}

pub(in crate::application) fn danger_style() -> WidgetStyle {
    WidgetStyle::new(WidgetTone::Danger, WidgetProminence::Normal)
}
