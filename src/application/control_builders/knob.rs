use crate::{
    application::{MappedWidget, ViewNode, primary_style, view_node_from_widget},
    runtime::WidgetMessageMapper,
    widgets::{
        KnobEditBatch, KnobMessage, KnobWidget, RetainedKnobWidget, ValueFormat, WidgetProminence,
        WidgetSizing, WidgetStyle,
    },
};

/// Builder for radial knobs with explicit automation lifecycle mapping.
pub struct KnobBuilder {
    value: f32,
    default_value: Option<f32>,
    sensitivity: Option<f32>,
    style: Option<WidgetStyle>,
    sizing: Option<crate::layout::Vector2>,
    enabled: bool,
    automation_active: bool,
    value_format: Option<ValueFormat>,
}

impl KnobBuilder {
    /// Apply an explicit widget style.
    pub fn style(mut self, style: WidgetStyle) -> Self {
        self.style = Some(style);
        self
    }

    /// Use the accent tone and strong prominence.
    pub fn primary(self) -> Self {
        self.style(primary_style())
    }

    /// Use a lower-prominence treatment.
    pub fn subtle(mut self) -> Self {
        let mut style = self.style.unwrap_or_default();
        style.prominence = WidgetProminence::Subtle;
        self.style = Some(style);
        self
    }

    /// Set the reset/default value.
    pub fn default_value(mut self, value: f32) -> Self {
        self.default_value = Some(value);
        self
    }

    /// Set vertical pointer sensitivity.
    pub fn sensitivity(mut self, sensitivity: f32) -> Self {
        self.sensitivity = Some(sensitivity);
        self
    }

    /// Set a compact fixed diameter.
    pub fn diameter(mut self, diameter: f32) -> Self {
        self.sizing = Some(crate::layout::Vector2::new(diameter, diameter));
        self
    }

    /// Enable or disable interaction.
    pub fn enabled(mut self, enabled: bool) -> Self {
        self.enabled = enabled;
        self
    }

    /// Paint the host-automation cue on the control.
    pub fn automation_active(mut self, active: bool) -> Self {
        self.automation_active = active;
        self
    }

    /// Attach a display-only policy for the retained automation value text.
    pub fn format(mut self, format: ValueFormat) -> Self {
        self.value_format = Some(format);
        self
    }

    /// Map explicit gesture lifecycle outputs into host messages.
    pub fn message<Message: 'static>(
        self,
        map: impl Fn(KnobMessage) -> Message + 'static,
    ) -> ViewNode<Message> {
        self.finish(WidgetMessageMapper::knob(map))
    }

    /// Emit a host message for the complete ordered knob edit lifecycle.
    pub fn on_edit<Message: 'static>(
        self,
        map: impl Fn(KnobEditBatch) -> Message + 'static,
    ) -> ViewNode<Message> {
        self.finish(WidgetMessageMapper::knob_edits(map))
    }

    fn finish<Message: 'static>(self, messages: WidgetMessageMapper<Message>) -> ViewNode<Message> {
        let mut knob = KnobWidget::new(0, self.value);
        if let Some(default_value) = self.default_value {
            knob = knob.with_default_value(default_value);
        }
        if let Some(sensitivity) = self.sensitivity {
            knob = knob.with_sensitivity(sensitivity);
        }
        if let Some(size) = self.sizing {
            knob.common.sizing = WidgetSizing::fixed(size);
        }
        knob.common.state.disabled = !self.enabled;
        knob.common.state.automation_active = self.automation_active;
        let mut node = view_node_from_widget(MappedWidget::new(
            RetainedKnobWidget::new(knob).with_value_format(self.value_format),
            messages,
        ));
        node.style = self.style;
        node
    }
}

/// Build a radial knob at a normalized value.
pub fn knob(value: f32) -> KnobBuilder {
    KnobBuilder {
        value,
        default_value: None,
        sensitivity: None,
        style: None,
        sizing: None,
        enabled: true,
        automation_active: false,
        value_format: None,
    }
}

/// Build a radial knob that maps all gesture lifecycle outputs.
pub fn knob_mapped<Message: 'static>(
    value: f32,
    map: impl Fn(KnobMessage) -> Message + 'static,
) -> ViewNode<Message> {
    knob(value).message(map)
}

/// Build a radial knob that forwards complete edit batches.
pub fn knob_edit_mapped<Message: 'static>(
    value: f32,
    map: impl Fn(KnobEditBatch) -> Message + 'static,
) -> ViewNode<Message> {
    knob(value).on_edit(map)
}
