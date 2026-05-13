use crate::widgets::{WidgetProminence, WidgetStyle, WidgetTone};

pub(in crate::application) fn primary_style() -> WidgetStyle {
    WidgetStyle {
        tone: WidgetTone::Accent,
        prominence: WidgetProminence::Strong,
    }
}

pub(in crate::application) fn danger_style() -> WidgetStyle {
    WidgetStyle {
        tone: WidgetTone::Danger,
        prominence: WidgetProminence::Normal,
    }
}
