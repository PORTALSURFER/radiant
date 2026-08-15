use crate::widgets::{RetainedSliderDomainWidget, RetainedSliderWidget};
use crate::{
    application::{
        MappedWidget, ViewNode, default_slider_sizing, primary_style, view_node_from_widget,
    },
    runtime::WidgetMessageMapper,
    widgets::{
        NumericAdjustment, SliderDomainError, SliderDomainMessage, SliderEditBatch, SliderMessage,
        SliderWidget, ValueFormat, WidgetProminence, WidgetSizing, WidgetStyle,
    },
};
use std::rc::Rc;

/// Builder for horizontal sliders that emit explicit host messages.
pub struct SliderBuilder {
    value: f32,
    style: Option<WidgetStyle>,
    sizing: Option<crate::layout::Vector2>,
    paints_focus: Option<bool>,
    track_height: Option<f32>,
    paints_track_border: bool,
    value_format: Option<ValueFormat>,
}

impl SliderBuilder {
    /// Apply an explicit widget style before binding this slider.
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

    /// Use compact toolbar-friendly slider sizing.
    pub fn compact(mut self) -> Self {
        self.sizing = Some(crate::layout::Vector2::new(92.0, 20.0));
        self
    }

    /// Control whether this slider paints focus affordances.
    pub fn paint_focus(mut self, paint: bool) -> Self {
        self.paints_focus = Some(paint);
        self
    }

    /// Use an explicit centered track height in logical pixels.
    pub fn track_height(mut self, height: f32) -> Self {
        self.track_height = Some(height);
        self
    }

    /// Paint a passive one-pixel outline around the track.
    pub fn track_border(mut self) -> Self {
        self.paints_track_border = true;
        self
    }

    /// Attach a display-only policy for the retained automation value text.
    pub fn format(mut self, format: ValueFormat) -> Self {
        self.value_format = Some(format);
        self
    }

    /// Emit a host message mapped from the normalized slider value.
    pub fn message<Message: 'static>(
        self,
        map: impl Fn(f32) -> Message + 'static,
    ) -> ViewNode<Message> {
        self.finish(WidgetMessageMapper::slider(move |message| match message {
            SliderMessage::ValueChanged { value } => map(value),
        }))
    }

    /// Emit a host message for the complete ordered slider edit lifecycle.
    pub fn on_edit<Message: 'static>(
        self,
        map: impl Fn(SliderEditBatch) -> Message + 'static,
    ) -> ViewNode<Message> {
        self.finish(WidgetMessageMapper::slider_edits(map))
    }

    fn slider_widget(&self, value: f32) -> SliderWidget {
        let mut slider = SliderWidget::new(
            0,
            value,
            self.sizing
                .map(WidgetSizing::fixed)
                .unwrap_or_else(default_slider_sizing),
        );
        if let Some(paint) = self.paints_focus {
            slider.common.paint.paints_focus = paint;
        }
        if let Some(track_height) = self.track_height {
            slider = slider.with_track_height(track_height);
        }
        slider.with_track_border(self.paints_track_border)
    }

    fn finish<Message: 'static>(self, messages: WidgetMessageMapper<Message>) -> ViewNode<Message> {
        let slider = self.slider_widget(self.value);
        let mut node = view_node_from_widget(MappedWidget::new(
            RetainedSliderWidget::new(slider).with_value_format(self.value_format),
            messages,
        ));
        node.style = self.style;
        node
    }
}

/// Builder for a horizontal slider with an application-owned `f32` domain.
pub struct SliderDomainBuilder<A> {
    normalized_value: f32,
    domain_value: f32,
    adjustment: Rc<A>,
    slider: SliderBuilder,
}

impl<A> SliderDomainBuilder<A> {
    /// Apply an explicit widget style before binding this slider.
    pub fn style(mut self, style: WidgetStyle) -> Self {
        self.slider = self.slider.style(style);
        self
    }

    /// Use the accent tone and strong prominence.
    pub fn primary(self) -> Self {
        self.style(primary_style())
    }

    /// Use a lower-prominence treatment.
    pub fn subtle(mut self) -> Self {
        self.slider = self.slider.subtle();
        self
    }

    /// Use compact toolbar-friendly slider sizing.
    pub fn compact(mut self) -> Self {
        self.slider = self.slider.compact();
        self
    }

    /// Control whether this slider paints focus affordances.
    pub fn paint_focus(mut self, paint: bool) -> Self {
        self.slider = self.slider.paint_focus(paint);
        self
    }

    /// Use an explicit centered track height in logical pixels.
    pub fn track_height(mut self, height: f32) -> Self {
        self.slider = self.slider.track_height(height);
        self
    }

    /// Paint a passive one-pixel outline around the track.
    pub fn track_border(mut self) -> Self {
        self.slider = self.slider.track_border();
        self
    }

    /// Attach a display-only policy for the mapped domain value text.
    pub fn format(mut self, format: ValueFormat) -> Self {
        self.slider = self.slider.format(format);
        self
    }

    /// Emit a host message for accepted domain changes or typed mapping
    /// failures.
    pub fn message<Message: 'static>(
        self,
        map: impl Fn(SliderDomainMessage<A::Error>) -> Message + 'static,
    ) -> ViewNode<Message>
    where
        A: NumericAdjustment<f32> + 'static,
        A::Error: Clone + 'static,
    {
        let Self {
            normalized_value,
            domain_value,
            adjustment,
            slider,
        } = self;
        let widget = RetainedSliderDomainWidget::new(
            slider.slider_widget(normalized_value),
            adjustment,
            domain_value,
        )
        .with_value_format(slider.value_format);
        let mut node =
            view_node_from_widget(MappedWidget::new(widget, WidgetMessageMapper::typed(map)));
        node.style = slider.style;
        node
    }
}

/// Build a horizontal normalized slider.
pub fn slider(value: f32) -> SliderBuilder {
    SliderBuilder {
        value,
        style: None,
        sizing: None,
        paints_focus: None,
        track_height: None,
        paints_track_border: false,
        value_format: None,
    }
}

/// Build a horizontal slider whose input values are mapped through an
/// application-owned `f32` adjustment.
pub fn slider_domain<A>(
    value: f32,
    adjustment: A,
) -> Result<SliderDomainBuilder<A>, SliderDomainError<A::Error>>
where
    A: NumericAdjustment<f32>,
{
    let normalized_value = crate::widgets::initial_normalized(value, &adjustment)?;
    Ok(SliderDomainBuilder {
        normalized_value,
        domain_value: value,
        adjustment: Rc::new(adjustment),
        slider: slider(normalized_value),
    })
}

/// Build a horizontal normalized slider that maps value changes.
pub fn slider_mapped<Message: 'static>(
    value: f32,
    map: impl Fn(f32) -> Message + 'static,
) -> ViewNode<Message> {
    slider(value).message(map)
}

/// Build a horizontal normalized slider that forwards complete edit batches.
pub fn slider_edit_mapped<Message: 'static>(
    value: f32,
    map: impl Fn(SliderEditBatch) -> Message + 'static,
) -> ViewNode<Message> {
    slider(value).on_edit(map)
}
