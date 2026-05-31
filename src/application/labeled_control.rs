use crate::{
    application::{ViewNode, column, text},
    widgets::{WidgetProminence, WidgetStyle, WidgetTone},
};

const DEFAULT_LABELED_CONTROL_LABEL_HEIGHT: f32 = 18.0;
const DEFAULT_LABELED_CONTROL_SPACING: f32 = 3.0;

/// Named construction fields for a label stacked above one control.
pub struct LabeledControlParts<Message> {
    /// Label shown above the control.
    pub label: String,
    /// Control or composite content shown below the label.
    pub control: ViewNode<Message>,
    /// Optional fixed height for the combined label and control stack.
    pub height: Option<f32>,
    /// Visual styling applied to the label.
    pub label_style: WidgetStyle,
    /// Fixed label row height.
    pub label_height: f32,
    /// Vertical spacing between the label and control.
    pub spacing: f32,
}

impl<Message> LabeledControlParts<Message> {
    /// Build labeled-control parts with compact control-panel defaults.
    pub fn new(label: impl Into<String>, control: ViewNode<Message>) -> Self {
        Self {
            label: label.into(),
            control,
            height: None,
            label_style: WidgetStyle {
                tone: WidgetTone::Accent,
                prominence: WidgetProminence::Subtle,
            },
            label_height: DEFAULT_LABELED_CONTROL_LABEL_HEIGHT,
            spacing: DEFAULT_LABELED_CONTROL_SPACING,
        }
    }

    /// Set fixed combined height for the label and control.
    pub fn height(mut self, height: f32) -> Self {
        self.height = Some(height);
        self
    }

    /// Override label style.
    pub fn label_style(mut self, style: WidgetStyle) -> Self {
        self.label_style = style;
        self
    }

    /// Override fixed label row height.
    pub fn label_height(mut self, height: f32) -> Self {
        self.label_height = height;
        self
    }

    /// Override vertical spacing between label and control.
    pub fn spacing(mut self, spacing: f32) -> Self {
        self.spacing = spacing;
        self
    }
}

/// Build a compact label-over-control group with a fixed combined height.
pub fn labeled_control<Message: 'static>(
    label: impl Into<String>,
    control: ViewNode<Message>,
    height: f32,
) -> ViewNode<Message> {
    labeled_control_from_parts(LabeledControlParts::new(label, control).height(height))
}

/// Build a compact label-over-control group from named parts.
pub fn labeled_control_from_parts<Message: 'static>(
    parts: LabeledControlParts<Message>,
) -> ViewNode<Message> {
    let mut view = column([
        text(parts.label)
            .style(parts.label_style)
            .fill_width()
            .height(parts.label_height),
        parts.control,
    ])
    .spacing(parts.spacing)
    .fill_width();
    if let Some(height) = parts.height {
        view = view.height(height);
    }
    view
}
